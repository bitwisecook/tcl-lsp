#!/usr/bin/env python3
"""Standalone Tcl/iRules analysis tool for Claude Code AI skills.

Imports server modules directly — no LSP server startup needed.
Run via: uv run --no-dev python ai/claude/tcl_ai.py <subcommand> <args>

Subcommands:
    context <file>              Build context pack (diagnostics + symbols + event metadata)
    diagnostics <file>          Show diagnostics from the analyzer
    validate <file>             Categorized validation report (errors, security, taint, etc.)
    review <file>               Security-focused diagnostic report
    convert <file>              Detect legacy patterns eligible for modernisation
    symbols <file>              Show document symbol hierarchy
    diagram <file>              Extract control flow from compiler IR
    optimize <file>             Show optimization suggestions and rewritten source
    event-order <file>          Show events in canonical firing order
    event-info <EVENT_NAME>     Show iRules event registry metadata
    command-info <CMD_NAME>     Show iRules command registry metadata
    call-graph <file>           Build call graph (procs, call edges, roots, leaves)
    symbol-graph <file>         Build symbol relationship graph (scopes, defs, refs)
    dataflow-graph <file>       Build data-flow graph (taint, effects, tainted vars)
    tk-layout <file>            Extract Tk widget tree from source
    generate-test <file>        Generate iRule test script from source
    cfg-paths <file>            Extract test-relevant CFG paths from iRule source
    help [topic]                Show available features and how to use them
"""

from __future__ import annotations

import argparse
import json
import os
import sys

from ai.shared.diagnostics import (
    CATEGORY_ORDER as _CATEGORY_ORDER,
)
from ai.shared.diagnostics import (
    CONTROL_FLOW_CODES,
    CONVERSION_MAP,
    CONVERTIBLE_CODES,
    IRULES_EVENT_PATTERN,
    REVIEW_CODES,
    SECURITY_CODES,
    TAINT_CODES,
    THREAD_CODES,
)
from ai.shared.diagnostics import (
    categorise as _categorise,
)

# Subcommand: diagnostics


def cmd_diagnostics(source: str, file_path: str) -> None:
    from core.analysis.analyser import analyse

    result = analyse(source)
    diags = result.diagnostics

    print(f"=== Diagnostics ({len(diags)} items) ===")
    if not diags:
        print("  (none)")
        return

    for d in diags:
        sev = d.severity.name
        code = d.code or ""
        line = d.range.start.line + 1
        col = d.range.start.character
        print(f"  {sev:<8s}  {code:<12s}  line {line}:{col}  {d.message}")


# Subcommand: symbols


def _print_scope_symbols(scope, depth: int = 0) -> int:
    """Recursively print symbols from a scope. Returns count."""
    count = 0
    indent = "  " * depth

    for proc in scope.procs.values():
        params = []
        for p in proc.params:
            params.append(p.name)
        param_str = f"({', '.join(params)})" if params else "()"
        line = proc.name_range.start.line + 1 if proc.name_range else "?"
        print(f"  {indent}Function {proc.name} {param_str} (line {line})")
        count += 1

    if scope.kind in ("global", "namespace"):
        for var in scope.variables.values():
            if var.definition_range:
                line = var.definition_range.start.line + 1
                print(f"  {indent}Variable {var.name} (line {line})")
                count += 1

    for child in scope.children:
        if child.kind == "namespace" and child.body_range:
            line = child.body_range.start.line + 1
            print(f"  {indent}Namespace {child.name} (line {line})")
            count += 1
            count += _print_scope_symbols(child, depth + 1)
        elif child.kind == "proc":
            count += _print_scope_symbols(child, depth + 1)

    return count


def cmd_symbols(source: str, file_path: str) -> None:
    from core.analysis.analyser import analyse

    result = analyse(source)
    events = _detect_events(source)

    # Buffer output to print header with count first
    lines: list[str] = []
    for name, line in events:
        lines.append(f"  Event {name} (line {line})")
    _count_scope_symbols(result.global_scope, lines)

    print(f"=== Symbol Definitions ({len(lines)} symbols) ===")
    if not lines:
        print("  (none)")
    else:
        for ln in lines:
            print(ln)


def _detect_events(source: str) -> list[tuple[str, int]]:
    """Detect iRule events and their line numbers."""
    events = []
    seen: set[str] = set()
    for match in IRULES_EVENT_PATTERN.finditer(source):
        name = match.group(1)
        if name not in seen:
            seen.add(name)
            line = source[: match.start()].count("\n") + 1
            events.append((name, line))
    return events


# Subcommand: diagram


def cmd_diagram(source: str, file_path: str) -> None:
    from ai.shared.irule_analysis import diagram_data

    data = diagram_data(source)

    if not data:
        print("=== Diagram Data ===")
        print("  (no data — source may be empty)")
        return

    if data.get("error"):
        print("=== Diagram Data ===")
        print(f"  Error: {data['error']}")
        return

    events = data.get("events", [])
    procedures = data.get("procedures", [])

    print("=== Diagram Data ===")
    if events:
        print(f"\n  Events ({len(events)}, in firing order):")
        for evt in events:
            name = evt.get("name", "?")
            pri = evt.get("priority")
            mult = evt.get("multiplicity", "?")
            pri_str = f" priority={pri}" if pri is not None else ""
            flow_count = len(evt.get("flow", []))
            print(f"    {name} ({mult}{pri_str}) — {flow_count} flow nodes")

    if procedures:
        print(f"\n  Procedures ({len(procedures)}):")
        for proc in procedures:
            name = proc.get("name", "?")
            params = proc.get("params", [])
            flow_count = len(proc.get("flow", []))
            print(f"    {name}({', '.join(params)}) — {flow_count} flow nodes")

    print("\n  --- Raw JSON ---")
    print(json.dumps(data, indent=2))


# Subcommand: optimize


def cmd_optimize(source: str, file_path: str) -> None:
    from core.compiler.optimiser import Optimisation, apply_optimisations, find_optimisations

    opts = find_optimisations(source)

    # Group related optimisations for display.
    _ELIM_CODES = frozenset(("O107", "O108", "O109"))
    groups: dict[int, list[Optimisation]] = {}
    ungrouped: list[Optimisation] = []
    for o in opts:
        if o.group is not None:
            groups.setdefault(o.group, []).append(o)
        else:
            ungrouped.append(o)

    display_items: list[tuple[Optimisation, list[Optimisation]]] = []  # (primary, members)
    for _gid, members in sorted(groups.items()):
        primary = next((m for m in members if m.code not in _ELIM_CODES), members[0])
        display_items.append((primary, members))
    for o in ungrouped:
        display_items.append((o, [o]))

    print(f"=== Optimizations ({len(display_items)} items, {len(opts)} rewrites) ===")
    if not opts:
        print("  (none)")
        return

    for primary, members in display_items:
        line = primary.range.start.line + 1
        col = primary.range.start.character
        if len(members) > 1:
            elim_count = sum(1 for m in members if m.code in _ELIM_CODES)
            suffix = (
                f"  (+{elim_count} dead store{'s' if elim_count > 1 else ''} eliminated)"
                if elim_count
                else ""
            )
            print(
                f"  {primary.code:<5s}  line {line}:{col}  {primary.message}  \u2192  {primary.replacement!r}{suffix}"
            )
            for m in members:
                if m is not primary:
                    ml = m.range.start.line + 1
                    mc = m.range.start.character
                    print(
                        f"    \u2514\u2500 {m.code:<5s}  line {ml}:{mc}  {m.message}  \u2192  {m.replacement!r}"
                    )
        elif getattr(primary, "hint_only", False):
            print(f"  {primary.code:<5s}  line {line}:{col}  {primary.message}  (hint)")
        else:
            print(
                f"  {primary.code:<5s}  line {line}:{col}  {primary.message}  \u2192  {primary.replacement!r}"
            )

    optimized = apply_optimisations(source, opts)
    if optimized != source:
        print()
        print("=== Optimized Source ===")
        for ln in optimized.split("\n"):
            print(f"    {ln}")


# Subcommand: event-order


def cmd_event_order(source: str, file_path: str) -> None:
    from ai.shared.irule_analysis import ordered_events

    events = ordered_events(source)
    print(f"=== Event Firing Order ({len(events)} events) ===")
    if not events:
        print("  (no events found)")
        return

    for evt in events:
        print(f"  {evt.index}. {evt.name} ({evt.multiplicity})")


# Subcommand: event-info


