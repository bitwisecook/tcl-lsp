"""JSON serialisation for compiler explorer results.

Shared by the Flask web explorer and the Pyodide WASM worker.
"""

from __future__ import annotations

import sys

from compiler.cfg import CFGBranch, CFGGoto, CFGReturn
from compiler.dataflow_graph import dataflow_graph_to_dict
from compiler.gvn import RedundantComputation
from compiler.interprocedural import InterproceduralAnalysis
from compiler.ir import (
    IRBarrier,
    IRFor,
    IRIf,
    IRModule,
    IRScript,
    IRSwitch,
)
from compiler.irules_flow import EventOrderEntry, IrulesFlowWarning
from compiler.optimiser import Optimisation
from compiler.shimmer import ShimmerWarning, ThunkingWarning
from compiler.taint import (
    TaintWarning,
)
from compiler.types import TypeKind

from .formatters import (
    format_lattice,
    format_taint,
    format_type,
    preview,
    range_dict,
    stmt_color_class,
    stmt_kind,
    stmt_summary,
    to_str,
)
from .pipeline import CompilerExplorerResult, FunctionSnapshot

# IR


def _serialise_script(script: IRScript) -> list[dict]:
    out = []
    for stmt in script.statements:
        node: dict = {
            "kind": stmt_kind(stmt),
            "summary": stmt_summary(stmt),
            "colorClass": stmt_color_class(stmt),
            "range": range_dict(stmt.range),
        }
        if isinstance(stmt, IRIf):
            children = []
            for i, clause in enumerate(stmt.clauses, 1):
                children.append(
                    {
                        "label": f"clause {i}: {preview(clause.condition, 60)}",
                        "range": range_dict(clause.condition_range),
                        "body": _serialise_script(clause.body),
                    }
                )
            if stmt.else_body is not None:
                children.append(
                    {
                        "label": "else",
                        "range": range_dict(stmt.else_range) if stmt.else_range else None,
                        "body": _serialise_script(stmt.else_body),
                    }
                )
            node["children"] = children
        elif isinstance(stmt, IRFor):
            node["children"] = [
                {
                    "label": "init",
                    "range": range_dict(stmt.init_range),
                    "body": _serialise_script(stmt.init),
                },
                {
                    "label": f"condition: {preview(stmt.condition, 60)}",
                    "range": range_dict(stmt.condition_range),
                    "body": [],
                },
                {
                    "label": "next",
                    "range": range_dict(stmt.next_range),
                    "body": _serialise_script(stmt.next),
                },
                {
                    "label": "body",
                    "range": range_dict(stmt.body_range),
                    "body": _serialise_script(stmt.body),
                },
            ]
        elif isinstance(stmt, IRSwitch):
            children = []
            for arm in stmt.arms:
                children.append(
                    {
                        "label": f"{'fallthrough' if arm.fallthrough else 'arm'}: {preview(arm.pattern, 48)}",
                        "range": range_dict(arm.pattern_range),
                        "body": _serialise_script(arm.body) if arm.body else [],
                    }
                )
            if stmt.default_body is not None:
                children.append(
                    {
                        "label": "default",
                        "range": range_dict(stmt.default_range) if stmt.default_range else None,
                        "body": _serialise_script(stmt.default_body),
                    }
                )
            node["children"] = children
        out.append(node)
    return out


def _serialise_ir(ir_module: IRModule) -> dict:
    procs = {}
    for qname in sorted(ir_module.procedures):
        proc = ir_module.procedures[qname]
        procs[qname] = {
            "params": list(proc.params),
            "range": range_dict(proc.range),
            "body": _serialise_script(proc.body),
        }
    return {
        "topLevel": _serialise_script(ir_module.top_level),
        "procedures": procs,
    }


# CFG


def _terminator_dict(term) -> dict | None:
    if term is None:
        return None
    if isinstance(term, CFGGoto):
        return {
            "type": "goto",
            "target": term.target,
            "range": range_dict(term.range) if term.range else None,
        }
    if isinstance(term, CFGBranch):
        return {
            "type": "branch",
            "condition": preview(to_str(term.condition), 80),
            "trueTarget": term.true_target,
            "falseTarget": term.false_target,
            "range": range_dict(term.range) if term.range else None,
        }
    if isinstance(term, CFGReturn):
        return {
            "type": "return",
            "value": preview(term.value, 60) if term.value else None,
            "range": range_dict(term.range) if term.range else None,
        }
    return None


