# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tcl Language Server -- pygls-based LSP implementation."""

from __future__ import annotations

import asyncio
import logging
import threading
import time
from collections import OrderedDict
from typing import TYPE_CHECKING

from lsprotocol import types

if TYPE_CHECKING:
    from analyser.semantic_model import AnalysisResult
from pygls.lsp.server import LanguageServer

import server._codes_init  # noqa: F401  # trigger all code registrations
from server._lsp_conv import to_lsp_location

from .features import (
    SEMANTIC_TOKEN_MODIFIERS,
    SEMANTIC_TOKEN_TYPES,
    compute_semantic_tokens_edits,
    semantic_tokens_full,
)
from .features._bigip_code_actions import get_bigip_code_actions
from .features._bigip_links import get_bigip_document_links
from .features._bigip_refs import (
    get_bigip_references,
    get_bigip_rename_edits,
    prepare_bigip_rename,
)
from .features._bigip_symbols import get_bigip_document_symbols
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
from .features.code_lens import get_code_lenses, resolve_code_lens
from .features.completion import get_completions
from .features.declaration import get_declaration
from .features.definition import get_bigip_definition, get_definition
from .features.document_highlight import get_document_highlights
from .features.document_links import get_document_links
from .features.document_symbols import get_document_symbols
from .features.folding import get_folding_ranges
from .features.formatting import get_formatting, get_range_formatting
from .features.hover import get_hover
from .features.implementation import get_implementations
from .features.inlay_hints import get_inlay_hints
from .features.linked_editing_range import get_linked_editing_ranges
from .features.references import find_method_call_sites, find_proc_call_sites, get_references
from .features.rename import get_rename_edits, prepare_rename
from .features.selection_range import get_selection_ranges
from .features.signature_help import get_signature_help
from .features.symbol_resolution import find_word_at_position
from .features.type_definition import get_type_definition
from .features.type_hierarchy import (
    prepare_type_hierarchy,
)
from .features.type_hierarchy import (
    subtypes as get_subtypes,
)
from .features.type_hierarchy import (
    supertypes as get_supertypes,
)
from .features.workspace_symbols import get_workspace_symbols

log = logging.getLogger(__name__)


try:
    from shared._build_info import FULL_VERSION as _version
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
    """Attach the LSP handler to our own concern loggers (not the root logger).

    Attaching to the root logger caused a feedback loop: every pygls
    internal debug message triggered another ``window/logMessage`` send
    which logged again.  Instead we attach only to the seven Python
    concern packages that carry our application messages.  Each
    package's modules log via ``logging.getLogger(__name__)``, so e.g.
    ``server.async_diagnostics`` is a child of the ``server`` logger
    and inherits the handler.
    """
    handler = _LspLogHandler()
    handler.setLevel(logging.DEBUG)
    handler.setFormatter(logging.Formatter("%(name)s: %(message)s"))
    for name in ("server", "compiler", "analyser", "dialects", "tooling", "ai", "shared"):
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


def _log_request(msg_id, method: str, params):
    level = logging.DEBUG if method in _FREQUENT_METHODS else logging.INFO
    log.log(level, "<-- request  %s (id=%s)", method, msg_id)
    return _orig_handle_request(msg_id, method, params)


def _log_notification(method: str, params):
    level = logging.DEBUG if method in _FREQUENT_METHODS else logging.INFO
    log.log(level, "<-- notify   %s", method)
    return _orig_handle_notification(method, params)


server.protocol._handle_request = _log_request  # type: ignore[assignment]
server.protocol._handle_notification = _log_notification  # type: ignore[assignment]


import server.commands as _commands  # noqa: E402
import server.diagnostics_pipeline as _dp  # noqa: E402
import server.lifecycle as _lifecycle  # noqa: E402
import server.settings as _settings  # noqa: E402
import server.state as _state  # noqa: E402
import server.workspace_init as _workspace_init  # noqa: E402

from .state import (  # noqa: E402
    _get_doc_source,
    _semantic_token_result_counter,
    _semantic_token_results,
    _semantic_token_results_lock,
    background_scanner,
    workspace_index,
    workspace_state,
)

_state.configure(server)
_dp.configure(server)
_settings.configure(server)
_settings.register(server)
_commands.register(server)
_workspace_init.register(server)
_lifecycle.register(server)


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
    "tcl9.1",
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


def _snapshot_read(uri: str):
    """Capture one document snapshot for a request handler.

    Returns ``(snap, source)`` where ``snap`` is the document's immutable
    ``_StateSnapshot`` (or ``None`` when untracked) and ``source`` is
    ``snap.source`` (falling back to the pygls workspace when untracked).
    Reading ``source`` / ``analysis`` / ``buffer`` / ``conf_wrapped`` /
    ``embedded_rules`` from this single ``snap`` keeps a handler consistent if a
    concurrent did_change swaps the live snapshot mid-request.
    """
    state = workspace_state.get(uri)
    snap = state.snap if state else None
    source = snap.source if snap else _get_doc_source(uri)
    return snap, source


