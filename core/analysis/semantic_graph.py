"""Semantic graph queries for Tcl/iRules source code.

Assembles data from the existing analysis infrastructure (analyser,
compiler, interprocedural analysis, taint analysis) into JSON-serializable
graph structures suitable for AI agent consumption.

Three graph types are exposed:

- **Call graph** — procs as nodes, call edges between them.
- **Symbol graph** — scope hierarchy with proc/variable definitions and
  reference locations.
- **Data-flow graph** — taint warnings, tainted variables, per-proc
  side-effect classification.
"""

from __future__ import annotations

from typing import Any

from core.analysis.semantic_model import AnalysisResult, Range, Scope

# Range serialisation (standalone — no MCP server dependency)


def _range_to_dict(r: Range) -> dict[str, Any]:
    return {
        "start": {"line": r.start.line, "character": r.start.character},
        "end": {"line": r.end.line, "character": r.end.character},
    }


def _pos_dict(line: int, character: int) -> dict[str, int]:
    return {"line": line, "character": character}


def find_proc_call_sites(name: str, qualified_name: str, analysis: AnalysisResult) -> list[Range]:
    """Find all locations where a proc is called."""
    qualified_no_prefix = qualified_name[2:] if qualified_name.startswith("::") else qualified_name
    call_forms = {name, qualified_name, qualified_no_prefix}
    locations: list[Range] = []
    seen: set[tuple[int, int, int, int]] = set()
    for invocation in analysis.command_invocations:
        resolved_name = invocation.resolved_qualified_name
        if resolved_name is not None:
            matches_target = resolved_name == qualified_name
        else:
            matches_target = invocation.name in call_forms
        if matches_target:
            key = (
                invocation.range.start.line,
                invocation.range.start.character,
                invocation.range.end.line,
                invocation.range.end.character,
            )
            if key in seen:
                continue
            seen.add(key)
            locations.append(invocation.range)
    return locations


# 1. Call Graph


def build_call_graph(
    source: str,
    *,
    cu=None,
    analysis: AnalysisResult | None = None,
) -> dict[str, Any]:
    """Build the full call graph for *source*.

    Returns a dict with ``nodes`` (procs), ``edges`` (caller→callee),
    ``roots`` (uncalled entry points), and ``leaf_procs`` (procs with
    no outgoing calls).
    """
    from core.analysis.analyser import analyse
    from core.compiler.compilation_unit import ensure_compilation_unit

    cu = ensure_compilation_unit(source, cu, context="semantic_graph.call_graph")
    if cu is None:
        return {"nodes": [], "edges": [], "roots": [], "leaf_procs": []}

    analysis = analysis if analysis is not None else analyse(source, cu=cu)
    interproc = cu.interproc
    ir_module = cu.ir_module

    # Build node list from interproc summaries
    nodes: list[dict[str, Any]] = []
    proc_names: set[str] = set()
    for qname, summary in interproc.procedures.items():
        proc_names.add(qname)
        ir_proc = ir_module.procedures.get(qname)
        line = ir_proc.range.start.line if ir_proc else None
        effects = _effect_region_str(summary.effect_reads | summary.effect_writes)
        nodes.append(
            {
                "name": qname,
                "params": list(summary.params),
                "line": line,
                "pure": summary.pure,
                "effects": effects,
            }
        )

    # Build edge list
    edges: list[dict[str, Any]] = []
    callees_by_caller: dict[str, set[str]] = {}
    called_set: set[str] = set()

    for qname, summary in interproc.procedures.items():
        callees_by_caller[qname] = set()
        for callee_qname in summary.calls:
            if callee_qname in proc_names:
                callees_by_caller[qname].add(callee_qname)
                called_set.add(callee_qname)
                # Find call site locations within this proc's body
                sites = _find_call_sites_in_scope(
                    analysis,
                    callee_qname,
                    qname,
                    ir_module,
                )
                edges.append(
                    {
                        "caller": qname,
                        "callee": callee_qname,
                        "call_sites": sites,
                    }
                )

    # Top-level calls
    top_level_calls = _find_top_level_calls(analysis, proc_names, ir_module)
    callees_by_caller["<top-level>"] = set()
    for callee_qname, sites in top_level_calls.items():
        callees_by_caller["<top-level>"].add(callee_qname)
        called_set.add(callee_qname)
        edges.append(
            {
                "caller": "<top-level>",
                "callee": callee_qname,
                "call_sites": sites,
            }
        )

    # Roots = uncalled procs (entry points)
    roots: list[str] = [qn for qn in proc_names if qn not in called_set]
    roots.sort()
    if top_level_calls or any(
        inv for inv in analysis.command_invocations if not _is_inside_proc(inv.range, ir_module)
    ):
        roots.insert(0, "<top-level>")

    # Leaves = procs that don't call other procs
    leaf_procs = [qn for qn in proc_names if not callees_by_caller.get(qn)]
    leaf_procs.sort()

    return {
        "nodes": nodes,
        "edges": edges,
        "roots": roots,
        "leaf_procs": leaf_procs,
    }