def _block_successors(term) -> list[str]:
    if isinstance(term, CFGGoto):
        return [term.target]
    if isinstance(term, CFGBranch):
        targets = [term.true_target]
        if term.false_target != term.true_target:
            targets.append(term.false_target)
        return targets
    return []


def _serialise_cfg_pre_ssa(snapshots: list[FunctionSnapshot]) -> list[dict]:
    out = []
    for snap in snapshots:
        blocks = []
        for bn, block in snap.cfg.blocks.items():
            stmts = [
                {
                    "summary": stmt_summary(stmt),
                    "colorClass": stmt_color_class(stmt),
                    "range": range_dict(stmt.range),
                }
                for stmt in block.statements
            ]
            successors = _block_successors(block.terminator)
            blocks.append(
                {
                    "name": bn,
                    "isEntry": bn == snap.cfg.entry,
                    "statements": stmts,
                    "terminator": _terminator_dict(block.terminator),
                    "successors": successors,
                }
            )
        out.append(
            {
                "name": snap.name,
                "entry": snap.cfg.entry,
                "blockCount": len(snap.cfg.blocks),
                "blocks": blocks,
            }
        )
    return out


def _serialise_cfg_post_ssa(snapshots: list[FunctionSnapshot]) -> list[dict]:
    out = []
    for snap in snapshots:
        blocks = []
        for bn, block in snap.cfg.blocks.items():
            ssa_block = snap.ssa.blocks.get(bn)
            phis = []
            if ssa_block and ssa_block.phis:
                for phi in ssa_block.phis:
                    incoming = {
                        pred: f"{phi.name}#{ver}" for pred, ver in sorted(phi.incoming.items())
                    }
                    phi_type = snap.analysis.types.get((phi.name, phi.version))
                    phis.append(
                        {
                            "name": phi.name,
                            "version": phi.version,
                            "incoming": incoming,
                            "type": format_type(phi_type) if phi_type else None,
                        }
                    )
            stmts = []
            for idx, stmt in enumerate(block.statements):
                uses = {}
                defs = {}
                if ssa_block and idx < len(ssa_block.statements):
                    ssa_stmt = ssa_block.statements[idx]
                    for name, ver in sorted(ssa_stmt.uses.items()):
                        u: dict = {"version": ver}
                        lattice = snap.analysis.values.get((name, ver))
                        if lattice:
                            u["lattice"] = format_lattice(lattice)
                        type_info = snap.analysis.types.get((name, ver))
                        if type_info and type_info.kind is not TypeKind.UNKNOWN:
                            u["type"] = format_type(type_info)
                        uses[name] = u
                    for name, ver in sorted(ssa_stmt.defs.items()):
                        d: dict = {"version": ver}
                        lattice = snap.analysis.values.get((name, ver))
                        if lattice:
                            d["lattice"] = format_lattice(lattice)
                        type_info = snap.analysis.types.get((name, ver))
                        if type_info and type_info.kind is not TypeKind.UNKNOWN:
                            d["type"] = format_type(type_info)
                        defs[name] = d
                stmts.append(
                    {
                        "summary": stmt_summary(stmt),
                        "colorClass": stmt_color_class(stmt),
                        "range": range_dict(stmt.range),
                        "uses": uses,
                        "defs": defs,
                    }
                )
            successors = _block_successors(block.terminator)
            blocks.append(
                {
                    "name": bn,
                    "isEntry": bn == snap.cfg.entry,
                    "isUnreachable": bn in snap.analysis.unreachable_blocks,
                    "phis": phis,
                    "statements": stmts,
                    "terminator": _terminator_dict(block.terminator),
                    "successors": successors,
                }
            )
            analysis = {
                "constantBranches": [
                    {
                        "block": b.block,
                        "condition": preview(to_str(b.condition), 60),
                        "value": b.value,
                        "takenTarget": b.taken_target,
                        "notTakenTarget": b.not_taken_target,
                    }
                    for b in snap.analysis.constant_branches
                ],
                "deadStores": [
                    {
                        "block": d.block,
                        "stmtIndex": d.statement_index,
                        "variable": d.variable,
                        "version": d.version,
                    }
                    for d in snap.analysis.dead_stores
                ],
                "unreachableBlocks": sorted(snap.analysis.unreachable_blocks),
                "inferredTypes": {
                    f"{name}#{ver}": format_type(tl)
                    for (name, ver), tl in sorted(snap.analysis.types.items())
                    if tl.kind in (TypeKind.KNOWN, TypeKind.SHIMMERED)
                },
            }
        out.append(
            {
                "name": snap.name,
                "entry": snap.cfg.entry,
                "blockCount": len(snap.cfg.blocks),
                "blocks": blocks,
                "analysis": analysis,
            }
        )
    return out


