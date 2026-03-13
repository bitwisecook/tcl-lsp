"""MCP server for Tcl/iRules analysis.

Exposes the tcl-lsp analysis engine as MCP tools over stdio.
Each tool accepts source code directly and returns structured JSON.

Implements the MCP protocol (JSON-RPC 2.0 over stdio) directly — no heavy
SDK, no pydantic, no C extensions.  The entire server is pure Python so it
runs cleanly from a zipapp.

Usage:
    python -m ai.mcp.tcl_mcp_server          # dev mode
    python tcl-lsp-mcp-server.pyz              # zipapp mode
"""

from __future__ import annotations

import json
import re
import sys
from typing import Any

from ai.shared.diagnostics import (
    CATEGORY_ORDER,
    CONVERSION_MAP,
    CONVERTIBLE_CODES,
    IRULES_EVENT_PATTERN,
    REVIEW_CODES,
    SECURITY_CODES,
    TAINT_CODES,
    THREAD_CODES,
    categorise,
)

# Lightweight MCP protocol layer (JSON-RPC 2.0 over stdio)

_MCP_PROTOCOL_VERSION = "2024-11-05"


def _read_message() -> dict | None:
    """Read one JSON-RPC message from stdin (newline-delimited)."""
    line = sys.stdin.readline()
    if not line:
        return None
    line = line.strip()
    if not line:
        return None
    return json.loads(line)


def _write_message(msg: dict) -> None:
    """Write one JSON-RPC message to stdout (newline-delimited)."""
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def _success(id: Any, result: Any) -> dict:
    return {"jsonrpc": "2.0", "id": id, "result": result}


def _error(id: Any, code: int, message: str) -> dict:
    return {"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}


# Tool registry

_TOOLS: dict[str, dict] = {}  # name -> {"description": ..., "schema": ..., "handler": callable}


def tool(
    name: str,
    description: str,
    params: dict[str, dict],
    required: list[str] | None = None,
):
    """Decorator to register an MCP tool."""

    def decorator(fn):
        _TOOLS[name] = {
            "description": description,
            "schema": {
                "type": "object",
                "properties": params,
                "required": required or [],
            },
            "handler": fn,
        }
        return fn

    return decorator


# MCP request handlers


def _handle_initialize(params: dict) -> dict:
    return {
        "protocolVersion": _MCP_PROTOCOL_VERSION,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "tcl-lsp", "version": _get_version()},
    }


def _handle_tools_list(params: dict) -> dict:
    tools = []
    for name, info in _TOOLS.items():
        tools.append(
            {
                "name": name,
                "description": info["description"],
                "inputSchema": info["schema"],
            }
        )
    return {"tools": tools}


def _handle_tools_call(params: dict) -> dict:
    name = params.get("name", "")
    arguments = params.get("arguments", {})

    if name not in _TOOLS:
        return {
            "content": [{"type": "text", "text": json.dumps({"error": f"Unknown tool: {name}"})}],
            "isError": True,
        }

    handler = _TOOLS[name]["handler"]
    try:
        result = handler(**arguments)
        return {"content": [{"type": "text", "text": result}]}
    except Exception as e:
        return {
            "content": [
                {
                    "type": "text",
                    "text": json.dumps({"error": str(e), "error_type": type(e).__name__}),
                }
            ],
            "isError": True,
        }


_DISPATCH = {
    "initialize": _handle_initialize,
    "tools/list": _handle_tools_list,
    "tools/call": _handle_tools_call,
}


def _get_version() -> str:
    try:
        from lsp._build_info import FULL_VERSION

        return FULL_VERSION
    except ImportError:
        return "dev"


def run_stdio() -> None:
    """Run the MCP server on stdin/stdout."""
    while True:
        msg = _read_message()
        if msg is None:
            break

        method = msg.get("method", "")
        msg_id = msg.get("id")
        params = msg.get("params", {})

        # Notifications (no id) — handle silently
        if msg_id is None:
            if method == "notifications/initialized":
                pass  # Client acknowledged initialization
            continue

        handler = _DISPATCH.get(method)
        if handler is None:
            if method == "ping":
                _write_message(_success(msg_id, {}))
            else:
                _write_message(_error(msg_id, -32601, f"Method not found: {method}"))
            continue

        try:
            result = handler(params)
            _write_message(_success(msg_id, result))
        except Exception as e:
            _write_message(_error(msg_id, -32603, str(e)))


# Session state

_session_dialect: str = "tcl8.6"

_IRULES_PATTERN = re.compile(r"^\s*when\s+[A-Z][A-Z0-9_]{2,}\s*\{", re.MULTILINE)

# Aliases — keep the underscore-prefixed names used throughout this module
_SECURITY_CODES = SECURITY_CODES
_TAINT_CODES = TAINT_CODES
_THREAD_CODES = THREAD_CODES
_CONVERTIBLE_CODES = CONVERTIBLE_CODES
_REVIEW_CODES = REVIEW_CODES
_CONVERSION_MAP = CONVERSION_MAP
_CATEGORY_ORDER = CATEGORY_ORDER
_categorise = categorise


def _detect_dialect(source: str) -> str:
    """Guess dialect from source content."""
    if _IRULES_PATTERN.search(source):
        return "f5-irules"
    return _session_dialect


def _configure_dialect(dialect: str | None = None) -> None:
    """Configure the registry for the given dialect."""
    from core.commands.registry.runtime import configure_signatures

    configure_signatures(dialect=dialect or _session_dialect)


# Serialization helpers


def _range_to_dict(r: Any) -> dict:
    """Convert a semantic_model.Range to a dict."""
    return {
        "start": {"line": r.start.line, "character": r.start.character},
        "end": {"line": r.end.line, "character": r.end.character},
    }


def _lsp_range_to_dict(r: Any) -> dict:
    """Convert an lsprotocol.types.Range to a dict."""
    return {
        "start": {"line": r.start.line, "character": r.start.character},
        "end": {"line": r.end.line, "character": r.end.character},
    }


def _diagnostic_to_dict(d: Any) -> dict:
    """Convert a semantic_model.Diagnostic to a dict."""
    result: dict[str, Any] = {
        "code": d.code or "",
        "severity": d.severity.name.lower(),
        "message": d.message,
        "range": _range_to_dict(d.range),
        "category": _categorise(d.code or ""),
    }
    if d.fixes:
        result["fixes"] = [
            {
                "range": _range_to_dict(f.range),
                "new_text": f.new_text,
                "description": f.description,
            }
            for f in d.fixes
        ]
    return result