def cmd_event_info(event_name: str) -> None:
    from core.commands.registry.info import lookup_event_info

    info = lookup_event_info(event_name, dialect="f5-irules")

    print("=== Event Info ===")
    print(f"  Event: {info.event}")
    print(f"  Known: {'yes' if info.known else 'no'}")
    print(f"  Deprecated: {'yes' if info.deprecated else 'no'}")
    print(f"  Valid commands: {info.valid_command_count}")
    if info.valid_commands:
        show = list(info.valid_commands[:20])
        print(f"  Sample commands: {', '.join(show)}")
        if info.valid_command_count > 20:
            print(f"    ... and {info.valid_command_count - 20} more")


# Subcommand: command-info


def cmd_command_info(command_name: str) -> None:
    from core.commands.registry.info import lookup_command_info

    info = lookup_command_info(command_name, dialect="f5-irules")

    print("=== Command Info ===")
    if not info.found:
        print(f"  Command '{command_name}' not found in registry")
        return

    print(f"  Command: {info.command}")
    if info.summary:
        print(f"  Summary: {info.summary}")
    if info.synopsis:
        for syn in info.synopsis:
            print(f"  Synopsis: {syn}")
    if info.switches:
        print(f"  Switches: {', '.join(info.switches)}")
    if info.valid_in_any_event:
        print("  Valid in: any event")
    elif info.valid_events:
        print(f"  Valid in: {', '.join(info.valid_events[:15])}")
        if len(info.valid_events) > 15:
            print(f"    ... and {len(info.valid_events) - 15} more")


# Subcommand: validate (categorized diagnostics)


def cmd_validate(source: str, file_path: str) -> None:
    from core.analysis.analyser import analyse

    result = analyse(source)
    diags = result.diagnostics

    if not diags:
        print("=== Validation Report ===")
        print("  No issues found. The code looks clean.")
        return

    # Group by category
    groups: dict[str, list] = {}
    for d in diags:
        code = d.code or ""
        if code in CONTROL_FLOW_CODES:
            cat = "irules"
        else:
            cat = _categorise(code)
        groups.setdefault(cat, []).append(d)

    total = len(diags)
    print(f"=== Validation Report ({total} issues) ===")

    for cat_key, cat_label in _CATEGORY_ORDER:
        cat_diags = groups.get(cat_key, [])
        if not cat_diags:
            continue
        print(f"\n  --- {cat_label} ({len(cat_diags)}) ---")
        for d in cat_diags:
            sev = d.severity.name
            code = d.code or ""
            line = d.range.start.line + 1
            col = d.range.start.character
            print(f"  {sev:<8s}  {code:<12s}  line {line}:{col}  {d.message}")


# Subcommand: review (security-focused diagnostics)


def cmd_review(source: str, file_path: str) -> None:
    from core.analysis.analyser import analyse

    result = analyse(source)
    diags = result.diagnostics

    security_diags = [d for d in diags if (d.code or "") in REVIEW_CODES]

    print("=== Security Review ===")
    if not security_diags:
        print("  No security issues detected by static analysis.")
        return

    # Group into sub-categories
    sec = [d for d in security_diags if (d.code or "") in SECURITY_CODES]
    taint = [d for d in security_diags if (d.code or "") in TAINT_CODES]
    thread = [d for d in security_diags if (d.code or "") in THREAD_CODES]

    for label, group in [("Security", sec), ("Taint Analysis", taint), ("Thread Safety", thread)]:
        if not group:
            continue
        print(f"\n  --- {label} ({len(group)}) ---")
        for d in group:
            sev = d.severity.name
            code = d.code or ""
            line = d.range.start.line + 1
            col = d.range.start.character
            print(f"  {sev:<8s}  {code:<12s}  line {line}:{col}  {d.message}")

    print(f"\n  Total security-related issues: {len(security_diags)}")


# Subcommand: convert (legacy pattern detection)


def cmd_convert(source: str, file_path: str) -> None:
    from core.analysis.analyser import analyse

    result = analyse(source)
    diags = result.diagnostics

    convertible = [d for d in diags if (d.code or "") in CONVERTIBLE_CODES]

    print("=== Legacy Pattern Detection ===")
    if not convertible:
        print("  No legacy patterns detected. The code already follows current best practices.")
        return

    print(f"  Found {len(convertible)} convertible pattern(s):\n")
    for d in convertible:
        code = d.code or ""
        line = d.range.start.line + 1
        col = d.range.start.character
        conversion = CONVERSION_MAP.get(code, "modernise")
        print(f"  {code:<12s}  line {line}:{col}  {d.message}")
        print(f"               Conversion: {conversion}")


# Subcommand: context (combined pack)


def cmd_context(source: str, file_path: str) -> None:
    from ai.shared.irule_analysis import ordered_events as _ordered_events
    from core.analysis.analyser import analyse

    basename = os.path.basename(file_path)
    ext = os.path.splitext(file_path)[1].lower()
    dialect = "f5-irules" if ext in (".irul", ".irule") else "tcl8.6"
    line_count = len(source.split("\n"))

    print("=== Context Pack ===")
    print(f"  Dialect: {dialect}")
    print(f"  File: {basename}")
    print(f"  Lines: {line_count}")

    # Diagnostics
    result = analyse(source)
    diags = result.diagnostics
    actionable = [d for d in diags if d.severity.name in ("ERROR", "WARNING")]

    print()
    if actionable:
        print(f"=== Diagnostics ({len(actionable)}) ===")
        for d in actionable[:12]:
            sev = d.severity.name
            code = d.code or ""
            line = d.range.start.line + 1
            cat = _categorise(code)
            print(f"  {sev} {code} line {line}: {d.message}  [{cat}]")
        if len(actionable) > 12:
            print(f"  ... and {len(actionable) - 12} more")
    else:
        print("=== Diagnostics ===")
        print("  (no errors or warnings)")

    # Symbols
    print()
    events_found = _detect_events(source)
    scope_count = 0

    symbols_lines = []
    for name, line in events_found:
        symbols_lines.append(f"  Event {name} (line {line})")
    scope_count = _count_scope_symbols(result.global_scope, symbols_lines)
    total = len(events_found) + scope_count

    if symbols_lines:
        print(f"=== Symbol Definitions ({total}) ===")
        for sl in symbols_lines[:15]:
            print(sl)
        if len(symbols_lines) > 15:
            print(f"  ... and {len(symbols_lines) - 15} more")
    else:
        print("=== Symbol Definitions ===")
        print("  (none)")

    # Event metadata and firing order
    evt_list = _ordered_events(source)
    if evt_list:
        print()
        print(f"=== Event Firing Order ({len(evt_list)} events) ===")
        for evt in evt_list:
            print(f"  {evt.index}. {evt.name} ({evt.multiplicity})")

        # Event registry metadata
        print()
        print(f"=== Event Metadata ({len(evt_list)} events) ===")
        try:
            from core.commands.registry.info import lookup_event_info

            for evt in evt_list[:8]:
                info = lookup_event_info(evt.name, dialect="f5-irules")
                samples = list(info.valid_commands[:8])
                print(
                    f"  {evt.name}: known={'yes' if info.known else 'no'}, "
                    f"validCommands={info.valid_command_count}"
                )
                if samples:
                    print(f"    sample: {', '.join(samples)}")
            if len(evt_list) > 8:
                print(f"  ... and {len(evt_list) - 8} more events")
        except Exception as exc:
            print(f"  (event metadata unavailable: {exc})")


def _count_scope_symbols(scope, lines: list[str], depth: int = 0) -> int:
    """Count and format scope symbols into lines list."""
    count = 0
    indent = "  " * depth

    for proc in scope.procs.values():
        params = [p.name for p in proc.params]
        param_str = f"({', '.join(params)})" if params else "()"
        line = proc.name_range.start.line + 1 if proc.name_range else "?"
        lines.append(f"  {indent}Function {proc.name} {param_str} (line {line})")
        count += 1

    if scope.kind in ("global", "namespace"):
        for var in scope.variables.values():
            if var.definition_range:
                line = var.definition_range.start.line + 1
                lines.append(f"  {indent}Variable {var.name} (line {line})")
                count += 1

    for child in scope.children:
        if child.kind == "namespace" and child.body_range:
            line = child.body_range.start.line + 1
            lines.append(f"  {indent}Namespace {child.name} (line {line})")
            count += 1
            count += _count_scope_symbols(child, lines, depth + 1)
        elif child.kind == "proc":
            count += _count_scope_symbols(child, lines, depth + 1)

    return count


# Subcommand: call-graph


