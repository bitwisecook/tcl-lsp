"""Bridge between the native C++ LSP server and the Python feature code.

Each function accepts simple types (strings, ints) from C++ and returns
a JSON-serialisable dict/list or None.  The C++ server converts the result
to lsp-framework types for the JSON-RPC response.

This module is imported by the embedded Python interpreter in the C++ server.
It is NOT used by the pygls server — the existing Python server continues to
call feature functions directly.
"""

from __future__ import annotations

import json
import logging
import threading
import time

log = logging.getLogger(__name__)

# Late-bound references — set by _init() on first call.
_workspace_state = None
_workspace_index = None
_background_scanner = None
_package_resolver = None
_formatter_config = None
_feature_config = None
_diagnostic_scheduler = None
_notification_callback = None
_initialised = False
_init_lock = threading.Lock()


def _init():
    """Lazily import heavy modules to avoid circular imports."""
    global _workspace_state, _workspace_index, _background_scanner
    global _package_resolver, _formatter_config, _feature_config
    global _diagnostic_scheduler, _initialised

    if _initialised:
        return
    with _init_lock:
        if _initialised:
            return

        from core.analysis.irules_checks import DEFAULT_GENERIC_VARIABLE_PATTERNS
        from core.commands.registry.runtime import configure_signatures
        from core.common.codes import default_disabled_diagnostics
        from core.formatting import FormatterConfig
        from core.packages import PackageResolver
        from lsp.async_diagnostics import DiagnosticScheduler
        from lsp.workspace.document_state import WorkspaceState
        from lsp.workspace.scanner import BackgroundScanner
        from lsp.workspace.workspace_index import WorkspaceIndex

        # Import server module to trigger code registrations.
        import core.common.codes_all  # noqa: F401

        _workspace_state = WorkspaceState()
        _workspace_index = WorkspaceIndex()
        _background_scanner = BackgroundScanner()
        _package_resolver = PackageResolver()
        _formatter_config = FormatterConfig()
        _diagnostic_scheduler = DiagnosticScheduler()

        # Feature config with defaults.
        from dataclasses import dataclass, field

        @dataclass
        class FeatureConfig:
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
            optimiser_enabled: bool = True
            disabled_optimisations: set = field(default_factory=set)
            shimmer_enabled: bool = True
            xc_diagnostics_enabled: bool = False
            line_length: int = 120
            generic_variable_patterns: list = field(
                default_factory=lambda: list(DEFAULT_GENERIC_VARIABLE_PATTERNS)
            )
            disabled_diagnostics: set = field(
                default_factory=lambda: set(default_disabled_diagnostics())
            )
            dialect_explicitly_set: bool = False

        _feature_config = FeatureConfig()
        _initialised = True
        log.info("Python bridge initialised")


def set_notification_callback(callback):
    """Register the C++ callback for server→client notifications."""
    global _notification_callback
    _notification_callback = callback


def _publish_diagnostics(uri, diagnostics_json, version=None):
    """Called by Python diagnostic pipeline to push diagnostics to the client."""
    if _notification_callback is not None:
        params = {"uri": uri, "diagnostics": diagnostics_json}
        if version is not None:
            params["version"] = version
        _notification_callback("textDocument/publishDiagnostics", json.dumps(params))


def _lsp_obj_to_dict(obj):
    """Convert an lsprotocol object to a JSON-serialisable dict."""
    if obj is None:
        return None
    if isinstance(obj, (str, int, float, bool)):
        return obj
    if isinstance(obj, (list, tuple)):
        return [_lsp_obj_to_dict(item) for item in obj]
    if isinstance(obj, dict):
        return {k: _lsp_obj_to_dict(v) for k, v in obj.items()}
    # lsprotocol types have __dict__ or cattrs-based serialisation.
    try:
        from lsprotocol.converters import get_converter
        converter = get_converter()
        return converter.unstructure(obj)
    except Exception:
        pass
    if hasattr(obj, "__dict__"):
        return {k: _lsp_obj_to_dict(v) for k, v in obj.__dict__.items()
                if not k.startswith("_") and v is not None}
    return obj


# Document sync