@server.feature(
    types.TEXT_DOCUMENT_SEMANTIC_TOKENS_FULL,
    types.SemanticTokensRegistrationOptions(
        legend=SEMANTIC_TOKENS_LEGEND,
        full=types.SemanticTokensFullDelta(delta=True),
        document_selector=_TCL_DOCUMENT_SELECTOR,
    ),
)
@_state.scoped_to_doc
def on_semantic_tokens_full(
    params: types.SemanticTokensParams,
) -> types.SemanticTokens:
    t0 = time.perf_counter()
    if not _state.config_for_uri(params.text_document.uri).semantic_tokens_enabled:
        return types.SemanticTokens(data=[])
    uri = params.text_document.uri
    state = workspace_state.get(uri)
    # One snapshot for source + analysis + chunk-cache + buffer + conf-wrapped,
    # so a concurrent did_change can't mix two versions into one token set.
    snap = state.snap if state else None
    source = snap.source if snap else _get_doc_source(uri)
    analysis = snap.analysis if snap else None
    # Try to use per-chunk semantic token cache (from the same snapshot).
    chunk_token_cache = None
    chunk_line_ranges = None
    cache_status = "no_state"
    if state is not None and snap is not None:
        cache_info = state.get_semantic_token_cache(snap)
        if cache_info is not None:
            chunk_token_cache, chunk_line_ranges = cache_info
            cache_status = (
                "full_hit" if all(e is not None for e in chunk_token_cache) else "partial"
            )
        else:
            cache_status = "no_cache"
    ls = snap.buffer.line_starts if snap and snap.buffer is not None else None
    # Reuse this snapshot's live spliced buffer (consistent with the captured
    # source) so the bigip collectors don't rebuild the O(n) rope; matched on
    # source so a torn snapshot can never feed a mismatched buffer.
    buf = (
        snap.buffer
        if snap is not None and snap.buffer is not None and snap.buffer.source == source
        else None
    )
    is_cw = snap is not None and snap.conf_wrapped
    data = semantic_tokens_full(
        source,
        analysis=analysis,
        is_bigip_conf=_dp._is_bigip_conf(uri) or is_cw,
        is_irules=_dp._is_irules_source(uri) or is_cw,
        is_apl=_dp._is_apl_source(uri),
        chunk_token_cache=chunk_token_cache,
        chunk_line_ranges=chunk_line_ranges,
        line_starts=ls,
        buffer=buf,
    )
    # Write back computed tokens to chunk cache (only where the snapshot the
    # tokens were computed for still matches the live chunk).
    if state is not None and chunk_token_cache is not None:
        state.store_semantic_token_cache(chunk_token_cache, snap)
    # Store result for delta support (P8).
    result_id = str(next(_semantic_token_result_counter))
    with _semantic_token_results_lock:
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
@_state.scoped_to_doc
def on_semantic_tokens_delta(
    params: types.SemanticTokensDeltaParams,
) -> types.SemanticTokens | types.SemanticTokensDelta:
    t0 = time.perf_counter()
    uri = params.text_document.uri
    previous_result_id = params.previous_result_id

    if not _state.config_for_uri(params.text_document.uri).semantic_tokens_enabled:
        return types.SemanticTokens(data=[])

    # Look up previous result for this URI.
    with _semantic_token_results_lock:
        prev = _semantic_token_results.get(uri)
    if prev is None or prev[0] != previous_result_id:
        # No matching previous result — fall back to full.
        log.info("[timing] semanticTokens/delta fallback to full (no prev result)")
        return on_semantic_tokens_full(
            types.SemanticTokensParams(text_document=params.text_document)
        )

    old_data = prev[1]

    # Compute fresh tokens from a single captured snapshot.
    state = workspace_state.get(uri)
    snap = state.snap if state else None
    source = snap.source if snap else _get_doc_source(uri)
    analysis = snap.analysis if snap else None
    chunk_token_cache = None
    chunk_line_ranges = None
    if state is not None and snap is not None:
        cache_info = state.get_semantic_token_cache(snap)
        if cache_info is not None:
            chunk_token_cache, chunk_line_ranges = cache_info
    ls = snap.buffer.line_starts if snap and snap.buffer is not None else None
    buf = (
        snap.buffer
        if snap is not None and snap.buffer is not None and snap.buffer.source == source
        else None
    )
    is_cw_delta = snap is not None and snap.conf_wrapped
    new_data = semantic_tokens_full(
        source,
        analysis=analysis,
        is_bigip_conf=_dp._is_bigip_conf(uri) or is_cw_delta,
        is_irules=_dp._is_irules_source(uri) or is_cw_delta,
        is_apl=_dp._is_apl_source(uri),
        chunk_token_cache=chunk_token_cache,
        chunk_line_ranges=chunk_line_ranges,
        line_starts=ls,
        buffer=buf,
    )
    if state is not None and chunk_token_cache is not None:
        state.store_semantic_token_cache(chunk_token_cache, snap)

    # Store new result.
    result_id = str(next(_semantic_token_result_counter))
    with _semantic_token_results_lock:
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
@_state.scoped_to_doc
def on_completion(
    params: types.CompletionParams,
) -> list[types.CompletionItem]:
    if not _state.config_for_uri(params.text_document.uri).completion_enabled:
        return []
    uri = params.text_document.uri
    state = workspace_state.get(uri)
    snap = state.snap if state else None
    source = snap.source if snap else _get_doc_source(uri)
    return get_completions(
        source,
        params.position.line,
        params.position.character,
        analysis=snap.analysis if snap else None,
        workspace_procs=workspace_index.all_proc_names(),
        workspace_rule_init_vars=workspace_index.all_rule_init_var_names(),
        workspace_command_usage=workspace_index.command_usage_counts(),
        workspace_proc_usage=workspace_index.proc_usage_counts(),
        formatter_config=_state.formatter_config_for_uri(uri),
        lines=snap.buffer.lines if snap and snap.buffer else None,
        embedded_rules=snap.embedded_rules if snap and snap.conf_wrapped else None,
    )