def cmd_call_graph(source: str, file_path: str) -> None:
    from core.analysis.semantic_graph import build_call_graph

    data = build_call_graph(source)
    nodes = data.get("nodes", [])
    edges = data.get("edges", [])
    roots = data.get("roots", [])
    leaves = data.get("leaf_procs", [])

    print(f"=== Call Graph ({len(nodes)} procs, {len(edges)} edges) ===")
    if not nodes and not edges:
        print("  (no procs found)")
        return

    if nodes:
        print(f"\n  Procs ({len(nodes)}):")
        for n in nodes:
            params = ", ".join(n.get("params", []))
            line = n.get("line")
            line_str = f" (line {line + 1})" if line is not None else ""
            pure_str = " [pure]" if n.get("pure") else ""
            print(f"    {n['name']}({params}){line_str}{pure_str}")

    if edges:
        print(f"\n  Edges ({len(edges)}):")
        for e in edges:
            sites = e.get("call_sites", [])
            site_str = ""
            if sites:
                lines = [str(s["line"] + 1) for s in sites[:3]]
                site_str = f" at line(s) {', '.join(lines)}"
                if len(sites) > 3:
                    site_str += f" (+{len(sites) - 3} more)"
            print(f"    {e['caller']} -> {e['callee']}{site_str}")

    if roots:
        print(f"\n  Roots: {', '.join(roots)}")
    if leaves:
        print(f"  Leaves: {', '.join(leaves)}")

    print("\n  --- Raw JSON ---")
    print(json.dumps(data, indent=2))


# Subcommand: symbol-graph


def cmd_symbol_graph(source: str, file_path: str) -> None:
    from core.analysis.semantic_graph import build_symbol_graph

    data = build_symbol_graph(source)
    summary = data.get("summary", {})
    tp = summary.get("total_procs", 0)
    tv = summary.get("total_variables", 0)
    tn = summary.get("total_namespaces", 0)

    print(f"=== Symbol Graph ({tp} procs, {tv} variables, {tn} namespaces) ===")

    scopes = data.get("scopes", [])
    if scopes:
        for scope in scopes:
            _print_scope(scope, depth=0)

    proc_refs = data.get("proc_references", {})
    if proc_refs:
        print(f"\n  Proc References ({len(proc_refs)} procs):")
        for qname, refs in proc_refs.items():
            lines = [str(r["line"] + 1) for r in refs[:5]]
            extra = f" (+{len(refs) - 5} more)" if len(refs) > 5 else ""
            print(f"    {qname}: line(s) {', '.join(lines)}{extra}")

    pkg = data.get("package_requires", [])
    if pkg:
        print(f"\n  Package Requires ({len(pkg)}):")
        for p in pkg:
            ver = f" {p['version']}" if p.get("version") else ""
            print(f"    {p['name']}{ver} (line {p['line'] + 1})")

    print("\n  --- Raw JSON ---")
    print(json.dumps(data, indent=2))


def _print_scope(scope: dict, depth: int) -> None:
    indent = "  " * (depth + 1)
    kind = scope.get("kind", "?")
    name = scope.get("name", "?")
    print(f"{indent}{kind} {name}")

    for proc in scope.get("procs", []):
        params = ", ".join(proc.get("params", []))
        line = proc.get("line")
        line_str = f" (line {line + 1})" if line is not None else ""
        refs = proc.get("ref_count", 0)
        print(f"{indent}  proc {proc['name']}({params}){line_str} [{refs} refs]")

    for var in scope.get("variables", []):
        line = var.get("line")
        line_str = f" (line {line + 1})" if line is not None else ""
        refs = len(var.get("references", []))
        print(f"{indent}  var {var['name']}{line_str} [{refs} refs]")

    for child in scope.get("children", []):
        _print_scope(child, depth + 1)


# Subcommand: dataflow-graph


def cmd_dataflow_graph(source: str, file_path: str) -> None:
    from core.analysis.semantic_graph import build_dataflow_graph

    data = build_dataflow_graph(source)
    summary = data.get("summary", {})
    tw = summary.get("total_taint_warnings", 0)
    tv = summary.get("tainted_variable_count", 0)
    pp = summary.get("pure_proc_count", 0)
    ip = summary.get("impure_proc_count", 0)

    print(
        f"=== Data-Flow Graph ({tw} taint warnings, {tv} tainted vars, {pp} pure / {ip} impure procs) ==="
    )

    warnings = data.get("taint_warnings", [])
    if warnings:
        print(f"\n  Taint Warnings ({len(warnings)}):")
        for w in warnings:
            line = w.get("line", "?")
            code = w.get("code", "?")
            var = w.get("variable", "")
            sink = w.get("sink_command", w.get("command", ""))
            var_str = f" ({var})" if var else ""
            print(
                f"    {code:<12s}  line {line + 1 if isinstance(line, int) else line}  {sink}{var_str}  {w.get('message', '')}"
            )

    tainted = data.get("tainted_variables", [])
    if tainted:
        print(f"\n  Tainted Variables ({len(tainted)}):")
        for t in tainted:
            print(f"    {t['scope']}: {t['variable']}")

    effects = data.get("proc_effects", [])
    if effects:
        print(f"\n  Proc Effects ({len(effects)}):")
        for e in effects:
            pure_str = "pure" if e.get("pure") else "impure"
            barrier_str = " [barrier]" if e.get("has_barrier") else ""
            r = e.get("reads", "NONE")
            w = e.get("writes", "NONE")
            rw = ""
            if r != "NONE" or w != "NONE":
                rw = f" reads={r} writes={w}"
            print(f"    {e['name']}: {pure_str}{rw}{barrier_str}")

    if not warnings and not tainted and not effects:
        print("  (no data-flow information)")

    print("\n  --- Raw JSON ---")
    print(json.dumps(data, indent=2))


# Subcommand: help


def cmd_help(topic: str | None = None) -> None:
    """Show available features and how to use them."""
    try:
        from core.help.kcs_db import list_features, search_help

        if topic:
            results = search_help(topic)
            if results:
                print(json.dumps(results, indent=2))
            else:
                catalogue = list_features()
                print(
                    json.dumps(
                        {
                            "error": f"No features match '{topic}'",
                            "available_sections": list(catalogue.keys()),
                        },
                        indent=2,
                    )
                )
        else:
            print(json.dumps(list_features(), indent=2))
        return
    except Exception:
        pass  # Fall back to markdown parsing

    catalogue = _build_feature_catalogue()
    if topic:
        matched = _search_catalogue(catalogue, topic)
        if matched:
            print(json.dumps(matched, indent=2))
        else:
            print(
                json.dumps(
                    {
                        "error": f"No features match '{topic}'",
                        "available_sections": list(catalogue.keys()),
                    },
                    indent=2,
                )
            )
    else:
        print(json.dumps(catalogue, indent=2))


def _search_catalogue(catalogue: dict, topic: str) -> dict:
    """Search the feature catalogue for entries matching a topic string."""
    topic_lower = topic.lower()
    matched: dict = {}
    for section, features in catalogue.items():
        if topic_lower in section.lower():
            matched[section] = features
            continue
        if not isinstance(features, list):
            continue
        hits = [f for f in features if _feature_matches(f, topic_lower)]
        if hits:
            matched[section] = hits
    return matched


def _feature_matches(feature: dict, topic: str) -> bool:
    """Check if a feature dict matches a topic string across all text fields."""
    for value in feature.values():
        if isinstance(value, str) and topic in value.lower():
            return True
    return False


def _build_feature_catalogue() -> dict:
    """Build a structured catalogue by reading KCS feature docs from disk."""
    import glob

    features_dir = _find_features_dir()
    if features_dir is None:
        return {"error": "Could not locate docs/kcs/features/ directory"}

    features: list[dict] = []
    for path in sorted(glob.glob(os.path.join(features_dir, "kcs-feature-*.md"))):
        parsed = _parse_kcs_feature(path)
        if parsed:
            features.append(parsed)

    # Group by surface for structured output
    catalogue: dict[str, list[dict]] = {}
    for feat in features:
        surfaces = [s.strip() for s in feat.get("surface", "").split(",")]
        # Primary grouping: put each feature under its first surface category
        cat = _surface_category(surfaces)
        catalogue.setdefault(cat, []).append(feat)

    return catalogue


def _find_features_dir() -> str | None:
    """Locate docs/kcs/features/ relative to the repo root."""
    # Walk up from this file to find the repo root
    here = os.path.dirname(os.path.abspath(__file__))
    for _ in range(5):
        candidate = os.path.join(here, "docs", "kcs", "features")
        if os.path.isdir(candidate):
            return candidate
        here = os.path.dirname(here)
    return None