def on_did_open(uri: str, language_id: str, text: str, version: int):
    """Handle textDocument/didOpen."""
    _init()
    _workspace_state.open(uri, text, version, language_id=language_id,
                          analyse=False)
    _trigger_diagnostics(uri, text, version)


def on_did_change(uri: str, text: str, version: int):
    """Handle textDocument/didChange."""
    _init()
    state = _workspace_state.get(uri)
    if state is not None:
        state.update_source_quick(text, version)
    _trigger_diagnostics(uri, text, version)


def on_did_close(uri: str):
    """Handle textDocument/didClose."""
    _init()
    _diagnostic_scheduler.cancel(uri)
    _workspace_state.close(uri)
    _workspace_index.remove(uri)
    _publish_diagnostics(uri, [], None)


def _trigger_diagnostics(uri: str, source: str, version: int | None):
    """Run analysis and publish diagnostics (synchronous for now)."""
    _init()
    state = _workspace_state.update(
        uri, source, version,
        line_length=_feature_config.line_length,
    )

    if not _feature_config.diagnostics_enabled:
        _publish_diagnostics(uri, [], version)
        return

    from lsp.features.diagnostics import get_basic_diagnostics
    basic_diags, _, _ = get_basic_diagnostics(
        source,
        analysis=state.analysis,
        cu=state.compilation_unit,
        optimiser_enabled=_feature_config.optimiser_enabled,
        disabled_diagnostics=_feature_config.disabled_diagnostics,
        disabled_optimisations=_feature_config.disabled_optimisations,
        line_length=_feature_config.line_length,
    )
    diag_dicts = [_lsp_obj_to_dict(d) for d in basic_diags]
    _publish_diagnostics(uri, diag_dicts, version)

    # Update workspace index.
    from lsp.workspace.workspace_index import EntrySource
    if state.analysis and not state.has_partial_commands:
        _workspace_index.update(uri, state.analysis, EntrySource.OPEN)


def on_did_change_configuration(settings_json: str):
    """Handle workspace/didChangeConfiguration."""
    _init()
    # TODO: parse settings and update _feature_config


def on_initialized(workspace_folders_json: str, settings_json: str):
    """Handle initialized notification — start workspace scanning."""
    _init()
    # TODO: parse workspace folders and start background scanning


# Feature handlers — each returns a JSON string or None.

def handle_feature(method: str, params_json: str) -> str | None:
    """Dispatch an LSP feature request to the appropriate Python handler."""
    _init()
    params = json.loads(params_json)
    result = _dispatch_feature(method, params)
    if result is None:
        return None
    return json.dumps(_lsp_obj_to_dict(result))


def _get_doc_source(uri: str) -> str:
    """Get document source text from workspace state."""
    state = _workspace_state.get(uri)
    if state is not None:
        return state.source
    return ""