def _find_call_sites_in_scope(
    analysis: AnalysisResult,
    callee_qname: str,
    caller_qname: str,
    ir_module: Any,
) -> list[dict[str, int]]:
    """Find call site locations of *callee_qname* within *caller_qname*'s body."""
    ir_proc = ir_module.procedures.get(caller_qname)
    if ir_proc is None:
        return []

    callee_short = callee_qname.lstrip(":")
    callee_forms = {callee_qname, callee_short}
    if callee_qname.startswith("::"):
        callee_forms.add(callee_qname[2:])

    sites: list[dict[str, int]] = []
    for inv in analysis.command_invocations:
        if inv.range.start.line < ir_proc.range.start.line:
            continue
        if inv.range.end.line > ir_proc.range.end.line:
            continue
        resolved = inv.resolved_qualified_name
        if resolved is not None:
            if resolved != callee_qname:
                continue
        elif inv.name not in callee_forms:
            continue
        sites.append(_pos_dict(inv.range.start.line, inv.range.start.character))

    return sites


def _find_top_level_calls(
    analysis: AnalysisResult,
    proc_names: set[str],
    ir_module: Any,
) -> dict[str, list[dict[str, int]]]:
    """Find calls to known procs from top-level (outside any proc body)."""
    result: dict[str, list[dict[str, int]]] = {}

    for inv in analysis.command_invocations:
        if _is_inside_proc(inv.range, ir_module):
            continue
        target_qname = _resolve_invocation_target(inv, proc_names)
        if target_qname is not None:
            result.setdefault(target_qname, []).append(
                _pos_dict(inv.range.start.line, inv.range.start.character)
            )

    return result


def _is_inside_proc(r: Range, ir_module: Any) -> bool:
    for ir_proc in ir_module.procedures.values():
        pr = ir_proc.range
        if r.start.line >= pr.start.line and r.end.line <= pr.end.line:
            return True
    return False


def _resolve_invocation_target(
    inv: Any,
    proc_names: set[str],
) -> str | None:
    if inv.resolved_qualified_name and inv.resolved_qualified_name in proc_names:
        return inv.resolved_qualified_name
    # Try matching by name forms
    for qname in proc_names:
        short = qname[2:] if qname.startswith("::") else qname
        if inv.name == qname or inv.name == short or inv.name == qname.lstrip(":"):
            return qname
    return None


def _effect_region_str(er: Any) -> str:
    """Human-readable string for an EffectRegion IntFlag."""
    if not er:
        return "NONE"
    # IntFlag.__str__ varies by Python version; use name when available.
    name = getattr(er, "name", None)
    if name:
        return name
    # Decompose composite flags
    parts: list[str] = []
    for member in type(er):
        if member.value and (er & member) == member:
            parts.append(member.name)
    return "|".join(parts) if parts else str(int(er))


# 2. Symbol Graph