def _parse_kcs_feature(path: str) -> dict | None:
    """Parse a KCS feature markdown file into a structured dict."""
    import re

    try:
        with open(path) as f:
            content = f.read()
    except OSError:
        return None

    result: dict = {"file": os.path.basename(path)}

    # Parse title: # KCS: feature — <name>
    title_match = re.search(r"^#\s+KCS:\s+feature\s+—\s+(.+)$", content, re.MULTILINE)
    if title_match:
        result["name"] = title_match.group(1).strip()
    else:
        return None

    # Parse sections by heading
    sections = re.split(r"^##\s+", content, flags=re.MULTILINE)
    for section in sections[1:]:
        lines = section.strip().split("\n", 1)
        heading = lines[0].strip()
        body = lines[1].strip() if len(lines) > 1 else ""
        key = heading.lower().replace(" ", "_").replace("/", "_")
        result[key] = body

    return result


def _surface_category(surfaces: list[str]) -> str:
    """Map surface tags to human-readable category names."""
    surface_set = set(surfaces)
    if "all-editors" in surface_set or "lsp" in surface_set:
        if "mcp" in surface_set or "claude-code" in surface_set:
            return "LSP + AI Features"
        return "LSP Features (all editors)"
    if "vscode-chat" in surface_set:
        return "VS Code AI Chat"
    if "claude-code" in surface_set:
        return "Claude Code Skills"
    if "mcp" in surface_set:
        return "MCP Tools"
    if "vscode-command" in surface_set:
        return "VS Code Commands"
    return "Other"


# Subcommand: generate-test


def cmd_generate_test(source: str, file_path: str) -> None:
    """Generate a test script for an iRule using the test framework.

    Analyzes the iRule source to extract events, commands, object references,
    and variable flow, then generates a complete Tcl test script using the
    iRule Event Orchestrator test framework.
    """

    from ai.shared.irule_analysis import ordered_events as _ordered_events

    basename = os.path.basename(file_path)

    # Extract events in firing order
    evt_list = _ordered_events(source)
    ordered_events = [e.name for e in evt_list]

    # Determine needed profiles from events
    profiles = _infer_profiles(ordered_events)

    # Extract iRule commands used
    commands_used = _extract_irule_commands(source)

    # Extract object references (pools, data groups, nodes, virtuals)
    objects = _extract_object_refs(source, commands_used)

    # Detect variable usage patterns
    variables = _extract_variables(source)

    # Generate the test script
    test_script = _build_test_script(
        basename=basename,
        source=source,
        ordered_events=ordered_events,
        profiles=profiles,
        commands_used=commands_used,
        objects=objects,
        variables=variables,
    )

    print(test_script)


def _infer_profiles(events: list[str]) -> list[str]:
    """Infer required profiles from events present in the iRule."""
    profiles = ["TCP"]

    http_events = {
        "HTTP_REQUEST",
        "HTTP_RESPONSE",
        "HTTP_REQUEST_DATA",
        "HTTP_RESPONSE_DATA",
        "HTTP_REQUEST_RELEASE",
        "HTTP_RESPONSE_RELEASE",
        "HTTP_REQUEST_SEND",
    }
    ssl_events = {
        "CLIENTSSL_HANDSHAKE",
        "CLIENTSSL_DATA",
        "CLIENTSSL_CLIENTCERT",
        "CLIENTSSL_CLIENTHELLO",
        "SERVERSSL_HANDSHAKE",
        "SERVERSSL_DATA",
        "SERVERSSL_SERVERHELLO",
    }
    dns_events = {"DNS_REQUEST", "DNS_RESPONSE"}
    udp_events = {"CLIENT_DATA"}

    event_set = set(events)

    if event_set & http_events:
        profiles.append("HTTP")
    if event_set & ssl_events:
        profiles.insert(1, "CLIENTSSL")
    if event_set & dns_events:
        profiles = ["UDP", "DNS"]
    elif event_set & udp_events and not (event_set & http_events):
        profiles = ["UDP"]

    return profiles


def _extract_irule_commands(source: str) -> list[str]:
    """Extract iRule command names used in the source."""
    import re

    commands: set[str] = set()
    irule_pattern = re.compile(r"\b([A-Z][A-Z0-9]*::[a-z_][a-z0-9_]*)\b")
    for m in irule_pattern.finditer(source):
        commands.add(m.group(1))

    toplevel_cmds = {
        "pool",
        "node",
        "snat",
        "snatpool",
        "reject",
        "drop",
        "discard",
        "class",
        "table",
        "persist",
        "event",
        "log",
        "after",
        "virtual",
    }
    for cmd in toplevel_cmds:
        if re.search(rf"\b{cmd}\b", source):
            commands.add(cmd)

    return sorted(commands)


def _clean_ref(raw: str) -> str:
    """Strip Tcl syntax characters from a reference name."""
    return raw.strip("{}[]\"'")


def _extract_object_refs(source: str, commands: list[str]) -> dict:
    """Extract object references (pools, data groups, nodes, virtuals)."""
    import re

    objects: dict[str, list[str]] = {
        "pools": [],
        "datagroups": [],
        "nodes": [],
        "virtuals": [],
    }

    for m in re.finditer(r"\bpool\s+(\S+)", source):
        name = _clean_ref(m.group(1))
        if name and not name.startswith("$") and name not in ("member",):
            objects["pools"].append(name)

    for m in re.finditer(r"\bclass\s+(?:match|lookup)\s+\S+\s+\S+\s+(\S+)", source):
        name = _clean_ref(m.group(1))
        if name and not name.startswith("$"):
            objects["datagroups"].append(name)

    for m in re.finditer(r"\bnode\s+(\d+\.\d+\.\d+\.\d+)", source):
        objects["nodes"].append(m.group(1))

    for m in re.finditer(r"\bvirtual\s+(\S+)", source):
        name = _clean_ref(m.group(1))
        if name and not name.startswith("$"):
            objects["virtuals"].append(name)

    for k in objects:
        objects[k] = sorted(set(objects[k]))

    return objects


def _extract_variables(source: str) -> dict:
    """Extract variable usage patterns from iRule source."""
    import re

    variables: dict[str, list[str]] = {"static": [], "connection": []}

    for m in re.finditer(r"(?:set\s+|[\$])static::(\w+)", source):
        variables["static"].append(m.group(1))

    for k in variables:
        variables[k] = sorted(set(variables[k]))

    return variables


def _extract_test_paths(source: str) -> list[dict]:
    """Extract test-relevant paths from iRule source using the compiler IR.

    Walks the structured flow data produced by extract_diagram_data() and
    collects unique paths to terminal actions (pool, reject, redirect, etc.).
    Each path records the chain of conditions that lead to the action.

    Also runs SCCP and taint analysis to annotate paths with:
    - ``taint_warnings``: taint diagnostics relevant to the path's action
    - ``priority``: "high" (security), "normal" (routing), "low" (logging)
    - ``questions``: user-facing questions for the agentic loop

    Returns a list of path dicts::

        [
            {
                "event": "HTTP_REQUEST",
                "conditions": [...],
                "action": {"command": "pool", "args": ["api_pool"]},
                "path_label": "HTTP_REQUEST → ... → pool api_pool",
                "priority": "normal",
                "taint_warnings": [],
                "questions": [
                    {"question": "When host is 'api.example.com', which pool should be selected?",
                     "field": "expected_pool", "suggested": "api_pool"}
                ],
            },
            ...
        ]
    """
    from ai.shared.irule_analysis import diagram_data

    data = diagram_data(source)
    if "error" in data:
        return []

    # Run taint analysis for path annotation
    taint_warnings = _get_taint_warnings(source)

    paths: list[dict] = []

    def _walk_flow(
        nodes: list[dict],
        event_name: str,
        conditions: list[dict],
    ) -> None:
        """Recursively walk flow nodes, collecting paths to actions."""
        for node in nodes:
            kind = node.get("kind")

            if kind == "action":
                command = node.get("command", "")
                args = node.get("args", [])
                # Build human-readable label
                parts = [event_name]
                for c in conditions:
                    if c["kind"] == "if":
                        parts.append(f"{c['condition']} ({c['branch']})")
                    elif c["kind"] == "switch":
                        parts.append(f"switch {c['subject']} = {c['pattern']}")
                parts.append(f"{command} {' '.join(args)}".strip())

                path = {
                    "event": event_name,
                    "conditions": list(conditions),
                    "action": {"command": command, "args": args},
                    "path_label": " → ".join(parts),
                }

                # Annotate with priority, taint, and questions
                _annotate_path(path, taint_warnings)

                paths.append(path)

            elif kind == "if":
                for branch in node.get("branches", []):
                    cond = branch.get("condition", "")
                    is_else = cond == "else"
                    cond_entry = {
                        "kind": "if",
                        "condition": cond,
                        "branch": "else" if is_else else "true",
                    }
                    _walk_flow(
                        branch.get("body", []),
                        event_name,
                        conditions + [cond_entry],
                    )

            elif kind == "switch":
                subject = node.get("subject", "")
                for arm in node.get("arms", []):
                    pattern = arm.get("pattern", "")
                    cond_entry = {
                        "kind": "switch",
                        "subject": subject,
                        "pattern": pattern,
                    }
                    _walk_flow(
                        arm.get("body", []),
                        event_name,
                        conditions + [cond_entry],
                    )

            elif kind in ("loop", "catch", "try"):
                body = node.get("body", [])
                _walk_flow(body, event_name, conditions)

    for event in data.get("events", []):
        event_name = event.get("name", "")
        flow = event.get("flow", [])
        _walk_flow(flow, event_name, [])

    # Also walk procedures (helper procs may have actions)
    for proc in data.get("procedures", []):
        proc_name = proc.get("name", "")
        flow = proc.get("flow", [])
        _walk_flow(flow, f"proc:{proc_name}", [])

    # Sort by priority: high first, then normal, then low
    priority_order = {"high": 0, "normal": 1, "low": 2}
    paths.sort(key=lambda p: priority_order.get(p.get("priority", "normal"), 1))

    return paths


