"""Tcl Language Server -- pygls-based LSP implementation."""

from __future__ import annotations

import asyncio
import logging
import re
import threading
import time
from dataclasses import dataclass, field

from lsprotocol import types
from pygls.lsp.server import LanguageServer

import core.common.codes_all  # noqa: F401  # trigger all code registrations
from core.analysis.analyser import analyse
from core.analysis.irules_checks import DEFAULT_GENERIC_VARIABLE_PATTERNS
from core.commands.registry import REGISTRY
from core.commands.registry.info import effective_event_requires
from core.commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from core.commands.registry.runtime import configure_signatures, is_irules_dialect
from core.common.codes import diagnostic_codes, optimisation_codes
from core.common.document_buffer import DocumentBuffer
from core.common.lsp import to_lsp_location
from core.common.user_config import (
    get_all_settings,
    load_user_config,
    save_settings_to_config,
)
from core.compiler.optimiser import optimise_source
from core.formatting import FormatterConfig
from core.minifier import minify_tcl
from core.packages import PackageResolver
from explorer.pipeline import run_pipeline as explorer_run_pipeline
from explorer.serialise import serialise_result as explorer_serialise_result

from .async_diagnostics import DiagnosticScheduler
from .features.call_hierarchy import (
    incoming_calls as get_incoming_calls,
)
from .features.call_hierarchy import (
    outgoing_calls as get_outgoing_calls,
)
from .features.call_hierarchy import (
    prepare_call_hierarchy as get_call_hierarchy,
)
from .features.code_actions import get_code_actions
from .features.completion import get_completions
from .features.definition import get_bigip_definition, get_definition
from .features.diagnostics import get_basic_diagnostics, get_deep_diagnostics, get_diagnostics
from .features.document_links import get_document_links
from .features.document_symbols import get_document_symbols
from .features.folding import get_folding_ranges
from .features.formatting import get_formatting, get_range_formatting
from .features.hover import get_hover
from .features.inlay_hints import get_inlay_hints
from .features.package_suggestions import rank_package_suggestions
from .features.references import get_references
from .features.rename import get_rename_edits, prepare_rename
from .features.selection_range import get_selection_ranges
from .features.semantic_tokens import (
    SEMANTIC_TOKEN_MODIFIERS,
    SEMANTIC_TOKEN_TYPES,
    compute_semantic_tokens_edits,
    semantic_tokens_full,
)
from .features.signature_help import get_signature_help
from .features.symbol_resolution import find_word_at_position
from .features.workspace_symbols import get_workspace_symbols
from .workspace.document_state import WorkspaceState
from .workspace.scanner import BackgroundScanner, path_to_uri, uri_to_path
from .workspace.workspace_index import EntrySource, WorkspaceIndex

log = logging.getLogger(__name__)


@dataclass
class FeatureConfig:
    """Runtime feature flags and diagnostic/optimiser filter state."""

    # Feature-level toggles
    hover_enabled: bool = True
    completion_enabled: bool = True
    diagnostics_enabled: bool = True
    formatting_enabled: bool = True
    semantic_tokens_enabled: bool = True
    code_actions_enabled: bool = True
    definition_enabled: bool = True
    references_enabled: bool = True
    document_symbols_enabled: bool = True
    folding_enabled: bool = True
    rename_enabled: bool = True
    signature_help_enabled: bool = True
    workspace_symbols_enabled: bool = True
    inlay_hints_enabled: bool = False
    call_hierarchy_enabled: bool = True
    document_links_enabled: bool = True
    selection_range_enabled: bool = True

    # Per-code diagnostic filters -- codes present here are *disabled*.
    disabled_diagnostics: set[str] = field(default_factory=set)

    # Optimiser master switch + per-code filters.
    optimiser_enabled: bool = True
    disabled_optimisations: set[str] = field(default_factory=set)

    # Shimmer detection master switch.
    shimmer_enabled: bool = True

    # XC translatability diagnostics (opt-in, for migration planning).
    xc_diagnostics_enabled: bool = False

    # Style: maximum line length for W111.
    line_length: int = 120

    # IRULE4002: regex patterns matching generic static:: / global variable
    # bare names (after stripping the ``static::`` prefix).  Empty list
    # disables the check.  Patterns are matched case-insensitively against
    # the full bare name.
    generic_variable_patterns: list[str] = field(
        default_factory=lambda: list(DEFAULT_GENERIC_VARIABLE_PATTERNS)
    )

    # True once the user explicitly sets ``tclLsp.dialect`` in settings.
    # When False, the server may auto-detect the dialect from the editor's
    # ``language_id``.
    dialect_explicitly_set: bool = False


try:
    from ._build_info import FULL_VERSION as _version
except ImportError:
    _version = "dev"

server = LanguageServer("tcl-lsp", f"v{_version}")


# Logging bridge: forward Python log records to the LSP client as
# ``window/logMessage`` so they appear in Zed's language-server log panel
# and VS Code's Output → Tcl LSP channel.


class _LspLogHandler(logging.Handler):
    """Logging handler that forwards records via ``window/logMessage``.

    A thread-local re-entrancy guard prevents infinite loops: pygls itself
    logs when sending ``window/logMessage``, which would trigger this handler
    again.  We also skip pygls internal loggers entirely to avoid the
    feedback path.

    When called from a background thread we marshal the send onto the
    server's asyncio event-loop via ``loop.call_soon_threadsafe`` so that
    pygls I/O is never performed off the event-loop thread.
    """

    _LEVEL_MAP = {
        logging.DEBUG: types.MessageType.Log,
        logging.INFO: types.MessageType.Info,
        logging.WARNING: types.MessageType.Warning,
        logging.ERROR: types.MessageType.Error,
        logging.CRITICAL: types.MessageType.Error,
    }

    _SKIP_LOGGERS = frozenset(
        {"pygls", "pygls.protocol", "pygls.server", "pygls.feature_manager", "pygls.client"}
    )

    def __init__(self) -> None:
        super().__init__()
        # Thread-local re-entrancy guard — each thread gets its own flag.
        self._local = threading.local()

    def _send_to_client(self, record: logging.LogRecord) -> None:
        """Send a log record to the LSP client (must run on the event-loop thread)."""
        if getattr(self._local, "emitting", False):
            return
        self._local.emitting = True
        try:
            msg_type = self._LEVEL_MAP.get(record.levelno, types.MessageType.Log)
            server.window_log_message(
                types.LogMessageParams(type=msg_type, message=self.format(record))
            )
        except Exception:
            # Server not yet initialised or already shut down — swallow.
            pass
        finally:
            self._local.emitting = False

    def emit(self, record: logging.LogRecord) -> None:
        # Skip pygls internals to avoid feedback paths.
        if record.name in self._SKIP_LOGGERS or record.name.startswith("pygls."):
            return
        # Marshal the send onto the server's event loop when possible so
        # that pygls I/O is never performed from background threads.
        loop = getattr(server, "loop", None)
        try:
            if loop is not None and loop.is_running():
                loop.call_soon_threadsafe(self._send_to_client, record)
            else:
                # Early startup / shutdown — send directly.
                self._send_to_client(record)
        except Exception:
            pass


def _install_lsp_log_handler() -> None:
    """Attach the LSP handler to our own loggers (not the root logger).

    Attaching to the root logger caused a feedback loop: every pygls
    internal debug message triggered another ``window/logMessage`` send
    which logged again.  Instead we attach only to the ``lsp`` and
    ``core`` loggers that carry our application messages.
    """
    handler = _LspLogHandler()
    handler.setLevel(logging.DEBUG)
    handler.setFormatter(logging.Formatter("%(name)s: %(message)s"))
    for name in ("lsp", "core"):
        lgr = logging.getLogger(name)
        lgr.addHandler(handler)
        # Ensure messages at DEBUG and above reach the handler.
        # A logger's default level is NOTSET (0) which inherits from
        # the root logger (typically WARNING), so we must set it
        # explicitly.
        if lgr.getEffectiveLevel() > logging.DEBUG:
            lgr.setLevel(logging.DEBUG)


_install_lsp_log_handler()


# Silence benign "Cancel notification for unknown message id" warnings.
# Read-only feature handlers complete synchronously, but async handlers
# (did_open, did_change) may schedule background tasks.  The default
# pygls handler logs at WARNING level which VS Code surfaces as [error].
def _quiet_handle_cancel(msg_id: str | int) -> None:
    future = server.protocol._request_futures.pop(msg_id, None)
    if future is None:
        log.debug('Cancel notification for already-completed message id "%s"', msg_id)
        return
    if future.cancel():
        log.info('Cancelled request with id "%s"', msg_id)


server.protocol._handle_cancel_notification = _quiet_handle_cancel  # type: ignore[invalid-assignment]


# Request / notification logging — wrap pygls dispatch so every incoming
# message is logged to the client channel.

_orig_handle_request = server.protocol._handle_request
_orig_handle_notification = server.protocol._handle_notification

# Noisy methods that fire on every keystroke or cursor move — log at DEBUG.
_FREQUENT_METHODS = frozenset(
    {
        "textDocument/semanticTokens/full",
        "textDocument/completion",
        "textDocument/hover",
        "textDocument/signatureHelp",
        "textDocument/documentSymbol",
        "textDocument/foldingRange",
        "textDocument/selectionRange",
        "textDocument/inlayHint",
        "textDocument/didChange",
        "$/cancelRequest",
    }
)


def _log_request(msg_id, method: str, params):  # type: ignore[override]
    level = logging.DEBUG if method in _FREQUENT_METHODS else logging.INFO
    log.log(level, "<-- request  %s (id=%s)", method, msg_id)
    return _orig_handle_request(msg_id, method, params)


def _log_notification(method: str, params):  # type: ignore[override]
    level = logging.DEBUG if method in _FREQUENT_METHODS else logging.INFO
    log.log(level, "<-- notify   %s", method)
    return _orig_handle_notification(method, params)


server.protocol._handle_request = _log_request  # type: ignore[assignment]
server.protocol._handle_notification = _log_notification  # type: ignore[assignment]


workspace_state = WorkspaceState()
workspace_index = WorkspaceIndex()
background_scanner = BackgroundScanner()
package_resolver = PackageResolver()
formatter_config = FormatterConfig()
feature_config = FeatureConfig()
diagnostic_scheduler = DiagnosticScheduler()
_loaded_packages: set[str] = set()
_SAFE_FIX_CODES = frozenset(
    {
        "W100",
        "W105",
        "W108",
        "W110",
        "W201",
        "W304",
        "IRULE2001",
    }
)


def _get_doc_source(uri: str) -> str:
    """Get document source text, handling virtual documents without backing files.

    Prefers the in-memory ``DocumentState`` source (always available for
    documents opened via ``textDocument/didOpen``).  Falls back to the pygls
    ``TextDocument`` which may read from disk.  Returns an empty string for
    virtual or untitled documents that have no backing file.
    """
    state = workspace_state.get(uri)
    if state is not None:
        return state.source
    doc = server.workspace.get_text_document(uri)
    try:
        return doc.source
    except (FileNotFoundError, OSError):
        return ""


