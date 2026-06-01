"""JSON serialisation for compiler explorer results.

Shared by the Flask web explorer and the Pyodide WASM worker.
"""

from __future__ import annotations

import sys

from compiler.cfg import CFGBranch, CFGGoto, CFGReturn
from compiler.dataflow_graph import dataflow_graph_to_dict
from compiler.gvn import RedundantComputation
from compiler.interprocedural import InterproceduralAnalysis
from compiler.ir import IRFor, IRIf, IRModule, IRScript, IRSwitch
from compiler.irules_flow import EventOrderEntry, IrulesFlowWarning
from compiler.optimiser import Optimisation
from compiler.shimmer import ShimmerWarning, ThunkingWarning
from compiler.taint import (
    TaintWarning,
)
from compiler.types import TypeKind
from tooling.explorer.annotations import (
    Annotation,
    Severity,
    collect_annotations,
    shimmer_severity,
    taint_severity,
)
from tooling.explorer.cfg_layout import build_cfg_edges
from tooling.explorer.pipeline import (
    ALL_VIEWS,
    AVAILABLE_DIALECTS,
    CompilerExplorerResult,
    FunctionSnapshot,
)

from .formatters import (
    format_lattice,
    format_return_shape,
    format_taint,
    format_type,
    preview,
    range_dict,
    stmt_color_class,
    stmt_kind,
    stmt_summary,
    to_str,
)

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


def _serialise_cfg_edges(snap: FunctionSnapshot) -> list[dict]:
    """Control-flow edges with routing lanes, shared with the ASCII gutter.

    The ``lane`` is computed once here (``cfg_layout``) so the web SVG router
    and the CLI/TUI box-drawing gutter nest edges identically — neither side
    re-derives lane assignment.
    """
    return [
        {
            "from": e.src,
            "to": e.dst,
            "fromPos": e.src_pos,
            "toPos": e.dst_pos,
            "kind": e.kind,
            "lane": e.lane,
        }
        for e in build_cfg_edges(snap.cfg)
    ]


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
                "edges": _serialise_cfg_edges(snap),
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
                "edges": _serialise_cfg_edges(snap),
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
        out.append(
            {
                "name": qname,
                "arity": arity,
                "pure": s.pure,
                "foldable": s.can_fold_static_calls,
                "returnShape": format_return_shape(s),
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
            "severity": shimmer_severity(w.code).value,
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
            # GVN findings surface redundant computations the optimiser
            # can fold — informational, not danger.
            "severity": Severity.INFO.value,
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
            "severity": taint_severity(w.code).value,
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
            "severity": Severity.WARNING.value,
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


def _serialise_annotation(ann: Annotation) -> dict:
    """Project an :class:`Annotation` to the wire shape.

    ``kind`` identifies the source, ``severity`` is the backend's
    classification (so renderers don't re-derive it from warning codes),
    ``priority`` lets renderers stable-sort within a line.  The CLI
    callout renderer and the GUI both consume this directly.
    """
    return {
        "range": range_dict(ann.range),
        "label": ann.label,
        "kind": ann.kind,
        "severity": ann.severity.value,
        "priority": ann.priority,
    }


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
            from analyser.semantic_model import Range

            top_range = range_dict(Range(start=first.start, end=last.end))
    for entry in result:
        if entry["kind"] == "top":
            entry["sourceRange"] = top_range
        else:
            ir_proc = ir_module.procedures.get(entry["name"])
            if ir_proc is not None:
                entry["sourceRange"] = range_dict(ir_proc.range)
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

    Each function entry also gets a pre-baked ``edges`` list — branch
    instructions paired with their targets, lane-assigned via
    :func:`cfg_layout.assign_lanes`.  The web GUI renders those lanes
    directly (no client-side lane computation); the CLI ignores them.
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
                    "edges": _wasm_edges_with_lanes(entry),
                }
            )
    return result


def _wasm_edges_with_lanes(entry: dict) -> list[dict]:
    """Extract branch edges from a WASM function entry and assign lanes.

    The JS gutter renderer used to do this by walking the DOM and
    re-running ``cfg_layout.assign_lanes`` byte-for-byte in JavaScript;
    pre-baking it here keeps lane assignment in one place (the same
    helper the CFG views use).
    """
    from tooling.explorer.cfg_layout import assign_lanes

    instructions = entry.get("instructions") or []
    raw: list[tuple[int, int, str]] = []  # (fromIdx, toIdx, kind)
    for instr in instructions:
        from_idx = instr.get("idx")
        if from_idx is None:
            continue
        bt = instr.get("branchTarget")
        if bt is None:
            continue
        # ``branchTarget`` is either a scalar int (a single branch) or
        # a dict carrying ``targets`` (a jump-table dispatch) — both
        # produced by the WASM codegen.
        targets: list[int] = []
        if isinstance(bt, int):
            targets = [bt]
        elif isinstance(bt, dict):
            # jumpTable shape: {"targets": [{"idx": int, ...}, ...],
            # "fallback": {"idx": int}}.  Be permissive about field
            # names; codegen revisions may add more.
            for t in bt.get("targets") or []:
                tidx = t.get("idx") if isinstance(t, dict) else None
                if isinstance(tidx, int):
                    targets.append(tidx)
            fb = bt.get("fallback")
            if isinstance(fb, dict):
                fidx = fb.get("idx")
                if isinstance(fidx, int):
                    targets.append(fidx)
        for to_idx in targets:
            kind = "forward" if to_idx >= from_idx else "back"
            raw.append((from_idx, to_idx, kind))
    if not raw:
        return []
    lanes = assign_lanes([(f, t) for (f, t, _k) in raw])
    return [
        {
            "fromIdx": f,
            "toIdx": t,
            "kind": k,
            "lane": lane,
        }
        for (f, t, k), lane in zip(raw, lanes)
    ]


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
    from shared.ranges import reset_highlight_source, set_highlight_source
    from tooling.explorer.pipeline import compute_stats

    # Widen braced/quoted word ranges to their closing delimiter against the
    # document source while serialising (see ``range_dict``).
    src_token = set_highlight_source(result.source)
    try:
        return _serialise_result(result, build_cfg, compute_stats)
    finally:
        reset_highlight_source(src_token)