# Hover
#
# Eldoc-style clients (Eglot) issue ``textDocument/hover`` on every cursor
# move, so the handler is structured to absorb bursts cheaply:
#
#   * a per-URI counter supersedes older requests when a newer one arrives
#     for the same document — the older request returns ``None`` after a
#     short debounce window without doing any work;
#   * results are cached by ``(uri, version, line, character)`` so repeated
#     requests at the same point return immediately;
#   * the actual hover computation runs in a worker thread so a slow hover
#     does not block the event loop;
#   * if the cached analysis is not yet ready (fresh ``didChange`` still
#     analysing in the process pool) we return ``None`` rather than running
#     a duplicate synchronous parse on the request path.

_HOVER_DEBOUNCE_S = 0.03
_HOVER_CACHE_MAX = 256
_hover_seq: dict[str, int] = {}
_hover_cache: OrderedDict[tuple[str, int | None, int, int], types.Hover | None] = OrderedDict()
_hover_lock = threading.Lock()


def _hover_cache_get(
    key: tuple[str, int | None, int, int],
) -> tuple[bool, types.Hover | None]:
    with _hover_lock:
        if key in _hover_cache:
            _hover_cache.move_to_end(key)
            return True, _hover_cache[key]
    return False, None


def _hover_cache_put(key: tuple[str, int | None, int, int], value: types.Hover | None) -> None:
    with _hover_lock:
        _hover_cache[key] = value
        _hover_cache.move_to_end(key)
        while len(_hover_cache) > _HOVER_CACHE_MAX:
            _hover_cache.popitem(last=False)


@server.feature(types.TEXT_DOCUMENT_HOVER)
@_state.scoped_to_doc
async def on_hover(params: types.HoverParams) -> types.Hover | None:
    if not _state.config_for_uri(params.text_document.uri).hover_enabled:
        return None
    t0 = time.perf_counter()
    uri = params.text_document.uri
    line = params.position.line
    character = params.position.character

    with _hover_lock:
        seq = _hover_seq.get(uri, 0) + 1
        _hover_seq[uri] = seq

    # Debounce: if another hover for the same URI arrives within the
    # window, drop this one. This collapses Eldoc bursts on rapid cursor
    # moves to a single computation for the final position.
    await asyncio.sleep(_HOVER_DEBOUNCE_S)
    with _hover_lock:
        if _hover_seq.get(uri, 0) != seq:
            log.debug(
                "[timing] hover %.0fms (superseded, uri=%s)",
                (time.perf_counter() - t0) * 1000,
                uri,
            )
            return None

    state = workspace_state.get(uri)
    # Capture one immutable snapshot and read version/analysis/source/lines/cu
    # all from it: a concurrent did_change (update_source_quick) can swap the
    # document's snapshot between reads, so reading these fields separately off
    # the live state could mix data from two document versions into one hover.
    snap = state.snap if state else None
    version = snap.version if snap else None
    cache_key = (uri, version, line, character)
    hit, cached = _hover_cache_get(cache_key)
    if hit:
        log.debug(
            "[timing] hover %.0fms (cache hit, uri=%s)",
            (time.perf_counter() - t0) * 1000,
            uri,
        )
        return cached

    analysis = snap.analysis if snap else None
    if analysis is None:
        # Fresh analysis is still pending in the diagnostics pipeline.
        # Return quickly instead of duplicating the parse on the request
        # thread; the next hover will pick up the cached analysis.
        log.debug(
            "[timing] hover %.0fms (no analysis, uri=%s)",
            (time.perf_counter() - t0) * 1000,
            uri,
        )
        return None

    # All from the captured snapshot, so source/lines/cu match ``analysis``.
    source = snap.source
    lines = snap.buffer.lines if snap.buffer is not None else None
    result = await asyncio.to_thread(
        get_hover,
        source,
        line,
        character,
        analysis=analysis,
        cu=snap.compilation_unit,
        lines=lines,
        analyse_if_missing=False,
    )
    # Cache regardless of supersession: the result is correctly keyed by
    # ``(uri, version, line, character)`` and remains valid for any future
    # hover at the same point until the document version changes.
    _hover_cache_put(cache_key, result)
    elapsed_ms = (time.perf_counter() - t0) * 1000
    # Recheck supersession after the threaded compute. If a newer hover
    # for this URI arrived while we were running, suppress this response
    # so a slow hover can't race ahead of a newer cursor position.
    with _hover_lock:
        if _hover_seq.get(uri, 0) != seq:
            log.debug(
                "[timing] hover %.0fms (superseded post-compute, uri=%s)",
                elapsed_ms,
                uri,
            )
            return None
    log.debug(
        "[timing] hover %.0fms (uri=%s, line=%d, char=%d, has_result=%s)",
        elapsed_ms,
        uri,
        line,
        character,
        "yes" if result else "no",
    )
    return result