def _camel_to_snake(name: str) -> str:
    """Convert lowerCamelCase/PascalCase names to snake_case."""
    first = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", first).lower()


def _normalise_formatter_settings(raw: dict) -> dict:
    """Map client formatter settings to FormatterConfig field names."""
    normalised: dict[str, object] = {}
    for key, value in raw.items():
        if not isinstance(key, str):
            continue
        field = _camel_to_snake(key)
        if field == "line_ending" and isinstance(value, str):
            mapping = {
                "lf": "\n",
                "crlf": "\r\n",
                "cr": "\r",
            }
            value = mapping.get(value.lower(), value)
        normalised[field] = value
    return normalised


# Keys that appear directly inside the ``tclLsp`` configuration namespace.
# Used both for flat-key routing and for detecting unwrapped payloads from
# clients that strip the ``tclLsp`` prefix (e.g. JetBrains, vscode-languageclient v10).
_KNOWN_TCL_LSP_SECTIONS = frozenset(
    {
        "formatting",
        "diagnostics",
        "optimiser",
        "shimmer",
        "features",
        "style",
        "xcDiagnostics",
        "runtimeValidation",
        "ai",
    }
)
_KNOWN_TCL_LSP_TOPLEVEL = frozenset(
    {
        "dialect",
        "extraCommands",
        "libraryPaths",
    }
)


def _extract_tcl_lsp_settings(settings: dict) -> dict:
    """Extract extension/server settings from multiple client payload shapes.

    Handles three payload formats:
    1. Nested:   ``{"tclLsp": {"optimiser": {"O109": false}}}``
    2. Flat:     ``{"tclLsp.optimiser.O109": false}``
    3. Unwrapped (no ``tclLsp`` prefix — e.g. JetBrains pull-model response):
       ``{"optimiser": {"O109": false}, "dialect": "tcl8.6"}``
    """
    extracted: dict[str, object] = {}

    nested = settings.get("tclLsp")
    if isinstance(nested, dict):
        extracted.update(nested)

    for key, value in settings.items():
        if not isinstance(key, str):
            continue
        if key.startswith("tclLsp."):
            subkey = key[len("tclLsp.") :]
        else:
            continue

        # Route dotted subkeys into nested dicts for known sections.
        section_handled = False
        for section in _KNOWN_TCL_LSP_SECTIONS:
            prefix = section + "."
            if subkey.startswith(prefix):
                section_key = subkey[len(prefix) :]
                current = extracted.get(section)
                if not isinstance(current, dict):
                    current = {}
                    extracted[section] = current
                current[section_key] = value
                section_handled = True
                break
        if not section_handled:
            extracted[subkey] = value

    # Fallback: detect unwrapped payloads from clients that already stripped
    # the ``tclLsp`` prefix (e.g. workspace/configuration pull responses or
    # JetBrains didChangeConfiguration notifications).
    if not extracted:
        if any(
            isinstance(k, str) and k in _KNOWN_TCL_LSP_SECTIONS | _KNOWN_TCL_LSP_TOPLEVEL
            for k in settings
        ):
            extracted.update(settings)

    return extracted


# Capabilities

_TCL_LANGUAGE_IDS = (
    # VS Code language IDs
    "tcl",
    "tcl-irule",
    "tcl-iapp",
    "tcl-apl",
    "tcl-bigip",
    "tcl8.4",
    "tcl8.5",
    "tcl9.0",
    "tcl-synopsys",
    "tcl-cadence",
    "tcl-xilinx",
    "tcl-quartus",
    "tcl-mentor",
    "tcl-expect",
    # Zed language names (used as language IDs)
    "Tcl",
    "iRules",
    "iApps",
    "APL",
)
_TCL_DOCUMENT_SELECTOR = [
    types.TextDocumentFilterLanguage(language=lang) for lang in _TCL_LANGUAGE_IDS
]

SEMANTIC_TOKENS_LEGEND = types.SemanticTokensLegend(
    token_types=SEMANTIC_TOKEN_TYPES,
    token_modifiers=SEMANTIC_TOKEN_MODIFIERS,
)

# Per-URI storage for semantic tokens delta support (P8).
# Maps URI → (result_id, flat token data).
_semantic_token_results: dict[str, tuple[str, list[int]]] = {}
_semantic_token_result_counter: int = 0


@server.feature(
    types.TEXT_DOCUMENT_SEMANTIC_TOKENS_FULL,
    types.SemanticTokensRegistrationOptions(
        legend=SEMANTIC_TOKENS_LEGEND,
        full=types.SemanticTokensFullDelta(delta=True),
        document_selector=_TCL_DOCUMENT_SELECTOR,
    ),
)
def on_semantic_tokens_full(
    params: types.SemanticTokensParams,
) -> types.SemanticTokens:
    global _semantic_token_result_counter
    t0 = time.perf_counter()
    if not feature_config.semantic_tokens_enabled:
        return types.SemanticTokens(data=[])
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    # Try to use per-chunk semantic token cache.
    chunk_token_cache = None
    chunk_line_ranges = None
    cache_status = "no_state"
    if state is not None:
        cache_info = state.get_semantic_token_cache()
        if cache_info is not None:
            chunk_token_cache, chunk_line_ranges = cache_info
            cache_status = (
                "full_hit" if all(e is not None for e in chunk_token_cache) else "partial"
            )
        else:
            cache_status = "no_cache"
    data = semantic_tokens_full(
        source,
        analysis=analysis,
        is_bigip_conf=_is_bigip_conf(uri),
        is_irules=_is_irules_source(uri),
        is_apl=_is_apl_source(uri),
        chunk_token_cache=chunk_token_cache,
        chunk_line_ranges=chunk_line_ranges,
    )
    # Write back computed tokens to chunk cache.
    if state is not None and chunk_token_cache is not None:
        state.store_semantic_token_cache(chunk_token_cache)
    # Store result for delta support (P8).
    _semantic_token_result_counter += 1
    result_id = str(_semantic_token_result_counter)
    _semantic_token_results[uri] = (result_id, data)
    elapsed_ms = (time.perf_counter() - t0) * 1000
    log.info(
        "[timing] semanticTokens/full %.0fms (cache=%s, analysis=%s, tokens=%d, lines=%d)",
        elapsed_ms,
        cache_status,
        "yes" if analysis else "no",
        len(data) // 5,
        source.count("\n") + 1,
    )
    return types.SemanticTokens(data=data, result_id=result_id)


@server.feature(types.TEXT_DOCUMENT_SEMANTIC_TOKENS_FULL_DELTA)
def on_semantic_tokens_delta(
    params: types.SemanticTokensDeltaParams,
) -> types.SemanticTokens | types.SemanticTokensDelta:
    global _semantic_token_result_counter
    t0 = time.perf_counter()
    uri = params.text_document.uri
    previous_result_id = params.previous_result_id

    if not feature_config.semantic_tokens_enabled:
        return types.SemanticTokens(data=[])

    # Look up previous result for this URI.
    prev = _semantic_token_results.get(uri)
    if prev is None or prev[0] != previous_result_id:
        # No matching previous result — fall back to full.
        log.info("[timing] semanticTokens/delta fallback to full (no prev result)")
        return on_semantic_tokens_full(
            types.SemanticTokensParams(text_document=params.text_document)
        )

    old_data = prev[1]

    # Compute fresh tokens.
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    chunk_token_cache = None
    chunk_line_ranges = None
    if state is not None:
        cache_info = state.get_semantic_token_cache()
        if cache_info is not None:
            chunk_token_cache, chunk_line_ranges = cache_info
    new_data = semantic_tokens_full(
        source,
        analysis=analysis,
        is_bigip_conf=_is_bigip_conf(uri),
        is_irules=_is_irules_source(uri),
        is_apl=_is_apl_source(uri),
        chunk_token_cache=chunk_token_cache,
        chunk_line_ranges=chunk_line_ranges,
    )
    if state is not None and chunk_token_cache is not None:
        state.store_semantic_token_cache(chunk_token_cache)

    # Store new result.
    _semantic_token_result_counter += 1
    result_id = str(_semantic_token_result_counter)
    _semantic_token_results[uri] = (result_id, new_data)

    # Compute edits.
    edits = compute_semantic_tokens_edits(old_data, new_data)
    elapsed_ms = (time.perf_counter() - t0) * 1000

    if not edits:
        log.info("[timing] semanticTokens/delta %.0fms (no changes)", elapsed_ms)
        return types.SemanticTokensDelta(edits=[], result_id=result_id)

    # If delta is larger than full, send full instead.
    edit_data_size = sum(len(ins) for _, _, ins in edits) + len(edits) * 2  # overhead
    if edit_data_size >= len(new_data):
        log.info(
            "[timing] semanticTokens/delta %.0fms → full (delta larger, tokens=%d)",
            elapsed_ms,
            len(new_data) // 5,
        )
        return types.SemanticTokens(data=new_data, result_id=result_id)

    lsp_edits = [
        types.SemanticTokensEdit(
            start=start,
            delete_count=delete_count,
            data=insert_data if insert_data else None,
        )
        for start, delete_count, insert_data in edits
    ]
    log.info(
        "[timing] semanticTokens/delta %.0fms (edits=%d, delta_size=%d, full_size=%d)",
        elapsed_ms,
        len(lsp_edits),
        edit_data_size,
        len(new_data),
    )
    return types.SemanticTokensDelta(edits=lsp_edits, result_id=result_id)


# Completion


@server.feature(types.TEXT_DOCUMENT_COMPLETION)
def on_completion(
    params: types.CompletionParams,
) -> list[types.CompletionItem]:
    if not feature_config.completion_enabled:
        return []
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_completions(
        source,
        params.position.line,
        params.position.character,
        analysis=analysis,
        workspace_procs=workspace_index.all_proc_names(),
        workspace_rule_init_vars=workspace_index.all_rule_init_var_names(),
        workspace_command_usage=workspace_index.command_usage_counts(),
        workspace_proc_usage=workspace_index.proc_usage_counts(),
        formatter_config=formatter_config,
        lines=state.lines if state else None,
    )


# Hover


@server.feature(types.TEXT_DOCUMENT_HOVER)
def on_hover(params: types.HoverParams) -> types.Hover | None:
    if not feature_config.hover_enabled:
        return None
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_hover(
        source,
        params.position.line,
        params.position.character,
        analysis=analysis,
        lines=state.lines if state else None,
    )


# Go to definition


