"""Console explorer for Tcl compiler and optimiser internals.

Features:
- Accept Tcl script from file, stdin, or --source.
- Show lowered IR, CFG (pre-SSA and post-SSA), and core-analysis facts by function.
- Show interprocedural procedure summaries.
- Show optimiser rewrites and optional optimised source.
- Render source with Rust-style caret/arrow callouts for salient compiler facts.
"""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass
from pathlib import Path

try:
    from lsp._build_info import BUILD_TIMESTAMP, FULL_VERSION
except ImportError:
    FULL_VERSION = "dev"
    BUILD_TIMESTAMP = ""

from core.compiler.cfg import CFGBranch, CFGGoto, CFGReturn
from core.compiler.gvn import RedundantComputation
from core.compiler.interprocedural import InterproceduralAnalysis, ProcSummary
from core.compiler.ir import (
    IRBarrier,
    IRCall,
    IRFor,
    IRIf,
    IRIfClause,
    IRModule,
    IRReturn,
    IRScript,
    IRStatement,
    IRSwitch,
)
from core.compiler.irules_flow import IrulesFlowWarning
from core.compiler.optimiser import Optimisation
from core.compiler.shimmer import ShimmerWarning, ThunkingWarning
from core.compiler.taint import (
    CollectWithoutReleaseWarning,
    ReleaseWithoutCollectWarning,
    TaintWarning,
)
from core.compiler.types import TypeKind

from .formatters import (
    LineIndex,
    format_lattice,
    format_taint,
    format_type,
    preview,
)
from .pipeline import (
    AVAILABLE_DIALECTS,
    CompilerExplorerResult,
    FunctionSnapshot,
    expand_show,
    run_pipeline,
)

# ANSI styling


class Ansi:
    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    RED = "\033[31m"
    YELLOW = "\033[33m"
    BLUE = "\033[34m"
    MAGENTA = "\033[35m"
    CYAN = "\033[36m"
    GREEN = "\033[32m"
    GRAY = "\033[90m"


@dataclass(slots=True, frozen=True)
class SourceAnnotation:
    range: "Range"
    label: str
    colour: str
    priority: int


def style(text: str, colour: str, enabled: bool) -> str:
    if not enabled:
        return text
    return f"{colour}{text}{Ansi.RESET}"


# Need Range import for SourceAnnotation type hint
from core.analysis.semantic_model import Range  # noqa: E402

# IR statement helpers (CLI-specific: use Ansi colours)


def _stmt_colour(stmt: IRStatement) -> str:
    from core.compiler.ir import IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr

    if isinstance(stmt, IRBarrier):
        return Ansi.YELLOW
    if isinstance(stmt, IRCall):
        return Ansi.CYAN
    if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr)):
        return Ansi.GREEN
    if isinstance(stmt, IRReturn):
        return Ansi.MAGENTA
    if isinstance(stmt, (IRIf, IRFor, IRSwitch)):
        return Ansi.BLUE
    return Ansi.GRAY


def _stmt_summary(stmt: IRStatement) -> str:
    """One-line summary of an IR statement (CLI version with expr_text)."""
    from core.compiler.expr_ast import expr_text as _expr_text
    from core.compiler.ir import IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr

    if isinstance(stmt, IRAssignConst):
        return f"assign-const {stmt.name} = {stmt.value}"
    if isinstance(stmt, IRAssignExpr):
        return f"assign-expr {stmt.name} = [expr {{{preview(_expr_text(stmt.expr), 48)}}}]"
    if isinstance(stmt, IRAssignValue):
        return f"assign-value {stmt.name} = {preview(stmt.value, 48)}"
    if isinstance(stmt, IRIncr):
        if stmt.amount is None:
            return f"incr {stmt.name}"
        return f"incr {stmt.name} {preview(stmt.amount, 32)}"
    if isinstance(stmt, IRCall):
        rendered_args = " ".join(preview(a, 20) for a in stmt.args[:4])
        if len(stmt.args) > 4:
            rendered_args += " ..."
        return f"call {stmt.command}{(' ' + rendered_args) if rendered_args else ''}"
    if isinstance(stmt, IRReturn):
        return f"return {preview(stmt.value, 48) if stmt.value is not None else ''}".strip()
    if isinstance(stmt, IRBarrier):
        details = f"{stmt.reason}"
        if stmt.command:
            details += f" ({stmt.command})"
        return f"barrier {details}"
    if isinstance(stmt, IRIf):
        return f"if ({len(stmt.clauses)} clause(s){', else' if stmt.else_body is not None else ''})"
    if isinstance(stmt, IRFor):
        from core.compiler.expr_ast import expr_text as _expr_text2

        return f"for ({preview(_expr_text2(stmt.condition), 40)})"
    if isinstance(stmt, IRSwitch):
        return f"switch {preview(stmt.subject, 40)} ({len(stmt.arms)} arm(s))"
    return stmt.__class__.__name__


def _terminator_summary(term: CFGGoto | CFGBranch | CFGReturn | None) -> str:
    from core.compiler.expr_ast import expr_text as _expr_text

    if term is None:
        return "<no terminator>"
    if isinstance(term, CFGGoto):
        return f"goto {term.target}"
    if isinstance(term, CFGBranch):
        return f"branch if {preview(_expr_text(term.condition), 60)} -> {term.true_target} / {term.false_target}"
    if isinstance(term, CFGReturn):
        if term.value is None:
            return "return"
        return f"return {preview(term.value, 48)}"
    return "<unknown terminator>"