def build_symbol_graph(
    source: str,
    *,
    cu=None,
    analysis: AnalysisResult | None = None,
) -> dict[str, Any]:
    """Build the symbol relationship graph for *source*.

    Returns scope hierarchy, proc definitions with reference counts,
    variable definitions with reference locations, and package dependencies.
    """
    from core.analysis.analyser import analyse
    from core.compiler.compilation_unit import ensure_compilation_unit

    cu = ensure_compilation_unit(source, cu, context="semantic_graph.symbol_graph")
    analysis = analysis if analysis is not None else analyse(source, cu=cu)

    # Walk scope tree
    scopes = [_scope_to_dict(analysis.global_scope, analysis)]

    # Proc references (all call sites for each proc)
    proc_references: dict[str, list[dict[str, int]]] = {}
    for qname, proc_def in analysis.all_procs.items():
        sites = find_proc_call_sites(proc_def.name, proc_def.qualified_name, analysis)
        if sites:
            proc_references[qname] = [_pos_dict(r.start.line, r.start.character) for r in sites]

    # Package requires
    package_requires: list[dict[str, Any]] = []
    for pr in analysis.package_requires:
        entry: dict[str, Any] = {"name": pr.name, "line": pr.range.start.line}
        if pr.version is not None:
            entry["version"] = pr.version
        package_requires.append(entry)

    # Summary counts
    total_procs = len(analysis.all_procs)
    total_variables = _count_variables(analysis.global_scope)
    total_namespaces = _count_namespaces(analysis.global_scope)

    return {
        "scopes": scopes,
        "proc_references": proc_references,
        "package_requires": package_requires,
        "summary": {
            "total_procs": total_procs,
            "total_variables": total_variables,
            "total_namespaces": total_namespaces,
        },
    }


def _scope_to_dict(scope: Scope, analysis: AnalysisResult) -> dict[str, Any]:
    """Recursively serialize a scope to a dict."""
    procs: list[dict[str, Any]] = []
    for proc_def in scope.procs.values():
        ref_sites = [
            inv
            for inv in analysis.command_invocations
            if (inv.resolved_qualified_name == proc_def.qualified_name or inv.name == proc_def.name)
        ]
        procs.append(
            {
                "name": proc_def.name,
                "qualified_name": proc_def.qualified_name,
                "params": [p.name for p in proc_def.params],
                "line": proc_def.name_range.start.line if proc_def.name_range else None,
                "ref_count": len(ref_sites),
            }
        )

    variables: list[dict[str, Any]] = []
    for var_def in scope.variables.values():
        if var_def.definition_range:
            refs = [_pos_dict(r.start.line, r.start.character) for r in var_def.references]
            variables.append(
                {
                    "name": var_def.name,
                    "line": var_def.definition_range.start.line,
                    "references": refs,
                }
            )

    children: list[dict[str, Any]] = []
    for child in scope.children:
        if child.kind == "namespace":
            children.append(_scope_to_dict(child, analysis))
        elif child.kind == "proc":
            # Include proc-scope variables
            proc_vars: list[dict[str, Any]] = []
            for var_def in child.variables.values():
                if var_def.definition_range:
                    refs = [_pos_dict(r.start.line, r.start.character) for r in var_def.references]
                    proc_vars.append(
                        {
                            "name": var_def.name,
                            "line": var_def.definition_range.start.line,
                            "references": refs,
                        }
                    )
            if proc_vars:
                children.append(
                    {
                        "kind": "proc",
                        "name": child.name,
                        "variables": proc_vars,
                        "children": [],
                    }
                )

    result: dict[str, Any] = {
        "kind": scope.kind,
        "name": scope.name,
    }
    if procs:
        result["procs"] = procs
    if variables:
        result["variables"] = variables
    if children:
        result["children"] = children

    return result


def _count_variables(scope: Scope) -> int:
    count = sum(1 for v in scope.variables.values() if v.definition_range)
    for child in scope.children:
        count += _count_variables(child)
    return count


def _count_namespaces(scope: Scope) -> int:
    count = 0
    for child in scope.children:
        if child.kind == "namespace":
            count += 1
        count += _count_namespaces(child)
    return count


# 3. Data-flow Graph