@server.feature(types.TEXT_DOCUMENT_DEFINITION)
def on_definition(
    params: types.DefinitionParams,
) -> list[types.Location]:
    if not feature_config.definition_enabled:
        return []
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None

    if _is_bigip_conf(uri):
        cfgs = background_scanner.bigip_configs
        current_cfg = cfgs.get(uri)
        if current_cfg is None:
            current_cfg = background_scanner.parse_bigip_source(uri, source)
        bigip_locations = get_bigip_definition(
            source,
            uri,
            params.position.line,
            params.position.character,
            current_config=current_cfg,
            workspace_configs=cfgs,
            lines=state.lines if state else None,
        )
        if bigip_locations:
            return bigip_locations

    # Try local file first
    locations = get_definition(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
    )
    if locations:
        return locations

    # Try workspace index for cross-file definitions
    line = params.position.line
    col = params.position.character
    word = find_word_at_position(source, line, col)
    if word:
        entries = workspace_index.find_proc(word)
        for entry in entries:
            if entry.proc:
                locations.append(to_lsp_location(entry.uri, entry.proc.name_range))
        # Try cross-file RULE_INIT variables (e.g. $::varname)
        if not locations and word.startswith("::"):
            var_entries = workspace_index.find_rule_init_var(word)
            for ve in var_entries:
                locations.append(to_lsp_location(ve.source_uri, ve.definition_range))

    return locations


# Find references


@server.feature(types.TEXT_DOCUMENT_REFERENCES)
def on_references(
    params: types.ReferenceParams,
) -> list[types.Location]:
    if not feature_config.references_enabled:
        return []
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    include_decl = params.context.include_declaration if params.context else True
    return get_references(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
        include_declaration=include_decl,
    )


# Document symbols


@server.feature(types.TEXT_DOCUMENT_DOCUMENT_SYMBOL)
def on_document_symbol(
    params: types.DocumentSymbolParams,
) -> list[types.DocumentSymbol]:
    if not feature_config.document_symbols_enabled:
        return []
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_document_symbols(source, analysis=analysis)


# Folding ranges


@server.feature(types.TEXT_DOCUMENT_FOLDING_RANGE)
def on_folding_range(
    params: types.FoldingRangeParams,
) -> list[types.FoldingRange]:
    if not feature_config.folding_enabled:
        return []
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_folding_ranges(source, analysis=analysis, lines=state.lines if state else None)


# Rename


@server.feature(
    types.TEXT_DOCUMENT_RENAME,
    types.RenameOptions(prepare_provider=True),
)
def on_rename(
    params: types.RenameParams,
) -> types.WorkspaceEdit | None:
    if not feature_config.rename_enabled:
        return None
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_rename_edits(
        source,
        uri,
        params.position.line,
        params.position.character,
        params.new_name,
        analysis=analysis,
    )


@server.feature(types.TEXT_DOCUMENT_PREPARE_RENAME)
def on_prepare_rename(
    params: types.PrepareRenameParams,
) -> types.PrepareRenamePlaceholder | None:
    if not feature_config.rename_enabled:
        return None
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return prepare_rename(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
    )


# Signature help


@server.feature(
    types.TEXT_DOCUMENT_SIGNATURE_HELP,
    types.SignatureHelpOptions(trigger_characters=[" "]),
)
def on_signature_help(
    params: types.SignatureHelpParams,
) -> types.SignatureHelp | None:
    if not feature_config.signature_help_enabled:
        return None
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_signature_help(
        source,
        params.position.line,
        params.position.character,
        analysis=analysis,
    )


# Workspace symbols


@server.feature(types.WORKSPACE_SYMBOL)
def on_workspace_symbol(
    params: types.WorkspaceSymbolParams,
) -> list[types.WorkspaceSymbol]:
    if not feature_config.workspace_symbols_enabled:
        return []
    return get_workspace_symbols(params.query, workspace_index)


# Inlay hints


@server.feature(types.TEXT_DOCUMENT_INLAY_HINT)
def on_inlay_hint(
    params: types.InlayHintParams,
) -> list[types.InlayHint]:
    if not feature_config.inlay_hints_enabled:
        return []
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_inlay_hints(
        source, params.range, analysis=analysis, lines=state.lines if state else None
    )


# Call hierarchy


@server.feature(types.TEXT_DOCUMENT_PREPARE_CALL_HIERARCHY)
def on_prepare_call_hierarchy(
    params: types.CallHierarchyPrepareParams,
) -> list[types.CallHierarchyItem]:
    if not feature_config.call_hierarchy_enabled:
        return []
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_call_hierarchy(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
    )


@server.feature(types.CALL_HIERARCHY_INCOMING_CALLS)
def on_incoming_calls(
    params: types.CallHierarchyIncomingCallsParams,
) -> list[types.CallHierarchyIncomingCall]:
    if not feature_config.call_hierarchy_enabled:
        return []
    item = params.item
    uri = item.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_incoming_calls(item, source, uri, analysis=analysis)


@server.feature(types.CALL_HIERARCHY_OUTGOING_CALLS)
def on_outgoing_calls(
    params: types.CallHierarchyOutgoingCallsParams,
) -> list[types.CallHierarchyOutgoingCall]:
    if not feature_config.call_hierarchy_enabled:
        return []
    item = params.item
    uri = item.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_outgoing_calls(item, source, uri, analysis=analysis)


# Document links


@server.feature(types.TEXT_DOCUMENT_DOCUMENT_LINK)
def on_document_link(
    params: types.DocumentLinkParams,
) -> list[types.DocumentLink]:
    if not feature_config.document_links_enabled:
        return []
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_document_links(source, analysis=analysis)


# Selection range


@server.feature(types.TEXT_DOCUMENT_SELECTION_RANGE)
def on_selection_range(
    params: types.SelectionRangeParams,
) -> list[types.SelectionRange] | None:
    if not feature_config.selection_range_enabled:
        return None
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    analysis = state.analysis if state else None
    return get_selection_ranges(
        source, list(params.positions), analysis=analysis, lines=state.lines if state else None
    )


# Code actions


@server.feature(
    types.TEXT_DOCUMENT_CODE_ACTION,
    types.CodeActionRegistrationOptions(
        document_selector=_TCL_DOCUMENT_SELECTOR,
        code_action_kinds=[
            types.CodeActionKind.QuickFix,
            types.CodeActionKind.RefactorExtract,
            types.CodeActionKind.RefactorInline,
            types.CodeActionKind.RefactorRewrite,
            types.CodeActionKind.Source,
        ],
    ),
)
def on_code_action(
    params: types.CodeActionParams,
) -> list[types.CodeAction] | None:
    if not feature_config.code_actions_enabled:
        return None
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    actions = get_code_actions(
        source,
        params.range,
        params.context,
        package_names=package_resolver.all_package_names(),
        lines=state.lines if state else None,
    )
    # Fill in the correct document URI for each action's edit
    for action in actions:
        if action.edit and action.edit.changes and "__current__" in action.edit.changes:
            remapped: dict[str, list[types.TextEdit]] = {}
            for uri, edits in action.edit.changes.items():
                if uri == "__current__":
                    continue
                remapped[uri] = list(edits)
            remapped[params.text_document.uri] = list(action.edit.changes["__current__"])
            action.edit.changes = remapped
    return actions or None


# Workspace commands


@server.command("tcl-lsp.optimiseDocument")
def on_optimise_document(uri: str) -> dict | None:
    source = _get_doc_source(uri)
    optimised, opts = optimise_source(source)
    items = []
    for o in opts:
        item: dict = {
            "code": o.code,
            "message": o.message,
            "startLine": o.range.start.line,
            "startCharacter": o.range.start.character,
            "endLine": o.range.end.line,
            "endCharacter": o.range.end.character,
            "replacement": o.replacement,
        }
        if o.group is not None:
            item["group"] = o.group
        if o.hint_only:
            item["hintOnly"] = True
        items.append(item)
    return {
        "optimisations": items,
        "source": optimised,
    }


@server.command("tcl-lsp.minifyDocument")
def on_minify_document(uri: str, compact: bool = False, aggressive: bool = False) -> dict | None:
    """Minify the Tcl document: strip comments, collapse whitespace, join commands."""
    source = _get_doc_source(uri)
    if aggressive:
        result = minify_tcl(source, aggressive=True)
        return {
            "source": result.source,
            "originalLength": result.original_length,
            "minifiedLength": result.minified_length,
            "symbolMap": result.symbol_map.format(),
            "optimisationsApplied": result.optimisations_applied,
        }
    if compact:
        minified, symbol_map = minify_tcl(source, compact_names=True)
        return {
            "source": minified,
            "originalLength": len(source),
            "minifiedLength": len(minified),
            "symbolMap": symbol_map.format(),
        }
    minified = minify_tcl(source)
    return {
        "source": minified,
        "originalLength": len(source),
        "minifiedLength": len(minified),
    }


@server.command("tcl-lsp.unminifyError")
def on_unminify_error(
    error_message: str,
    symbol_map: str,
    minified_source: str = "",
    original_source: str = "",
) -> dict:
    """Translate a minified-code error message back to original names."""
    from core.minifier import unminify_error

    translated = unminify_error(
        error_message,
        symbol_map=symbol_map,
        minified_source=minified_source or None,
        original_source=original_source or None,
    )
    return {
        "originalError": error_message,
        "translatedError": translated,
        "changed": translated != error_message,
    }


@server.command("tcl-lsp.describeIruleEvent")
def on_describe_irule_event(event_name: str) -> dict:
    """Return deterministic registry metadata for an iRules event."""
    event = (event_name or "").strip()
    when_values = REGISTRY.argument_values("when", 0, "f5-irules")
    known_events = {value.value for value in when_values}
    deprecated_events = {
        value.value for value in when_values if "deprecated" in (value.detail or "").lower()
    }

    is_known = event in known_events
    if not event or not is_known:
        valid_commands: list[str] = []
    else:
        event_set = REGISTRY.commands_for_event("f5-irules", event)
        valid_commands = sorted(event_set.valid_commands)
    return {
        "event": event,
        "known": is_known,
        "deprecated": event in deprecated_events,
        "validCommandCount": len(valid_commands),
        "sampleCommands": valid_commands[:80],
    }


@server.command("tcl-lsp.describeIruleCommand")
def on_describe_irule_command(command_name: str) -> dict:
    """Return deterministic registry metadata for an iRules command."""
    name = (command_name or "").strip()
    spec = REGISTRY.get(name, "f5-irules")
    if spec is None and name:
        lowered = name.lower()
        for candidate in REGISTRY.command_names("f5-irules"):
            if candidate.lower() == lowered:
                spec = REGISTRY.get(candidate, "f5-irules")
                name = candidate
                break

    if spec is None:
        return {
            "found": False,
            "command": name,
        }

    synopsis = tuple(spec.hover.synopsis) if spec.hover and spec.hover.synopsis else ()

    result: dict = {
        "found": True,
        "command": name,
        "summary": spec.hover.summary if spec.hover else "",
        "synopsis": list(synopsis),
        "switches": list(spec.switch_names()),
    }

    requires = effective_event_requires(name, spec.event_requires, dialect="f5-irules")
    if requires is not None:
        result["validEvents"] = EVENT_REGISTRY.events_matching(requires)
        result["anyEvent"] = not (
            requires.client_side
            or requires.server_side
            or requires.transport is not None
            or bool(requires.profiles)
            or bool(requires.also_in)
            or requires.init_only
            or requires.flow
            or requires.capability is not None
        )
        result["eventRequires"] = {
            "clientSide": requires.client_side,
            "serverSide": requires.server_side,
            "transport": requires.transport,
            "profiles": sorted(requires.profiles),
        }

    return result