def _invalidate_hover_cache(uri: str) -> None:
    """Drop hover cache entries and the request counter for a URI.

    Called from ``did_close`` so that a reopen-with-version-reset (clients
    commonly reset to ``1`` after close) cannot serve stale hover content
    keyed on the previous session's matching ``(uri, version, line, char)``.
    """
    with _hover_lock:
        _hover_seq.pop(uri, None)
        keys = [k for k in _hover_cache if k[0] == uri]
        for k in keys:
            _hover_cache.pop(k, None)


# Go to definition


@server.feature(types.TEXT_DOCUMENT_DEFINITION)
@_state.scoped_to_doc
def on_definition(
    params: types.DefinitionParams,
) -> list[types.Location]:
    if not _state.config_for_uri(params.text_document.uri).definition_enabled:
        return []
    uri = params.text_document.uri
    state = workspace_state.get(uri)
    snap = state.snap if state else None
    source = snap.source if snap else _get_doc_source(uri)
    analysis = snap.analysis if snap else None
    lines = snap.buffer.lines if snap and snap.buffer else None

    if _dp._is_bigip_conf(uri):
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
            lines=lines,
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
        # The cross-document proc fallback must only fire when the cursor
        # sits on a command head (a call site), matching the in-document
        # provider.  Otherwise an argument word that happens to collide
        # with a sibling proc/class name in another file produces a
        # false-positive jump (e.g. ``set foo 1`` jumping to a proc
        # ``foo`` elsewhere).
        if _position_is_command_head(analysis, line, col):
            # Prefer fully-qualified names synthesised from this file's
            # ``namespace import`` (and tcllib ``X::import`` wrapper)
            # declarations — that's how Tcl itself would resolve the call
            # at runtime.  Only fall through to a tail-name search when no
            # import rewrites produce a hit.
            if analysis is not None and analysis.namespace_imports:
                from analyser.namespace_imports import rewrite_via_imports

                for candidate in rewrite_via_imports(word, analysis.namespace_imports):
                    for entry in workspace_index.find_proc(candidate):
                        if entry.proc and entry.proc.qualified_name == candidate:
                            locations.append(to_lsp_location(entry.uri, entry.proc.name_range))
            if not locations:
                entries = workspace_index.find_proc(word)
                for entry in entries:
                    if entry.proc:
                        locations.append(to_lsp_location(entry.uri, entry.proc.name_range))
        # Cross-file RULE_INIT variables (e.g. ``$::varname``) are
        # variable references, not command heads, so they're resolved
        # independently of the command-head gate above.
        if not locations and word.startswith("::"):
            var_entries = workspace_index.find_rule_init_var(word)
            for ve in var_entries:
                locations.append(to_lsp_location(ve.source_uri, ve.definition_range))

    return locations


def _position_is_command_head(
    analysis: AnalysisResult | None,
    line: int,
    col: int,
) -> bool:
    """Whether the cursor sits on a command head (call site).

    Uses the analyser's ``command_invocations`` — each records the span
    of a command word — so the cross-document definition fallback only
    treats genuine call sites as proc references.
    """
    if analysis is None:
        return False
    from shared.position import position_in_range

    return any(position_in_range(line, col, inv.range) for inv in analysis.command_invocations)


# Go to type definition


@server.feature(types.TEXT_DOCUMENT_TYPE_DEFINITION)
@_state.scoped_to_doc
def on_type_definition(
    params: types.TypeDefinitionParams,
) -> list[types.Location] | None:
    if not _state.config_for_uri(params.text_document.uri).type_definition_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    # Pass None when analysis hasn't completed yet; the feature function
    # will run a throwaway analyse() inline.  This matches the behaviour
    # of on_references / on_definition and keeps the feature working on
    # slow CI runners where activate() may return before the fire-and-
    # forget did_open task has populated the analysis.
    analysis = snap.analysis if snap else None
    return get_type_definition(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
        workspace_classes=workspace_index.iter_classes(),
    )


# Go to declaration


@server.feature(types.TEXT_DOCUMENT_DECLARATION)
@_state.scoped_to_doc
def on_declaration(
    params: types.DeclarationParams,
) -> list[types.Location] | None:
    if not _state.config_for_uri(params.text_document.uri).declaration_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    # See on_type_definition for the rationale behind passing analysis=None
    # when analysis has not been populated yet.
    analysis = snap.analysis if snap else None
    return get_declaration(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
    )