def _serialise_result(result: CompilerExplorerResult, build_cfg, compute_stats) -> dict:
    from shared.ranges import reset_highlight_source, set_highlight_source

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
            # Optimised asm/wasm ranges index into the optimised source.
            opt_token = set_highlight_source(result.optimised_source)
            try:
                try:
                    asm_optimised = _serialise_asm(opt_ir, cfg_module=opt_cfg)
                except Exception as exc:
                    print(f"warning: optimised asm serialisation failed: {exc}", file=sys.stderr)
                try:
                    wasm_optimised = _serialise_wasm(opt_ir, optimise=True, cfg_module=opt_cfg)
                except Exception as exc:
                    print(f"warning: optimised wasm serialisation failed: {exc}", file=sys.stderr)
            finally:
                reset_highlight_source(opt_token)

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

    # Annotations: single backend pass, single classification.  Renderers
    # read ``severity`` directly — no client-side warning-code mapping.
    annotations, _omitted = collect_annotations(result, views=ALL_VIEWS, max_annotations=-1)
    annotation_dicts = [_serialise_annotation(a) for a in annotations]
    annotations_by_line = _group_annotations_by_line(annotation_dicts, result.source)

    return {
        "meta": _serialise_meta(),
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
        "annotations": annotation_dicts,
        "annotationsByLine": annotations_by_line,
        "stats": compute_stats(result),
    }


# Pre-grouped annotations by source line.  The GUI's old client-side
# pass walked the source, built a line index, and bucketed annotations
# — pure data shaping.  Doing it here once gives every renderer the
# same buckets (and the GUI just iterates without parsing).
def _group_annotations_by_line(annotation_dicts: list[dict], source: str) -> dict[str, list[int]]:
    """Bucket annotation indices by 0-based source line.

    Returns ``{line_no_as_string: [annotation_index, ...]}`` so the JSON
    is a plain object (JS) / dict (Python) — neither side has to scan
    the annotation list to find which annotations decorate which line.
    """
    if not annotation_dicts:
        return {}
    # Build line starts from the source once; binary-search per annotation.
    line_starts = [0]
    for i, ch in enumerate(source):
        if ch == "\n":
            line_starts.append(i + 1)
    buckets: dict[str, list[int]] = {}
    for idx, ann in enumerate(annotation_dicts):
        offset = ann["range"]["startOffset"]
        # Binary search: largest line_start <= offset.
        lo, hi = 0, len(line_starts) - 1
        while lo < hi:
            mid = (lo + hi + 1) >> 1
            if line_starts[mid] <= offset:
                lo = mid
            else:
                hi = mid - 1
        buckets.setdefault(str(lo), []).append(idx)
    return buckets


# Frontend-driving metadata: which dialects exist, which views render,
# which severity classes mean what.  The HTML / JS bootstrap reads this
# instead of hardcoding dropdowns or tab lists, so adding a dialect or
# view is a one-line backend edit.
_VIEW_META: tuple[tuple[str, str, str], ...] = (
    # (id, label, group)  — group is informational; CLI uses VIEW_GROUPS.
    ("greentree", "Green Tree", "compiler"),
    ("ir", "IR", "compiler"),
    ("cfg", "CFG (pre-SSA)", "compiler"),
    ("ssa", "CFG (post-SSA)", "compiler"),
    ("loops", "Loops", "compiler"),
    ("types", "Types", "compiler"),
    ("intervals", "Intervals", "compiler"),
    ("bounds", "Bounds", "compiler"),
    ("dataflow", "Data Flow", "compiler"),
    ("interproc", "Interprocedural", "compiler"),
    ("rendered", "Rendered Props", "compiler"),
    ("opt", "Optimisations", "optimiser"),
    ("gvn", "GVN", "optimiser"),
    ("shimmer", "Shimmer", "optimiser"),
    ("taint", "Taint", "optimiser"),
    ("irules", "iRules Flow", "optimiser"),
    ("eventOrder", "Event Order", "optimiser"),
    ("callouts", "Source Callouts", "optimiser"),
    ("asm", "Tcl ASM", "codegen"),
    ("asmOpt", "Tcl ASM (opt)", "codegen"),
    ("wasm", "WASM", "codegen"),
    ("wasmOpt", "WASM (opt)", "codegen"),
)


def _serialise_meta() -> dict:
    return {
        "dialects": list(AVAILABLE_DIALECTS),
        "views": [
            {"id": vid, "label": label, "group": group} for (vid, label, group) in _VIEW_META
        ],
        "severities": [s.value for s in Severity],
    }