def _apply_non_overlapping_fixes(
    source: str,
    fixes: list[tuple[int, int, str, str, str]],
) -> tuple[str, list[dict[str, str]]]:
    """Apply non-overlapping text replacements using source offsets."""
    valid = [fix for fix in fixes if 0 <= fix[0] <= fix[1] < len(source)]
    valid.sort(key=lambda item: (item[0], item[1]))

    accepted: list[tuple[int, int, str, str, str]] = []
    current_end = -1
    for fix in valid:
        start, end, _text, _code, _description = fix
        if start <= current_end:
            continue
        accepted.append(fix)
        current_end = end

    rewritten = source
    applied: list[dict[str, str]] = []
    for start, end, new_text, code, description in reversed(accepted):
        rewritten = rewritten[:start] + new_text + rewritten[end + 1 :]
        applied.append(
            {
                "code": code,
                "description": description or "Apply safe fix",
            }
        )
    applied.reverse()
    return rewritten, applied


def _collect_pending_safe_fixes(
    source: str,
) -> list[tuple[int, int, str, str, str]]:
    """Collect first safe fix per diagnostic for *source*."""
    analysis = analyse(source)
    pending: list[tuple[int, int, str, str, str]] = []
    for diagnostic in analysis.diagnostics:
        if diagnostic.code not in _SAFE_FIX_CODES:
            continue
        if not diagnostic.fixes:
            continue
        fix = diagnostic.fixes[0]
        pending.append(
            (
                fix.range.start.offset,
                fix.range.end.offset,
                fix.new_text,
                diagnostic.code,
                fix.description,
            )
        )
    return pending


def _apply_safe_fixes_iteratively(
    source: str,
    *,
    max_passes: int = 4,
) -> tuple[str, list[dict[str, str]]]:
    """Apply safe fixes in multiple passes until source stabilises."""
    rewritten = source
    applied_total: list[dict[str, str]] = []
    for _ in range(max_passes):
        pending = _collect_pending_safe_fixes(rewritten)
        if not pending:
            break
        next_source, applied = _apply_non_overlapping_fixes(rewritten, pending)
        if not applied or next_source == rewritten:
            break
        applied_total.extend(applied)
        rewritten = next_source
    return rewritten, applied_total


@server.command("tcl-lsp.fixAllSafeIssues")
def on_fix_all_safe_issues(uri: str) -> dict | None:
    """Apply all non-overlapping safe code fixes for a document."""
    source = _get_doc_source(uri)
    rewritten, applied = _apply_safe_fixes_iteratively(source)
    return {
        "source": rewritten,
        "applied": applied,
    }


@server.command("tcl-lsp.listIruleEvents")
def on_list_irule_events() -> dict:
    """Return all known iRules event names from the registry."""
    when_values = REGISTRY.argument_values("when", 0, "f5-irules")
    events = sorted(v.value for v in when_values if v.value)
    return {
        "events": events,
        "count": len(events),
    }


@server.command("tcl-lsp.listSubcommands")
def on_list_subcommands(command_name: str) -> dict:
    """Return subcommand metadata for a command from the registry."""
    name = (command_name or "").strip()
    spec = REGISTRY.get_any(name)
    if spec is None or not spec.subcommands:
        return {"command": name, "subcommands": []}
    return {
        "command": name,
        "subcommands": [
            {
                "name": sub.name,
                "detail": sub.detail or "",
                "synopsis": sub.synopsis or "",
                "pure": sub.pure,
                "mutator": sub.mutator,
                "deprecated": sub.deprecated_replacement is not None,
            }
            for sub in sorted(spec.subcommands.values(), key=lambda s: s.name)
        ],
    }


@server.command("tcl-lsp.listKnownPackages")
def on_list_known_packages() -> dict:
    """Return all package names discovered by PackageResolver."""
    return {
        "packages": sorted(package_resolver.all_package_names()),
    }


@server.command("tcl-lsp.suggestPackagesForSymbol")
def on_suggest_packages_for_symbol(symbol: str) -> dict:
    """Suggest package names for a symbol/command heuristic."""
    query = (symbol or "").strip()
    if not query:
        return {"symbol": query, "suggestions": []}

    suggestions = rank_package_suggestions(
        query,
        package_resolver.all_package_names(),
        20,
    )
    return {
        "symbol": query,
        "suggestions": suggestions,
    }


@server.command("tcl-lsp.searchHelp")
def on_search_help(query: str = "", include_screenshots: bool = False) -> dict:
    """Search the KCS help database for features and documentation."""
    try:
        from core.help.kcs_db import (
            get_feature,
            get_screenshot_base64,
            list_features,
            search_help,
        )
    except Exception:
        return {"error": "KCS help database not available"}

    if query:
        results = search_help(query)
        if not results:
            # Try exact feature lookup
            feat = get_feature(query)
            if feat:
                results = [feat]

        screenshots: dict = {}
        if include_screenshots:
            for r in results:
                from core.help.kcs_db import list_screenshots_for_feature

                for ss in list_screenshots_for_feature(r.get("file", "")):
                    if ss["has_image"]:
                        img = get_screenshot_base64(ss["ref_id"])
                        if img:
                            screenshots[ss["ref_id"]] = img

        return {"results": results, "screenshots": screenshots}

    catalogue = list_features()
    return {"catalogue": catalogue}


@server.command("tcl-lsp.compilerExplorer")
def on_compiler_explorer(source: str, dialect: str) -> dict | None:
    """Run the full compiler explorer pipeline and return serialised JSON."""
    if not source or not source.strip():
        return {
            "error": "No source was received from the editor.",
            "details": "Open a Tcl/iRule file in the active editor and try again.",
        }
    try:
        result = explorer_run_pipeline(source, dialect=dialect or None)
        if not result.snapshots:
            basic_diags, _, _ = get_basic_diagnostics(
                source,
                optimiser_enabled=False,
                disabled_diagnostics=feature_config.disabled_diagnostics,
                disabled_optimisations=feature_config.disabled_optimisations,
                line_length=feature_config.line_length,
            )
            error_diags = [
                diagnostic
                for diagnostic in basic_diags
                if diagnostic.severity == types.DiagnosticSeverity.Error
            ]
            if error_diags:
                first = error_diags[0]
                code = first.code if isinstance(first.code, str) else "E000"
                line = first.range.start.line + 1
                col = first.range.start.character + 1
                details = f"{code} at line {line}, column {col}: {first.message}"
                diagnostics = []
                for diagnostic in error_diags[:5]:
                    diag_code = diagnostic.code if isinstance(diagnostic.code, str) else "E000"
                    diagnostics.append(
                        {
                            "code": diag_code,
                            "message": diagnostic.message,
                            "line": diagnostic.range.start.line + 1,
                            "column": diagnostic.range.start.character + 1,
                        }
                    )
                return {
                    "error": "Source script issues prevented compiler explorer output.",
                    "details": details,
                    "diagnostics": diagnostics,
                }
            return {
                "error": "Compiler explorer could not build IR from the active source.",
                "details": "No compilable Tcl/iRule commands were found in the current editor.",
            }
        return explorer_serialise_result(result)
    except Exception as exc:
        import traceback

        return {"error": str(exc), "traceback": traceback.format_exc()}


@server.command("tcl-lsp.tkPreview")
def on_tk_preview(source: str) -> dict | None:
    """Extract Tk widget tree from source code for GUI preview."""
    if not source or not source.strip():
        return None
    try:
        from core.tk.extract import extract_tk_layout

        return extract_tk_layout(source)
    except Exception as exc:
        import traceback

        return {"error": str(exc), "traceback": traceback.format_exc()}


@server.command("tcl-lsp.diagramData")
def on_diagram_data(source: str) -> dict | None:
    """Extract structured flow data from an iRule for diagram generation."""
    if not source or not source.strip():
        return None
    try:
        from core.diagram.extract import extract_diagram_data

        return extract_diagram_data(source)
    except Exception as exc:
        import traceback

        return {"error": str(exc), "traceback": traceback.format_exc()}


@server.command("tcl-lsp.xcTranslate")
def on_xc_translate(source: str, output_format: str = "both") -> dict | None:
    """Translate an iRule to F5 XC configuration."""
    if not source or not source.strip():
        return None
    try:
        from core.xc.json_api import render_json
        from core.xc.terraform import render_terraform
        from core.xc.translator import translate_irule

        configure_signatures(dialect="f5-irules")
        result = translate_irule(source)

        output: dict = {}
        if output_format in ("terraform", "both"):
            output["terraform"] = render_terraform(result)
        if output_format in ("json", "both"):
            output["json_api"] = render_json(result)

        output["coverage_pct"] = result.coverage_pct
        output["translatable_count"] = result.translatable_count
        output["partial_count"] = result.partial_count
        output["untranslatable_count"] = result.untranslatable_count
        output["advisory_count"] = result.advisory_count
        output["items"] = [
            {
                "status": item.status.name.lower(),
                "kind": item.kind.name.lower(),
                "command": item.irule_command,
                "xc_description": item.xc_description,
                "note": item.note,
                "diagnostic_code": item.diagnostic_code,
            }
            for item in result.items
        ]
        return output
    except Exception as exc:
        import traceback

        return {"error": str(exc), "traceback": traceback.format_exc()}


# BIG-IP rule extraction


@server.command("tcl-lsp.extractRule")
def on_extract_rule(uri: str, offset: int) -> dict | None:
    """Find the ``ltm rule`` / ``gtm rule`` containing *offset* and return it.

    Returns ``{name, fullPath, body, bodyStartOffset, bodyEndOffset}``
    or ``None`` if the cursor is not inside a rule block.
    """
    from core.bigip.rule_extract import find_rule_at_offset

    try:
        doc = server.workspace.get_text_document(uri)
    except Exception:
        return None
    rule = find_rule_at_offset(doc.source, offset)
    if rule is None:
        return None
    return {
        "name": rule.name,
        "fullPath": rule.full_path,
        "body": rule.body,
        "bodyStartOffset": rule.body_start_offset,
        "bodyEndOffset": rule.body_end_offset,
        "uri": uri,
    }


@server.command("tcl-lsp.listRules")
def on_list_rules(uri: str) -> list[dict] | None:
    """Return all ``ltm rule`` / ``gtm rule`` blocks in the given document."""
    from core.bigip.rule_extract import find_embedded_rules

    try:
        doc = server.workspace.get_text_document(uri)
    except Exception:
        return None
    rules = find_embedded_rules(doc.source)
    return [
        {
            "name": r.name,
            "fullPath": r.full_path,
            "body": r.body,
            "bodyStartOffset": r.body_start_offset,
            "bodyEndOffset": r.body_end_offset,
            "blockStartLine": r.range.start.line,
            "uri": uri,
        }
        for r in rules
    ]