def _dispatch_feature(method: str, params: dict):
    """Route a feature request to its Python handler."""
    uri = _extract_uri(params)
    source = _get_doc_source(uri) if uri else ""
    state = _workspace_state.get(uri) if uri else None
    analysis = state.analysis if state else None
    lines = state.lines if state else None

    match method:
        case "textDocument/semanticTokens/full":
            if not _feature_config.semantic_tokens_enabled:
                return {"data": []}
            from lsp.features.semantic_tokens import semantic_tokens_full
            data = semantic_tokens_full(source, analysis=analysis)
            return {"data": data}

        case "textDocument/completion":
            if not _feature_config.completion_enabled:
                return []
            from lsp.features.completion import get_completions
            line = params.get("position", {}).get("line", 0)
            char = params.get("position", {}).get("character", 0)
            return get_completions(
                source, line, char,
                analysis=analysis,
                workspace_procs=_workspace_index.all_proc_names(),
                workspace_rule_init_vars=_workspace_index.all_rule_init_var_names(),
                workspace_command_usage=_workspace_index.command_usage_counts(),
                workspace_proc_usage=_workspace_index.proc_usage_counts(),
                formatter_config=_formatter_config,
                lines=lines,
            )

        case "textDocument/hover":
            if not _feature_config.hover_enabled:
                return None
            from lsp.features.hover import get_hover
            line = params.get("position", {}).get("line", 0)
            char = params.get("position", {}).get("character", 0)
            return get_hover(source, line, char, analysis=analysis,
                             lines=lines)

        case "textDocument/definition":
            if not _feature_config.definition_enabled:
                return []
            from lsp.features.definition import get_definition
            line = params.get("position", {}).get("line", 0)
            char = params.get("position", {}).get("character", 0)
            return get_definition(source, uri, line, char,
                                  analysis=analysis)

        case "textDocument/references":
            if not _feature_config.references_enabled:
                return []
            from lsp.features.references import get_references
            line = params.get("position", {}).get("line", 0)
            char = params.get("position", {}).get("character", 0)
            ctx = params.get("context", {})
            include_decl = ctx.get("includeDeclaration", True)
            return get_references(source, uri, line, char,
                                  analysis=analysis,
                                  include_declaration=include_decl)

        case "textDocument/documentSymbol":
            if not _feature_config.document_symbols_enabled:
                return []
            from lsp.features.document_symbols import get_document_symbols
            return get_document_symbols(source, analysis=analysis)

        case "textDocument/foldingRange":
            if not _feature_config.folding_enabled:
                return []
            from lsp.features.folding import get_folding_ranges
            return get_folding_ranges(source, analysis=analysis,
                                      lines=lines)

        case "textDocument/rename":
            if not _feature_config.rename_enabled:
                return None
            from lsp.features.rename import get_rename_edits
            line = params.get("position", {}).get("line", 0)
            char = params.get("position", {}).get("character", 0)
            new_name = params.get("newName", "")
            return get_rename_edits(source, uri, line, char, new_name,
                                    analysis=analysis)

        case "textDocument/prepareRename":
            if not _feature_config.rename_enabled:
                return None
            from lsp.features.rename import prepare_rename
            line = params.get("position", {}).get("line", 0)
            char = params.get("position", {}).get("character", 0)
            return prepare_rename(source, uri, line, char,
                                  analysis=analysis)

        case "textDocument/signatureHelp":
            if not _feature_config.signature_help_enabled:
                return None
            from lsp.features.signature_help import get_signature_help
            line = params.get("position", {}).get("line", 0)
            char = params.get("position", {}).get("character", 0)
            return get_signature_help(source, line, char,
                                      analysis=analysis)

        case "textDocument/formatting":
            if not _feature_config.formatting_enabled:
                return None
            from lsp.features.formatting import get_formatting
            from lsprotocol.types import FormattingOptions
            opts = params.get("options", {})
            fmt_opts = FormattingOptions(
                tab_size=opts.get("tabSize", 4),
                insert_spaces=opts.get("insertSpaces", True),
            )
            return get_formatting(source, fmt_opts, _formatter_config,
                                  lines=lines)

        case "textDocument/rangeFormatting":
            if not _feature_config.formatting_enabled:
                return None
            from lsp.features.formatting import get_range_formatting
            from lsprotocol.converters import get_converter
            from lsprotocol.types import DocumentRangeFormattingParams
            converter = get_converter()
            full_params = converter.structure(params,
                                             DocumentRangeFormattingParams)
            return get_range_formatting(
                source, full_params.range, full_params.options,
                _formatter_config, lines=lines)

        case "textDocument/codeAction":
            if not _feature_config.code_actions_enabled:
                return None
            from lsp.features.code_actions import get_code_actions
            from lsprotocol.converters import get_converter
            from lsprotocol.types import CodeActionParams
            converter = get_converter()
            full_params = converter.structure(params, CodeActionParams)
            actions = get_code_actions(
                source, full_params.range, full_params.context,
                package_names=_package_resolver.all_package_names(),
                lines=lines)
            # Remap __current__ URI.
            for action in actions:
                if (action.edit and action.edit.changes
                        and "__current__" in action.edit.changes):
                    remapped = {}
                    for edit_uri, edits in action.edit.changes.items():
                        if edit_uri == "__current__":
                            continue
                        remapped[edit_uri] = list(edits)
                    remapped[uri] = list(action.edit.changes["__current__"])
                    action.edit.changes = remapped
            return actions or None

        case "workspace/symbol":
            if not _feature_config.workspace_symbols_enabled:
                return []
            from lsp.features.workspace_symbols import get_workspace_symbols
            query = params.get("query", "")
            return get_workspace_symbols(query, _workspace_index)

        case "textDocument/inlayHint":
            if not _feature_config.inlay_hints_enabled:
                return []
            from lsp.features.inlay_hints import get_inlay_hints
            from lsprotocol.converters import get_converter
            from lsprotocol.types import Range as LspRange
            converter = get_converter()
            rng = converter.structure(params.get("range", {}), LspRange)
            return get_inlay_hints(source, rng, analysis=analysis,
                                   lines=lines)

        case "textDocument/prepareCallHierarchy":
            if not _feature_config.call_hierarchy_enabled:
                return []
            from lsp.features.call_hierarchy import (
                prepare_call_hierarchy as get_call_hierarchy,
            )
            line = params.get("position", {}).get("line", 0)
            char = params.get("position", {}).get("character", 0)
            return get_call_hierarchy(source, uri, line, char,
                                      analysis=analysis)

        case "callHierarchy/incomingCalls":
            if not _feature_config.call_hierarchy_enabled:
                return []
            from lsp.features.call_hierarchy import (
                incoming_calls as get_incoming_calls,
            )
            from lsprotocol.converters import get_converter
            from lsprotocol.types import CallHierarchyItem
            converter = get_converter()
            item = converter.structure(params.get("item", {}),
                                       CallHierarchyItem)
            item_uri = item.uri
            item_source = _get_doc_source(item_uri)
            item_state = _workspace_state.get(item_uri)
            item_analysis = item_state.analysis if item_state else None
            return get_incoming_calls(item, item_source, item_uri,
                                      analysis=item_analysis)

        case "callHierarchy/outgoingCalls":
            if not _feature_config.call_hierarchy_enabled:
                return []
            from lsp.features.call_hierarchy import (
                outgoing_calls as get_outgoing_calls,
            )
            from lsprotocol.converters import get_converter
            from lsprotocol.types import CallHierarchyItem
            converter = get_converter()
            item = converter.structure(params.get("item", {}),
                                       CallHierarchyItem)
            item_uri = item.uri
            item_source = _get_doc_source(item_uri)
            item_state = _workspace_state.get(item_uri)
            item_analysis = item_state.analysis if item_state else None
            return get_outgoing_calls(item, item_source, item_uri,
                                      analysis=item_analysis)

        case "textDocument/documentLink":
            if not _feature_config.document_links_enabled:
                return []
            from lsp.features.document_links import get_document_links
            return get_document_links(source, analysis=analysis)

        case "textDocument/selectionRange":
            if not _feature_config.selection_range_enabled:
                return None
            from lsp.features.selection_range import get_selection_ranges
            from lsprotocol.converters import get_converter
            from lsprotocol.types import Position
            converter = get_converter()
            positions = [converter.structure(p, Position)
                         for p in params.get("positions", [])]
            return get_selection_ranges(source, positions,
                                        analysis=analysis, lines=lines)

        case "textDocument/semanticTokens/full/delta":
            # Delta is handled in C++ via previous result caching.
            # Fall back to full for now.
            if not _feature_config.semantic_tokens_enabled:
                return {"data": []}
            from lsp.features.semantic_tokens import semantic_tokens_full
            data = semantic_tokens_full(source, analysis=analysis)
            return {"data": data}

        case _:
            log.warning("Unhandled feature method: %s", method)
            return None


def _extract_uri(params: dict) -> str:
    """Extract the document URI from LSP params."""
    td = params.get("textDocument")
    if isinstance(td, dict):
        return td.get("uri", "")
    return ""


def handle_command(command: str, arguments_json: str) -> str | None:
    """Execute a custom workspace command."""
    _init()
    args = json.loads(arguments_json) if arguments_json else []
    result = _dispatch_command(command, args)
    if result is None:
        return None
    return json.dumps(result)


def _dispatch_command(command: str, args: list):
    """Route a custom command to its Python handler."""
    match command:
        case "tcl-lsp.optimiseDocument":
            from core.compiler.optimiser import optimise_source
            uri = args[0] if args else ""
            source = _get_doc_source(uri)
            optimised, opts = optimise_source(source)
            items = []
            for o in opts:
                item = {
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
            return {"optimisations": items, "source": optimised}

        case _:
            log.warning("Unhandled command: %s", command)
            return None