# Other passes


def _serialise_interproc(interproc: InterproceduralAnalysis) -> list[dict]:
    out = []
    for qname in sorted(interproc.procedures):
        s = interproc.procedures[qname]
        arity = f"{s.arity.min}+" if s.arity.is_unlimited else f"{s.arity.min}..{s.arity.max}"
        return_shape = "unknown"
        if s.returns_constant:
            return_shape = f"const({s.constant_return!r})"
        elif s.return_passthrough_param is not None:
            return_shape = f"passthrough({s.return_passthrough_param})"
        elif s.return_depends_on_params:
            return_shape = f"depends({','.join(s.return_depends_on_params)})"
        out.append(
            {
                "name": qname,
                "arity": arity,
                "pure": s.pure,
                "foldable": s.can_fold_static_calls,
                "returnShape": return_shape,
                "calls": list(s.calls),
                "hasBarrier": s.has_barrier,
                "hasUnknownCalls": s.has_unknown_calls,
                "writesGlobal": s.writes_global,
            }
        )
    return out


def _serialise_optimisations(opts: list[Optimisation]) -> list[dict]:
    return [
        {
            "code": o.code,
            "message": o.message,
            "range": range_dict(o.range),
            "replacement": o.replacement,
        }
        for o in opts
    ]


def _serialise_shimmer(warnings: list[ShimmerWarning | ThunkingWarning]) -> list[dict]:
    out = []
    for w in warnings:
        d: dict = {
            "code": w.code,
            "message": w.message,
            "range": range_dict(w.range),
        }
        if isinstance(w, ShimmerWarning):
            d["variable"] = w.variable
            d["fromType"] = w.from_type.name.lower()
            d["toType"] = w.to_type.name.lower()
            d["command"] = w.command
            d["inLoop"] = w.in_loop
        elif isinstance(w, ThunkingWarning):
            d["variable"] = w.variable
            d["typeA"] = w.type_a.name.lower()
            d["typeB"] = w.type_b.name.lower()
        out.append(d)
    return out


def _serialise_gvn(warnings: list[RedundantComputation]) -> list[dict]:
    return [
        {
            "code": w.code,
            "message": w.message,
            "expression": w.expression_text,
            "range": range_dict(w.range),
            "firstRange": range_dict(w.first_range),
        }
        for w in warnings
    ]


def _serialise_taint(
    warnings: list[TaintWarning],
) -> list[dict]:
    out = []
    for w in warnings:
        d: dict = {
            "code": w.code,
            "message": w.message,
            "range": range_dict(w.range),
        }
        if isinstance(w, TaintWarning):
            d["variable"] = w.variable
            d["sinkCommand"] = w.sink_command
        out.append(d)
    return out


def _serialise_irules_flow(warnings: list[IrulesFlowWarning]) -> list[dict]:
    return [
        {
            "code": w.code,
            "message": w.message,
            "range": range_dict(w.range),
        }
        for w in warnings
    ]


def _serialise_event_order(entries: list[EventOrderEntry]) -> list[dict]:
    return [
        {
            "event": e.event,
            "base_priority": e.base_priority,
            "priority_offset": e.priority_offset,
            "multiplicity": e.multiplicity,
            "range": range_dict(e.range),
        }
        for e in entries
    ]


def _serialise_types(snapshots: list[FunctionSnapshot]) -> list[dict]:
    out = []
    for snap in snapshots:
        entries = []
        for (name, ver), tl in sorted(snap.analysis.types.items()):
            if tl.kind is TypeKind.UNKNOWN:
                continue
            entries.append(
                {
                    "variable": name,
                    "version": ver,
                    "type": format_type(tl),
                    "kind": tl.kind.name.lower(),
                }
            )
        if entries:
            out.append({"name": snap.name, "entries": entries})
    return out