@server.command("tcl-lsp.extractLinkedObjects")
def on_extract_linked_objects(
    uri: str,
    offset: int,
    max_depth: int = 4,
    max_nodes: int = 250,
    extra_offsets: list | None = None,
) -> dict | None:
    """Return a transitive BIG-IP object subgraph around one or more cursors.

    *extra_offsets* is an optional list of ``[uri, offset]`` pairs for
    additional cursor positions beyond the primary *uri*/*offset*.  Each
    seed object becomes a root in the resulting graph.
    """
    from core.bigip.link_extract import extract_linked_bigip_objects

    # Build the full list of (uri, offset) seed positions.
    seed_positions: list[tuple[str, int]] = [(uri, offset)]
    if extra_offsets:
        for item in extra_offsets:
            if isinstance(item, (list, tuple)) and len(item) >= 2:
                seed_positions.append((str(item[0]), int(item[1])))

    # Collect all seed URIs whose sources we need.
    seed_uris: set[str] = {u for u, _ in seed_positions}

    # Ensure every seed URI has a parsed config.
    configs = background_scanner.bigip_configs
    for seed_uri in seed_uris:
        if seed_uri in configs:
            continue
        try:
            src = _get_doc_source(seed_uri)
        except Exception:
            return None
        parsed = background_scanner.parse_bigip_source(seed_uri, src)
        if parsed is None:
            return None
        configs = background_scanner.bigip_configs

    sources: dict[str, str] = {}
    for cfg_uri in configs:
        try:
            sources[cfg_uri] = _get_doc_source(cfg_uri)
            continue
        except Exception:
            pass

        path = uri_to_path(cfg_uri)
        if not path:
            continue
        try:
            with open(path, encoding="utf-8", errors="replace") as handle:
                sources[cfg_uri] = handle.read()
        except OSError:
            continue

    # Make sure every seed URI is present in sources.
    for seed_uri in seed_uris:
        if seed_uri not in sources:
            try:
                sources[seed_uri] = _get_doc_source(seed_uri)
            except Exception:
                return None

    return extract_linked_bigip_objects(
        offsets=seed_positions,
        sources=sources,
        configs=configs,
        max_depth=max_depth,
        max_nodes=max_nodes,
    )


@server.command("tcl-lsp.writeRuleBack")
def on_write_rule_back(
    uri: str,
    body_start_offset: int,
    body_end_offset: int,
    new_body: str,
) -> bool:
    """Replace the iRule body in the configuration file.

    Applies a ``workspace/applyEdit`` to replace the text between
    *body_start_offset* and *body_end_offset* with *new_body*.
    """
    try:
        doc = server.workspace.get_text_document(uri)
        source = doc.source
    except Exception:
        return False

    buf = DocumentBuffer.from_source(source)
    start_pos = buf.offset_to_position(body_start_offset)
    end_pos = buf.offset_to_position(body_end_offset)
    start = types.Position(line=start_pos.line, character=start_pos.character)
    end = types.Position(line=end_pos.line, character=end_pos.character)

    edit = types.WorkspaceEdit(
        changes={
            uri: [
                types.TextEdit(
                    range=types.Range(start=start, end=end),
                    new_text=new_body,
                )
            ]
        }
    )
    server.workspace_apply_edit(
        types.ApplyWorkspaceEditParams(edit=edit, label="Write iRule back to config")
    )
    return True


# Formatting


@server.feature(
    types.TEXT_DOCUMENT_FORMATTING,
    types.DocumentFormattingRegistrationOptions(
        document_selector=_TCL_DOCUMENT_SELECTOR,
    ),
)
def on_formatting(
    params: types.DocumentFormattingParams,
) -> list[types.TextEdit] | None:
    if not feature_config.formatting_enabled:
        return None
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    edits = get_formatting(
        source, params.options, formatter_config, lines=state.lines if state else None
    )
    return edits or None


@server.feature(
    types.TEXT_DOCUMENT_RANGE_FORMATTING,
    types.DocumentRangeFormattingRegistrationOptions(
        document_selector=_TCL_DOCUMENT_SELECTOR,
    ),
)
def on_range_formatting(
    params: types.DocumentRangeFormattingParams,
) -> list[types.TextEdit] | None:
    if not feature_config.formatting_enabled:
        return None
    uri = params.text_document.uri
    source = _get_doc_source(uri)
    state = workspace_state.get(uri)
    edits = get_range_formatting(
        source,
        params.range,
        params.options,
        formatter_config,
        lines=state.lines if state else None,
    )
    return edits or None


# Diagnostics


def _publish_diags_to_client(
    uri: str,
    diagnostics: list[types.Diagnostic],
    version: int | None = None,
) -> None:
    """Push a diagnostics notification to the client."""
    server.text_document_publish_diagnostics(
        types.PublishDiagnosticsParams(
            uri=uri,
            diagnostics=diagnostics,
            version=version,
        )
    )


def _update_workspace_index(uri: str, source: str, state: object) -> None:
    """Update workspace index after a stable analysis."""
    from .workspace.document_state import DocumentState

    if not isinstance(state, DocumentState):
        return
    if not state.analysis or state.has_partial_commands:
        return
    workspace_index.update(uri, state.analysis, EntrySource.OPEN)
    if _is_irules_source(uri):
        if state.analysis.all_procs:
            workspace_index.update_irules_globals(
                uri,
                state.analysis.all_procs,
            )
        from core.compiler.irules_flow import extract_rule_init_vars

        exports = extract_rule_init_vars(source, cu=state.compilation_unit)
        workspace_index.update_rule_init_vars(uri, exports)
    _load_packages_if_needed(state.analysis)


def _publish_diagnostics_sync(
    uri: str,
    source: str,
    version: int | None = None,
    *,
    force_reanalyse: bool = False,
) -> None:
    """Synchronous diagnostic publish — used during configuration changes."""
    state = workspace_state.update(
        uri,
        source,
        version,
        force_reanalyse=force_reanalyse,
        line_length=feature_config.line_length,
    )
    partial_mode = state.has_partial_commands

    if feature_config.diagnostics_enabled:
        diagnostics = get_diagnostics(
            source,
            analysis=state.analysis,
            cu=state.compilation_unit,
            optimiser_enabled=feature_config.optimiser_enabled and not partial_mode,
            shimmer_enabled=feature_config.shimmer_enabled and not partial_mode,
            taint_enabled=not partial_mode,
            xc_diagnostics_enabled=feature_config.xc_diagnostics_enabled,
            disabled_diagnostics=feature_config.disabled_diagnostics,
            disabled_optimisations=feature_config.disabled_optimisations,
            uri=uri,
            line_length=feature_config.line_length,
        )
    else:
        diagnostics = []

    _publish_diags_to_client(uri, diagnostics, version)
    _update_workspace_index(uri, source, state)


async def _publish_diagnostics(
    uri: str,
    source: str,
    version: int | None = None,
    *,
    force_reanalyse: bool = False,
) -> None:
    """Analyse source and publish diagnostics with async deep-pass scheduling.

    Phase 1 runs analysis + style checks in a background thread so
    the asyncio event loop stays responsive (editors can still receive
    responses to hover, completion, semantic-token requests while
    analysis is in progress).

    Phase 2 (background): schedules the expensive compiler passes
    (optimiser, shimmer, taint, GVN, iRules flow) in a background
    thread.  When complete, the full set (basic + deep) is published
    in a single notification.  If the document changes before the
    deep pass finishes, the task is cancelled automatically.
    """
    t_start = time.perf_counter()

    # Yield the event loop before doing any CPU work.  This ensures
    # that responses for any already-dispatched requests are flushed
    # to the editor before we start blocking.
    await asyncio.sleep(0)

    # Phase 0 (fast): update source/version/chunks on the event loop
    # so that queued semantic-token requests can be served immediately
    # with the new source text — even before analysis completes.
    t_quick = time.perf_counter()
    state = workspace_state.get(uri)
    if state is None:
        state = workspace_state.open(uri, source, version, analyse=False)
        needs_analysis = True
    else:
        needs_analysis = state.update_source_quick(source, version)
    quick_ms = (time.perf_counter() - t_quick) * 1000

    # Yield the event loop so queued requests (e.g. semantic tokens)
    # can be dispatched with the new source text before the heavy
    # analysis pass blocks.
    await asyncio.sleep(0)

    # Snapshot config values before entering the thread — these are
    # only mutated on the event loop so reading them here is safe.
    diagnostics_enabled = feature_config.diagnostics_enabled
    disabled_diagnostics = set(feature_config.disabled_diagnostics)
    line_length = feature_config.line_length
    optimiser_enabled = feature_config.optimiser_enabled
    disabled_optimisations = set(feature_config.disabled_optimisations)

    # Full analysis: compile, analyse, build chunk caches.
    # Run in a background thread so the event loop stays responsive for
    # hover, completion, and semantic-token requests during analysis.
    t_update = time.perf_counter()
    did_analyse = needs_analysis or force_reanalyse
    if did_analyse:
        await asyncio.to_thread(
            state.update,
            source,
            version,
            force_reanalyse=force_reanalyse,
            line_length=line_length,
        )
        # If the document changed while we were analysing in the background
        # thread, bail — a newer _publish_diagnostics coroutine will handle it.
        if state.version != version:
            log.info("[timing] workspace_state.update stale (version changed), bailing")
            return
    update_ms = (time.perf_counter() - t_update) * 1000
    log.info(
        "[timing] workspace_state.update %.0fms (quick=%.0fms, uri=%s, lines=%d)",
        update_ms,
        quick_ms,
        uri,
        len(state.buffer.line_starts),
    )
    partial_mode = state.has_partial_commands

    # After analysis completes, ask the client to re-request semantic
    # tokens so they benefit from analysis enrichment (regex_positions).
    if did_analyse and state.analysis is not None:
        try:
            server.workspace_semantic_tokens_refresh(None)
        except Exception:
            pass  # client may not support refresh

    # Yield the event loop after the full update so queued requests
    # can be dispatched with analysis-enriched state.
    await asyncio.sleep(0)

    if not diagnostics_enabled:
        basic_diags: list[types.Diagnostic] = []
        analysis_result = None
        suppressed: set[str] = set()
    else:
        # Cache lookup also touches state — do it on the event loop.
        cached_style = state.get_cached_style_diagnostics(
            disabled_diagnostics=disabled_diagnostics,
            line_length=line_length,
        )

        def _phase1():
            """Run CPU-heavy diagnostics in a thread to keep the event loop free."""
            return get_basic_diagnostics(
                source,
                analysis=state.analysis,
                cu=state.compilation_unit,
                optimiser_enabled=optimiser_enabled and not partial_mode,
                disabled_diagnostics=disabled_diagnostics,
                disabled_optimisations=disabled_optimisations,
                line_length=line_length,
                cached_style_diagnostics=cached_style,
            )

        t_phase1 = time.perf_counter()
        basic_diags, analysis_result, suppressed = await asyncio.to_thread(_phase1)
        phase1_ms = (time.perf_counter() - t_phase1) * 1000
        log.info("[timing] phase1 diagnostics %.0fms (diags=%d)", phase1_ms, len(basic_diags))

    if not diagnostics_enabled:
        _publish_diags_to_client(uri, [], version)
        _update_workspace_index(uri, source, state)
        return

    _publish_diags_to_client(uri, basic_diags, version)
    _update_workspace_index(uri, source, state)
    total_ms = (time.perf_counter() - t_start) * 1000
    log.info("[timing] _publish_diagnostics total %.0fms (basic diags published)", total_ms)

    # Phase 2: deep diagnostics — background thread.
    # Skip deep passes during partial-command states (mid-typing).
    # Taint and iRules flow always run when not partial, so the only
    # gate is whether we're in partial mode.
    if partial_mode:
        return

    # Capture current config for the closure (avoid races if config changes).
    opt_enabled = feature_config.optimiser_enabled and not partial_mode
    shimmer_enabled = feature_config.shimmer_enabled and not partial_mode
    taint_enabled = not partial_mode
    xc_enabled = feature_config.xc_diagnostics_enabled
    disabled_diags = set(feature_config.disabled_diagnostics)
    disabled_opts = set(feature_config.disabled_optimisations)
    cu = state.compilation_unit

    # Deep diagnostic proc cache: if all procs are unchanged, reuse.
    cached_deep = state.get_cached_deep_diagnostics()
    # Capture state ref and version for the closure so a stale background
    # thread does not overwrite a newer document's cached diagnostics.
    _state_ref = state
    _scheduled_version = version

    def _deep_fn() -> list[types.Diagnostic]:
        if cached_deep is not None:
            return cached_deep
        result = get_deep_diagnostics(
            source,
            suppressed,
            cu=cu,
            analysis=analysis_result,
            optimiser_enabled=opt_enabled,
            shimmer_enabled=shimmer_enabled,
            taint_enabled=taint_enabled,
            xc_diagnostics_enabled=xc_enabled,
            disabled_diagnostics=disabled_diags,
            disabled_optimisations=disabled_opts,
            uri=uri,
            generic_variable_patterns=feature_config.generic_variable_patterns,
        )
        # Only store if the document hasn't been updated since we started.
        if _state_ref.version == _scheduled_version:
            _state_ref.store_deep_diagnostics(result)
        return result

    diagnostic_scheduler.schedule(
        uri,
        version,
        basic_diags,
        _deep_fn,
        _publish_diags_to_client,
    )