# Annotations (source callouts — CLI-specific with Ansi colours)


def _append_barrier_annotations(
    script: IRScript, scope_name: str, out: list[SourceAnnotation]
) -> None:
    for stmt in script.statements:
        if isinstance(stmt, IRBarrier):
            out.append(
                SourceAnnotation(
                    range=stmt.range,
                    label=f"{scope_name}: compiler barrier ({stmt.reason})",
                    colour=Ansi.YELLOW,
                    priority=2,
                )
            )
            continue
        if isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                _append_barrier_annotations(clause.body, scope_name, out)
            if stmt.else_body is not None:
                _append_barrier_annotations(stmt.else_body, scope_name, out)
            continue
        if isinstance(stmt, IRFor):
            _append_barrier_annotations(stmt.init, scope_name, out)
            _append_barrier_annotations(stmt.body, scope_name, out)
            _append_barrier_annotations(stmt.next, scope_name, out)
            continue
        if isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body is not None:
                    _append_barrier_annotations(arm.body, scope_name, out)
            if stmt.default_body is not None:
                _append_barrier_annotations(stmt.default_body, scope_name, out)


def _collect_annotations(
    result: CompilerExplorerResult,
    *,
    views: frozenset[str],
    max_annotations: int,
) -> tuple[list[SourceAnnotation], int]:
    annotations: list[SourceAnnotation] = []
    ir_module = result.ir_module
    snapshots = result.snapshots

    if views & {"ir", "cfg", "ssa", "interproc"}:
        _append_barrier_annotations(ir_module.top_level, "::top", annotations)
        for qname, proc in ir_module.procedures.items():
            _append_barrier_annotations(proc.body, qname, annotations)

        for snap in snapshots:
            for dead in snap.analysis.dead_stores:
                block = snap.cfg.blocks.get(dead.block)
                if block is None:
                    continue
                if dead.statement_index < 0 or dead.statement_index >= len(block.statements):
                    continue
                stmt = block.statements[dead.statement_index]
                annotations.append(
                    SourceAnnotation(
                        range=stmt.range,
                        label=f"{snap.name}: dead store {dead.variable}#{dead.version}",
                        colour=Ansi.YELLOW,
                        priority=1,
                    )
                )
            for branch in snap.analysis.constant_branches:
                block = snap.cfg.blocks.get(branch.block)
                if block is None:
                    continue
                term = block.terminator
                if not isinstance(term, CFGBranch) or term.range is None:
                    continue
                direction = "true" if branch.value else "false"
                annotations.append(
                    SourceAnnotation(
                        range=term.range,
                        label=(
                            f"{snap.name}: branch is always {direction}; "
                            f"takes {branch.taken_target}"
                        ),
                        colour=Ansi.BLUE,
                        priority=0,
                    )
                )
            for block_name in sorted(snap.analysis.unreachable_blocks):
                block = snap.cfg.blocks.get(block_name)
                if block is None:
                    continue
                if block.statements:
                    target_range = block.statements[0].range
                else:
                    term = block.terminator
                    if isinstance(term, (CFGGoto, CFGBranch, CFGReturn)) and term.range is not None:
                        target_range = term.range
                    else:
                        continue
                annotations.append(
                    SourceAnnotation(
                        range=target_range,
                        label=f"{snap.name}: unreachable block {block_name}",
                        colour=Ansi.MAGENTA,
                        priority=3,
                    )
                )

    if "opt" in views:
        for opt in result.optimisations:
            annotations.append(
                SourceAnnotation(
                    range=opt.range,
                    label=f"{opt.code}: {opt.message} -> {preview(opt.replacement, 40)}",
                    colour=Ansi.GREEN,
                    priority=-1,
                )
            )

    if "shimmer" in views and result.shimmer_warnings:
        for w in result.shimmer_warnings:
            colour = Ansi.RED if w.code == "S102" else Ansi.YELLOW
            annotations.append(
                SourceAnnotation(
                    range=w.range,
                    label=f"{w.code}: {w.message}",
                    colour=colour,
                    priority=1 if w.code == "S102" else 2,
                )
            )

    if "gvn" in views and result.gvn_warnings:
        for w in result.gvn_warnings:
            annotations.append(
                SourceAnnotation(
                    range=w.range,
                    label=f"{w.code}: {w.message or w.expression_text}",
                    colour=Ansi.GREEN,
                    priority=1,
                )
            )

    if "taint" in views and result.taint_warnings:
        for w in result.taint_warnings:
            annotations.append(
                SourceAnnotation(
                    range=w.range,
                    label=f"{w.code}: {w.message}",
                    colour=Ansi.RED,
                    priority=0,
                )
            )

    if "irules" in views and result.irules_flow_warnings:
        for w in result.irules_flow_warnings:
            annotations.append(
                SourceAnnotation(
                    range=w.range,
                    label=f"{w.code}: {w.message}",
                    colour=Ansi.YELLOW,
                    priority=1,
                )
            )

    annotations.sort(
        key=lambda a: (
            a.range.start.offset,
            a.priority,
            a.range.end.offset - a.range.start.offset,
            a.label,
        )
    )

    if max_annotations >= 0 and len(annotations) > max_annotations:
        omitted = len(annotations) - max_annotations
        return annotations[:max_annotations], omitted

    return annotations, 0