def _serialise_taint_tracking(snapshots: list[FunctionSnapshot]) -> list[dict]:
    out = []
    for snap in snapshots:
        entries = []
        for (name, ver), tl in sorted(snap.analysis.taints.items()):
            if not tl.tainted:
                continue
            entries.append(
                {
                    "variable": name,
                    "version": ver,
                    "taint": format_taint(tl),
                }
            )
        if entries:
            out.append({"name": snap.name, "entries": entries})
    return out


def _serialise_rendered_properties(snapshots: list[FunctionSnapshot]) -> list[dict]:
    out = []
    for snap in snapshots:
        entries = []
        for (name, ver), rp in sorted(snap.analysis.rendered_props.items()):
            may_flags = [
                f.name for f in type(rp.may).__members__.values() if f in rp.may and f.value
            ]
            must_flags = [
                f.name for f in type(rp.must).__members__.values() if f in rp.must and f.value
            ]
            if not may_flags and not must_flags:
                continue
            entries.append(
                {
                    "variable": name,
                    "version": ver,
                    "may": may_flags,
                    "must": must_flags,
                }
            )
        if entries:
            out.append({"name": snap.name, "entries": entries})
    return out


# Annotations (source callouts)


def _collect_annotations(result: CompilerExplorerResult) -> list[dict]:
    annotations: list[dict] = []

    def walk_barriers(script, scope):
        for stmt in script.statements:
            if isinstance(stmt, IRBarrier):
                annotations.append(
                    {
                        "range": range_dict(stmt.range),
                        "label": f"{scope}: compiler barrier ({stmt.reason})",
                        "kind": "barrier",
                    }
                )
            elif isinstance(stmt, IRIf):
                for clause in stmt.clauses:
                    walk_barriers(clause.body, scope)
                if stmt.else_body:
                    walk_barriers(stmt.else_body, scope)
            elif isinstance(stmt, IRFor):
                walk_barriers(stmt.init, scope)
                walk_barriers(stmt.body, scope)
                walk_barriers(stmt.next, scope)
            elif isinstance(stmt, IRSwitch):
                for arm in stmt.arms:
                    if arm.body:
                        walk_barriers(arm.body, scope)
                if stmt.default_body:
                    walk_barriers(stmt.default_body, scope)

    walk_barriers(result.ir_module.top_level, "::top")
    for qname, proc in result.ir_module.procedures.items():
        walk_barriers(proc.body, qname)

    for snap in result.snapshots:
        for dead in snap.analysis.dead_stores:
            block = snap.cfg.blocks.get(dead.block)
            if not block or dead.statement_index >= len(block.statements):
                continue
            stmt = block.statements[dead.statement_index]
            annotations.append(
                {
                    "range": range_dict(stmt.range),
                    "label": f"{snap.name}: dead store {dead.variable}#{dead.version}",
                    "kind": "deadStore",
                }
            )
        for branch in snap.analysis.constant_branches:
            block = snap.cfg.blocks.get(branch.block)
            if not block:
                continue
            term = block.terminator
            if not isinstance(term, CFGBranch) or term.range is None:
                continue
            direction = "true" if branch.value else "false"
            annotations.append(
                {
                    "range": range_dict(term.range),
                    "label": f"{snap.name}: branch always {direction}; takes {branch.taken_target}",
                    "kind": "constantBranch",
                }
            )
        for bn in sorted(snap.analysis.unreachable_blocks):
            block = snap.cfg.blocks.get(bn)
            if not block:
                continue
            if block.statements:
                r = block.statements[0].range
            else:
                term = block.terminator
                if isinstance(term, (CFGGoto, CFGBranch, CFGReturn)) and term.range is not None:
                    r = term.range
                else:
                    continue
            annotations.append(
                {
                    "range": range_dict(r),
                    "label": f"{snap.name}: unreachable block {bn}",
                    "kind": "unreachable",
                }
            )

    for opt in result.optimisations:
        annotations.append(
            {
                "range": range_dict(opt.range),
                "label": f"{opt.code}: {opt.message} -> {preview(opt.replacement, 40)}",
                "kind": "optimisation",
            }
        )

    for w in result.shimmer_warnings:
        annotations.append(
            {
                "range": range_dict(w.range),
                "label": f"{w.code}: {w.message}",
                "kind": "shimmer" if not isinstance(w, ThunkingWarning) else "thunking",
            }
        )

    for w in result.gvn_warnings:
        annotations.append(
            {
                "range": range_dict(w.range),
                "label": f"{w.code}: {w.message or w.expression_text}",
                "kind": "gvn",
            }
        )

    for w in result.taint_warnings:
        annotations.append(
            {
                "range": range_dict(w.range),
                "label": f"{w.code}: {w.message}",
                "kind": "taint",
            }
        )

    for w in result.irules_flow_warnings:
        annotations.append(
            {
                "range": range_dict(w.range),
                "label": f"{w.code}: {w.message}",
                "kind": "irulesFlow",
            }
        )

    annotations.sort(key=lambda a: (a["range"]["startOffset"], a["range"]["endOffset"]))
    return annotations