def _load_packages_if_needed(analysis: object) -> None:
    """Resolve and load packages referenced by ``package require``."""
    from core.analysis.semantic_model import AnalysisResult

    if not isinstance(analysis, AnalysisResult):
        return
    if not analysis.package_requires:
        return
    for pkg_req in analysis.package_requires:
        if pkg_req.name in _loaded_packages:
            continue
        source_files = package_resolver.resolve(pkg_req.name, pkg_req.version)
        if not source_files:
            continue
        _loaded_packages.add(pkg_req.name)
        for file_path in source_files:
            pkg_uri = path_to_uri(file_path)
            if workspace_state.get(pkg_uri) is not None:
                continue  # Already open
            if workspace_index.get_analysis(pkg_uri) is not None:
                continue  # Already indexed
            scan_result = background_scanner.rescan_file(file_path)
            if scan_result:
                workspace_index.update(
                    pkg_uri,
                    scan_result.analysis,
                    EntrySource.PACKAGE,
                )


# Document sync


def _is_bigip_conf(uri: str) -> bool:
    """Check if a URI points to a BIG-IP configuration file."""
    from .workspace.scanner import _BIGIP_CONF_NAMES

    basename = uri.rsplit("/", 1)[-1].lower() if "/" in uri else uri.lower()
    return basename in _BIGIP_CONF_NAMES


def _is_irules_source(uri: str) -> bool:
    """Check whether a URI points to an iRules source file.

    Checks the editor's ``language_id`` first (set by the editor when the
    user selects a language mode), then falls back to the file extension.
    """
    lang_id = workspace_state.get_language_id(uri).lower()
    if lang_id in ("irules", "irul", "irule"):
        return True
    basename = uri.rsplit("/", 1)[-1].lower() if "/" in uri else uri.lower()
    return basename.endswith(".irul") or basename.endswith(".irule")


def _is_apl_source(uri: str) -> bool:
    """Check whether a URI points to an APL (iApp presentation) file.

    Checks the editor's ``language_id`` first, then falls back to the file
    extension or basename.
    """
    lang_id = workspace_state.get_language_id(uri).lower()
    if lang_id in ("tcl-apl", "apl-lang", "apl"):
        return True
    basename = uri.rsplit("/", 1)[-1].lower() if "/" in uri else uri.lower()
    return basename.endswith(".apl") or basename == "presentation"


def _publish_bigip_diagnostics(
    uri: str,
    source: str,
    version: int | None = None,
) -> None:
    """Parse a BIG-IP conf file and publish cross-reference diagnostics."""
    from core.bigip.diagnostics import get_bigip_diagnostics
    from core.bigip.parser import parse_bigip_conf

    try:
        config = parse_bigip_conf(source)
        background_scanner.parse_bigip_source(uri, source)
    except Exception:
        log.debug("bigip: failed to parse %s", uri, exc_info=True)
        server.text_document_publish_diagnostics(
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[], version=version)
        )
        return

    if feature_config.diagnostics_enabled:
        diagnostics = get_bigip_diagnostics(
            config, disabled_codes=feature_config.disabled_diagnostics
        )
    else:
        diagnostics = []

    server.text_document_publish_diagnostics(
        types.PublishDiagnosticsParams(
            uri=uri,
            diagnostics=diagnostics,
            version=version,
        )
    )


def _uri_to_dir(uri: str) -> str | None:
    """Extract the directory path from a file URI."""
    if uri.startswith("file://"):
        path = uri[7:]
    else:
        path = uri
    import os

    return os.path.dirname(path) if path else None


def _publish_apl_diagnostics(
    uri: str,
    source: str,
    version: int | None = None,
) -> None:
    """Parse an APL presentation file and publish cross-reference diagnostics."""

    from core.analysis.semantic_model import Severity
    from core.bigip.iapp_diagnostics import validate_iapp_presentation
    from core.common.lsp import to_lsp_range

    base_dir = _uri_to_dir(uri)
    model = background_scanner.parse_apl_source(uri, source, base_dir)
    if model is None:
        server.text_document_publish_diagnostics(
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[], version=version)
        )
        return

    if not feature_config.diagnostics_enabled:
        server.text_document_publish_diagnostics(
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[], version=version)
        )
        return

    # Look for sibling implementation to cross-validate
    impl_var_refs = _find_sibling_impl_vars(uri, base_dir)

    severity_map = {
        Severity.ERROR: types.DiagnosticSeverity.Error,
        Severity.WARNING: types.DiagnosticSeverity.Warning,
        Severity.INFO: types.DiagnosticSeverity.Information,
        Severity.HINT: types.DiagnosticSeverity.Hint,
    }
    raw = validate_iapp_presentation(model, impl_var_refs)
    results: list[types.Diagnostic] = []
    for d in raw:
        if feature_config.disabled_diagnostics and d.code in feature_config.disabled_diagnostics:
            continue
        results.append(
            types.Diagnostic(
                range=to_lsp_range(d.range),
                message=d.message,
                severity=severity_map.get(d.severity, types.DiagnosticSeverity.Warning),
                source="tcl-lsp",
                code=d.code or None,
            )
        )
    server.text_document_publish_diagnostics(
        types.PublishDiagnosticsParams(uri=uri, diagnostics=results, version=version)
    )


def _find_sibling_impl_vars(uri: str, base_dir: str | None) -> list | None:
    """Find and extract iApp variable references from a sibling implementation file."""
    import os

    from core.bigip.iapp_vars import extract_iapp_var_refs

    # Check open documents first
    impl_uri = background_scanner.find_sibling_impl_source(uri)
    if impl_uri is not None:
        try:
            impl_doc = server.workspace.get_text_document(impl_uri)
            if impl_doc is not None:
                return extract_iapp_var_refs(impl_doc.source)
        except Exception:
            pass

    # Try to find implementation file on disk
    if not base_dir:
        return None

    candidates: list[str] = []
    # Extensionless "implementation" file
    impl_path = os.path.join(base_dir, "implementation")
    if os.path.isfile(impl_path):
        candidates.append(impl_path)
    # Files with iApp extensions
    try:
        for fname in os.listdir(base_dir):
            ext = os.path.splitext(fname)[1].lower()
            if ext in (".iapp", ".iappimpl", ".impl"):
                candidates.append(os.path.join(base_dir, fname))
    except OSError:
        pass

    for path in candidates:
        try:
            with open(path, encoding="utf-8", errors="replace") as f:
                return extract_iapp_var_refs(f.read())
        except OSError:
            continue
    return None


@server.feature(types.TEXT_DOCUMENT_DID_OPEN)
async def did_open(params: types.DidOpenTextDocumentParams) -> None:
    t_open = time.perf_counter()
    uri = params.text_document.uri
    lang_id = params.text_document.language_id or ""
    n_lines = params.text_document.text.count("\n") + 1
    log.info("Opened %s (language_id=%r, lines=%d)", uri, lang_id, n_lines)
    # Pre-create the document state so the language_id is stored before
    # _publish_diagnostics calls workspace_state.update (which would
    # otherwise create the entry without one).
    #
    # analyse=False keeps the event loop free — the full analysis runs
    # later in _publish_diagnostics via asyncio.to_thread.
    workspace_state.open(
        uri,
        params.text_document.text,
        params.text_document.version,
        language_id=lang_id,
        analyse=False,
    )

    # Auto-detect dialect from the editor's language selection when the
    # user hasn't explicitly configured one.
    if (
        not feature_config.dialect_explicitly_set
        and not is_irules_dialect()
        and _is_irules_source(uri)
    ):
        log.info("Auto-switching to f5-irules dialect (language_id=%r)", lang_id)
        configure_signatures(dialect="f5-irules")
        server.window_show_message(
            types.ShowMessageParams(
                type=types.MessageType.Info,
                message="Switched to iRules dialect for F5 iRules support.",
            )
        )

    if _is_bigip_conf(uri):
        _publish_bigip_diagnostics(
            uri,
            params.text_document.text,
            params.text_document.version,
        )
        return
    if _is_apl_source(uri):
        _publish_apl_diagnostics(
            uri,
            params.text_document.text,
            params.text_document.version,
        )
        return
    await _publish_diagnostics(
        uri,
        params.text_document.text,
        params.text_document.version,
    )
    log.info("[timing] did_open total %.0fms (uri=%s)", (time.perf_counter() - t_open) * 1000, uri)


@server.feature(types.TEXT_DOCUMENT_DID_CHANGE)
async def did_change(params: types.DidChangeTextDocumentParams) -> None:
    doc = server.workspace.get_text_document(params.text_document.uri)
    if _is_bigip_conf(params.text_document.uri):
        _publish_bigip_diagnostics(
            params.text_document.uri,
            doc.source,
            params.text_document.version,
        )
        return
    if _is_apl_source(params.text_document.uri):
        _publish_apl_diagnostics(
            params.text_document.uri,
            doc.source,
            params.text_document.version,
        )
        return
    await _publish_diagnostics(
        params.text_document.uri,
        doc.source,
        params.text_document.version,
    )