def _get_taint_warnings(source: str) -> list[dict]:
    """Run taint analysis and return warnings as simple dicts."""
    from ai.shared.irule_analysis import taint_warnings

    return taint_warnings(source)


# Actions that indicate security-sensitive paths
_SECURITY_ACTIONS = frozenset({"reject", "drop", "discard", "HTTP::respond"})
# Actions that indicate routing decisions
_ROUTING_ACTIONS = frozenset({"pool", "node", "snat", "snatpool", "virtual"})
# Taint codes that are security-critical
_SECURITY_TAINT_CODES = frozenset(
    {
        "T100",
        "T101",
        "T102",
        "IRULE3001",
        "IRULE3002",
        "IRULE3003",
    }
)


def _annotate_path(
    path: dict,
    taint_warnings: list[dict],
) -> None:
    """Annotate a path with priority, taint info, and agentic questions."""
    cmd = path["action"]["command"]
    conditions = path["conditions"]

    # --- Priority ---
    if cmd in _SECURITY_ACTIONS:
        path["priority"] = "high"
    elif cmd in _ROUTING_ACTIONS:
        path["priority"] = "normal"
    else:
        path["priority"] = "low"

    # Elevate priority if any condition references tainted sources
    tainted_refs = {
        "HTTP::uri",
        "HTTP::path",
        "HTTP::host",
        "HTTP::query",
        "HTTP::header",
        "HTTP::cookie",
        "URI::query",
    }
    for c in conditions:
        cond_text = c.get("condition", "") + c.get("subject", "")
        if any(ref in cond_text for ref in tainted_refs):
            if path["priority"] == "low":
                path["priority"] = "normal"

    # --- Taint warnings relevant to the action command ---
    relevant_taints = []
    for tw in taint_warnings:
        sink = tw.get("sink_command", "")
        if sink == cmd or (cmd == "pool" and sink in ("pool", "node")):
            relevant_taints.append(tw)
        # Also flag if any taint warning matches a security code
        if tw.get("code") in _SECURITY_TAINT_CODES:
            if path["priority"] != "high":
                path["priority"] = "high"
    path["taint_warnings"] = relevant_taints

    # --- Generate questions for the agentic loop ---
    path["questions"] = _generate_path_questions(path)


def _generate_path_questions(path: dict) -> list[dict]:
    """Generate user-facing questions for a specific code path.

    Returns questions that an AI agent should ask the user to produce
    accurate test assertions rather than guessing expected values.
    """
    questions: list[dict] = []
    cmd = path["action"]["command"]
    args = path["action"].get("args", [])
    conditions = path["conditions"]
    event = path["event"]

    # Build a human-readable condition summary
    cond_summary = _build_condition_summary(conditions)

    if cmd == "pool" and args:
        pool_name = args[0]
        if cond_summary:
            questions.append(
                {
                    "question": f"When {cond_summary}, should the request go to pool '{pool_name}'?",
                    "field": "expected_pool",
                    "suggested": pool_name,
                }
            )
        else:
            questions.append(
                {
                    "question": f"Should all requests in {event} go to pool '{pool_name}'?",
                    "field": "expected_pool",
                    "suggested": pool_name,
                }
            )

    elif cmd == "reject":
        if cond_summary:
            questions.append(
                {
                    "question": f"When {cond_summary}, should the connection be rejected?",
                    "field": "expected_reject",
                    "suggested": "yes",
                }
            )
        else:
            questions.append(
                {
                    "question": f"Should all connections in {event} be rejected?",
                    "field": "expected_reject",
                    "suggested": "yes",
                }
            )

    elif cmd == "HTTP::redirect":
        redirect_target = args[0] if args else "?"
        questions.append(
            {
                "question": f"When {cond_summary or 'unconditionally'}, what URL should the redirect go to?",
                "field": "expected_redirect",
                "suggested": redirect_target,
            }
        )

    elif cmd == "HTTP::respond":
        status = args[0] if args else "200"
        questions.append(
            {
                "question": f"When {cond_summary or 'unconditionally'}, what HTTP status should be returned?",
                "field": "expected_status",
                "suggested": status,
            }
        )

    elif cmd == "node" and args:
        questions.append(
            {
                "question": f"When {cond_summary or 'unconditionally'}, should traffic go to node '{args[0]}'?",
                "field": "expected_node",
                "suggested": args[0],
            }
        )

    # For paths with conditions on user input, ask about input values
    for c in conditions:
        if c["kind"] == "if" and c["branch"] == "else":
            questions.append(
                {
                    "question": "What input value should trigger the 'else' / fallback path?",
                    "field": "fallback_input",
                    "suggested": None,
                }
            )
            break
        if c["kind"] == "switch" and c.get("pattern") == "default":
            questions.append(
                {
                    "question": f"What input value should trigger the 'default' case of switch on {c.get('subject', '?')}?",
                    "field": "default_input",
                    "suggested": None,
                }
            )
            break

    # For tainted paths, ask about sanitization expectations
    if path.get("taint_warnings"):
        tw = path["taint_warnings"][0]
        questions.append(
            {
                "question": f"The variable '{tw.get('variable', '?')}' carries user input into '{tw.get('sink_command', cmd)}'. Is sanitization needed here?",
                "field": "needs_sanitization",
                "suggested": "yes",
            }
        )

    return questions


def _build_condition_summary(conditions: list[dict]) -> str:
    """Build a human-readable summary of path conditions for questions."""
    parts: list[str] = []
    for c in conditions:
        if c["kind"] == "if":
            cond = c.get("condition", "")
            if cond == "else":
                parts.append("no prior conditions match")
            else:
                parts.append(cond)
        elif c["kind"] == "switch":
            subject = c.get("subject", "?")
            pattern = c.get("pattern", "?")
            if pattern == "default":
                parts.append(f"{subject} doesn't match other cases")
            else:
                parts.append(f"{subject} matches '{pattern}'")
    return " and ".join(parts)


def _build_test_description(path: dict) -> str:
    """Build a sanitised test description from a path dict."""
    action = path["action"]
    conditions = path["conditions"]
    cmd = action["command"]

    desc_parts: list[str] = []
    for c in conditions:
        if c["kind"] == "if":
            desc_parts.append(c["condition"])
        elif c["kind"] == "switch":
            desc_parts.append(f"{c['subject']} = {c['pattern']}")

    if desc_parts:
        desc = f"{cmd} when {' and '.join(desc_parts)}"
    else:
        desc = f"{cmd} (unconditional)"
    # Sanitize for Tcl string (remove embedded quotes, truncate)
    desc = desc.replace('"', "'").replace("\\", "")
    if len(desc) > 80:
        desc = desc[:77] + "..."
    return desc


def _build_test_body(event_name: str, path: dict) -> list[str]:
    """Build the inner body lines for a CFG test case."""
    action = path["action"]
    conditions = path["conditions"]
    cmd = action["command"]
    args = action.get("args", [])
    body: list[str] = []

    # Condition comments
    if conditions:
        body.append("# Path conditions:")
        for c in conditions:
            if c["kind"] == "if":
                body.append(f"#   if {c['condition']} ({c['branch']})")
            elif c["kind"] == "switch":
                body.append(f"#   switch {c['subject']} matches {c['pattern']}")
        body.append("")

    # Request setup
    body.extend(_build_request_setup(event_name, conditions))
    body.append("")

    # Assertion
    body.extend(_build_assertion(cmd, args))
    return body