def _lsp_diagnostic_to_dict(d: Any) -> dict:
    """Convert an lsprotocol.types.Diagnostic to a dict."""
    code = d.code if d.code else ""
    return {
        "code": code,
        "severity": d.severity.name.lower() if d.severity else "error",
        "message": d.message,
        "range": _lsp_range_to_dict(d.range),
    }


def _proc_to_dict(proc_def: Any) -> dict:
    """Serialize a ProcDef."""
    params = []
    for p in proc_def.params:
        param: dict[str, Any] = {"name": p.name}
        if p.has_default:
            param["default"] = p.default_value
        params.append(param)
    result: dict[str, Any] = {
        "name": proc_def.name,
        "qualified_name": proc_def.qualified_name,
        "params": params,
    }
    if proc_def.name_range:
        result["range"] = _range_to_dict(proc_def.name_range)
    if proc_def.doc:
        result["doc"] = proc_def.doc
    return result


def _scope_symbols(scope: Any, depth: int = 0) -> list[dict]:
    """Recursively collect symbols from a scope."""
    symbols: list[dict] = []
    for proc in scope.procs.values():
        sym = _proc_to_dict(proc)
        sym["kind"] = "function"
        symbols.append(sym)
    if scope.kind in ("global", "namespace"):
        for var in scope.variables.values():
            if var.definition_range:
                symbols.append(
                    {
                        "kind": "variable",
                        "name": var.name,
                        "range": _range_to_dict(var.definition_range),
                        "references": len(var.references),
                    }
                )
    for child in scope.children:
        if child.kind == "namespace" and child.body_range:
            ns: dict[str, Any] = {
                "kind": "namespace",
                "name": child.name,
                "range": _range_to_dict(child.body_range),
                "children": _scope_symbols(child, depth + 1),
            }
            symbols.append(ns)
    return symbols


def _detect_events(source: str) -> list[dict]:
    """Detect iRule events and their line numbers."""
    events = []
    seen: set[str] = set()
    for match in IRULES_EVENT_PATTERN.finditer(source):
        name = match.group(1)
        if name not in seen:
            seen.add(name)
            line = source[: match.start()].count("\n")
            events.append({"name": name, "line": line})
    return events


def _lsp_location_to_dict(loc: Any) -> dict:
    """Convert lsprotocol.types.Location to a dict."""
    return {"uri": loc.uri, "range": _lsp_range_to_dict(loc.range)}


def _completion_item_to_dict(item: Any) -> dict:
    """Convert lsprotocol.types.CompletionItem to a dict."""
    result: dict[str, Any] = {"label": item.label}
    if item.kind is not None:
        result["kind"] = item.kind.name.lower()
    if item.detail:
        result["detail"] = item.detail
    if item.documentation:
        if hasattr(item.documentation, "value"):
            result["documentation"] = item.documentation.value
        else:
            result["documentation"] = str(item.documentation)
    if item.insert_text:
        result["insert_text"] = item.insert_text
    if item.sort_text:
        result["sort_text"] = item.sort_text
    return result


def _document_symbol_to_dict(sym: Any) -> dict:
    """Convert lsprotocol.types.DocumentSymbol to a dict."""
    result: dict[str, Any] = {
        "name": sym.name,
        "kind": sym.kind.name.lower() if sym.kind else "unknown",
        "range": _lsp_range_to_dict(sym.range),
        "selection_range": _lsp_range_to_dict(sym.selection_range),
    }
    if sym.detail:
        result["detail"] = sym.detail
    if sym.children:
        result["children"] = [_document_symbol_to_dict(c) for c in sym.children]
    return result


def _code_action_to_dict(action: Any) -> dict:
    """Convert lsprotocol.types.CodeAction to a dict."""
    result: dict[str, Any] = {"title": action.title}
    if action.kind:
        result["kind"] = action.kind.value if hasattr(action.kind, "value") else str(action.kind)
    if action.edit and action.edit.changes:
        edits: list[dict] = []
        for uri, text_edits in action.edit.changes.items():
            for te in text_edits:
                edits.append(
                    {
                        "uri": uri,
                        "range": _lsp_range_to_dict(te.range),
                        "new_text": te.new_text,
                    }
                )
        result["edits"] = edits
    if action.diagnostics:
        result["diagnostics"] = [_lsp_diagnostic_to_dict(d) for d in action.diagnostics]
    if action.command:
        result["command"] = {
            "title": action.command.title,
            "command": action.command.command,
            "arguments": action.command.arguments or [],
        }
    return result


def _text_edit_to_dict(te: Any) -> dict:
    """Convert lsprotocol.types.TextEdit to a dict."""
    return {"range": _lsp_range_to_dict(te.range), "new_text": te.new_text}


# Analysis tools

_STR = {"type": "string"}
_INT = {"type": "integer"}