@server.feature(types.TEXT_DOCUMENT_DID_CLOSE)
def did_close(params: types.DidCloseTextDocumentParams) -> None:
    uri = params.text_document.uri
    log.info("Closed %s", uri)
    diagnostic_scheduler.cancel(uri)
    if _is_bigip_conf(uri):
        background_scanner.remove_bigip_config(uri)
        server.text_document_publish_diagnostics(
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[])
        )
        return
    if _is_apl_source(uri):
        background_scanner.remove_apl_model(uri)
        server.text_document_publish_diagnostics(
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[])
        )
        return
    _semantic_token_results.pop(uri, None)
    workspace_state.close(uri)
    # If this file was also scanned in the background, revert to that entry
    # so cross-file references keep working after the file is closed.
    bg_analysis = background_scanner.get_cached(uri)
    if bg_analysis is not None:
        workspace_index.update(uri, bg_analysis, EntrySource.BACKGROUND)
    else:
        workspace_index.remove(uri)
    # Clear diagnostics for closed file
    server.text_document_publish_diagnostics(
        types.PublishDiagnosticsParams(uri=uri, diagnostics=[])
    )


# Workspace scanning


_scan_lock = threading.Lock()


def _run_background_scan() -> None:
    """Execute background scan in a daemon thread.

    Each file is analysed with a per-file timeout so that a single
    pathological file cannot block the entire scan.  The daemon thread
    runs independently of the asyncio event loop — the GIL switch
    interval ensures the event loop stays responsive.

    Concurrent scan requests are deduplicated: if a scan is already
    running, subsequent calls return immediately.
    """
    if not _scan_lock.acquire(blocking=False):
        log.debug("Background scan already running — skipping duplicate request")
        return
    t0 = time.perf_counter()
    try:
        open_uris = frozenset(uri for uri, _ in workspace_state.items())
        results = background_scanner.scan_all(skip_uris=open_uris)
        for uri, scan_result in results.items():
            # Don't overwrite entries for currently open files
            if workspace_state.get(uri) is not None:
                continue
            workspace_index.update(uri, scan_result.analysis, EntrySource.BACKGROUND)
        # Register iRules procs as globally available
        for uri, procs in background_scanner.irules_procs.items():
            workspace_index.update_irules_globals(uri, procs)
        # Register RULE_INIT variables as cross-file available
        for uri, exports in background_scanner.irules_rule_init_vars.items():
            workspace_index.update_rule_init_vars(uri, exports)
        elapsed_ms = (time.perf_counter() - t0) * 1000
        log.info("[timing] background scan %.0fms (%d files)", elapsed_ms, len(results))
    except Exception:
        log.error("Background scan failed", exc_info=True)
    finally:
        _scan_lock.release()


@server.feature(types.INITIALIZED)
def on_initialized(params: types.InitializedParams) -> None:
    """After client initialization, scan workspace for Tcl files."""
    from core.common.dialect import active_dialect

    # Load user config from ~/.config/tcl-lsp/config.ini.
    user_config = load_user_config()

    # Apply XDG settings as baseline defaults.  Editor settings received
    # later via ``didChangeConfiguration`` will override these.
    xdg_settings = get_all_settings(user_config)
    if xdg_settings:
        formatting = xdg_settings.get("formatting")
        if isinstance(formatting, dict) and formatting:
            global formatter_config
            formatter_config = FormatterConfig.from_dict(_normalise_formatter_settings(formatting))
        _apply_feature_settings(xdg_settings)

    process_start = getattr(server, "_process_start_time", None)
    if process_start is not None:
        startup_ms = (time.monotonic() - process_start) * 1000
        log.info("[timing] server ready: %.0fms from process start", startup_ms)

    log.info(
        "Server initialized (version=%s, dialect=%s)",
        _version,
        active_dialect(),
    )

    # Advise editors that don't request semantic tokens.
    caps = server.client_capabilities
    st = getattr(getattr(caps, "text_document", None), "semantic_tokens", None)
    if st is None:
        log.info("Client did not advertise semantic token support")
        server.window_show_message(
            types.ShowMessageParams(
                type=types.MessageType.Info,
                message=(
                    "Tip: enable semantic tokens for richer Tcl highlighting. "
                    'In Zed, add \'"semantic_tokens": "full"\' to your '
                    "language settings."
                ),
            )
        )

    roots: list[str] = []
    ws = server.workspace
    if ws.root_path:
        roots.append(ws.root_path)
    background_scanner.configure(workspace_roots=roots)
    threading.Thread(target=_run_background_scan, daemon=True).start()

    # Proactively pull editor settings so that clients using the pull model
    # (vscode-languageclient v10, JetBrains) have their settings applied
    # immediately rather than waiting for a configuration change event.
    _pull_and_apply_configuration()


# Custom commands (workspace/executeCommand)

_DIALECT_COMMAND = "tcl-lsp.setDialect"
_EXPORT_CONFIG_COMMAND = "tcl-lsp.exportConfig"
_DIALECT_LABELS = {
    "tcl8.4": "Tcl 8.4",
    "tcl8.5": "Tcl 8.5",
    "tcl8.6": "Tcl 8.6",
    "tcl9.0": "Tcl 9.0",
    "f5-irules": "F5 iRules",
    "f5-iapps": "F5 iApps",
    "eda-tools": "EDA Tools",
}


@server.feature(
    types.WORKSPACE_EXECUTE_COMMAND,
    types.ExecuteCommandOptions(commands=[_DIALECT_COMMAND, _EXPORT_CONFIG_COMMAND]),
)
def on_execute_command(
    params: types.ExecuteCommandParams,
) -> object:
    """Handle custom commands."""
    if params.command == _DIALECT_COMMAND:
        args = params.arguments or []
        if args:
            dialect = str(args[0])
        else:
            dialect = ""
        return _switch_dialect(dialect)
    if params.command == _EXPORT_CONFIG_COMMAND:
        return _export_config()
    return None


def _switch_dialect(dialect: str) -> dict:
    """Switch the active dialect and re-publish diagnostics."""
    from core.commands.registry.dialects import KNOWN_DIALECTS
    from core.common.dialect import active_dialect

    if dialect and dialect not in KNOWN_DIALECTS:
        return {"success": False, "error": f"Unknown dialect: {dialect!r}"}

    prev = active_dialect()
    changed = configure_signatures(dialect=dialect or None)
    current = active_dialect()
    feature_config.dialect_explicitly_set = True
    log.info("Dialect set to %s (was %s)", current, prev)

    if changed:
        server.window_show_message(
            types.ShowMessageParams(
                type=types.MessageType.Info,
                message=f"Switched dialect to {_DIALECT_LABELS.get(current, current)}.",
            )
        )
        # Re-analyse all open documents with the new dialect.
        for uri, state in workspace_state.items():
            _publish_diagnostics_sync(
                uri,
                state.source,
                state.version,
                force_reanalyse=True,
            )

    return {"success": True, "dialect": current}


def _export_config() -> dict:
    """Export the current effective settings to the XDG config file."""

    # Build the current effective settings dict.
    settings: dict[str, object] = {}

    # Diagnostics — collect disabled codes.
    diag: dict[str, object] = {}
    for code in _ALL_DIAGNOSTIC_CODES:
        if code in feature_config.disabled_diagnostics:
            diag[code] = False
    current_patterns = list(feature_config.generic_variable_patterns)
    if current_patterns and current_patterns != list(DEFAULT_GENERIC_VARIABLE_PATTERNS):
        diag["genericVariablePatterns"] = current_patterns
    if diag:
        settings["diagnostics"] = diag

    # Optimiser
    opt: dict[str, object] = {"enabled": feature_config.optimiser_enabled}
    for code in _ALL_OPTIMISATION_CODES:
        if code in feature_config.disabled_optimisations:
            opt[code] = False
    settings["optimiser"] = opt

    # Shimmer
    settings["shimmer"] = {"enabled": feature_config.shimmer_enabled}

    # XC Diagnostics
    settings["xcDiagnostics"] = {"enabled": feature_config.xc_diagnostics_enabled}

    # Features
    feat: dict[str, object] = {}
    for json_key, attr in _FEATURE_TOGGLE_KEYS.items():
        feat[json_key] = getattr(feature_config, attr)
    settings["features"] = feat

    # Style
    settings["style"] = {"lineLength": feature_config.line_length}

    # Build defaults so only non-default values are written.
    default_cfg = FeatureConfig()
    default_features: dict[str, object] = {
        json_key: getattr(default_cfg, attr) for json_key, attr in _FEATURE_TOGGLE_KEYS.items()
    }
    defaults: dict[str, object] = {
        "features": default_features,
        "optimiser": {"enabled": default_cfg.optimiser_enabled},
        "shimmer": {"enabled": default_cfg.shimmer_enabled},
        "xcDiagnostics": {"enabled": default_cfg.xc_diagnostics_enabled},
        "style": {"lineLength": default_cfg.line_length},
    }

    try:
        path = save_settings_to_config(settings, only_non_default=True, defaults=defaults)
        server.window_show_message(
            types.ShowMessageParams(
                type=types.MessageType.Info,
                message=f"Settings exported to {path}",
            )
        )
        return {"success": True, "path": path}
    except Exception as exc:
        log.error("Failed to export config", exc_info=True)
        return {"success": False, "error": str(exc)}


@server.feature(types.SHUTDOWN)
def on_shutdown(params: None) -> None:
    """Cancel pending background tasks on shutdown."""
    diagnostic_scheduler.cancel_all()


@server.feature(types.WORKSPACE_DID_CHANGE_WATCHED_FILES)
def did_change_watched_files(
    params: types.DidChangeWatchedFilesParams,
) -> None:
    """React to file system changes for non-open files."""
    for change in params.changes:
        uri = change.uri
        # Skip files that are currently open (they get fresh analysis on edit)
        if workspace_state.get(uri) is not None:
            continue

        if change.type == types.FileChangeType.Deleted:
            workspace_index.remove(uri)
            background_scanner.remove_file(uri)
        elif change.type in (
            types.FileChangeType.Created,
            types.FileChangeType.Changed,
        ):
            file_path = uri_to_path(uri)
            if file_path:
                scan_result = background_scanner.rescan_file(file_path)
                if scan_result:
                    workspace_index.update(
                        uri,
                        scan_result.analysis,
                        EntrySource.BACKGROUND,
                    )
                    # Update iRules globals if applicable
                    if scan_result.dialect_hint == "f5-irules":
                        workspace_index.update_irules_globals(
                            uri,
                            scan_result.analysis.all_procs,
                        )
                        if scan_result.rule_init_exports:
                            workspace_index.update_rule_init_vars(
                                uri,
                                scan_result.rule_init_exports,
                            )


# Configuration

# Loaded from the self-registering code registry (core.common.codes).
_ALL_DIAGNOSTIC_CODES = diagnostic_codes()
_ALL_OPTIMISATION_CODES = optimisation_codes()