# Bytecode assembly


def _serialise_asm(ir_module: IRModule, *, cfg_module=None) -> list[dict]:
    """Generate a structured Tcl bytecode disassembly view for the explorer.

    Mirrors ``_serialise_wasm``: each function entry carries both a
    ``text`` WAT snippet (for copy/paste and the legacy renderer) and
    an ``instructions`` list with resolved jump targets, source
    ranges, and label anchors.  The frontend switches to the rich
    renderer when ``instructions`` is present.
    """
    from compiler.codegen.bytecode import codegen_module, format_module_explorer

    if cfg_module is None:
        from compiler.cfg import build_cfg

        cfg_module = build_cfg(ir_module)
    module_asm = codegen_module(cfg_module, ir_module)
    result = format_module_explorer(module_asm)
    # Tag each entry with ``instrCount`` at the top level (the
    # function-explorer already records it) and wire proc source
    # ranges from the IR so the UI can jump to the proc body on
    # click.  ``::top`` spans the whole script; procs use their
    # IRProcedure.range.
    top_stmts = ir_module.top_level.statements
    top_range = None
    if top_stmts:
        first = getattr(top_stmts[0], "range", None)
        last = getattr(top_stmts[-1], "range", None)
        if first is not None and last is not None:
            top_range = {
                "startLine": first.start.line,
                "startCol": first.start.character,
                "startOffset": first.start.offset,
                "endLine": last.end.line,
                "endCol": last.end.character,
                "endOffset": last.end.offset,
            }
    for entry in result:
        if entry["kind"] == "top":
            entry["sourceRange"] = top_range
        else:
            ir_proc = ir_module.procedures.get(entry["name"])
            if ir_proc is not None:
                rng = ir_proc.range
                entry["sourceRange"] = {
                    "startLine": rng.start.line,
                    "startCol": rng.start.character,
                    "startOffset": rng.start.offset,
                    "endLine": rng.end.line,
                    "endCol": rng.end.character,
                    "endOffset": rng.end.offset,
                }
            else:
                entry["sourceRange"] = None
    return result


# WebAssembly (WAT)


def _serialise_wasm(ir_module: IRModule, *, optimise: bool = False, cfg_module=None) -> list[dict]:
    """Generate a structured WASM disassembly view for the compiler explorer.

    Returns one entry per function (plus a synthetic ``(module)`` entry
    with imports, types, and data segments).  Each entry carries both
    a flat WAT ``text`` (for copy/paste and the old renderer) and a
    rich ``instructions`` list with per-instruction metadata — resolved
    call targets, paired branch targets, source ranges, indent levels,
    and explorer labels.  The frontend switches to the rich renderer
    when ``instructions`` is present; consumers that don't understand
    the new shape can continue to read ``text``.
    """
    from compiler.codegen.wasm import wasm_codegen_module

    if cfg_module is None:
        from compiler.cfg import build_cfg

        cfg_module = build_cfg(ir_module)
    wasm_module = wasm_codegen_module(cfg_module, ir_module, optimise=optimise)
    explorer_entries = wasm_module.to_explorer_json()
    # Attach ``text`` (a per-function WAT snippet) to each function so
    # the old text-only renderer still works on consumers that do not
    # understand the structured shape.  The synthetic ``(module)``
    # header carries the full-module WAT as its ``text``.
    full_wat = wasm_module.to_wat()
    result: list[dict] = []
    for entry in explorer_entries:
        if entry["kind"] == "module":
            # ``instrCount`` on the synthetic module header is left at 0
            # so the tab-badge total (computed as the sum over all
            # entries in ``data.wasm``) doesn't double-count the body
            # instructions.  The real total is available as
            # ``totalInstrCount`` for UI that wants to display it.
            result.append(
                {
                    **entry,
                    "text": full_wat,
                    "totalInstrCount": sum(len(f.body) for f in wasm_module.functions),
                }
            )
        else:
            result.append(
                {
                    **entry,
                    "text": _format_function_wat_snippet(entry),
                }
            )
    return result