@tool(
    "analyze",
    "Full analysis of Tcl/iRules source: diagnostics, symbols, events, and event metadata.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code to analyze"},
        "dialect": {
            **_STR,
            "description": "Language dialect (tcl8.6, f5-irules, etc.). Auto-detected if empty.",
        },
    },
    required=["source"],
)
def _tool_analyze(source: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from ai.shared.irule_analysis import ordered_events_as_dicts
    from core.analysis.analyser import analyse

    result = analyse(source)
    diags = [_diagnostic_to_dict(d) for d in result.diagnostics]
    symbols = _scope_symbols(result.global_scope)
    events = _detect_events(source)

    return json.dumps(
        {
            "diagnostics": diags,
            "diagnostic_count": len(diags),
            "symbols": symbols,
            "events": events,
            "event_order": ordered_events_as_dicts(source),
        }
    )


@tool(
    "validate",
    "Categorized validation report: errors, security, taint, performance, style, etc.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code to validate"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source"],
)
def _tool_validate(source: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from core.analysis.analyser import analyse

    result = analyse(source)
    groups: dict[str, list[dict]] = {}
    for d in result.diagnostics:
        code = d.code or ""
        cat = _categorise(code)
        groups.setdefault(cat, []).append(_diagnostic_to_dict(d))

    categories = {}
    for cat_key, cat_label in _CATEGORY_ORDER:
        if cat_key in groups:
            categories[cat_key] = {"label": cat_label, "items": groups[cat_key]}

    return json.dumps({"categories": categories, "total": len(result.diagnostics)})


@tool(
    "review",
    "Security-focused analysis: security warnings, taint tracking, thread safety.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code to review"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source"],
)
def _tool_review(source: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from core.analysis.analyser import analyse

    result = analyse(source)
    security = [
        _diagnostic_to_dict(d) for d in result.diagnostics if (d.code or "") in _SECURITY_CODES
    ]
    taint = [_diagnostic_to_dict(d) for d in result.diagnostics if (d.code or "") in _TAINT_CODES]
    thread = [_diagnostic_to_dict(d) for d in result.diagnostics if (d.code or "") in _THREAD_CODES]
    total = len(security) + len(taint) + len(thread)

    return json.dumps(
        {"security": security, "taint": taint, "thread_safety": thread, "total": total}
    )


@tool(
    "convert",
    "Detect legacy patterns eligible for modernisation.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code to scan"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source"],
)
def _tool_convert(source: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from core.analysis.analyser import analyse

    result = analyse(source)
    patterns = []
    for d in result.diagnostics:
        code = d.code or ""
        if code in _CONVERTIBLE_CODES:
            entry = _diagnostic_to_dict(d)
            entry["conversion"] = _CONVERSION_MAP.get(code, "modernise")
            patterns.append(entry)

    return json.dumps({"patterns": patterns, "total": len(patterns)})


@tool(
    "optimize",
    "Find optimization opportunities and produce rewritten source.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code to optimize"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source"],
)
def _tool_optimize(source: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from core.compiler.optimiser import apply_optimisations, find_optimisations

    opts = find_optimisations(source)
    items = []
    for o in opts:
        item: dict = {
            "code": o.code,
            "message": o.message,
            "range": _range_to_dict(o.range),
            "replacement": o.replacement,
        }
        if o.group is not None:
            item["group"] = o.group
        if o.hint_only:
            item["hintOnly"] = True
        items.append(item)

    # Build grouped view: each group is a single logical optimisation.
    groups: dict[int, list[dict]] = {}
    ungrouped: list[dict] = []
    for item in items:
        gid = item.get("group")
        if gid is not None:
            groups.setdefault(gid, []).append(item)
        else:
            ungrouped.append(item)
    grouped_items: list[dict] = []
    _ELIM_CODES = frozenset(("O107", "O108", "O109"))
    for gid, members in sorted(groups.items()):
        primary = next((m for m in members if m["code"] not in _ELIM_CODES), members[0])
        grouped_items.append(
            {
                "code": primary["code"],
                "message": primary["message"],
                "range": primary["range"],
                "replacement": primary["replacement"],
                "group": gid,
                "members": members,
            }
        )
    grouped_items.extend(ungrouped)

    optimized = apply_optimisations(source, opts) if opts else source
    return json.dumps(
        {
            "optimizations": grouped_items,
            "total": len(grouped_items),
            "optimized_source": optimized,
            "changed": optimized != source,
        }
    )


# LSP-equivalent tools

_SYNTHETIC_URI = "file:///source.tcl"


@tool(
    "hover",
    "Get hover information (docs, type info, taint) for a position.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "line": {**_INT, "description": "0-based line number"},
        "character": {**_INT, "description": "0-based character offset"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source", "line", "character"],
)
def _tool_hover(source: str, line: int, character: int, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from lsp.features.hover import get_hover

    result = get_hover(source, line, character)
    if result is None:
        return json.dumps(None)
    value = result.contents.value if hasattr(result.contents, "value") else str(result.contents)
    return json.dumps({"contents": value})


@tool(
    "complete",
    "Get completions at a position (commands, variables, procs, switches).",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "line": {**_INT, "description": "0-based line number"},
        "character": {**_INT, "description": "0-based character offset"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source", "line", "character"],
)
def _tool_complete(source: str, line: int, character: int, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from lsp.features.completion import get_completions

    items = get_completions(source, line, character)
    return json.dumps({"items": [_completion_item_to_dict(i) for i in items], "total": len(items)})


@tool(
    "goto_definition",
    "Find definition of the symbol (proc or variable) at a position.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "line": {**_INT, "description": "0-based line number"},
        "character": {**_INT, "description": "0-based character offset"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source", "line", "character"],
)
def _tool_goto_definition(source: str, line: int, character: int, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from lsp.features.definition import get_definition

    locations = get_definition(source, _SYNTHETIC_URI, line, character)
    return json.dumps({"locations": [_lsp_location_to_dict(loc) for loc in locations]})


@tool(
    "find_references",
    "Find all references to the symbol at a position.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "line": {**_INT, "description": "0-based line number"},
        "character": {**_INT, "description": "0-based character offset"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source", "line", "character"],
)
def _tool_find_references(source: str, line: int, character: int, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from lsp.features.references import get_references

    refs = get_references(source, _SYNTHETIC_URI, line, character)
    return json.dumps({"references": [_lsp_location_to_dict(r) for r in refs], "total": len(refs)})


@tool(
    "symbols",
    "Get document symbol hierarchy (procs, variables, namespaces, events).",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source"],
)
def _tool_symbols(source: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from lsp.features.document_symbols import get_document_symbols

    syms = get_document_symbols(source)
    return json.dumps({"symbols": [_document_symbol_to_dict(s) for s in syms]})


@tool(
    "code_actions",
    "Get available code actions (quick fixes, refactorings) for a range.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "start_line": {**_INT, "description": "0-based start line"},
        "start_character": {**_INT, "description": "0-based start character"},
        "end_line": {**_INT, "description": "0-based end line"},
        "end_character": {**_INT, "description": "0-based end character"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source", "start_line", "start_character", "end_line", "end_character"],
)
def _tool_code_actions(
    source: str,
    start_line: int,
    start_character: int,
    end_line: int,
    end_character: int,
    dialect: str = "",
) -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from lsprotocol import types as lsp

    from lsp.features.code_actions import get_code_actions
    from lsp.features.diagnostics import get_diagnostics

    range_ = lsp.Range(
        start=lsp.Position(line=start_line, character=start_character),
        end=lsp.Position(line=end_line, character=end_character),
    )

    # Get LSP diagnostics to populate the context
    lsp_diags = get_diagnostics(source)

    # Filter to those overlapping the requested range
    overlap = []
    for d in lsp_diags:
        dr = d.range
        if dr.end.line < start_line or dr.start.line > end_line:
            continue
        if dr.end.line == start_line and dr.end.character < start_character:
            continue
        if dr.start.line == end_line and dr.start.character > end_character:
            continue
        overlap.append(d)

    context = lsp.CodeActionContext(diagnostics=overlap)
    actions = get_code_actions(source, range_, context)
    return json.dumps({"actions": [_code_action_to_dict(a) for a in actions]})


@tool(
    "format_source",
    "Format Tcl/iRules source code.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code to format"},
        "indent_size": {**_INT, "description": "Spaces per indent level (default 4)"},
        "indent_style": {**_STR, "description": "'spaces' or 'tabs' (default 'spaces')"},
        "brace_style": {**_STR, "description": "'k_and_r' (default 'k_and_r')"},
        "max_line_length": {**_INT, "description": "Maximum line length (default 120)"},
    },
    required=["source"],
)
def _tool_format_source(
    source: str,
    indent_size: int = 4,
    indent_style: str = "spaces",
    brace_style: str = "k_and_r",
    max_line_length: int = 120,
) -> str:
    from core.formatting import format_tcl
    from core.formatting.config import BraceStyle, FormatterConfig, IndentStyle

    config = FormatterConfig(
        indent_size=indent_size,
        indent_style=IndentStyle.TABS if indent_style.lower() == "tabs" else IndentStyle.SPACES,
        brace_style=BraceStyle.K_AND_R,
        max_line_length=max_line_length,
    )
    formatted = format_tcl(source, config)
    return json.dumps({"formatted": formatted, "changed": formatted != source})


@tool(
    "rename",
    "Rename a symbol (proc or variable) at a position.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "line": {**_INT, "description": "0-based line number of the symbol"},
        "character": {**_INT, "description": "0-based character offset of the symbol"},
        "new_name": {**_STR, "description": "The new name for the symbol"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source", "line", "character", "new_name"],
)
def _tool_rename(source: str, line: int, character: int, new_name: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from lsp.features.rename import get_rename_edits

    result = get_rename_edits(source, _SYNTHETIC_URI, line, character, new_name)
    if result is None:
        return json.dumps(None)
    edits: list[dict] = []
    if result.changes:
        for uri, text_edits in result.changes.items():
            for te in text_edits:
                edits.append(_text_edit_to_dict(te))
    return json.dumps({"edits": edits, "total": len(edits)})


# Registry tools


@tool(
    "event_info",
    "Look up iRules event metadata: valid commands, deprecation status, properties.",
    params={"event_name": {**_STR, "description": "iRules event name (e.g. HTTP_REQUEST)"}},
    required=["event_name"],
)
def _tool_event_info(event_name: str) -> str:
    _configure_dialect("f5-irules")

    from core.commands.registry.info import lookup_event_info

    info = lookup_event_info(event_name, dialect="f5-irules")

    return json.dumps(
        {
            "event": info.event,
            "known": info.known,
            "deprecated": info.deprecated,
            "valid_commands": list(info.valid_commands),
            "valid_command_count": info.valid_command_count,
        }
    )


@tool(
    "command_info",
    "Look up iRules/Tcl command metadata: synopsis, switches, valid events.",
    params={"command_name": {**_STR, "description": "Command name (e.g. HTTP::uri, string, set)"}},
    required=["command_name"],
)
def _tool_command_info(command_name: str) -> str:
    _configure_dialect("f5-irules")

    from core.commands.registry.info import lookup_command_info

    info = lookup_command_info(command_name, dialect="f5-irules")
    if not info.found:
        return json.dumps({"command": command_name, "found": False})

    result: dict[str, Any] = {"command": info.command, "found": True}
    if info.summary:
        result["summary"] = info.summary
    if info.synopsis:
        result["synopsis"] = list(info.synopsis)
    if info.switches:
        result["switches"] = list(info.switches)
    if info.valid_events:
        result["valid_events"] = list(info.valid_events)

    return json.dumps(result)


@tool(
    "event_order",
    "Show iRules events in their canonical firing order with multiplicity.",
    params={"source": {**_STR, "description": "iRules source code containing when blocks"}},
    required=["source"],
)
def _tool_event_order(source: str) -> str:
    _configure_dialect("f5-irules")

    from ai.shared.irule_analysis import ordered_events_as_dicts

    events = ordered_events_as_dicts(source)
    return json.dumps({"events": events, "total": len(events)})


@tool(
    "diagram",
    "Extract control flow diagram data from compiler IR.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source"],
)
def _tool_diagram(source: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from ai.shared.irule_analysis import diagram_data

    data = diagram_data(source)
    return json.dumps(data)


# Semantic graph tools


@tool(
    "call_graph",
    "Build the full call graph: procs as nodes, call edges, entry-point roots, leaf procs.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source"],
)
def _tool_call_graph(source: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from core.analysis.semantic_graph import build_call_graph

    return json.dumps(build_call_graph(source))


@tool(
    "symbol_graph",
    "Build the symbol relationship graph: scope hierarchy, proc/variable definitions, references, package dependencies.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source"],
)
def _tool_symbol_graph(source: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from core.analysis.semantic_graph import build_symbol_graph

    return json.dumps(build_symbol_graph(source))


@tool(
    "dataflow_graph",
    "Build the data-flow graph: taint warnings, tainted variables, per-proc side-effect classification.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source"],
)
def _tool_dataflow_graph(source: str, dialect: str = "") -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from core.analysis.semantic_graph import build_dataflow_graph

    return json.dumps(build_dataflow_graph(source))


# Configuration


@tool(
    "xc_translate",
    "Translate an iRule to F5 Distributed Cloud (XC) configuration. "
    "Maps iRule constructs to XC L7 routes, service policies, origin pools, and header processing. "
    "Returns both the translated config and a coverage report of translatable vs untranslatable constructs.",
    params={
        "source": {**_STR, "description": "iRules source code to translate"},
        "output_format": {
            **_STR,
            "description": "Output format: 'terraform', 'json', or 'both' (default: 'both')",
        },
    },
    required=["source"],
)
def _tool_xc_translate(source: str, output_format: str = "both") -> str:
    _configure_dialect("f5-irules")

    from core.xc.json_api import render_json
    from core.xc.terraform import render_terraform
    from core.xc.translator import translate_irule

    result = translate_irule(source)

    output: dict[str, Any] = {}

    if output_format in ("terraform", "both"):
        output["terraform"] = render_terraform(result)
    if output_format in ("json", "both"):
        output["json_api"] = render_json(result)

    output["coverage_pct"] = result.coverage_pct
    output["translatable"] = [
        {
            "command": i.irule_command,
            "kind": i.kind.name.lower(),
            "xc_description": i.xc_description,
        }
        for i in result.items
        if i.status.name == "TRANSLATED"
    ]
    output["untranslatable"] = [
        {
            "command": i.irule_command,
            "reason": i.xc_description,
            "suggestion": i.note,
            "diagnostic_code": i.diagnostic_code,
        }
        for i in result.items
        if i.status.name == "UNTRANSLATABLE"
    ]
    output["partial"] = [
        {
            "command": i.irule_command,
            "reason": i.xc_description,
            "suggestion": i.note,
            "diagnostic_code": i.diagnostic_code,
        }
        for i in result.items
        if i.status.name == "PARTIAL"
    ]
    output["advisory"] = [
        {
            "command": i.irule_command,
            "xc_description": i.xc_description,
            "note": i.note,
        }
        for i in result.items
        if i.status.name == "ADVISORY"
    ]
    output["summary"] = (
        f"Coverage: {result.coverage_pct:.1f}% — "
        f"{result.translatable_count} translatable, "
        f"{result.partial_count} partial, "
        f"{result.untranslatable_count} untranslatable, "
        f"{result.advisory_count} advisory"
    )

    return json.dumps(output)


# ── Refactoring tools ─────────────────────────────────────────────────


@tool(
    "extract_variable",
    "Extract a selected expression into a named variable (set var [expr]).",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "start_line": {**_INT, "description": "0-based start line of selection"},
        "start_character": {**_INT, "description": "0-based start character"},
        "end_line": {**_INT, "description": "0-based end line of selection"},
        "end_character": {**_INT, "description": "0-based end character"},
        "var_name": {**_STR, "description": "Name for the new variable (default: 'result')"},
    },
    required=["source", "start_line", "start_character", "end_line", "end_character"],
)
def _tool_extract_variable(
    source: str,
    start_line: int,
    start_character: int,
    end_line: int,
    end_character: int,
    var_name: str = "result",
) -> str:
    from core.refactoring._extract_variable import extract_variable

    result = extract_variable(
        source, start_line, start_character, end_line, end_character, var_name
    )
    if result is None:
        return json.dumps(None)
    return json.dumps(
        {
            "title": result.title,
            "rewritten": result.apply(source),
            "edit_count": len(result.edits),
        }
    )


@tool(
    "inline_variable",
    "Inline a single-use variable — replace the reference with its value and remove the set.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "line": {**_INT, "description": "0-based line of the set command"},
        "character": {**_INT, "description": "0-based character offset"},
    },
    required=["source", "line", "character"],
)
def _tool_inline_variable(source: str, line: int, character: int) -> str:
    from core.refactoring._inline_variable import inline_variable

    result = inline_variable(source, line, character)
    if result is None:
        return json.dumps(None)
    return json.dumps(
        {
            "title": result.title,
            "rewritten": result.apply(source),
            "edit_count": len(result.edits),
        }
    )


@tool(
    "if_to_switch",
    "Convert an if/elseif chain that tests the same variable to a switch statement.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "line": {**_INT, "description": "0-based line of the if command"},
        "character": {**_INT, "description": "0-based character offset"},
    },
    required=["source", "line", "character"],
)
def _tool_if_to_switch(source: str, line: int, character: int) -> str:
    from core.refactoring._if_to_switch import if_to_switch

    result = if_to_switch(source, line, character)
    if result is None:
        return json.dumps(None)
    return json.dumps(
        {
            "title": result.title,
            "rewritten": result.apply(source),
            "edit_count": len(result.edits),
        }
    )


@tool(
    "switch_to_dict",
    "Convert a switch where every arm sets the same variable to a dict lookup.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "line": {**_INT, "description": "0-based line of the switch command"},
        "character": {**_INT, "description": "0-based character offset"},
    },
    required=["source", "line", "character"],
)
def _tool_switch_to_dict(source: str, line: int, character: int) -> str:
    from core.refactoring._switch_to_dict import switch_to_dict

    result = switch_to_dict(source, line, character)
    if result is None:
        return json.dumps(None)
    return json.dumps(
        {
            "title": result.title,
            "rewritten": result.apply(source),
            "edit_count": len(result.edits),
        }
    )


@tool(
    "brace_expr",
    "Convert unbraced expr arguments to braced form for safety and performance.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "line": {**_INT, "description": "0-based line of the expr command"},
        "character": {**_INT, "description": "0-based character offset"},
    },
    required=["source", "line", "character"],
)
def _tool_brace_expr(source: str, line: int, character: int) -> str:
    from core.refactoring._brace_expr import brace_expr

    result = brace_expr(source, line, character)
    if result is None:
        return json.dumps(None)
    return json.dumps(
        {
            "title": result.title,
            "rewritten": result.apply(source),
            "edit_count": len(result.edits),
        }
    )


@tool(
    "extract_datagroup",
    "Extract an if/elseif chain or switch with literal values to an iRules data-group. "
    "Type-aware: detects IP addresses (with CIDR), integers, and strings automatically. "
    "Returns both the rewritten iRule code and the tmsh data-group definition.",
    params={
        "source": {**_STR, "description": "iRules source code"},
        "line": {**_INT, "description": "0-based line of the if or switch command"},
        "character": {**_INT, "description": "0-based character offset"},
        "dg_name": {**_STR, "description": "Data-group name (auto-generated if empty)"},
    },
    required=["source", "line", "character"],
)
def _tool_extract_datagroup(
    source: str,
    line: int,
    character: int,
    dg_name: str = "",
) -> str:
    _configure_dialect("f5-irules")

    from core.refactoring._extract_datagroup import extract_to_datagroup

    result = extract_to_datagroup(source, line, character, dg_name=dg_name)
    if result is None:
        return json.dumps(None)
    return json.dumps(
        {
            "title": result.title,
            "rewritten": result.apply(source),
            "data_group_definition": result.data_group_tcl(),
            "data_group": {
                "name": result.data_group.name,
                "value_type": result.data_group.value_type,
                "record_count": len(result.data_group.records),
                "records": [{"key": k, "value": v} for k, v in result.data_group.records],
            },
            "edit_count": len(result.edits),
        }
    )


@tool(
    "suggest_datagroup_extractions",
    "AI-enhanced scan: find all patterns in iRules source that could be extracted to "
    "data-groups. Returns structured context (pattern type, inferred value type, CIDR "
    "detection, body shape analysis, confidence level) for each candidate. Use this to "
    "plan data-group extractions, consolidate across events, and choose optimal names.",
    params={
        "source": {**_STR, "description": "iRules source code to scan"},
    },
    required=["source"],
)
def _tool_suggest_datagroup_extractions(source: str) -> str:
    _configure_dialect("f5-irules")

    from core.refactoring._extract_datagroup import suggest_datagroup_extraction

    candidates = suggest_datagroup_extraction(source)
    # Serialise — strip the static_result (not JSON-safe, use extract_datagroup to get it).
    output = []
    for c in candidates:
        entry = {k: v for k, v in c.items() if k != "static_result"}
        entry["has_static_extraction"] = c.get("static_result") is not None
        output.append(entry)
    return json.dumps({"candidates": output, "total": len(output)})


@tool(
    "refactor",
    "List all available refactorings at a position or selection. "
    "Returns the names and descriptions of applicable refactoring actions. "
    "Use the specific refactoring tools (extract_variable, inline_variable, "
    "if_to_switch, switch_to_dict, brace_expr, extract_datagroup) to apply.",
    params={
        "source": {**_STR, "description": "Tcl or iRules source code"},
        "start_line": {**_INT, "description": "0-based start line"},
        "start_character": {**_INT, "description": "0-based start character"},
        "end_line": {**_INT, "description": "0-based end line"},
        "end_character": {**_INT, "description": "0-based end character"},
        "dialect": {**_STR, "description": "Language dialect. Auto-detected if empty."},
    },
    required=["source", "start_line", "start_character", "end_line", "end_character"],
)
def _tool_refactor(
    source: str,
    start_line: int,
    start_character: int,
    end_line: int,
    end_character: int,
    dialect: str = "",
) -> str:
    _configure_dialect(dialect or _detect_dialect(source))

    from core.refactoring._brace_expr import brace_expr as _be
    from core.refactoring._extract_datagroup import extract_to_datagroup as _edg
    from core.refactoring._extract_variable import extract_variable as _ev
    from core.refactoring._if_to_switch import if_to_switch as _its
    from core.refactoring._inline_variable import inline_variable as _iv
    from core.refactoring._switch_to_dict import switch_to_dict as _std

    available: list[dict] = []
    line = start_line
    char = start_character
    has_selection = start_line != end_line or start_character != end_character

    if has_selection:
        r = _ev(source, start_line, start_character, end_line, end_character)
        if r:
            available.append({"tool": "extract_variable", "title": r.title})

    r2 = _iv(source, line, char)
    if r2:
        available.append({"tool": "inline_variable", "title": r2.title})

    r3 = _its(source, line, char)
    if r3:
        available.append({"tool": "if_to_switch", "title": r3.title})

    r4 = _std(source, line, char)
    if r4:
        available.append({"tool": "switch_to_dict", "title": r4.title})

    r5 = _be(source, line, char)
    if r5:
        available.append({"tool": "brace_expr", "title": r5.title})

    r6 = _edg(source, line, char)
    if r6:
        available.append({"tool": "extract_datagroup", "title": r6.title})

    return json.dumps({"available": available, "total": len(available)})


@tool(
    "tk_layout",
    "Extract the Tk widget tree from source code containing `package require Tk`. "
    "Returns the widget hierarchy, geometry managers, and visual options as JSON.",
    params={
        "source": {**_STR, "description": "Tcl/Tk source code containing widget creation commands"},
    },
    required=["source"],
)
def _tool_tk_layout(source: str) -> str:
    from core.tk.extract import extract_tk_layout

    layout = extract_tk_layout(source)
    return json.dumps(layout)


@tool(
    "help",
    "Show available features and how to use them across the LSP, MCP tools, "
    "VS Code chat participants, Claude Code skills, and supported editors.",
    params={
        "topic": {
            **_STR,
            "description": "Optional topic to filter (e.g. 'mcp', 'irule', 'format', 'diagnostics'). "
            "Omit to get the full feature catalogue.",
        }
    },
)
def _tool_help(topic: str = "") -> str:
    try:
        from core.help.kcs_db import list_features, search_help

        if topic:
            results = search_help(topic)
            if results:
                return json.dumps(results, indent=2)
            catalogue = list_features()
            return json.dumps(
                {
                    "error": f"No features match '{topic}'",
                    "available_sections": list(catalogue.keys()),
                }
            )
        return json.dumps(list_features(), indent=2)
    except Exception:
        pass  # Fall back to markdown parsing

    from ai.claude.tcl_ai import _build_feature_catalogue, _search_catalogue

    catalogue = _build_feature_catalogue()
    if topic:
        matched = _search_catalogue(catalogue, topic)
        if matched:
            return json.dumps(matched, indent=2)
        return json.dumps(
            {
                "error": f"No features match '{topic}'",
                "available_sections": list(catalogue.keys()),
            }
        )
    return json.dumps(catalogue, indent=2)


@tool(
    "set_dialect",
    "Set the active Tcl dialect for this session.",
    params={
        "dialect": {
            **_STR,
            "description": "One of: tcl8.4, tcl8.5, tcl8.6, tcl9.0, f5-irules, f5-iapps, eda-tools",
        }
    },
    required=["dialect"],
)
def _tool_set_dialect(dialect: str) -> str:
    global _session_dialect

    from core.commands.registry.runtime import configure_signatures

    old = _session_dialect
    changed = configure_signatures(dialect=dialect)
    if changed:
        _session_dialect = dialect
    return json.dumps({"dialect": dialect, "previous": old, "changed": changed})


@tool(
    "generate_irule_test",
    "Generate a complete iRule test script using the Event Orchestrator framework. "
    "Analyzes the iRule source to extract events, commands, pools, data groups, and "
    "variables, then generates test scenarios with assertions.",
    params={
        "source": {**_STR, "description": "iRule source code to generate tests for"},
    },
    required=["source"],
)
def _tool_generate_irule_test(source: str) -> str:
    _configure_dialect("f5-irules")

    from ai.claude.tcl_ai import (
        _build_test_script_with_metadata,
        _extract_irule_commands,
        _extract_object_refs,
        _extract_variables,
        _infer_profiles,
    )
    from ai.shared.irule_analysis import ordered_events

    evt_list = ordered_events(source)
    ordered_event_names = [e.name for e in evt_list]
    profiles = _infer_profiles(ordered_event_names)
    commands_used = _extract_irule_commands(source)
    objects = _extract_object_refs(source, commands_used)
    variables = _extract_variables(source)

    test_script, meta = _build_test_script_with_metadata(
        basename="irule.tcl",
        source=source,
        ordered_events=ordered_event_names,
        profiles=profiles,
        commands_used=commands_used,
        objects=objects,
        variables=variables,
    )

    multi_tmm = meta["multi_tmm_detected"]
    cfg_paths = meta["cfg_paths"]

    return json.dumps(
        {
            "test_script": test_script,
            "events": ordered_event_names,
            "profiles": profiles,
            "commands_used": commands_used,
            "pools": objects.get("pools", []),
            "datagroups": objects.get("datagroups", []),
            "multi_tmm_detected": multi_tmm,
            "multi_tmm_hint": (
                "This iRule uses patterns that behave differently across TMMs "
                "(static:: writes in hot events, counters, or shared table state). "
                "The generated test includes a multi-TMM scenario using fakeCMP "
                "distribution.  Use ::orch::fakecmp_suggest_sources to plan which "
                "client addresses hit which TMMs."
            )
            if multi_tmm
            else None,
            "cfg_paths": cfg_paths,
            "cfg_hint": (
                "CFG analysis found {} unique paths to terminal actions. "
                "The generated test includes a test case per path. Use "
                "irule_cfg_paths to inspect paths individually for deeper analysis."
            ).format(len(cfg_paths))
            if cfg_paths
            else None,
        }
    )


def _fakecmp_hash(
    src_addr: str, src_port: int, dst_addr: str, dst_port: int, tmm_count: int
) -> int:
    """Canonical Python implementation of the fakeCMP hash.

    Must stay in sync with _fakecmp_hash in orchestrator.tcl.
    Cross-language parity is verified by test_fakecmp_python_tcl_parity.
    """
    h = 0
    for octet in src_addr.split("."):
        h = (h * 31 + int(octet)) & 0x7FFFFFFF
    h = (h * 31 + src_port) & 0x7FFFFFFF
    for octet in dst_addr.split("."):
        h = (h * 31 + int(octet)) & 0x7FFFFFFF
    h = (h * 31 + dst_port) & 0x7FFFFFFF
    return h % tmm_count


def _validate_ipv4(addr: str, field: str) -> dict[str, str] | None:
    """Validate an IPv4 address.  Returns error dict or None."""
    parts = addr.split(".")
    if len(parts) != 4:
        return {"field": field, "message": f"expected IPv4 (a.b.c.d), got '{addr}'", "value": addr}
    for p in parts:
        try:
            v = int(p)
        except ValueError:
            return {"field": field, "message": f"non-integer octet '{p}'", "value": addr}
        if v < 0 or v > 255:
            return {"field": field, "message": f"octet {v} out of range (0-255)", "value": addr}
    return None


def _validate_port(port: int, field: str) -> dict[str, str] | None:
    """Validate a port number.  Returns error dict or None."""
    if not isinstance(port, int) or port < 0 or port > 65535:
        return {"field": field, "message": "must be 0-65535", "value": str(port)}
    return None


def _fakecmp_validation_error(field_errors: list[dict[str, str]]) -> str:
    """Return structured JSON error for fakeCMP tool validation failures.

    Schema: {"error": "<summary>", "fields": [{"field": "...", "message": "...", "value": "..."}]}
    """
    summary = "; ".join(f"{e['field']}: {e['message']}" for e in field_errors)
    return json.dumps({"error": summary, "fields": field_errors})


@tool(
    "fakecmp_which_tmm",
    "Look up which TMM a connection 4-tuple maps to via fakeCMP hash. "
    "fakeCMP is a deterministic simulated hash (NOT the real BIG-IP CMP algorithm). "
    "Use this to plan multi-TMM test scenarios: figure out which TMM a "
    "client address lands on before writing the test.",
    params={
        "tmm_count": {**_INT, "description": "Number of TMMs (2+)"},
        "src_addr": {**_STR, "description": "Source IP (e.g. '10.0.0.1')"},
        "src_port": {**_INT, "description": "Source port (e.g. 12345)"},
        "dst_addr": {**_STR, "description": "Destination IP (e.g. '192.168.1.100')"},
        "dst_port": {**_INT, "description": "Destination port (e.g. 443)"},
    },
    required=["tmm_count", "src_addr", "src_port", "dst_addr", "dst_port"],
)
def _tool_fakecmp_which_tmm(
    tmm_count: int, src_addr: str, src_port: int, dst_addr: str, dst_port: int
) -> str:
    # Validate inputs
    field_errors: list[dict[str, str]] = []
    if not isinstance(tmm_count, int) or tmm_count < 2:
        field_errors.append(
            {"field": "tmm_count", "message": "must be >= 2", "value": str(tmm_count)}
        )
    err = _validate_ipv4(src_addr, "src_addr")
    if err:
        field_errors.append(err)
    err = _validate_port(src_port, "src_port")
    if err:
        field_errors.append(err)
    err = _validate_ipv4(dst_addr, "dst_addr")
    if err:
        field_errors.append(err)
    err = _validate_port(dst_port, "dst_port")
    if err:
        field_errors.append(err)
    if field_errors:
        return _fakecmp_validation_error(field_errors)

    tmm_id = _fakecmp_hash(src_addr, src_port, dst_addr, dst_port, tmm_count)
    return json.dumps(
        {
            "tmm_id": tmm_id,
            "tmm_count": tmm_count,
            "tuple": f"{src_addr}:{src_port} -> {dst_addr}:{dst_port}",
        }
    )


@tool(
    "fakecmp_suggest_sources",
    "Find client_addr/port combos that land on each TMM via fakeCMP hash. "
    "Returns a distribution plan mapping each TMM ID to suggested source tuples. "
    "Use this when generating multi-TMM tests to ensure traffic hits all TMMs.",
    params={
        "tmm_count": {**_INT, "description": "Number of TMMs (2+)"},
        "count": {**_INT, "description": "How many sources per TMM (default 1)"},
        "dst_addr": {**_STR, "description": "Destination/VIP IP (default '192.168.1.100')"},
        "dst_port": {**_INT, "description": "Destination port (default 443)"},
    },
    required=["tmm_count"],
)
def _tool_fakecmp_suggest_sources(
    tmm_count: int,
    count: int = 1,
    dst_addr: str = "192.168.1.100",
    dst_port: int = 443,
) -> str:
    # Validate inputs
    field_errors: list[dict[str, str]] = []
    if not isinstance(tmm_count, int) or tmm_count < 2:
        field_errors.append(
            {"field": "tmm_count", "message": "must be >= 2", "value": str(tmm_count)}
        )
    if not isinstance(count, int) or count < 1:
        field_errors.append({"field": "count", "message": "must be >= 1", "value": str(count)})
    err = _validate_ipv4(dst_addr, "dst_addr")
    if err:
        field_errors.append(err)
    err = _validate_port(dst_port, "dst_port")
    if err:
        field_errors.append(err)
    if field_errors:
        return _fakecmp_validation_error(field_errors)

    result: dict[int, list[dict[str, str | int]]] = {t: [] for t in range(tmm_count)}
    need = tmm_count * count
    found = 0

    for octet in range(1, 255):
        if found >= need:
            break
        for port in range(10001, 10255):
            if found >= need:
                break
            addr = f"10.0.0.{octet}"
            tmm = _fakecmp_hash(addr, port, dst_addr, dst_port, tmm_count)
            if len(result[tmm]) < count:
                result[tmm].append({"addr": addr, "port": port})
                found += 1

    return json.dumps(
        {
            "tmm_count": tmm_count,
            "plan": result,
            "usage_hint": (
                "In your test, iterate over the plan and set "
                "::orch::configure -client_addr $addr -client_port $port "
                "before each ::orch::run_http_request call."
            ),
        }
    )


@tool(
    "irule_cfg_paths",
    "Extract test-relevant control flow paths from an iRule using compiler IR analysis. "
    "Returns all unique paths through the code that reach terminal actions (pool, reject, "
    "redirect, etc.), with the chain of branch conditions along each path. "
    "Use this during the agentic loop to understand branching structure before writing tests, "
    "or to verify that generated tests cover all code paths.",
    params={
        "source": {**_STR, "description": "iRule source code to analyze"},
    },
    required=["source"],
)
def _tool_irule_cfg_paths(source: str) -> str:
    _configure_dialect("f5-irules")

    from ai.claude.tcl_ai import _extract_test_paths

    paths = _extract_test_paths(source)

    # Group by event for readability
    by_event: dict[str, list[dict]] = {}
    for p in paths:
        event = p["event"]
        by_event.setdefault(event, []).append(p)

    # Collect all questions across paths
    all_questions = []
    for p in paths:
        for q in p.get("questions", []):
            all_questions.append(
                {
                    "path_label": p["path_label"],
                    "priority": p.get("priority", "normal"),
                    **q,
                }
            )

    # Separate high-priority (security) paths
    high_priority = [p for p in paths if p.get("priority") == "high"]

    return json.dumps(
        {
            "total_paths": len(paths),
            "high_priority_paths": len(high_priority),
            "paths_by_event": {
                event: {
                    "count": len(event_paths),
                    "paths": event_paths,
                }
                for event, event_paths in by_event.items()
            },
            "questions": all_questions,
            "coverage_hint": (
                "Each path represents a unique route through the iRule to a terminal "
                "action.  To achieve full branch coverage, write at least one test per "
                "path.  Pay special attention to paths with 'else' conditions — they "
                "represent fallback/default behavior that is often under-tested."
            ),
            "agentic_hint": (
                "Present the 'questions' list to the user to gather expected values "
                "before generating tests.  Questions are ordered by priority — ask "
                "about high-priority (security) paths first.  Use the user's answers "
                "to fill in assertion values instead of guessing from the source code.  "
                "If the user provides custom input values, use those instead of the "
                "suggested defaults derived from branch conditions."
            ),
        }
    )


@tool(
    "unminify_error",
    "Translate a Tcl or iRule error message from minified code back to original names using a symbol map.",
    params={
        "error_message": {
            **_STR,
            "description": (
                "The raw error text from tclsh, an iRule log, or a BIG-IP "
                "/var/log/ltm entry produced by running minified code."
            ),
        },
        "symbol_map": {
            **_STR,
            "description": (
                "The symbol map text produced by the minifier's --symbol-map "
                "flag (original name <-> compacted name mapping)."
            ),
        },
        "minified_source": {
            **_STR,
            "description": (
                "The minified Tcl source that was running when the error "
                "occurred.  Optional; enables line-number remapping."
            ),
        },
        "original_source": {
            **_STR,
            "description": (
                "The original pre-minification Tcl source.  Optional; "
                "enables line-number remapping."
            ),
        },
    },
    required=["error_message", "symbol_map"],
)
def _tool_unminify_error(
    error_message: str,
    symbol_map: str,
    minified_source: str = "",
    original_source: str = "",
) -> str:
    from core.minifier import unminify_error

    translated = unminify_error(
        error_message,
        symbol_map=symbol_map,
        minified_source=minified_source or None,
        original_source=original_source or None,
    )
    return json.dumps(
        {
            "original_error": error_message,
            "translated_error": translated,
            "changed": translated != error_message,
        }
    )


# Entry point

if __name__ == "__main__":
    run_stdio()