# Find references


@server.feature(types.TEXT_DOCUMENT_REFERENCES)
@_state.scoped_to_doc
def on_references(
    params: types.ReferenceParams,
) -> list[types.Location]:
    if not _state.config_for_uri(params.text_document.uri).references_enabled:
        return []
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    include_decl = params.context.include_declaration if params.context else True
    # BIG-IP files: walk every indexed config and emit one Location
    # per token-bounded occurrence of the path-shaped token at the
    # cursor.  Tcl files keep their existing var/proc/class path.
    is_cw = snap is not None and snap.conf_wrapped
    if _dp._is_bigip_conf(uri) and not is_cw:
        scanner = getattr(_state, "background_scanner", None)
        ws_configs = scanner.bigip_configs if scanner else None
        ws_sources = {u: _get_doc_source(u) for u in (ws_configs or {})} if ws_configs else None
        return get_bigip_references(
            source,
            uri=uri,
            line=params.position.line,
            character=params.position.character,
            workspace_configs=ws_configs,
            workspace_sources=ws_sources,
            include_declaration=include_decl,
        )
    return get_references(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
        include_declaration=include_decl,
    )


# Document highlight


@server.feature(types.TEXT_DOCUMENT_DOCUMENT_HIGHLIGHT)
@_state.scoped_to_doc
def on_document_highlight(
    params: types.DocumentHighlightParams,
) -> list[types.DocumentHighlight] | None:
    if not _state.config_for_uri(params.text_document.uri).document_highlight_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    # See on_type_definition for the rationale behind passing analysis=None
    # when analysis has not been populated yet.
    analysis = snap.analysis if snap else None
    return get_document_highlights(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
    )


# Document symbols


@server.feature(types.TEXT_DOCUMENT_DOCUMENT_SYMBOL)
@_state.scoped_to_doc
def on_document_symbol(
    params: types.DocumentSymbolParams,
) -> list[types.DocumentSymbol]:
    if not _state.config_for_uri(params.text_document.uri).document_symbols_enabled:
        return []
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    chunks = snap.chunks if snap else None
    # BIG-IP / SCF files outline the parsed object inventory grouped
    # module → kind → object.  Conf-wrapped iRule files keep the Tcl
    # path because the user is editing iRule bodies, not the
    # surrounding stanza.
    is_cw = snap is not None and snap.conf_wrapped
    if _dp._is_bigip_conf(uri) and not is_cw:
        return get_bigip_document_symbols(source)
    return get_document_symbols(
        source,
        analysis=analysis,
        chunks=chunks,
        embedded_rules=snap.embedded_rules if snap and is_cw else None,
    )


# Folding ranges


@server.feature(types.TEXT_DOCUMENT_FOLDING_RANGE)
@_state.scoped_to_doc
def on_folding_range(
    params: types.FoldingRangeParams,
) -> list[types.FoldingRange]:
    if not _state.config_for_uri(params.text_document.uri).folding_enabled:
        return []
    uri = params.text_document.uri
    state = workspace_state.get(uri)
    # Fold ranges don't require a full AnalysisResult: the comment and
    # body-argument collectors work on source tokens alone and already
    # cover proc/namespace/control-structure bodies.  Returning ``[]``
    # while analysis is still running would leave VS Code with a cached
    # empty result and no fold markers until the next didChange.
    snap = state.snap if state else None
    source = snap.source if snap else _get_doc_source(uri)
    return get_folding_ranges(
        source,
        analysis=snap.analysis if snap else None,
        lines=snap.buffer.lines if snap and snap.buffer else None,
    )


# Rename


@server.feature(
    types.TEXT_DOCUMENT_RENAME,
    types.RenameOptions(prepare_provider=True),
)
@_state.scoped_to_doc
def on_rename(
    params: types.RenameParams,
) -> types.WorkspaceEdit | None:
    if not _state.config_for_uri(params.text_document.uri).rename_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    is_cw = snap is not None and snap.conf_wrapped
    if _dp._is_bigip_conf(uri) and not is_cw:
        scanner = getattr(_state, "background_scanner", None)
        ws_configs = scanner.bigip_configs if scanner else None
        ws_sources = {u: _get_doc_source(u) for u in (ws_configs or {})} if ws_configs else None
        return get_bigip_rename_edits(
            source,
            uri=uri,
            line=params.position.line,
            character=params.position.character,
            new_name=params.new_name,
            workspace_configs=ws_configs,
            workspace_sources=ws_sources,
        )
    return get_rename_edits(
        source,
        uri,
        params.position.line,
        params.position.character,
        params.new_name,
        analysis=analysis,
    )