def build_dataflow_graph(
    source: str,
    *,
    cu=None,
) -> dict[str, Any]:
    """Build the data-flow / taint graph for *source*.

    Returns taint warnings, tainted variables per scope, and per-proc
    side-effect classification.
    """
    from core.compiler.compilation_unit import ensure_compilation_unit
    from core.compiler.taint import (
        CollectWithoutReleaseWarning,
        ReleaseWithoutCollectWarning,
        TaintWarning,
        find_taint_warnings,
    )

    cu = ensure_compilation_unit(source, cu, context="semantic_graph.dataflow_graph")
    if cu is None:
        return {
            "taint_warnings": [],
            "tainted_variables": [],
            "proc_effects": [],
            "summary": {
                "total_taint_warnings": 0,
                "tainted_variable_count": 0,
                "pure_proc_count": 0,
                "impure_proc_count": 0,
            },
        }

    warnings = find_taint_warnings(source, cu=cu)
    interproc = cu.interproc

    # Taint warnings
    taint_warnings: list[dict[str, Any]] = []
    for w in warnings:
        entry: dict[str, Any] = {
            "code": w.code,
            "line": w.range.start.line,
            "message": w.message,
        }
        if isinstance(w, TaintWarning):
            entry["variable"] = w.variable
            entry["sink_command"] = w.sink_command
        elif isinstance(w, CollectWithoutReleaseWarning):
            entry["command"] = w.command
        elif isinstance(w, ReleaseWithoutCollectWarning):
            entry["command"] = w.command
        taint_warnings.append(entry)

    # Tainted variables per scope
    tainted_variables: list[dict[str, Any]] = []

    # Top-level tainted variables
    for (var_name, _version), lattice in cu.top_level.analysis.taints.items():
        if lattice.tainted:
            tainted_variables.append(
                {
                    "scope": "<top-level>",
                    "variable": var_name,
                }
            )

    # Per-proc tainted variables
    seen_tainted: set[tuple[str, str]] = set()
    for qname, fu in cu.procedures.items():
        for (var_name, _version), lattice in fu.analysis.taints.items():
            if lattice.tainted:
                key = (qname, var_name)
                if key not in seen_tainted:
                    seen_tainted.add(key)
                    tainted_variables.append(
                        {
                            "scope": qname,
                            "variable": var_name,
                        }
                    )

    # Proc effects
    proc_effects: list[dict[str, Any]] = []
    pure_count = 0
    impure_count = 0
    for qname, summary in interproc.procedures.items():
        reads = _effect_region_str(summary.effect_reads)
        writes = _effect_region_str(summary.effect_writes)
        proc_effects.append(
            {
                "name": qname,
                "pure": summary.pure,
                "reads": reads,
                "writes": writes,
                "has_barrier": summary.has_barrier,
            }
        )
        if summary.pure:
            pure_count += 1
        else:
            impure_count += 1

    return {
        "taint_warnings": taint_warnings,
        "tainted_variables": tainted_variables,
        "proc_effects": proc_effects,
        "summary": {
            "total_taint_warnings": len(taint_warnings),
            "tainted_variable_count": len(tainted_variables),
            "pure_proc_count": pure_count,
            "impure_proc_count": impure_count,
        },
    }


def build_semantic_graph_bundle(source: str) -> dict[str, Any]:
    """Build call/symbol/dataflow graphs via one shared ``CompilationUnit``."""
    from core.analysis.analyser import analyse
    from core.compiler.compilation_unit import ensure_compilation_unit

    cu = ensure_compilation_unit(source, context="semantic_graph.bundle")
    if cu is None:
        return {
            "call_graph": {"nodes": [], "edges": [], "roots": [], "leaf_procs": []},
            "symbol_graph": {
                "scopes": [],
                "proc_references": {},
                "package_requires": [],
                "summary": {"total_procs": 0, "total_variables": 0, "total_namespaces": 0},
            },
            "dataflow_graph": {
                "taint_warnings": [],
                "tainted_variables": [],
                "proc_effects": [],
                "summary": {
                    "total_taint_warnings": 0,
                    "tainted_variable_count": 0,
                    "pure_proc_count": 0,
                    "impure_proc_count": 0,
                },
            },
        }

    analysis = analyse(source, cu=cu)
    return {
        "call_graph": build_call_graph(source, cu=cu, analysis=analysis),
        "symbol_graph": build_symbol_graph(source, cu=cu, analysis=analysis),
        "dataflow_graph": build_dataflow_graph(source, cu=cu),
    }