def _build_assertion(cmd: str, args: list[str]) -> list[str]:
    """Build assertion lines for a terminal action."""
    if cmd == "pool" and args:
        return [f'::orch::assert_that pool_selected equals "{args[0]}"']
    if cmd == "reject":
        return ["::orch::assert_that decision connection reject was_called"]
    if cmd in ("drop", "discard"):
        return [f"::orch::assert_that decision connection {cmd} was_called"]
    if cmd == "HTTP::redirect":
        return ["::orch::assert_that decision http redirect was_called"]
    if cmd == "HTTP::respond":
        return [
            "# Verify HTTP::respond was called",
            "::orch::assert_that decision http respond was_called",
        ]
    if cmd == "node" and args:
        return [
            f"# Verify node selection: {' '.join(args)}",
            f'::orch::assert_that node_selected equals "{args[0]}"',
        ]
    return [f"# Verify: {cmd} {' '.join(args)}"]


def _build_request_setup(
    event_name: str,
    conditions: list[dict],
) -> list[str]:
    """Build HTTP/DNS/TCP request setup lines based on path conditions.

    Examines the conditions to extract host, URI, header hints and generates
    appropriate ::orch::run_* calls with parameters that would satisfy the
    branch conditions.
    """
    import re

    # Extract hints from conditions
    host_hint = None
    uri_hint = None
    header_hints: list[tuple[str, str]] = []

    for c in conditions:
        cond_text = ""
        if c["kind"] == "if":
            cond_text = c.get("condition", "")
        elif c["kind"] == "switch":
            subject = c.get("subject", "")
            pattern = c.get("pattern", "")
            if "host" in subject.lower() or "HTTP::host" in subject:
                host_hint = _cfg_pattern_to_value(pattern)
            elif "uri" in subject.lower() or "HTTP::uri" in subject or "HTTP::path" in subject:
                uri_hint = _cfg_pattern_to_value(pattern)
            continue

        host_related = "HTTP::host" in cond_text or "host" in cond_text.lower()
        uri_related = (
            "HTTP::uri" in cond_text
            or "HTTP::path" in cond_text
            or "uri" in cond_text.lower()
            or "path" in cond_text.lower()
        )
        if host_related and not uri_related:
            m = re.search(r'eq\s+"([^"]+)"', cond_text)
            if m and host_hint is None:
                host_hint = m.group(1)
            elif host_hint is None:
                m = re.search(r"eq\s+(\S+)", cond_text)
                if m and not m.group(1).startswith("$"):
                    host_hint = m.group(1).strip('"{}')
        if uri_related:
            m = re.search(r'(?:eq|starts_with|matches)\s+"([^"]+)"', cond_text)
            if m and uri_hint is None:
                uri_hint = m.group(1)
            elif uri_hint is None:
                m = re.search(r"(?:eq|starts_with|matches)\s+(\S+)", cond_text)
                if m and not m.group(1).startswith("$"):
                    uri_hint = m.group(1).strip('"{}')
        if "HTTP::header" in cond_text:
            m = re.search(r'HTTP::header\s+"?([^"\s]+)"?', cond_text)
            if m:
                header_name = m.group(1)
                header_hints.append((header_name, "test-value"))

    # Generate the request
    result: list[str] = []
    if event_name in ("HTTP_REQUEST", "HTTP_RESPONSE") or "HTTP" in event_name:
        host = host_hint or "example.com"
        uri = uri_hint or "/"
        result.append(f'::orch::run_http_request -host "{host}" -uri "{uri}"')
        for hname, hval in header_hints:
            result.append(f"# Ensure header: {hname} = {hval}")
    elif event_name in ("DNS_REQUEST", "DNS_RESPONSE"):
        result.append('set ::state::dns::qname "example.com"')
        result.append('set ::state::dns::qtype "A"')
        result.append(f"::itest::fire_event {event_name}")
    elif event_name in ("CLIENT_ACCEPTED", "SERVER_CONNECTED"):
        result.append('::orch::configure -client_addr "10.0.0.1"')
        result.append(f"::itest::fire_event {event_name}")
    else:
        result.append(f"::itest::fire_event {event_name}")
    return result


def _cfg_pattern_to_value(pattern: str) -> str:
    """Convert a switch glob/regex pattern to a concrete test value.

    For example, '/api/*' -> '/api/test', 'example.com' -> 'example.com'.
    """
    # Strip surrounding quotes/braces
    pattern = pattern.strip('"{}')

    if pattern == "default":
        return None  # type: ignore[return-value]

    # Replace glob wildcards with concrete values
    if "*" in pattern:
        return pattern.replace("*", "test")
    if "?" in pattern:
        return pattern.replace("?", "x")
    return pattern


def _needs_multi_tmm(source: str, commands_used: list[str], variables: dict) -> bool:
    """Detect whether an iRule should be tested in multi-TMM mode.

    Returns True when the iRule uses patterns that behave differently
    across TMMs: static:: variables written outside RULE_INIT, shared
    counters, table commands used for cross-connection state, etc.
    """
    import re

    static_vars = variables.get("static", [])
    uses_table = "table" in commands_used

    # Pattern 1: static:: written in hot events (not just RULE_INIT)
    # Look for set static:: or incr static:: outside RULE_INIT
    hot_events = {
        "HTTP_REQUEST",
        "HTTP_RESPONSE",
        "CLIENT_ACCEPTED",
        "SERVER_CONNECTED",
        "DNS_REQUEST",
        "DNS_RESPONSE",
    }
    static_in_hot = False
    for event in hot_events:
        # Find the event body (rough heuristic)
        pattern = rf"when\s+{event}\s*\{{(.*?)\}}"
        m = re.search(pattern, source, re.DOTALL)
        if m:
            body = m.group(1)
            if re.search(r"\b(?:set|incr)\s+static::", body):
                static_in_hot = True
                break

    # Pattern 2: rate limiting / counter patterns
    has_counter_pattern = bool(re.search(r"\bincr\s+(?:static::\w+|\$?\w*count\w*)", source))

    # Pattern 3: uses table for shared state
    uses_shared_table = uses_table and bool(re.search(r"\btable\s+(?:incr|set|add)", source))

    return static_in_hot or (has_counter_pattern and static_vars) or uses_shared_table


def _build_test_script(
    basename: str,
    source: str,
    ordered_events: list[str],
    profiles: list[str],
    commands_used: list[str],
    objects: dict,
    variables: dict,
    *,
    _cfg_paths: list[dict] | None = None,
    _multi_tmm: bool | None = None,
) -> str:
    """Build a complete Tcl test script from extracted iRule metadata.

    Uses Jinja2 templates for structure and the Tcl formatter as a
    post-generation filter.  Optional ``_cfg_paths`` and ``_multi_tmm``
    accept pre-computed values to avoid redundant work.
    """
    from ai.claude.templates.render import render_test_case, render_test_script

    test_name = basename.replace(".tcl", "").replace(".irul", "").replace(".irule", "")

    # Build shared setup block
    setup_lines = _build_setup_lines(objects, variables)

    # Build test case blocks
    cfg_paths = _cfg_paths if _cfg_paths is not None else _extract_test_paths(source)
    test_blocks = _build_all_test_blocks(
        test_name,
        cfg_paths,
        ordered_events,
        profiles,
        commands_used,
        objects,
        render_test_case,
    )

    # Multi-TMM config
    multi_tmm = (
        _multi_tmm if _multi_tmm is not None else _needs_multi_tmm(source, commands_used, variables)
    )
    multi_tmm_ctx = None
    if multi_tmm:
        multi_tmm_ctx = _build_multi_tmm_context(test_name, source, profiles, objects, variables)

    return render_test_script(
        test_name=test_name,
        basename=basename,
        source=source,
        profiles=profiles,
        setup_lines=setup_lines,
        test_blocks=test_blocks,
        multi_tmm=multi_tmm_ctx,
    )


def _build_setup_lines(objects: dict, variables: dict) -> list[str]:
    """Build the setup body lines for pools, data groups, and static vars."""
    setup_lines: list[str] = []
    for pool_name in objects.get("pools", []):
        setup_lines.append(f"    ::orch::add_pool {pool_name} {{{{10.0.0.1:80}} {{10.0.0.2:80}}}}")
    for dg_name in objects.get("datagroups", []):
        setup_lines.append(f"    ::orch::add_datagroup {dg_name} string {{")
        setup_lines.append('        "example_key" "example_value"')
        setup_lines.append("    }")
    for var in variables.get("static", []):
        setup_lines.append(f'    ::orch::configure_static {var} ""')
    return setup_lines