def _format_function_wat_snippet(entry: dict) -> str:
    """Render a single function's instructions as a WAT snippet.

    Used as a legacy fallback for consumers that haven't migrated to
    the structured ``instructions`` list — emits ``func $name``
    followed by each instruction's ``fullText`` at its ``indent``.
    """
    sig_parts = [f"(param {p['name']} {p['type']})" for p in entry.get("params", [])]
    for r in entry.get("results", []):
        sig_parts.append(f"(result {r})")
    sig = " ".join(sig_parts)
    lines = [f"(func ${entry['name']} {sig}".rstrip()]
    for loc in entry.get("locals", []):
        lines.append(f"  (local {loc['name']} {loc['type']})")
    for instr in entry.get("instructions", []):
        prefix = "  " + ("    " * instr["indent"])
        lines.append(f"{prefix}{instr['fullText']}")
    lines.append(")")
    return "\n".join(lines)


# Top-level serialisation


def serialise_result(result: CompilerExplorerResult) -> dict:
    """Convert a CompilerExplorerResult to a JSON-serialisable dict.

    This is the single serialisation function used by both the Flask
    endpoint and the Pyodide worker.
    """
    from compiler.cfg import build_cfg

    from .pipeline import compute_stats

    # Build CFG once for the original IR and share across asm + wasm.
    cfg_module = build_cfg(result.ir_module)

    asm_optimised = None
    wasm_optimised = None
    if result.optimised_source and result.optimised_source != result.source:
        try:
            from compiler.lowering import lower_to_ir

            opt_ir = lower_to_ir(result.optimised_source)
            opt_cfg = build_cfg(opt_ir)
        except Exception:
            opt_ir = None
            opt_cfg = None
        if opt_ir is not None:
            try:
                asm_optimised = _serialise_asm(opt_ir, cfg_module=opt_cfg)
            except Exception as exc:
                print(f"warning: optimised asm serialisation failed: {exc}", file=sys.stderr)
            try:
                wasm_optimised = _serialise_wasm(opt_ir, optimise=True, cfg_module=opt_cfg)
            except Exception as exc:
                print(f"warning: optimised wasm serialisation failed: {exc}", file=sys.stderr)

    try:
        asm = _serialise_asm(result.ir_module, cfg_module=cfg_module)
    except Exception as exc:
        print(f"warning: asm serialisation failed: {exc}", file=sys.stderr)
        asm = None

    try:
        wasm = _serialise_wasm(result.ir_module, cfg_module=cfg_module)
    except Exception as exc:
        print(f"warning: wasm serialisation failed: {exc}", file=sys.stderr)
        wasm = None

    return {
        "ir": _serialise_ir(result.ir_module),
        "cfgPreSsa": _serialise_cfg_pre_ssa(result.snapshots),
        "cfgPostSsa": _serialise_cfg_post_ssa(result.snapshots),
        "interprocedural": _serialise_interproc(result.interproc),
        "optimisations": _serialise_optimisations(result.optimisations),
        "shimmer": _serialise_shimmer(result.shimmer_warnings),
        "gvn": _serialise_gvn(result.gvn_warnings),
        "taintWarnings": _serialise_taint(result.taint_warnings),
        "taintTracking": _serialise_taint_tracking(result.snapshots),
        "renderedProperties": _serialise_rendered_properties(result.snapshots),
        "irulesFlow": _serialise_irules_flow(result.irules_flow_warnings),
        "eventOrder": _serialise_event_order(result.event_order),
        "dataflow": dataflow_graph_to_dict(result.dataflow_graph)
        if result.dataflow_graph is not None
        else None,
        "types": _serialise_types(result.snapshots),
        "asm": asm,
        "asmOptimised": asm_optimised,
        "wasm": wasm,
        "wasmOptimised": wasm_optimised,
        "optimisedSource": result.optimised_source
        if result.optimised_source != result.source
        else None,
        "annotations": _collect_annotations(result),
        "stats": compute_stats(result),
    }
