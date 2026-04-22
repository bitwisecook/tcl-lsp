"""Analysis/graph verbs: symbols, diagram, callgraph, symbolgraph, dataflow."""

from __future__ import annotations

import argparse
import json
import re
from typing import Any, cast

from core.analysis.analyser import analyse
from core.analysis.semantic_graph import (
    build_call_graph,
    build_dataflow_graph,
    build_symbol_graph,
)
from core.commands.registry.runtime import configure_signatures

from ._registry import verb
from ._utils import (
    _add_input_arguments,
    _combine_sources,
    _read_input_documents,
    _write_text_output,
)

_WHEN_EVENT_PATTERN = re.compile(r"\bwhen\s+([A-Z_][A-Z0-9_]*)")


@verb("symbols", aliases=("syms",), help="Emit symbol definitions for the resolved input.")
def _configure_symbols(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit symbol data as JSON.",
    )
    p.set_defaults(handler=_run_symbols)


@verb("diagram", help="Extract control-flow diagram data from compiler IR.")
def _configure_diagram(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit diagram data as JSON.",
    )
    p.set_defaults(handler=_run_diagram)


@verb("callgraph", aliases=("call-graph",), help="Build call graph data for resolved source.")
def _configure_callgraph(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit call graph data as JSON.",
    )
    p.set_defaults(handler=_run_callgraph)


@verb("symbolgraph", aliases=("symbol-graph",), help="Build symbol relationship graph data.")
def _configure_symbolgraph(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit symbol graph data as JSON.",
    )
    p.set_defaults(handler=_run_symbolgraph)


@verb(
    "dataflow",
    aliases=("dataflow-graph",),
    help="Build taint/effect data-flow graph data.",
)
def _configure_dataflow(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit data-flow graph data as JSON.",
    )
    p.set_defaults(handler=_run_dataflow)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _detect_event_entries(source: str) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []
    seen: set[str] = set()
    for match in _WHEN_EVENT_PATTERN.finditer(source):
        name = match.group(1)
        if name in seen:
            continue
        seen.add(name)
        line = source[: match.start()].count("\n") + 1
        entries.append(
            {
                "kind": "event",
                "name": name,
                "line": line,
                "depth": 0,
            }
        )
    return entries


def _collect_scope_symbol_entries(scope: Any, *, depth: int = 0) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []

    for proc in scope.procs.values():
        params = [param.name for param in proc.params]
        line = proc.name_range.start.line + 1 if proc.name_range else None
        entries.append(
            {
                "kind": "function",
                "name": proc.name,
                "line": line,
                "depth": depth,
                "params": params,
            }
        )

    if scope.kind in ("global", "namespace"):
        for variable in scope.variables.values():
            if variable.definition_range is None:
                continue
            entries.append(
                {
                    "kind": "variable",
                    "name": variable.name,
                    "line": variable.definition_range.start.line + 1,
                    "depth": depth,
                }
            )

    for child in scope.children:
        if child.kind == "namespace" and child.body_range is not None:
            entries.append(
                {
                    "kind": "namespace",
                    "name": child.name,
                    "line": child.body_range.start.line + 1,
                    "depth": depth,
                }
            )
            entries.extend(_collect_scope_symbol_entries(child, depth=depth + 1))
        elif child.kind == "proc":
            entries.extend(_collect_scope_symbol_entries(child, depth=depth + 1))

    return entries