@server.feature(types.TEXT_DOCUMENT_PREPARE_RENAME)
@_state.scoped_to_doc
def on_prepare_rename(
    params: types.PrepareRenameParams,
) -> types.PrepareRenamePlaceholder | None:
    if not _state.config_for_uri(params.text_document.uri).rename_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    is_cw = snap is not None and snap.conf_wrapped
    if _dp._is_bigip_conf(uri) and not is_cw:
        bigip_range = prepare_bigip_rename(
            source,
            line=params.position.line,
            character=params.position.character,
        )
        if bigip_range is None:
            return None
        # Extract the placeholder text directly from the source slice
        # the range covers — that way the rename input pre-fills with
        # the actual ``/Common/<name>`` the user clicked on.
        lines = source.split("\n")
        line_text = lines[bigip_range.start.line] if bigip_range.start.line < len(lines) else ""
        placeholder = line_text[bigip_range.start.character : bigip_range.end.character]
        return types.PrepareRenamePlaceholder(
            range=bigip_range,
            placeholder=placeholder,
        )
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
@_state.scoped_to_doc
def on_signature_help(
    params: types.SignatureHelpParams,
) -> types.SignatureHelp | None:
    if not _state.config_for_uri(params.text_document.uri).signature_help_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    return get_signature_help(
        source,
        params.position.line,
        params.position.character,
        analysis=analysis,
    )


# Workspace symbols


@server.feature(types.WORKSPACE_SYMBOL)
@_state.scoped_to_doc
def on_workspace_symbol(
    params: types.WorkspaceSymbolParams,
) -> list[types.WorkspaceSymbol]:
    if not _state.feature_config.workspace_symbols_enabled:
        return []
    return get_workspace_symbols(params.query, workspace_index)


# Inlay hints


@server.feature(types.TEXT_DOCUMENT_INLAY_HINT)
@_state.scoped_to_doc
def on_inlay_hint(
    params: types.InlayHintParams,
) -> list[types.InlayHint]:
    cfg = _state.config_for_uri(params.text_document.uri)
    if not (cfg.inlay_type_hints_enabled or cfg.inlay_parameter_hints_enabled):
        return []
    uri = params.text_document.uri
    state = workspace_state.get(uri)
    snap = state.snap if state else None
    if snap is not None and snap.analysis is None:
        return []
    source = snap.source if snap else _get_doc_source(uri)
    return get_inlay_hints(
        source,
        params.range,
        analysis=snap.analysis if snap else None,
        lines=snap.buffer.lines if snap and snap.buffer else None,
        type_hints=cfg.inlay_type_hints_enabled,
        parameter_hints=cfg.inlay_parameter_hints_enabled,
    )


# Call hierarchy


@server.feature(types.TEXT_DOCUMENT_PREPARE_CALL_HIERARCHY)
@_state.scoped_to_doc
def on_prepare_call_hierarchy(
    params: types.CallHierarchyPrepareParams,
) -> list[types.CallHierarchyItem]:
    if not _state.config_for_uri(params.text_document.uri).call_hierarchy_enabled:
        return []
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    return get_call_hierarchy(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
    )


def _cross_document_incoming(
    item: types.CallHierarchyItem,
    uri: str,
    analysis: AnalysisResult | None,
) -> list[tuple[str, AnalysisResult]]:
    """Other workspace documents whose callers should join the result.

    The folder scan indexes every workspace file, but incoming-call
    discovery would otherwise only see the open buffer.  Each indexed
    document keeps a lightweight ``command_invocations`` list, so we use
    that to cheaply pre-filter to the files that actually call the
    target, then produce a *full* analysis for just those — the open
    buffer's analysis when the file is open, otherwise a fresh analysis
    of its on-disk source — so callers can be grouped by containing proc.
    """
    if analysis is None:
        return []
    from pathlib import Path

    from analyser import analyse
    from server.workspace.scanner import uri_to_path

    from .features.call_hierarchy import resolve_call_target

    target = resolve_call_target(item, analysis)
    if target is None:
        return []

    extra: list[tuple[str, AnalysisResult]] = []
    for other_uri in workspace_index.document_uris():
        if other_uri == uri:
            continue
        cached = workspace_index.indexed_analysis(other_uri)
        if cached is None:
            continue
        if not find_proc_call_sites(target.name, target.qualified_name, cached):
            continue
        other_state = workspace_state.get(other_uri)
        if other_state is not None and other_state.analysis is not None:
            extra.append((other_uri, other_state.analysis))
            continue
        path = uri_to_path(other_uri)
        if not path:
            continue
        try:
            other_source = Path(path).read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        extra.append((other_uri, analyse(other_source)))
    return extra


@server.feature(types.CALL_HIERARCHY_INCOMING_CALLS)
@_state.scoped_to_doc
def on_incoming_calls(
    params: types.CallHierarchyIncomingCallsParams,
) -> list[types.CallHierarchyIncomingCall]:
    if not _state.config_for_uri(params.item.uri).call_hierarchy_enabled:
        return []
    item = params.item
    uri = item.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    extra = _cross_document_incoming(item, uri, analysis)
    return get_incoming_calls(item, source, uri, analysis=analysis, extra_documents=extra)


@server.feature(types.CALL_HIERARCHY_OUTGOING_CALLS)
@_state.scoped_to_doc
def on_outgoing_calls(
    params: types.CallHierarchyOutgoingCallsParams,
) -> list[types.CallHierarchyOutgoingCall]:
    if not _state.config_for_uri(params.item.uri).call_hierarchy_enabled:
        return []
    item = params.item
    uri = item.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    return get_outgoing_calls(item, source, uri, analysis=analysis)