def _build_all_test_blocks(
    test_name: str,
    cfg_paths: list[dict],
    ordered_events: list[str],
    profiles: list[str],
    commands_used: list[str],
    objects: dict,
    render_fn: object,
) -> list[str]:
    """Build all test case blocks as rendered strings."""
    from collections import defaultdict

    blocks: list[str] = []

    if cfg_paths:
        # CFG-informed: generate tests based on actual control flow paths
        blocks.append(
            "# CFG-informed test cases"
            "#\n"
            "# Generated from control flow analysis of the iRule.\n"
            "# Each test targets a specific branch path through the code."
        )

        by_event: dict[str, list[dict]] = defaultdict(list)
        for p in cfg_paths:
            by_event[p["event"]].append(p)

        test_idx = 0
        for event_name, event_paths in by_event.items():
            if event_name.startswith("proc:"):
                continue
            for path in event_paths:
                test_idx += 1
                desc = _build_test_description(path)
                body = _build_test_body(event_name, path)
                blocks.append(
                    render_fn(
                        test_id=f"{test_name}-cfg-{test_idx}.0",
                        desc=desc,
                        body=body,
                    )
                )
    else:
        # Fallback: generate template-based tests from command heuristics
        blocks.append("# Test cases")
        has_http = "HTTP" in profiles
        has_dns = "DNS" in profiles

        if has_http:
            blocks.extend(
                _build_http_test_blocks(
                    test_name, ordered_events, commands_used, objects, render_fn
                )
            )
        elif has_dns:
            blocks.extend(_build_dns_test_blocks(test_name, render_fn))
        else:
            blocks.extend(_build_tcp_test_blocks(test_name, render_fn))

    return blocks


def _build_http_test_blocks(
    test_name: str,
    events: list[str],
    commands: list[str],
    objects: dict,
    render_fn: object,
) -> list[str]:
    """Build HTTP test case blocks as rendered strings."""
    blocks: list[str] = []

    # Test 1: happy path
    body = ['::orch::run_http_request -host "example.com" -uri "/"']
    if "pool" in commands and objects.get("pools"):
        body.append(f'::orch::assert_that pool_selected equals "{objects["pools"][0]}"')
    else:
        body.append('# ::orch::assert_that pool_selected equals "your_pool"')
    if "reject" in commands or "drop" in commands:
        body.append("::orch::assert_that decision connection reject was_not_called")
    blocks.append(render_fn(test_id=f"{test_name}-1.0", desc="basic request routing", body=body))

    # Test 2: headers
    if any(c.startswith("HTTP::header") for c in commands):
        blocks.append(
            render_fn(
                test_id=f"{test_name}-1.1",
                desc="header manipulation",
                body=[
                    '::orch::run_http_request -host "example.com" -uri "/"',
                    '# ::orch::assert_that http_header "X-Custom" equals "value"',
                ],
            )
        )

    # Test 3: edge case
    blocks.append(
        render_fn(
            test_id=f"{test_name}-2.0",
            desc="empty host handling",
            body=[
                '::orch::run_http_request -host "" -uri "/"',
                '# ::orch::assert_that pool_selected equals "default_pool"',
            ],
        )
    )

    # Test 4: rejection path
    if "reject" in commands or "drop" in commands:
        body = [
            '::orch::run_http_request -host "evil.com" -uri "/"',
            "::orch::assert_that decision connection reject was_called",
        ]
        if "log" in commands:
            body.append('# ::orch::assert_that log matches "*rejected*"')
        blocks.append(render_fn(test_id=f"{test_name}-2.1", desc="rejects bad requests", body=body))

    # Test 5: redirect
    if "HTTP::redirect" in commands:
        blocks.append(
            render_fn(
                test_id=f"{test_name}-3.0",
                desc="redirect behavior",
                body=[
                    '::orch::run_http_request -host "example.com" -uri "/"',
                    "::orch::assert_that decision http redirect was_called",
                ],
            )
        )

    # Test 6: keep-alive
    if "HTTP_RESPONSE" in events:
        body = ['::orch::run_http_request -host "example.com" -uri "/first"']
        if "pool" in commands and objects.get("pools"):
            pool = objects["pools"][0]
            body.append(f'::orch::assert_that pool_selected equals "{pool}"')
        body.append("")
        body.append('::orch::run_next_request -host "example.com" -uri "/second"')
        if "pool" in commands and objects.get("pools"):
            body.append(f'::orch::assert_that pool_selected equals "{pool}"')
        body.append("")
        body.append("::orch::close_connection")
        blocks.append(
            render_fn(
                test_id=f"{test_name}-4.0",
                desc="keep-alive multiple requests",
                body=body,
            )
        )

    return blocks


def _build_dns_test_blocks(test_name: str, render_fn: object) -> list[str]:
    """Build DNS test case blocks."""
    return [
        render_fn(
            test_id=f"{test_name}-1.0",
            desc="DNS query handling",
            body=[
                'set ::state::dns::qname "example.com"',
                'set ::state::dns::qtype "A"',
                "::itest::fire_event DNS_REQUEST",
                "# ::orch::assert_that decision dns return was_called",
            ],
        )
    ]


def _build_tcp_test_blocks(test_name: str, render_fn: object) -> list[str]:
    """Build TCP test case blocks."""
    return [
        render_fn(
            test_id=f"{test_name}-1.0",
            desc="TCP connection handling",
            body=[
                '::orch::configure -client_addr "10.0.0.1"',
                "::itest::fire_event CLIENT_ACCEPTED",
                "::orch::assert_that decision connection reject was_not_called",
            ],
        )
    ]


def _build_multi_tmm_context(
    test_name: str,
    source: str,
    profiles: list[str],
    objects: dict,
    variables: dict,
) -> dict:
    """Build the template context dict for multi-TMM test generation."""
    static_vars = variables.get("static", [])
    check_var = None
    for v in static_vars:
        if v not in ("rate_limit", "mode", "version", "debug"):
            check_var = v
            break
    return {
        "source": source.strip(),
        "profiles_str": " ".join(profiles),
        "pools": objects.get("pools", []),
        "test_name": test_name,
        "check_var": check_var,
    }


def _build_test_script_with_metadata(
    basename: str,
    source: str,
    ordered_events: list[str],
    profiles: list[str],
    commands_used: list[str],
    objects: dict,
    variables: dict,
) -> tuple[str, dict]:
    """Build a test script and return (script, metadata) without redundant work.

    The metadata dict contains ``cfg_paths`` and ``multi_tmm_detected`` that
    ``_build_test_script`` already computes internally.  This avoids MCP
    callers having to recompute them.
    """
    cfg_paths = _extract_test_paths(source)
    multi_tmm = _needs_multi_tmm(source, commands_used, variables)

    script = _build_test_script(
        basename=basename,
        source=source,
        ordered_events=ordered_events,
        profiles=profiles,
        commands_used=commands_used,
        objects=objects,
        variables=variables,
        _cfg_paths=cfg_paths,
        _multi_tmm=multi_tmm,
    )
    return script, {"cfg_paths": cfg_paths, "multi_tmm_detected": multi_tmm}


# CLI


def _configure_dialect_from_path(file_path: str) -> None:
    """Configure the command registry dialect based on file extension."""
    from core.commands.registry.runtime import configure_signatures

    ext = os.path.splitext(file_path)[1].lower()
    dialect = "f5-irules" if ext in (".irul", ".irule") else "tcl8.6"
    configure_signatures(dialect=dialect)


def _read_file(path: str) -> str:
    abs_path = os.path.abspath(path)
    if not os.path.isfile(abs_path):
        print(f"Error: file not found: {abs_path}", file=sys.stderr)
        sys.exit(1)
    with open(abs_path) as f:
        return f.read()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Standalone Tcl/iRules analysis tool for Claude Code AI skills",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""\
examples:
  %(prog)s context samples/for_screenshots/ai-scene.irul
  %(prog)s diagnostics samples/for_screenshots/ai-scene.irul
  %(prog)s validate samples/for_screenshots/ai-scene.irul
  %(prog)s review samples/for_screenshots/ai-scene.irul
  %(prog)s convert samples/for_screenshots/ai-scene.irul
  %(prog)s diagram samples/for_screenshots/ai-scene.irul
  %(prog)s event-order samples/for_screenshots/ai-scene.irul
  %(prog)s event-info HTTP_REQUEST
  %(prog)s command-info HTTP::uri
  %(prog)s optimize samples/for_screenshots/22-optimiser-before.tcl