_FEATURE_TOGGLE_KEYS = {
    "hover": "hover_enabled",
    "completion": "completion_enabled",
    "diagnostics": "diagnostics_enabled",
    "formatting": "formatting_enabled",
    "semanticTokens": "semantic_tokens_enabled",
    "codeActions": "code_actions_enabled",
    "definition": "definition_enabled",
    "references": "references_enabled",
    "documentSymbols": "document_symbols_enabled",
    "folding": "folding_enabled",
    "rename": "rename_enabled",
    "signatureHelp": "signature_help_enabled",
    "workspaceSymbols": "workspace_symbols_enabled",
    "inlayHints": "inlay_hints_enabled",
    "callHierarchy": "call_hierarchy_enabled",
    "documentLinks": "document_links_enabled",
    "selectionRange": "selection_range_enabled",
}


def _apply_feature_settings(tcl_settings: dict) -> bool:
    """Apply feature toggles and diagnostic/optimiser filters.

    Returns True if diagnostics need to be republished.
    """
    global feature_config
    changed = False

    # Feature-level toggles  (tclLsp.features.hover etc.)
    features = tcl_settings.get("features")
    if isinstance(features, dict):
        for json_key, attr in _FEATURE_TOGGLE_KEYS.items():
            val = features.get(json_key)
            if val is None:
                # Also accept snake_case variant from flat key extraction.
                val = features.get(_camel_to_snake(json_key))
            if isinstance(val, bool) and val != getattr(feature_config, attr):
                setattr(feature_config, attr, val)
                changed = True

    # Also accept flat keys: tclLsp.features.hover -> features.hover
    for key, value in tcl_settings.items():
        if isinstance(key, str) and key.startswith("features.") and isinstance(value, bool):
            json_key = key[len("features.") :]
            attr = _FEATURE_TOGGLE_KEYS.get(json_key) or _FEATURE_TOGGLE_KEYS.get(
                _camel_to_snake(json_key)
            )
            if attr and value != getattr(feature_config, attr):
                setattr(feature_config, attr, value)
                changed = True

    # Per-diagnostic-code filters  (tclLsp.diagnostics.W100 etc.)
    diagnostics_section = tcl_settings.get("diagnostics")
    if isinstance(diagnostics_section, dict):
        new_disabled: set[str] = set()
        for code in _ALL_DIAGNOSTIC_CODES:
            val = diagnostics_section.get(code)
            if isinstance(val, bool) and not val:
                new_disabled.add(code)
        if new_disabled != feature_config.disabled_diagnostics:
            feature_config.disabled_diagnostics = new_disabled
            changed = True

        # Warn about unrecognised codes.
        _DIAG_NON_CODE_KEYS = {"genericVariablePatterns", "generic_variable_patterns"}
        unknown_diag = {
            k
            for k, v in diagnostics_section.items()
            if v is False and k not in _ALL_DIAGNOSTIC_CODES and k not in _DIAG_NON_CODE_KEYS
        }
        if unknown_diag:
            log.warning(
                "Unrecognised diagnostic codes in settings (ignored): %s", sorted(unknown_diag)
            )

    # Style settings  (tclLsp.style.lineLength)
    style_section = tcl_settings.get("style")
    if isinstance(style_section, dict):
        ll = style_section.get("lineLength")
        if isinstance(ll, int) and ll > 0 and ll != feature_config.line_length:
            feature_config.line_length = ll
            changed = True

    # Shimmer detection toggle  (tclLsp.shimmer.enabled)
    shimmer_section = tcl_settings.get("shimmer")
    if isinstance(shimmer_section, dict):
        shimmer_master = shimmer_section.get("enabled")
        if isinstance(shimmer_master, bool) and shimmer_master != feature_config.shimmer_enabled:
            feature_config.shimmer_enabled = shimmer_master
            changed = True

    # XC diagnostics toggle  (tclLsp.xcDiagnostics.enabled)
    xc_section = tcl_settings.get("xcDiagnostics")
    if isinstance(xc_section, dict):
        xc_enabled = xc_section.get("enabled")
        if isinstance(xc_enabled, bool) and xc_enabled != feature_config.xc_diagnostics_enabled:
            feature_config.xc_diagnostics_enabled = xc_enabled
            changed = True

    # Optimiser master switch + per-code  (tclLsp.optimiser.*)
    optimiser_section = tcl_settings.get("optimiser")
    if isinstance(optimiser_section, dict):
        master = optimiser_section.get("enabled")
        if isinstance(master, bool) and master != feature_config.optimiser_enabled:
            feature_config.optimiser_enabled = master
            changed = True

        new_disabled_opts: set[str] = set()
        for code in _ALL_OPTIMISATION_CODES:
            val = optimiser_section.get(code)
            if isinstance(val, bool) and not val:
                new_disabled_opts.add(code)
        if new_disabled_opts != feature_config.disabled_optimisations:
            feature_config.disabled_optimisations = new_disabled_opts
            changed = True

        unknown_opt = {
            k
            for k, v in optimiser_section.items()
            if v is False and k not in _ALL_OPTIMISATION_CODES and k != "enabled"
        }
        if unknown_opt:
            log.warning(
                "Unrecognised optimisation codes in settings (ignored): %s", sorted(unknown_opt)
            )

    # Generic variable patterns  (tclLsp.diagnostics.genericVariablePatterns)
    if isinstance(diagnostics_section, dict):
        patterns = diagnostics_section.get("genericVariablePatterns")
        if patterns is None:
            patterns = diagnostics_section.get("generic_variable_patterns")
        if isinstance(patterns, list):
            new_patterns = [str(p) for p in patterns if isinstance(p, str)]
            if not new_patterns:
                new_patterns = None  # treat empty list as "use defaults"
            if (
                new_patterns is not None
                and new_patterns != feature_config.generic_variable_patterns
            ):
                feature_config.generic_variable_patterns = new_patterns
                changed = True

    return changed


# ---- Settings debounce ----
# Rapid didChangeConfiguration notifications (e.g. dialect ping-pong during
# startup) are coalesced so only the final settings are applied.
_pending_settings: dict | None = None
_pending_settings_handle: asyncio.TimerHandle | None = None
_SETTINGS_DEBOUNCE_S = 0.3


def _apply_all_settings(tcl_settings: dict) -> None:
    """Debounced settings application.

    Stores *tcl_settings* and schedules :func:`_apply_all_settings_now`
    after a short delay.  If another call arrives within the debounce
    window the timer is reset so that only the latest settings are
    applied.  This prevents the dialect ping-pong problem where two
    rapid ``didChangeConfiguration`` notifications each trigger a full
    rebuild of every open document.
    """
    global _pending_settings, _pending_settings_handle
    _pending_settings = tcl_settings
    if _pending_settings_handle is not None:
        _pending_settings_handle.cancel()
    try:
        loop = asyncio.get_event_loop()
    except RuntimeError:
        # No running loop (e.g. during tests) — apply immediately.
        _apply_all_settings_now()
        return
    _pending_settings_handle = loop.call_later(
        _SETTINGS_DEBOUNCE_S,
        _apply_all_settings_now,
    )


def _apply_all_settings_now() -> None:
    """Apply the most recent settings (called after debounce timer).

    This is the shared implementation used by both the push-model
    ``workspace/didChangeConfiguration`` handler and the pull-model
    ``workspace/configuration`` callback.
    """
    global _pending_settings, _pending_settings_handle, formatter_config
    settings = _pending_settings
    _pending_settings = None
    _pending_settings_handle = None
    if settings is None:
        return

    tcl_settings = settings

    formatting = tcl_settings.get("formatting")
    if isinstance(formatting, dict) and formatting:
        # Merge onto the current config so partial editor settings don't
        # clobber XDG baseline values for keys the editor didn't send.
        current = formatter_config.to_dict()
        current.update(_normalise_formatter_settings(formatting))
        formatter_config = FormatterConfig.from_dict(current)

    extra_commands_setting = tcl_settings.get("extraCommands")
    if extra_commands_setting is None:
        extra_commands_setting = tcl_settings.get("extra_commands")
    if extra_commands_setting is None:
        extra_commands = None
    elif isinstance(extra_commands_setting, list):
        extra_commands = [str(cmd) for cmd in extra_commands_setting]
    else:
        extra_commands = []

    # Library paths for background scanning and package resolution
    library_paths_setting = tcl_settings.get("libraryPaths")
    if library_paths_setting is None:
        library_paths_setting = tcl_settings.get("library_paths")
    if isinstance(library_paths_setting, list):
        library_paths = [str(p) for p in library_paths_setting]
        background_scanner.configure(library_paths=library_paths)
        package_resolver.configure(search_paths=library_paths)
        _loaded_packages.clear()
        threading.Thread(target=_run_background_scan, daemon=True).start()

    dialect_setting = tcl_settings.get("dialect")
    if isinstance(dialect_setting, str) and dialect_setting:
        feature_config.dialect_explicitly_set = True
    signatures_changed = configure_signatures(
        dialect=dialect_setting if isinstance(dialect_setting, str) else None,
        extra_commands=extra_commands,
    )
    if signatures_changed:
        from core.common.dialect import active_dialect

        log.info(
            "Dialect changed to %s (explicit=%s)",
            active_dialect(),
            feature_config.dialect_explicitly_set,
        )

    features_changed = _apply_feature_settings(tcl_settings)

    if not signatures_changed and not features_changed:
        return

    if signatures_changed:
        # Cancel any in-flight background deep-diagnostic tasks so they
        # do not overwrite the fresh diagnostics we are about to publish
        # with stale results computed under the old dialect/signatures.
        diagnostic_scheduler.cancel_all()

    for uri, state in workspace_state.items():
        _publish_diagnostics_sync(
            uri,
            state.source,
            state.version,
            force_reanalyse=signatures_changed,
        )


def _pull_and_apply_configuration() -> None:
    """Pull configuration from the client via ``workspace/configuration``.

    Used as a fallback when ``didChangeConfiguration`` arrives with null
    settings (pull-model clients such as vscode-languageclient v10 and
    JetBrains).  Also called proactively from ``on_initialized`` to pick
    up initial editor settings.
    """
    params = types.ConfigurationParams(items=[types.ConfigurationItem(section="tclLsp")])

    def _on_result(result: list[object] | None) -> None:  # type: ignore[type-arg]
        if not result:
            return
        item = result[0]
        if not isinstance(item, dict):
            return
        # The response is already unwrapped (section was "tclLsp"), so it
        # matches the format _extract_tcl_lsp_settings would produce.
        _apply_all_settings(item)

    try:
        server.workspace_configuration(params, callback=_on_result)
    except Exception:
        log.debug("workspace/configuration pull failed", exc_info=True)


@server.feature(types.WORKSPACE_DID_CHANGE_CONFIGURATION)
def did_change_configuration(params: types.DidChangeConfigurationParams) -> None:
    settings = params.settings
    if not settings:
        # Pull-model: client sent a bare notification; fetch the actual values.
        _pull_and_apply_configuration()
        return

    tcl_settings = _extract_tcl_lsp_settings(settings)
    _apply_all_settings(tcl_settings)