# Type hierarchy


@server.feature(types.TEXT_DOCUMENT_PREPARE_TYPE_HIERARCHY)
@_state.scoped_to_doc
def on_prepare_type_hierarchy(
    params: types.TypeHierarchyPrepareParams,
) -> list[types.TypeHierarchyItem]:
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    return prepare_type_hierarchy(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
    )


@server.feature(types.TYPE_HIERARCHY_SUPERTYPES)
@_state.scoped_to_doc
def on_supertypes(
    params: types.TypeHierarchySupertypesParams,
) -> list[types.TypeHierarchyItem]:
    item = params.item
    uri = item.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    return get_supertypes(item, analysis=analysis, source=source)


@server.feature(types.TYPE_HIERARCHY_SUBTYPES)
@_state.scoped_to_doc
def on_subtypes(
    params: types.TypeHierarchySubtypesParams,
) -> list[types.TypeHierarchyItem]:
    item = params.item
    uri = item.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    return get_subtypes(item, analysis=analysis, source=source)


# Go to implementation


@server.feature(types.TEXT_DOCUMENT_IMPLEMENTATION)
@_state.scoped_to_doc
def on_implementation(
    params: types.ImplementationParams,
) -> list[types.Location] | None:
    if not _state.config_for_uri(params.text_document.uri).implementation_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    # See on_type_definition for the rationale behind passing analysis=None
    # when state.analysis has not been populated yet.
    analysis = snap.analysis if snap else None
    return get_implementations(
        source,
        uri,
        params.position.line,
        params.position.character,
        analysis=analysis,
        workspace_classes=workspace_index.iter_classes(),
    )


# Document links


@server.feature(types.TEXT_DOCUMENT_DOCUMENT_LINK)
@_state.scoped_to_doc
def on_document_link(
    params: types.DocumentLinkParams,
) -> list[types.DocumentLink]:
    if not _state.config_for_uri(params.text_document.uri).document_links_enabled:
        return []
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    is_cw = snap is not None and snap.conf_wrapped
    # BIG-IP / SCF files emit object-reference links from the
    # parser's iRule scanner (same path ``f5 grep`` uses).  Tcl
    # files keep their existing ``source`` / ``package require``
    # link path.  Conf-wrapped iRule files get both: BIG-IP links
    # for the surrounding stanza references plus Tcl links inside
    # rule bodies.
    if _dp._is_bigip_conf(uri) and not is_cw:
        workspace_configs = (
            _state.background_scanner.bigip_configs
            if hasattr(_state, "background_scanner") and _state.background_scanner
            else None
        )
        return get_bigip_document_links(
            source,
            uri=uri,
            workspace_configs=workspace_configs,
        )
    # Return empty when analysis hasn't completed — get_document_links
    # would call analyse(source) synchronously, blocking the event loop.
    if snap is not None and snap.analysis is None:
        return []
    analysis = snap.analysis if snap else None
    return get_document_links(source, analysis=analysis)


# Code lens


@server.feature(types.TEXT_DOCUMENT_CODE_LENS)
@_state.scoped_to_doc
def on_code_lens(
    params: types.CodeLensParams,
) -> list[types.CodeLens] | None:
    if not _state.config_for_uri(params.text_document.uri).code_lens_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    # See on_type_definition for the rationale behind passing analysis=None
    # when state.analysis has not been populated yet.
    analysis = snap.analysis if snap else None
    return get_code_lenses(source, uri, analysis)


@server.feature(types.CODE_LENS_RESOLVE)
@_state.scoped_to_doc
def on_code_lens_resolve(lens: types.CodeLens) -> types.CodeLens:
    def find_refs(uri: str, qname: str) -> list[types.Location]:
        # The lens title comes from ``workspace_index.proc_usage_counts()``
        # (workspace-wide), so the peek must sweep every indexed URI to
        # stay consistent — scanning only the clicked document would
        # under-report call sites in multi-file projects.
        locations: list[types.Location] = []
        proc_name = qname.rsplit("::", 1)[-1] or qname
        for indexed_uri in workspace_index.all_uris():
            analysis = workspace_index.get_analysis(indexed_uri)
            if analysis is None:
                continue
            ranges = find_proc_call_sites(proc_name, qname, analysis)
            if not ranges:
                continue
            locations.extend(to_lsp_location(indexed_uri, r) for r in ranges)
        return locations

    def find_method_refs(uri: str, class_qname: str, method_name: str) -> list[types.Location]:
        # Sweep every indexed document for method dispatch sites, matching the
        # workspace-wide scope of the proc reference peek above.
        locations: list[types.Location] = []
        for indexed_uri in workspace_index.all_uris():
            analysis = workspace_index.get_analysis(indexed_uri)
            if analysis is None:
                continue
            ranges = find_method_call_sites(class_qname, method_name, analysis)
            if not ranges:
                continue
            locations.extend(to_lsp_location(indexed_uri, r) for r in ranges)
        return locations

    return resolve_code_lens(lens, workspace_index, find_refs, find_method_refs)