""",
    )

    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("context", help="Build context pack (diagnostics + symbols + events)")
    p.add_argument("file", help="Tcl/iRule file to analyze")

    p = sub.add_parser("diagnostics", help="Show diagnostics from the analyzer")
    p.add_argument("file", help="Tcl/iRule file to analyze")

    p = sub.add_parser("validate", help="Categorized validation report")
    p.add_argument("file", help="Tcl/iRule file to validate")

    p = sub.add_parser("review", help="Security-focused diagnostic report")
    p.add_argument("file", help="Tcl/iRule file to review")

    p = sub.add_parser("convert", help="Detect legacy patterns eligible for modernisation")
    p.add_argument("file", help="Tcl/iRule file to scan")

    p = sub.add_parser("symbols", help="Show document symbol hierarchy")
    p.add_argument("file", help="Tcl/iRule file to analyze")

    p = sub.add_parser("diagram", help="Extract control flow from compiler IR")
    p.add_argument("file", help="Tcl/iRule file to analyze")

    p = sub.add_parser("optimize", help="Show optimization suggestions and rewritten source")
    p.add_argument("file", help="Tcl/iRule file to optimize")

    p = sub.add_parser("event-order", help="Show events in canonical firing order")
    p.add_argument("file", help="Tcl/iRule file to analyze")

    p = sub.add_parser("event-info", help="Show iRules event registry metadata")
    p.add_argument("event", help="iRules event name (e.g. HTTP_REQUEST)")

    p = sub.add_parser("command-info", help="Show iRules command registry metadata")
    p.add_argument("name", help="iRules command name (e.g. HTTP::uri)")

    p = sub.add_parser("call-graph", help="Build call graph (procs, call edges, roots, leaves)")
    p.add_argument("file", help="Tcl/iRule file to analyze")

    p = sub.add_parser("symbol-graph", help="Build symbol relationship graph (scopes, defs, refs)")
    p.add_argument("file", help="Tcl/iRule file to analyze")

    p = sub.add_parser(
        "dataflow-graph", help="Build data-flow graph (taint, effects, tainted vars)"
    )
    p.add_argument("file", help="Tcl/iRule file to analyze")

    p = sub.add_parser("tk-layout", help="Extract Tk widget tree from source")
    p.add_argument("file", help="Tcl/Tk file to analyze")

    p = sub.add_parser("generate-test", help="Generate iRule test script from source")
    p.add_argument("file", help="iRule file to generate tests for")

    p = sub.add_parser("cfg-paths", help="Extract test-relevant CFG paths from iRule source")
    p.add_argument("file", help="iRule file to analyze")

    p = sub.add_parser("refactor", help="List available refactorings for a file")
    p.add_argument("file", help="Tcl/iRule file to scan for refactorings")

    p = sub.add_parser("suggest-datagroups", help="AI-enhanced data-group extraction scan")
    p.add_argument("file", help="iRule file to scan for data-group candidates")

    p = sub.add_parser("extract-datagroup", help="Static data-group extraction at a line")
    p.add_argument("file", help="iRule file")
    p.add_argument("--line", type=int, required=True, help="0-based line of the if/switch")
    p.add_argument("--name", default="", help="Data-group name (auto-generated if empty)")

    p = sub.add_parser("help", help="Show available features and how to use them")
    p.add_argument(
        "topic",
        nargs="?",
        default=None,
        help="Optional topic to filter (e.g. 'mcp', 'irule', 'format')",
    )

    args = parser.parse_args()

    # Commands that don't need a file
    if args.command == "help":
        cmd_help(args.topic)
        return
    if args.command == "event-info":
        cmd_event_info(args.event)
        return
    if args.command == "command-info":
        cmd_command_info(args.name)
        return

    # Commands that need a file
    source = _read_file(args.file)
    file_path = os.path.abspath(args.file)

    match args.command:
        case "context":
            cmd_context(source, file_path)
        case "diagnostics":
            cmd_diagnostics(source, file_path)
        case "validate":
            cmd_validate(source, file_path)
        case "review":
            cmd_review(source, file_path)
        case "convert":
            cmd_convert(source, file_path)
        case "symbols":
            cmd_symbols(source, file_path)
        case "diagram":
            cmd_diagram(source, file_path)
        case "optimize":
            cmd_optimize(source, file_path)
        case "event-order":
            cmd_event_order(source, file_path)
        case "call-graph":
            cmd_call_graph(source, file_path)
        case "symbol-graph":
            cmd_symbol_graph(source, file_path)
        case "dataflow-graph":
            cmd_dataflow_graph(source, file_path)
        case "tk-layout":
            from core.tk.extract import extract_tk_layout

            layout = extract_tk_layout(source)
            print(json.dumps(layout, indent=2))
        case "generate-test":
            cmd_generate_test(source, file_path)
        case "cfg-paths":
            paths = _extract_test_paths(source)
            print(json.dumps(paths, indent=2))
        case "refactor":
            cmd_refactor(source, file_path)
        case "suggest-datagroups":
            cmd_suggest_datagroups(source, file_path)
        case "extract-datagroup":
            cmd_extract_datagroup(source, file_path, args.line, args.name)


def cmd_refactor(source: str, file_path: str) -> None:
    """List all available refactorings in the source file."""
    _configure_dialect_from_path(file_path)

    from core.parsing.command_segmenter import segment_commands
    from core.refactoring._brace_expr import brace_expr
    from core.refactoring._extract_datagroup import extract_to_datagroup
    from core.refactoring._if_to_switch import if_to_switch
    from core.refactoring._inline_variable import inline_variable
    from core.refactoring._switch_to_dict import switch_to_dict

    available: list[dict] = []
    for seg in segment_commands(source):
        line = seg.range.start.line
        char = seg.range.start.character

        r = inline_variable(source, line, char)
        if r:
            available.append({"line": line, "tool": "inline_variable", "title": r.title})

        r2 = if_to_switch(source, line, char)
        if r2:
            available.append({"line": line, "tool": "if_to_switch", "title": r2.title})

        r3 = switch_to_dict(source, line, char)
        if r3:
            available.append({"line": line, "tool": "switch_to_dict", "title": r3.title})

        r4 = brace_expr(source, line, char)
        if r4:
            available.append({"line": line, "tool": "brace_expr", "title": r4.title})

        r5 = extract_to_datagroup(source, line, char)
        if r5:
            available.append({"line": line, "tool": "extract_datagroup", "title": r5.title})

    print(f"## Available Refactorings ({len(available)})\n")
    if not available:
        print("No refactorings available.")
        return
    for item in available:
        print(f"- Line {item['line']}: **{item['title']}** (`{item['tool']}`)")


def cmd_suggest_datagroups(source: str, file_path: str) -> None:
    """AI-enhanced data-group extraction scan."""
    _configure_dialect_from_path(file_path)

    from core.refactoring._extract_datagroup import suggest_datagroup_extraction

    candidates = suggest_datagroup_extraction(source)
    print(f"## Data-Group Extraction Candidates ({len(candidates)})\n")
    if not candidates:
        print("No data-group extraction candidates found.")
        return
    for i, c in enumerate(candidates, 1):
        static = "yes" if c.get("static_result") is not None else "no"
        print(f"### Candidate {i}: {c['suggested_name']}")
        print(f"- **Line**: {c['line']}")
        print(f"- **Pattern**: {c['pattern_type']}")
        print(f"- **Variable**: `${c['variable']}`")
        print(f"- **Type**: {c['inferred_type']}")
        if c.get("has_cidr"):
            print("- **CIDR**: yes (contains network prefixes)")
        print(f"- **Values** ({c['value_count']}): {', '.join(repr(v) for v in c['values'][:10])}")
        if c["value_count"] > 10:
            print(f"  ... and {c['value_count'] - 10} more")
        print(f"- **Body shape**: {c['body_shape']}")
        print(f"- **Confidence**: {c['confidence']}")
        print(f"- **Static extraction**: {static}")
        print()


def cmd_extract_datagroup(source: str, file_path: str, line: int, dg_name: str) -> None:
    """Static data-group extraction at a specific line."""
    _configure_dialect_from_path(file_path)

    from core.refactoring._extract_datagroup import extract_to_datagroup

    result = extract_to_datagroup(source, line, 0, dg_name=dg_name)
    if result is None:
        print("No data-group extraction possible at this line.")
        return

    print(f"## {result.title}\n")
    print("### Data-group definition (tmsh)\n")
    print("```")
    print(result.data_group_tcl())
    print("```\n")
    print("### Rewritten iRule code\n")
    print("```tcl")
    print(result.apply(source))
    print("```")


if __name__ == "__main__":
    main()