# Print functions (ANSI terminal output)


def print_source_callouts(
    source: str,
    annotations: list[SourceAnnotation],
    *,
    use_colour: bool,
    omitted_annotations: int,
) -> None:
    print()
    print(style("source-callouts", Ansi.BOLD, use_colour))

    line_index = LineIndex(source)
    line_no_width = max(2, len(str(line_index.line_count())))

    if not annotations:
        print(style("  (no salient annotations)", Ansi.DIM, use_colour))
        return

    for line_number in range(line_index.line_count()):
        text = line_index.line_text(line_number)
        gutter = str(line_number + 1).rjust(line_no_width)
        print(f"{style(gutter, Ansi.DIM, use_colour)} | {text}")

        line_start = line_index.line_start(line_number)
        line_end_exclusive = line_index.line_end_exclusive(line_number)
        line_end_inclusive = max(line_start, line_end_exclusive - 1)

        line_annotations = [
            ann
            for ann in annotations
            if not (
                ann.range.end.offset < line_start or ann.range.start.offset > line_end_inclusive
            )
        ]
        for ann in line_annotations:
            seg_start = max(ann.range.start.offset, line_start)
            seg_end = min(ann.range.end.offset, line_end_inclusive)
            start_col = max(0, seg_start - line_start)
            end_col = max(start_col, seg_end - line_start)

            marker = (" " * start_col) + "^" + ("-" * max(0, end_col - start_col))
            marker_line = f"{' ' * line_no_width} | {marker}"
            arrow_line = f"{' ' * line_no_width} | {' ' * start_col}+--> {ann.label}"

            print(style(marker_line, ann.colour, use_colour))
            print(style(arrow_line, ann.colour, use_colour))

    if omitted_annotations > 0:
        print()
        omitted_text = (
            f"... {omitted_annotations} more annotations omitted (increase --max-annotations)."
        )
        print(style(omitted_text, Ansi.DIM, use_colour))