# Pull diagnostics
#
# These handlers are only registered when the user opts in via
# ``tclLsp.features.pullDiagnostics``.  Registering them causes pygls to
# advertise ``diagnosticProvider`` in ServerCapabilities, which flips
# vscode-languageclient into pull mode and disables the push pipeline that
# the existing test suite depends on.

if _state.feature_config.pull_diagnostics_enabled:
    server.feature(types.TEXT_DOCUMENT_DIAGNOSTIC)(_dp.on_document_diagnostic)
    server.feature(types.WORKSPACE_DIAGNOSTIC)(_dp.on_workspace_diagnostic)


# Selection range


@server.feature(types.TEXT_DOCUMENT_SELECTION_RANGE)
@_state.scoped_to_doc
def on_selection_range(
    params: types.SelectionRangeParams,
) -> list[types.SelectionRange] | None:
    if not _state.config_for_uri(params.text_document.uri).selection_range_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    analysis = snap.analysis if snap else None
    return get_selection_ranges(
        source,
        list(params.positions),
        analysis=analysis,
        lines=snap.buffer.lines if snap and snap.buffer else None,
    )


# Linked editing range


@server.feature(types.TEXT_DOCUMENT_LINKED_EDITING_RANGE)
@_state.scoped_to_doc
def on_linked_editing_range(
    params: types.LinkedEditingRangeParams,
) -> types.LinkedEditingRanges | None:
    if not _state.config_for_uri(params.text_document.uri).linked_editing_range_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    # See on_type_definition for the rationale behind passing analysis=None
    # when state.analysis has not been populated yet.
    analysis = snap.analysis if snap else None
    return get_linked_editing_ranges(
        source,
        params.position.line,
        params.position.character,
        analysis=analysis,
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
@_state.scoped_to_doc
def on_code_action(
    params: types.CodeActionParams,
) -> list[types.CodeAction] | None:
    if not _state.config_for_uri(params.text_document.uri).code_actions_enabled:
        return None
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    # BIG-IP files get a separate code-action provider that wraps
    # the query engine's rename/cascade machinery as quick-fix
    # entries (no analysis dependency, parses the source directly).
    is_cw = snap is not None and snap.conf_wrapped
    if _dp._is_bigip_conf(uri) and not is_cw:
        return get_bigip_code_actions(source, uri=uri, range_=params.range) or None
    # Skip code actions when analysis hasn't completed yet — running
    # analyse(source) synchronously here would block the event loop
    # for the entire analysis duration.  Code actions depend on
    # diagnostics, which aren't available until analysis finishes.
    if snap is not None and snap.analysis is None:
        return None
    actions = get_code_actions(
        source,
        params.range,
        params.context,
        uri=uri,
        package_names=_state.package_resolver_for_uri(uri).all_package_names(),
        lines=snap.buffer.lines if snap and snap.buffer else None,
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


# Formatting


@server.feature(
    types.TEXT_DOCUMENT_FORMATTING,
    types.DocumentFormattingRegistrationOptions(
        document_selector=_TCL_DOCUMENT_SELECTOR,
    ),
)
@_state.scoped_to_doc
def on_formatting(
    params: types.DocumentFormattingParams,
) -> list[types.TextEdit] | None:
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    edits = get_formatting(
        source,
        params.options,
        _state.formatter_config_for_uri(uri),
        lines=snap.buffer.lines if snap and snap.buffer else None,
    )
    return edits or None


@server.feature(
    types.TEXT_DOCUMENT_RANGE_FORMATTING,
    types.DocumentRangeFormattingRegistrationOptions(
        document_selector=_TCL_DOCUMENT_SELECTOR,
    ),
)
@_state.scoped_to_doc
def on_range_formatting(
    params: types.DocumentRangeFormattingParams,
) -> list[types.TextEdit] | None:
    uri = params.text_document.uri
    snap, source = _snapshot_read(uri)
    edits = get_range_formatting(
        source,
        params.range,
        params.options,
        _state.formatter_config_for_uri(uri),
        lines=snap.buffer.lines if snap and snap.buffer else None,
    )
    return edits or None


# Format on save (fallback for editors without native format-on-save)


@server.feature(types.TEXT_DOCUMENT_WILL_SAVE_WAIT_UNTIL)
@_state.scoped_to_doc
def on_will_save_wait_until(
    params: types.WillSaveTextDocumentParams,
) -> list[types.TextEdit] | None:
    uri = params.text_document.uri
    cfg = _state.config_for_uri(uri)
    if not cfg.will_save_wait_until_enabled:
        return None
    snap, source = _snapshot_read(uri)
    from tooling.formatter.config import IndentStyle

    fmt_cfg = _state.formatter_config_for_uri(uri)
    options = types.FormattingOptions(
        tab_size=fmt_cfg.indent_size,
        insert_spaces=fmt_cfg.indent_style == IndentStyle.SPACES,
    )
    edits = get_formatting(
        source, options, fmt_cfg, lines=snap.buffer.lines if snap and snap.buffer else None
    )
    return edits or None