def _append_symbolgraph_scope(
    lines: list[str], scope: dict[str, Any], *, depth: int = 0
) -> None:
    indent = "  " * depth
    kind = str(scope.get("kind", "?"))
    name = str(scope.get("name", "?"))
    lines.append(f"{indent}{kind} {name}")

    procs = scope.get("procs")
    if isinstance(procs, list):
        for proc in procs:
            if not isinstance(proc, dict):
                continue
            proc_dict = cast(dict[str, Any], proc)
            params_raw = proc_dict.get("params")
            params = (
                ", ".join(str(item) for item in params_raw) if isinstance(params_raw, list) else ""
            )
            line = proc_dict.get("line")
            line_suffix = f" (line {line + 1})" if isinstance(line, int) else ""
            refs = proc_dict.get("ref_count", 0)
            lines.append(
                f"{indent}  proc {proc_dict.get('name', '?')}({params}){line_suffix} [{refs} refs]"
            )

    variables = scope.get("variables")
    if isinstance(variables, list):
        for variable in variables:
            if not isinstance(variable, dict):
                continue
            variable_dict = cast(dict[str, Any], variable)
            line = variable_dict.get("line")
            line_suffix = f" (line {line + 1})" if isinstance(line, int) else ""
            refs_raw = variable_dict.get("references")
            refs = len(refs_raw) if isinstance(refs_raw, list) else 0
            lines.append(
                f"{indent}  var {variable_dict.get('name', '?')}{line_suffix} [{refs} refs]"
            )

    children = scope.get("children")
    if isinstance(children, list):
        for child in children:
            if isinstance(child, dict):
                _append_symbolgraph_scope(lines, cast(dict[str, Any], child), depth=depth + 1)


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def _run_symbols(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    analysis = analyse(source)
    event_entries = _detect_event_entries(source)
    scope_entries = _collect_scope_symbol_entries(analysis.global_scope)
    entries = [*event_entries, *scope_entries]

    payload = {
        "count": len(entries),
        "dialect": args.dialect,
        "inputs": [document.label for document in documents],
        "symbols": entries,
    }
    if args.json:
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 0

    if not entries:
        _write_text_output(args.output, "no symbols")
        return 0

    lines: list[str] = [f"symbols: {len(entries)}"]
    for entry in entries:
        depth_raw = entry.get("depth", 0)
        depth = depth_raw if isinstance(depth_raw, int) else 0
        indent = "  " * depth
        kind = str(entry.get("kind", "?"))
        name = str(entry.get("name", "?"))
        line = entry.get("line")
        line_suffix = f" (line {line})" if isinstance(line, int) else ""
        if kind == "function":
            params_raw = entry.get("params", [])
            params = (
                ", ".join(str(item) for item in params_raw) if isinstance(params_raw, list) else ""
            )
            lines.append(f"{indent}function {name}({params}){line_suffix}")
        else:
            lines.append(f"{indent}{kind} {name}{line_suffix}")
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_diagram(args: argparse.Namespace) -> int:
    from core.diagram.extract import extract_diagram_data

    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    data = extract_diagram_data(source)

    if args.json:
        _write_text_output(args.output, json.dumps(data, indent=2))
        return 1 if "error" in data else 0

    if "error" in data:
        _write_text_output(args.output, f"diagram error: {data['error']}")
        return 1

    events = data.get("events", [])
    procedures = data.get("procedures", [])
    lines = [
        f"diagram: events={len(events)} procedures={len(procedures)}",
    ]
    if events:
        lines.append("events:")
        for event in events:
            flow_count = len(event.get("flow", []))
            multiplicity = str(event.get("multiplicity", "unknown"))
            lines.append(f"  {event.get('name', '?')} ({multiplicity}) nodes={flow_count}")
    if procedures:
        lines.append("procedures:")
        for proc in procedures:
            params = ", ".join(str(item) for item in proc.get("params", []))
            flow_count = len(proc.get("flow", []))
            lines.append(f"  {proc.get('name', '?')}({params}) nodes={flow_count}")
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_callgraph(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)
    data = build_call_graph(source)

    if args.json:
        _write_text_output(args.output, json.dumps(data, indent=2))
        return 0

    nodes = data.get("nodes", [])
    edges = data.get("edges", [])
    roots = data.get("roots", [])
    leaves = data.get("leaf_procs", [])

    lines = [f"call graph: procs={len(nodes)} edges={len(edges)}"]
    if nodes:
        lines.append("procs:")
        for node in nodes:
            params = ", ".join(str(item) for item in node.get("params", []))
            line = node.get("line")
            line_suffix = f" (line {line + 1})" if isinstance(line, int) else ""
            pure_suffix = " [pure]" if node.get("pure") else ""
            lines.append(f"  {node['name']}({params}){line_suffix}{pure_suffix}")
    if edges:
        lines.append("edges:")
        for edge in edges:
            lines.append(f"  {edge['caller']} -> {edge['callee']}")
    if roots:
        lines.append(f"roots: {', '.join(str(item) for item in roots)}")
    if leaves:
        lines.append(f"leaves: {', '.join(str(item) for item in leaves)}")
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_symbolgraph(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)
    data = build_symbol_graph(source)

    if args.json:
        _write_text_output(args.output, json.dumps(data, indent=2))
        return 0

    summary = data.get("summary", {})
    total_procs = summary.get("total_procs", 0)
    total_variables = summary.get("total_variables", 0)
    total_namespaces = summary.get("total_namespaces", 0)

    lines = [
        f"symbol graph: procs={total_procs} variables={total_variables} namespaces={total_namespaces}"
    ]
    scopes = data.get("scopes", [])
    if scopes:
        lines.append("scopes:")
        for scope in scopes:
            if isinstance(scope, dict):
                _append_symbolgraph_scope(lines, scope, depth=1)
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_dataflow(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)
    data = build_dataflow_graph(source)

    if args.json:
        _write_text_output(args.output, json.dumps(data, indent=2))
        return 0

    summary = data.get("summary", {})
    lines = [
        "dataflow:"
        f" taintWarnings={summary.get('total_taint_warnings', 0)}"
        f" taintedVars={summary.get('tainted_variable_count', 0)}"
        f" pure={summary.get('pure_proc_count', 0)}"
        f" impure={summary.get('impure_proc_count', 0)}"
    ]

    warnings = data.get("taint_warnings", [])
    if warnings:
        lines.append("taint warnings:")
        for warning in warnings:
            if not isinstance(warning, dict):
                continue
            line = warning.get("line")
            line_no = line + 1 if isinstance(line, int) else "?"
            code = warning.get("code", "")
            message = warning.get("message", "")
            lines.append(f"  {code} line {line_no}: {message}")

    _write_text_output(args.output, "\n".join(lines))
    return 0