def _print_ir_script(
    script: IRScript,
    *,
    prefix: str,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    for idx, stmt in enumerate(script.statements):
        is_last = idx == (len(script.statements) - 1)
        connector = "`-- " if is_last else "|-- "
        label = _stmt_summary(stmt)
        span = line_index.format_range(stmt.range)

        print(
            f"{prefix}{connector}"
            f"{style(label, _stmt_colour(stmt), use_colour)} "
            f"{style(f'[{span}]', Ansi.DIM, use_colour)}"
        )

        child_prefix = prefix + ("    " if is_last else "|   ")

        if isinstance(stmt, IRIf):
            _print_ir_if(stmt, prefix=child_prefix, line_index=line_index, use_colour=use_colour)
            continue

        if isinstance(stmt, IRFor):
            _print_ir_for(stmt, prefix=child_prefix, line_index=line_index, use_colour=use_colour)
            continue

        if isinstance(stmt, IRSwitch):
            _print_ir_switch(
                stmt, prefix=child_prefix, line_index=line_index, use_colour=use_colour
            )


def _print_ir_if(
    stmt: IRIf,
    *,
    prefix: str,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    from core.compiler.expr_ast import expr_text as _expr_text

    children: list[tuple[str, IRIfClause | IRScript]] = []
    for i, clause in enumerate(stmt.clauses, start=1):
        children.append((f"clause {i}: {preview(_expr_text(clause.condition), 60)}", clause))
    if stmt.else_body is not None:
        children.append(("else", stmt.else_body))

    for idx, (label, payload) in enumerate(children):
        is_last = idx == (len(children) - 1)
        connector = "`-- " if is_last else "|-- "

        if isinstance(payload, IRScript):
            span = (
                line_index.format_range(stmt.else_range)
                if stmt.else_range is not None
                else "?:?-?:?"
            )
            print(
                f"{prefix}{connector}{style(label, Ansi.BLUE, use_colour)} "
                f"{style(f'[{span}]', Ansi.DIM, use_colour)}"
            )
            child_prefix = prefix + ("    " if is_last else "|   ")
            _print_ir_script(
                payload, prefix=child_prefix, line_index=line_index, use_colour=use_colour
            )
            continue

        clause = payload
        clause_span = line_index.format_range(clause.condition_range)
        print(
            f"{prefix}{connector}{style(label, Ansi.BLUE, use_colour)} "
            f"{style(f'[{clause_span}]', Ansi.DIM, use_colour)}"
        )
        child_prefix = prefix + ("    " if is_last else "|   ")
        _print_ir_script(
            clause.body, prefix=child_prefix, line_index=line_index, use_colour=use_colour
        )


def _print_ir_switch(
    stmt: IRSwitch,
    *,
    prefix: str,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    header = f"subject: {preview(stmt.subject, 60)}"
    subject_span = line_index.format_range(stmt.subject_range)
    print(
        f"{prefix}|-- {style(header, Ansi.BLUE, use_colour)} {style(f'[{subject_span}]', Ansi.DIM, use_colour)}"
    )

    arm_count = len(stmt.arms)
    for i, arm in enumerate(stmt.arms):
        is_last = i == (arm_count - 1) and stmt.default_body is None
        connector = "`-- " if is_last else "|-- "
        kind = "arm"
        if arm.fallthrough:
            kind = "fallthrough"
        arm_span = line_index.format_range(arm.pattern_range)
        print(
            f"{prefix}{connector}{style(f'{kind}: {preview(arm.pattern, 48)}', Ansi.BLUE, use_colour)} "
            f"{style(f'[{arm_span}]', Ansi.DIM, use_colour)}"
        )

        if arm.body is not None:
            child_prefix = prefix + ("    " if is_last else "|   ")
            _print_ir_script(
                arm.body, prefix=child_prefix, line_index=line_index, use_colour=use_colour
            )

    if stmt.default_body is not None:
        default_span = (
            line_index.format_range(stmt.default_range)
            if stmt.default_range is not None
            else "?:?-?:?"
        )
        print(
            f"{prefix}`-- {style('default', Ansi.BLUE, use_colour)} {style(f'[{default_span}]', Ansi.DIM, use_colour)}"
        )
        _print_ir_script(
            stmt.default_body,
            prefix=prefix + "    ",
            line_index=line_index,
            use_colour=use_colour,
        )


def _print_ir_for(
    stmt: IRFor,
    *,
    prefix: str,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    from core.compiler.expr_ast import expr_text as _expr_text

    init_span = line_index.format_range(stmt.init_range)
    cond_span = line_index.format_range(stmt.condition_range)
    next_span = line_index.format_range(stmt.next_range)
    body_span = line_index.format_range(stmt.body_range)

    print(
        f"{prefix}|-- {style('init', Ansi.BLUE, use_colour)} {style(f'[{init_span}]', Ansi.DIM, use_colour)}"
    )
    _print_ir_script(
        stmt.init, prefix=prefix + "|   ", line_index=line_index, use_colour=use_colour
    )

    print(
        f"{prefix}|-- {style(f'condition: {preview(_expr_text(stmt.condition), 60)}', Ansi.BLUE, use_colour)} {style(f'[{cond_span}]', Ansi.DIM, use_colour)}"
    )

    print(
        f"{prefix}|-- {style('next', Ansi.BLUE, use_colour)} {style(f'[{next_span}]', Ansi.DIM, use_colour)}"
    )
    _print_ir_script(
        stmt.next, prefix=prefix + "|   ", line_index=line_index, use_colour=use_colour
    )

    print(
        f"{prefix}`-- {style('body', Ansi.BLUE, use_colour)} {style(f'[{body_span}]', Ansi.DIM, use_colour)}"
    )
    _print_ir_script(
        stmt.body, prefix=prefix + "    ", line_index=line_index, use_colour=use_colour
    )


def print_ir_module(ir_module: IRModule, *, line_index: LineIndex, use_colour: bool) -> None:
    print(style("compiler-ir", Ansi.BOLD, use_colour))
    print(style("top-level", Ansi.CYAN, use_colour))
    _print_ir_script(ir_module.top_level, prefix="  ", line_index=line_index, use_colour=use_colour)

    if not ir_module.procedures:
        print(style("procedures: (none)", Ansi.DIM, use_colour))
        return

    print(style("procedures", Ansi.CYAN, use_colour))
    for qname in sorted(ir_module.procedures):
        proc = ir_module.procedures[qname]
        params = " ".join(proc.params)
        span = line_index.format_range(proc.range)
        header = f"{qname} {{{params}}}" if params else f"{qname} {{}}"
        print(
            f"  {style(header, Ansi.CYAN, use_colour)} {style(f'[{span}]', Ansi.DIM, use_colour)}"
        )
        _print_ir_script(proc.body, prefix="    ", line_index=line_index, use_colour=use_colour)


def _format_uses(uses: dict[str, int]) -> str:
    if not uses:
        return "{}"
    items = ", ".join(f"{name}#{ver}" for name, ver in sorted(uses.items()))
    return "{" + items + "}"


def _format_defs(
    defs: dict[str, int],
    values,
    types=None,
) -> str:
    if not defs:
        return "{}"
    items: list[str] = []
    for name, ver in sorted(defs.items()):
        lattice = values.get((name, ver))
        type_info = types.get((name, ver)) if types else None
        parts = f"{name}#{ver}"
        if lattice is not None:
            parts += f"={format_lattice(lattice)}"
        if type_info is not None and type_info.kind is not TypeKind.UNKNOWN:
            parts += f":{format_type(type_info)}"
        items.append(parts)
    return "{" + ", ".join(items) + "}"


def _print_function_analysis_summary(snap: FunctionSnapshot, *, use_colour: bool) -> None:
    print(style("  analysis", Ansi.BOLD, use_colour))

    if snap.analysis.constant_branches:
        print(style("    constant-branches", Ansi.BLUE, use_colour))
        for branch in snap.analysis.constant_branches:
            direction = "true" if branch.value else "false"
            line = (
                f"      {branch.block}: {preview(branch.condition, 48)} is always {direction} "
                f"(take {branch.taken_target}, skip {branch.not_taken_target})"
            )
            print(style(line, Ansi.BLUE, use_colour))
    else:
        print(style("    constant-branches: none", Ansi.DIM, use_colour))

    if snap.analysis.dead_stores:
        print(style("    dead-stores", Ansi.YELLOW, use_colour))
        for dead in snap.analysis.dead_stores:
            print(
                style(
                    f"      {dead.block} stmt#{dead.statement_index}: {dead.variable}#{dead.version}",
                    Ansi.YELLOW,
                    use_colour,
                )
            )
    else:
        print(style("    dead-stores: none", Ansi.DIM, use_colour))

    if snap.analysis.unreachable_blocks:
        unreachable = ", ".join(sorted(snap.analysis.unreachable_blocks))
        print(style(f"    unreachable: {unreachable}", Ansi.MAGENTA, use_colour))
    else:
        print(style("    unreachable: none", Ansi.DIM, use_colour))

    interesting_types = {
        k: v
        for k, v in snap.analysis.types.items()
        if v.kind in (TypeKind.KNOWN, TypeKind.SHIMMERED)
    }
    if interesting_types:
        print(style("    inferred-types", Ansi.GREEN, use_colour))
        for (name, ver), tl in sorted(interesting_types.items()):
            print(style(f"      {name}#{ver}: {format_type(tl)}", Ansi.GREEN, use_colour))
    else:
        print(style("    inferred-types: none", Ansi.DIM, use_colour))


def print_cfg_pre_ssa(
    snapshots: list[FunctionSnapshot],
    *,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    print()
    print(style("cfg-pre-ssa", Ansi.BOLD, use_colour))

    for snap in snapshots:
        print(style(f"function {snap.name}", Ansi.CYAN, use_colour))
        print(
            style(f"  entry={snap.cfg.entry} blocks={len(snap.cfg.blocks)}", Ansi.DIM, use_colour)
        )

        for block_name, block in snap.cfg.blocks.items():
            entry_suffix = " [entry]" if block_name == snap.cfg.entry else ""
            print(style(f"  block {block_name}{entry_suffix}", Ansi.BOLD, use_colour))

            for idx, stmt in enumerate(block.statements):
                span = line_index.format_range(stmt.range)
                summary = _stmt_summary(stmt)
                print(
                    f"    [{idx}] {style(summary, _stmt_colour(stmt), use_colour)} "
                    f"{style(f'[{span}]', Ansi.DIM, use_colour)}"
                )

            term = block.terminator
            term_summary = _terminator_summary(term)
            term_span = ""
            if isinstance(term, (CFGGoto, CFGBranch, CFGReturn)) and term.range is not None:
                term_span = f" [{line_index.format_range(term.range)}]"
            print(style(f"    term {term_summary}{term_span}", Ansi.BLUE, use_colour))

        print()


def print_cfg_post_ssa(
    snapshots: list[FunctionSnapshot],
    *,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    print()
    print(style("cfg-post-ssa", Ansi.BOLD, use_colour))

    for snap in snapshots:
        print(style(f"function {snap.name}", Ansi.CYAN, use_colour))
        print(
            style(f"  entry={snap.cfg.entry} blocks={len(snap.cfg.blocks)}", Ansi.DIM, use_colour)
        )

        for block_name, block in snap.cfg.blocks.items():
            ssa_block = snap.ssa.blocks.get(block_name)
            entry_suffix = " [entry]" if block_name == snap.cfg.entry else ""
            dead_suffix = " [unreachable]" if block_name in snap.analysis.unreachable_blocks else ""
            print(style(f"  block {block_name}{entry_suffix}{dead_suffix}", Ansi.BOLD, use_colour))

            if ssa_block is not None and ssa_block.phis:
                for phi in ssa_block.phis:
                    incoming = ", ".join(
                        f"{pred}:{phi.name}#{ver}" for pred, ver in sorted(phi.incoming.items())
                    )
                    phi_type = snap.analysis.types.get((phi.name, phi.version))
                    type_suffix = ""
                    if phi_type is not None and phi_type.kind is not TypeKind.UNKNOWN:
                        type_suffix = f" : {format_type(phi_type)}"
                    print(
                        style(
                            f"    phi {phi.name}#{phi.version} <- {incoming}{type_suffix}",
                            Ansi.MAGENTA,
                            use_colour,
                        )
                    )

            for idx, stmt in enumerate(block.statements):
                span = line_index.format_range(stmt.range)
                summary = _stmt_summary(stmt)
                uses = "{}"
                defs = "{}"
                if ssa_block is not None and idx < len(ssa_block.statements):
                    ssa_stmt = ssa_block.statements[idx]
                    uses = _format_uses(ssa_stmt.uses)
                    defs = _format_defs(ssa_stmt.defs, snap.analysis.values, snap.analysis.types)
                print(
                    f"    [{idx}] {style(summary, _stmt_colour(stmt), use_colour)} "
                    f"{style(f'[{span}]', Ansi.DIM, use_colour)}"
                )
                print(style(f"         uses={uses} defs={defs}", Ansi.DIM, use_colour))

            term = block.terminator
            term_summary = _terminator_summary(term)
            term_span = ""
            if isinstance(term, (CFGGoto, CFGBranch, CFGReturn)) and term.range is not None:
                term_span = f" [{line_index.format_range(term.range)}]"
            print(style(f"    term {term_summary}{term_span}", Ansi.BLUE, use_colour))

        _print_function_analysis_summary(snap, use_colour=use_colour)
        print()


def _summary_return_shape(summary: ProcSummary) -> str:
    if summary.returns_constant:
        return f"const({summary.constant_return!r})"
    if summary.return_passthrough_param is not None:
        return f"passthrough({summary.return_passthrough_param})"
    if summary.return_depends_on_params:
        deps = ",".join(summary.return_depends_on_params)
        return f"depends({deps})"
    return "unknown"


def print_interprocedural(
    interproc: InterproceduralAnalysis,
    *,
    use_colour: bool,
) -> None:
    print(style("interprocedural", Ansi.BOLD, use_colour))

    if not interproc.procedures:
        print(style("  (no procedures)", Ansi.DIM, use_colour))
        return

    for qname in sorted(interproc.procedures):
        summary = interproc.procedures[qname]
        arity = (
            f"{summary.arity.min}+"
            if summary.arity.is_unlimited
            else f"{summary.arity.min}..{summary.arity.max}"
        )
        calls = ", ".join(summary.calls) if summary.calls else "-"
        status_colour = Ansi.GREEN if summary.pure else Ansi.YELLOW
        fold = "yes" if summary.can_fold_static_calls else "no"
        line = (
            f"  {qname} arity={arity} pure={summary.pure} foldable={fold} "
            f"return={_summary_return_shape(summary)}"
        )
        print(style(line, status_colour, use_colour))
        print(style(f"    calls: {calls}", Ansi.DIM, use_colour))
        print(
            style(
                "    flags: "
                f"barrier={summary.has_barrier} "
                f"unknown_calls={summary.has_unknown_calls} "
                f"writes_global={summary.writes_global}",
                Ansi.DIM,
                use_colour,
            )
        )


def print_optimiser(
    optimisations: list[Optimisation],
    *,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    print(style("optimiser", Ansi.BOLD, use_colour))

    if not optimisations:
        print(style("  no rewrites", Ansi.DIM, use_colour))
        return

    for opt in optimisations:
        span = line_index.format_range(opt.range)
        msg = f"  {opt.code}: {opt.message}"
        repl = f"-> {preview(opt.replacement, 80)}"
        print(
            style(
                f"{msg} {style(f'[{span}]', Ansi.DIM, use_colour)} {repl}", Ansi.GREEN, use_colour
            )
        )


def print_shimmer_warnings(
    warnings: list[ShimmerWarning | ThunkingWarning],
    *,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    print(style("shimmer-detection", Ansi.BOLD, use_colour))

    if not warnings:
        print(style("  no shimmer warnings", Ansi.DIM, use_colour))
        return

    for w in warnings:
        span = line_index.format_range(w.range)
        colour = Ansi.RED if w.code == "S102" else Ansi.YELLOW
        print(
            style(
                f"  {w.code}: {w.message} {style(f'[{span}]', Ansi.DIM, use_colour)}",
                colour,
                use_colour,
            )
        )


def print_types(
    snapshots: list[FunctionSnapshot],
    *,
    use_colour: bool,
) -> None:
    print(style("type-inference", Ansi.BOLD, use_colour))

    any_types = False
    for snap in snapshots:
        interesting = {
            k: v
            for k, v in snap.analysis.types.items()
            if v.kind in (TypeKind.KNOWN, TypeKind.SHIMMERED, TypeKind.OVERDEFINED)
        }
        if not interesting:
            continue
        any_types = True
        print(style(f"  function {snap.name}", Ansi.CYAN, use_colour))
        for (name, ver), tl in sorted(interesting.items()):
            colour = Ansi.YELLOW if tl.kind is TypeKind.SHIMMERED else Ansi.GREEN
            print(style(f"    {name}#{ver}: {format_type(tl)}", colour, use_colour))

    if not any_types:
        print(style("  no type information inferred", Ansi.DIM, use_colour))


def print_gvn_warnings(
    warnings: list[RedundantComputation],
    *,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    print(style("gvn-cse", Ansi.BOLD, use_colour))

    if not warnings:
        print(style("  no redundant computations detected", Ansi.DIM, use_colour))
        return

    for w in warnings:
        span = line_index.format_range(w.range)
        first_span = line_index.format_range(w.first_range)
        msg = w.message or "redundant computation"
        print(
            style(
                f"  {w.code}: {msg} {style(f'[{span}]', Ansi.DIM, use_colour)}",
                Ansi.GREEN,
                use_colour,
            )
        )
        print(style(f"    expr: {preview(w.expression_text, 80)}", Ansi.CYAN, use_colour))
        print(style(f"    first seen: [{first_span}]", Ansi.DIM, use_colour))


def print_taint(
    warnings: list[TaintWarning | CollectWithoutReleaseWarning | ReleaseWithoutCollectWarning],
    snapshots: list[FunctionSnapshot],
    *,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    print(style("taint-analysis", Ansi.BOLD, use_colour))

    if not warnings:
        print(style("  no taint warnings", Ansi.DIM, use_colour))
    else:
        print(style("  warnings", Ansi.BOLD, use_colour))
        for w in warnings:
            span = line_index.format_range(w.range)
            colour = Ansi.RED
            details = ""
            if isinstance(w, TaintWarning):
                details = f" ({w.variable} -> {w.sink_command})"
            elif isinstance(w, (CollectWithoutReleaseWarning, ReleaseWithoutCollectWarning)):
                details = f" ({w.command})"
                colour = Ansi.YELLOW
            print(
                style(
                    f"    {w.code}: {w.message}{details} "
                    f"{style(f'[{span}]', Ansi.DIM, use_colour)}",
                    colour,
                    use_colour,
                )
            )

    any_tainted = False
    for snap in snapshots:
        tainted = {k: v for k, v in snap.analysis.taints.items() if v.tainted}
        if not tainted:
            continue
        if not any_tainted:
            print(style("  taint-tracking", Ansi.BOLD, use_colour))
            any_tainted = True
        print(style(f"    function {snap.name}", Ansi.CYAN, use_colour))
        for (name, ver), tl in sorted(tainted.items()):
            print(style(f"      {name}#{ver}: {format_taint(tl)}", Ansi.YELLOW, use_colour))

    if not any_tainted and not warnings:
        return
    if not any_tainted:
        print(style("  taint-tracking: no tainted values", Ansi.DIM, use_colour))


def print_irules_flow(
    warnings: list[IrulesFlowWarning],
    *,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    print(style("irules-flow", Ansi.BOLD, use_colour))

    if not warnings:
        print(style("  no iRules flow warnings", Ansi.DIM, use_colour))
        return

    for w in warnings:
        span = line_index.format_range(w.range)
        print(
            style(
                f"  {w.code}: {w.message} {style(f'[{span}]', Ansi.DIM, use_colour)}",
                Ansi.YELLOW,
                use_colour,
            )
        )


def _print_coloured_listing(
    label: str,
    text: str,
    rules: list[tuple[str, str]],
    *,
    use_colour: bool,
) -> None:
    """Print *text* with prefix-based syntax colouring.

    *rules* is an ordered list of ``(prefix, ansi_code)`` pairs.  For each
    line the first matching prefix determines the colour; unmatched lines
    are printed plain.
    """
    print(style(label, Ansi.BOLD, use_colour))
    for line in text.splitlines():
        stripped = line.lstrip()
        for prefix, colour in rules:
            if stripped.startswith(prefix):
                print(style(line, colour, use_colour))
                break
        else:
            print(line)


_ASM_RULES: list[tuple[str, str]] = [
    ("(", Ansi.CYAN),
    ("#", Ansi.DIM),
    ("ByteCode", Ansi.BOLD),
    ("%v", Ansi.GREEN),
]

_WASM_RULES: list[tuple[str, str]] = [
    ("(module", Ansi.CYAN),
    ("(func", Ansi.CYAN),
    (";;", Ansi.DIM),
    ("(local", Ansi.GREEN),
    ("(param", Ansi.GREEN),
]


def print_asm(ir_module: IRModule, *, cfg_module=None, use_colour: bool) -> None:
    """Render bytecode assembly for all functions in the module."""
    from core.compiler.codegen import codegen_module, format_module_asm

    if cfg_module is None:
        from core.compiler.cfg import build_cfg

        cfg_module = build_cfg(ir_module)
    module_asm = codegen_module(cfg_module, ir_module)
    text = format_module_asm(module_asm)
    _print_coloured_listing("bytecode-asm", text, _ASM_RULES, use_colour=use_colour)


def print_wasm(
    ir_module: IRModule, *, optimise: bool = False, cfg_module=None, use_colour: bool
) -> None:
    """Render WebAssembly text (WAT) for all functions in the module."""
    from core.compiler.codegen.wasm import wasm_codegen_module

    if cfg_module is None:
        from core.compiler.cfg import build_cfg

        cfg_module = build_cfg(ir_module)
    wasm_module = wasm_codegen_module(cfg_module, ir_module, optimise=optimise)
    wat = wasm_module.to_wat()
    label = "wasm-wat-optimised" if optimise else "wasm-wat"
    _print_coloured_listing(label, wat, _WASM_RULES, use_colour=use_colour)


def print_source_listing(title: str, source: str, *, use_colour: bool) -> None:
    print()
    print(style(title, Ansi.BOLD, use_colour))
    line_index = LineIndex(source)
    width = max(2, len(str(line_index.line_count())))
    for ln in range(line_index.line_count()):
        gutter = str(ln + 1).rjust(width)
        print(f"{style(gutter, Ansi.DIM, use_colour)} | {line_index.line_text(ln)}")


# CLI entry points


def load_source(path: str | None, source_arg: str | None) -> str:
    if source_arg is not None:
        return source_arg
    if path and path != "-":
        return Path(path).read_text(encoding="utf-8")
    if path == "-" or not sys.stdin.isatty():
        return sys.stdin.read()
    raise ValueError("No Tcl input provided. Use a file path, --source, or pipe stdin.")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Explore Tcl compiler and optimiser internals.\n\n"
            "Views: ir, cfg, ssa, interproc, types, opt, gvn, shimmer, taint, irules, callouts\n"
            "Groups: all (default), compiler, optimiser"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "path",
        nargs="?",
        help="Path to Tcl file to inspect, or '-' to read from stdin.",
    )
    parser.add_argument("--source", help="Inline Tcl source to inspect.")
    parser.add_argument(
        "--show",
        default="all",
        help=(
            "Comma-separated list of views to display. "
            "Individual: ir, cfg, ssa, interproc, types, opt, gvn, shimmer, taint, irules, callouts. "
            "Groups: all (default), compiler, optimiser. "
            "Example: --show ir,types,opt"
        ),
    )
    parser.add_argument(
        "--focus",
        choices=("all", "compiler", "optimiser"),
        default=None,
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--dialect",
        choices=AVAILABLE_DIALECTS,
        default="tcl8.6",
        help="Tcl dialect profile to use for compilation (default: tcl8.6).",
    )
    parser.add_argument(
        "--show-optimised-source",
        action="store_true",
        help="Print line-numbered optimised source when rewrites are found.",
    )
    parser.add_argument(
        "--no-source-callouts",
        action="store_true",
        help="Disable source callouts with caret/arrow annotations.",
    )
    parser.add_argument(
        "--max-annotations",
        type=int,
        default=80,
        help="Maximum number of source callouts to render (-1 for unlimited).",
    )
    parser.add_argument(
        "--no-colour",
        "--no-color",
        dest="no_colour",
        action="store_true",
        help="Disable ANSI colours.",
    )
    args = parser.parse_args(argv)

    show_raw = args.focus if args.focus is not None else args.show
    try:
        args.views = expand_show(show_raw)
    except argparse.ArgumentTypeError as exc:
        parser.error(str(exc))
    return args


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    use_colour = (not args.no_colour) and sys.stdout.isatty()

    try:
        source = load_source(args.path, args.source)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    views = args.views

    try:
        result = run_pipeline(source, dialect=args.dialect)
    except Exception as exc:
        print(f"error: compiler exploration failed: {exc}", file=sys.stderr)
        return 2

    line_index = LineIndex(source)
    total_dead_stores = sum(len(s.analysis.dead_stores) for s in result.snapshots)
    total_unreachable = sum(len(s.analysis.unreachable_blocks) for s in result.snapshots)

    # Summary header
    _version = FULL_VERSION
    if BUILD_TIMESTAMP:
        _version += f" ({BUILD_TIMESTAMP})"
    print(style(f"compiler-optimiser-explorer {_version}", Ansi.BOLD, use_colour))
    parts = [
        f"dialect={args.dialect}",
        f"procedures={len(result.ir_module.procedures)}",
        f"functions={len(result.snapshots)}",
        f"blocks={result.total_blocks}",
        f"dead_stores={total_dead_stores}",
        f"unreachable_blocks={total_unreachable}",
        f"rewrites={len(result.optimisations)}",
        f"shimmer={len(result.shimmer_warnings)}",
    ]
    if result.gvn_warnings:
        parts.append(f"gvn={len(result.gvn_warnings)}")
    if result.taint_warnings:
        parts.append(f"taint={len(result.taint_warnings)}")
    if result.irules_flow_warnings:
        parts.append(f"irules_flow={len(result.irules_flow_warnings)}")
    print(style(" ".join(parts), Ansi.DIM, use_colour))
    print(style(f"views: {','.join(sorted(views))}", Ansi.DIM, use_colour))
    print()

    # Selected views
    if "ir" in views:
        print_ir_module(result.ir_module, line_index=line_index, use_colour=use_colour)

    if "cfg" in views:
        print_cfg_pre_ssa(result.snapshots, line_index=line_index, use_colour=use_colour)

    if "ssa" in views:
        print_cfg_post_ssa(result.snapshots, line_index=line_index, use_colour=use_colour)

    if "interproc" in views:
        print_interprocedural(result.interproc, use_colour=use_colour)
        print()

    if "types" in views:
        print_types(result.snapshots, use_colour=use_colour)
        print()

    if "opt" in views:
        print_optimiser(result.optimisations, line_index=line_index, use_colour=use_colour)
        if args.show_optimised_source and result.optimised_source != source:
            print_source_listing("optimised-source", result.optimised_source, use_colour=use_colour)
        print()

    if "gvn" in views:
        print_gvn_warnings(result.gvn_warnings, line_index=line_index, use_colour=use_colour)
        print()

    if "shimmer" in views:
        print_shimmer_warnings(
            result.shimmer_warnings, line_index=line_index, use_colour=use_colour
        )
        print()

    if "taint" in views:
        print_taint(
            result.taint_warnings,
            result.snapshots,
            line_index=line_index,
            use_colour=use_colour,
        )
        print()

    if "irules" in views:
        print_irules_flow(result.irules_flow_warnings, line_index=line_index, use_colour=use_colour)
        print()

    if views & {"asm", "wasm"}:
        from core.compiler.cfg import build_cfg

        cfg = build_cfg(result.ir_module)
        if "asm" in views:
            print_asm(result.ir_module, cfg_module=cfg, use_colour=use_colour)
            print()
        if "wasm" in views:
            try:
                print_wasm(result.ir_module, cfg_module=cfg, use_colour=use_colour)
                print()
            except Exception as exc:
                print(f"warning: wasm generation failed: {exc}", file=sys.stderr)

    if ("asm-opt" in views or "wasm-opt" in views) and result.optimised_source != result.source:
        from core.compiler.cfg import build_cfg
        from core.compiler.lowering import lower_to_ir

        try:
            opt_ir = lower_to_ir(result.optimised_source)
            opt_cfg = build_cfg(opt_ir)
        except Exception as exc:
            print(f"warning: optimised IR lowering failed: {exc}", file=sys.stderr)
            opt_ir = None
            opt_cfg = None
        if opt_ir is not None:
            if "asm-opt" in views:
                try:
                    print_asm(opt_ir, cfg_module=opt_cfg, use_colour=use_colour)
                    print()
                except Exception as exc:
                    print(f"warning: optimised asm generation failed: {exc}", file=sys.stderr)
            if "wasm-opt" in views:
                try:
                    print_wasm(opt_ir, optimise=True, cfg_module=opt_cfg, use_colour=use_colour)
                    print()
                except Exception as exc:
                    print(f"warning: optimised wasm generation failed: {exc}", file=sys.stderr)

    if "callouts" in views and not args.no_source_callouts:
        annotations, omitted = _collect_annotations(
            result,
            views=views,
            max_annotations=args.max_annotations,
        )
        print_source_callouts(
            source,
            annotations,
            use_colour=use_colour,
            omitted_annotations=omitted,
        )

    return 0
