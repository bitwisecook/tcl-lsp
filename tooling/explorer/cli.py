"""Console explorer for Tcl compiler and optimiser internals.

Features:
- Accept Tcl script from file, stdin, or --source.
- Show lowered IR, CFG (pre-SSA and post-SSA), and core-analysis facts by function.
- Show TclOO method bodies (IR view) and interprocedural procedure + method summaries.
- Show optimiser rewrites and optional optimised source.
- Render source with Rust-style caret/arrow callouts for salient compiler facts.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

try:
    from shared._build_info import BUILD_TIMESTAMP, FULL_VERSION
except ImportError:
    FULL_VERSION = "dev"
    BUILD_TIMESTAMP = ""

from compiler.cfg import CFGBranch, CFGGoto, CFGReturn
from compiler.gvn import RedundantComputation
from compiler.interprocedural import InterproceduralAnalysis
from compiler.ir import (
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
from compiler.irules_flow import IrulesFlowWarning
from compiler.optimiser import Optimisation
from compiler.shimmer import ShimmerWarning, ThunkingWarning
from compiler.taint import (
    TaintWarning,
)
from compiler.types import TypeKind
from tooling.cli.formatters import (
    LineIndex,
    format_lattice,
    format_return_shape,
    format_taint,
    format_type,
    preview,
    stmt_summary,
)
from tooling.explorer.annotations import Annotation, Severity, collect_annotations
from tooling.explorer.cfg_layout import build_cfg_edges
from tooling.explorer.pipeline import (
    ALL_VIEWS,
    AVAILABLE_DIALECTS,
    VIEW_GROUPS,
    CompilerExplorerResult,
    FunctionSnapshot,
    run_pipeline,
)


def expand_show(raw: str) -> frozenset[str]:
    """Expand a comma-separated ``--show`` value into a set of view names.

    Lives in the CLI adapter (not the pipeline library) because it raises
    :class:`argparse.ArgumentTypeError` for an unknown view — argparse is
    an adapter concern, so the pipeline module stays argparse-free.
    """
    views: set[str] = set()
    for token in raw.split(","):
        token = token.strip()
        if not token:
            continue
        if token in VIEW_GROUPS:
            views |= VIEW_GROUPS[token]
        elif token in ALL_VIEWS:
            views.add(token)
        else:
            raise argparse.ArgumentTypeError(
                f"unknown view {token!r}; choose from: "
                f"{', '.join(sorted(ALL_VIEWS | set(VIEW_GROUPS)))}"
            )
    return frozenset(views)


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


def style(text: str, colour: str, enabled: bool) -> str:
    if not enabled:
        return text
    return f"{colour}{text}{Ansi.RESET}"


# Severity → ANSI colour table.  The backend classifies every annotation
# (``tooling/explorer/annotations.py``); the CLI only maps its severity
# to a terminal colour, identical to what the GUI does in CSS.  Kind is
# still used as a tie-breaker so optimiser callouts stay green and
# unreachable blocks stay magenta even though both classify as warning.
_SEVERITY_COLOUR: dict[Severity, str] = {
    Severity.ERROR: Ansi.RED,
    Severity.WARNING: Ansi.YELLOW,
    Severity.INFO: Ansi.BLUE,
}

_KIND_COLOUR_OVERRIDE: dict[str, str] = {
    "optimisation": Ansi.GREEN,
    "gvn": Ansi.GREEN,
    "unreachable": Ansi.MAGENTA,
}


def _annotation_colour(ann: Annotation) -> str:
    return _KIND_COLOUR_OVERRIDE.get(ann.kind) or _SEVERITY_COLOUR[ann.severity]


# IR statement helpers (CLI-specific: use Ansi colours)


def _stmt_colour(stmt: IRStatement) -> str:
    from compiler.ir import IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr

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


def _terminator_summary(term: CFGGoto | CFGBranch | CFGReturn | None) -> str:
    from compiler.expr_ast import expr_text as _expr_text

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


# Print functions (ANSI terminal output)


# Unicode box-drawing connectors for the IR / callout trees.  The terminal
# surfaces (flat ``--text`` and the Textual TUI) and the rest of the repo's
# tree renderers (``tooling/tcl/verbs/pkg.py``, ``scripts/dev/tcl_test_client``)
# all draw with these glyphs, and the CFG gutter (``_GUTTER_GLYPH``) uses the
# same line set — so we draw the trees with them too instead of the old 7-bit
# ASCII ``|-- `` / ``` `-- ``` connectors.  Textual renders a capable terminal,
# so there is no reason to fall back to ASCII here.
_TREE_BRANCH = "├── "  # a child that has siblings after it
_TREE_LAST = "└── "  # the last child
_TREE_VBAR = "│   "  # continuation column under a branch
_TREE_GAP = "    "  # continuation column under the last child


def print_source_callouts(
    source: str,
    annotations: list[Annotation],
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
        print(f"{style(gutter, Ansi.DIM, use_colour)} │ {text}")

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

            marker = (" " * start_col) + "^" + ("─" * max(0, end_col - start_col))
            marker_line = f"{' ' * line_no_width} │ {marker}"
            arrow_line = f"{' ' * line_no_width} │ {' ' * start_col}╰─▶ {ann.label}"

            colour = _annotation_colour(ann)
            print(style(marker_line, colour, use_colour))
            print(style(arrow_line, colour, use_colour))

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
        connector = _TREE_LAST if is_last else _TREE_BRANCH
        label = stmt_summary(stmt)
        span = line_index.format_range(stmt.range)

        print(
            f"{prefix}{connector}"
            f"{style(label, _stmt_colour(stmt), use_colour)} "
            f"{style(f'[{span}]', Ansi.DIM, use_colour)}"
        )

        child_prefix = prefix + (_TREE_GAP if is_last else _TREE_VBAR)

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
    from compiler.expr_ast import expr_text as _expr_text

    children: list[tuple[str, IRIfClause | IRScript]] = []
    for i, clause in enumerate(stmt.clauses, start=1):
        children.append((f"clause {i}: {preview(_expr_text(clause.condition), 60)}", clause))
    if stmt.else_body is not None:
        children.append(("else", stmt.else_body))

    for idx, (label, payload) in enumerate(children):
        is_last = idx == (len(children) - 1)
        connector = _TREE_LAST if is_last else _TREE_BRANCH

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
            child_prefix = prefix + (_TREE_GAP if is_last else _TREE_VBAR)
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
        child_prefix = prefix + (_TREE_GAP if is_last else _TREE_VBAR)
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
        f"{prefix}{_TREE_BRANCH}{style(header, Ansi.BLUE, use_colour)} {style(f'[{subject_span}]', Ansi.DIM, use_colour)}"
    )

    arm_count = len(stmt.arms)
    for i, arm in enumerate(stmt.arms):
        is_last = i == (arm_count - 1) and stmt.default_body is None
        connector = _TREE_LAST if is_last else _TREE_BRANCH
        kind = "arm"
        if arm.fallthrough:
            kind = "fallthrough"
        arm_span = line_index.format_range(arm.pattern_range)
        print(
            f"{prefix}{connector}{style(f'{kind}: {preview(arm.pattern, 48)}', Ansi.BLUE, use_colour)} "
            f"{style(f'[{arm_span}]', Ansi.DIM, use_colour)}"
        )

        if arm.body is not None:
            child_prefix = prefix + (_TREE_GAP if is_last else _TREE_VBAR)
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
            f"{prefix}{_TREE_LAST}{style('default', Ansi.BLUE, use_colour)} {style(f'[{default_span}]', Ansi.DIM, use_colour)}"
        )
        _print_ir_script(
            stmt.default_body,
            prefix=prefix + _TREE_GAP,
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
    from compiler.expr_ast import expr_text as _expr_text

    init_span = line_index.format_range(stmt.init_range)
    cond_span = line_index.format_range(stmt.condition_range)
    next_span = line_index.format_range(stmt.next_range)
    body_span = line_index.format_range(stmt.body_range)

    print(
        f"{prefix}{_TREE_BRANCH}{style('init', Ansi.BLUE, use_colour)} {style(f'[{init_span}]', Ansi.DIM, use_colour)}"
    )
    _print_ir_script(
        stmt.init, prefix=prefix + _TREE_VBAR, line_index=line_index, use_colour=use_colour
    )

    print(
        f"{prefix}{_TREE_BRANCH}{style(f'condition: {preview(_expr_text(stmt.condition), 60)}', Ansi.BLUE, use_colour)} {style(f'[{cond_span}]', Ansi.DIM, use_colour)}"
    )

    print(
        f"{prefix}{_TREE_BRANCH}{style('next', Ansi.BLUE, use_colour)} {style(f'[{next_span}]', Ansi.DIM, use_colour)}"
    )
    _print_ir_script(
        stmt.next, prefix=prefix + _TREE_VBAR, line_index=line_index, use_colour=use_colour
    )

    print(
        f"{prefix}{_TREE_LAST}{style('body', Ansi.BLUE, use_colour)} {style(f'[{body_span}]', Ansi.DIM, use_colour)}"
    )
    _print_ir_script(
        stmt.body, prefix=prefix + _TREE_GAP, line_index=line_index, use_colour=use_colour
    )


def print_ir_module(ir_module: IRModule, *, line_index: LineIndex, use_colour: bool) -> None:
    print(style("compiler-ir", Ansi.BOLD, use_colour))
    print(style("top-level", Ansi.CYAN, use_colour))
    _print_ir_script(ir_module.top_level, prefix="  ", line_index=line_index, use_colour=use_colour)

    if not ir_module.procedures:
        print(style("procedures: (none)", Ansi.DIM, use_colour))
    else:
        print(style("procedures", Ansi.CYAN, use_colour))
        for qname in sorted(ir_module.procedures):
            proc = ir_module.procedures[qname]
            params = " ".join(proc.params)
            span = line_index.format_range(proc.range)
            header = f"{qname} {{{params}}}" if params else f"{qname} {{}}"
            print(
                f"  {style(header, Ansi.CYAN, use_colour)} "
                f"{style(f'[{span}]', Ansi.DIM, use_colour)}"
            )
            _print_ir_script(proc.body, prefix="    ", line_index=line_index, use_colour=use_colour)

    # TclOO method bodies (analysis-only; not emitted by codegen).
    if ir_module.methods:
        print(style("methods", Ansi.CYAN, use_colour))
        for mqname in sorted(ir_module.methods):
            method = ir_module.methods[mqname]
            params = " ".join(method.params)
            header = f"{mqname} {{{params}}}" if params else f"{mqname} {{}}"
            tags = method.kind
            if method.instance_vars:
                tags += ", ivars: " + " ".join(sorted(method.instance_vars))
            print(
                f"  {style(header, Ansi.CYAN, use_colour)} "
                f"{style(f'[{tags}]', Ansi.DIM, use_colour)}"
            )
            _print_ir_script(
                method.body, prefix="    ", line_index=line_index, use_colour=use_colour
            )


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
    rt = snap.analysis.return_type
    rt_interesting = rt is not None and rt.kind in (TypeKind.KNOWN, TypeKind.SHIMMERED)
    if interesting_types or rt_interesting:
        print(style("    inferred-types", Ansi.GREEN, use_colour))
        if rt_interesting:
            print(style(f"      (return): {format_type(rt)}", Ansi.GREEN, use_colour))
        for (name, ver), tl in sorted(interesting_types.items()):
            print(style(f"      {name}#{ver}: {format_type(tl)}", Ansi.GREEN, use_colour))
    else:
        print(style("    inferred-types: none", Ansi.DIM, use_colour))


# Box-drawing glyph for a 4-bit cell mask (N=1, E=2, S=4, W=8).  Used to merge
# overlapping edge segments in the ASCII control-flow gutter so crossings and
# fan-in/out render as proper junctions.
_GUTTER_GLYPH = {
    0: " ",
    1: "│",
    4: "│",
    5: "│",
    2: "─",
    8: "─",
    10: "─",
    6: "┌",
    12: "┐",
    3: "└",
    9: "┘",
    7: "├",
    14: "┬",
    13: "┤",
    11: "┴",
    15: "┼",
}

_N, _E, _S, _W = 1, 2, 4, 8


def _cfg_gutter(n_rows: int, edges: list[tuple[int, int, int, str]]) -> list[str]:
    """Render an ASCII control-flow gutter, one string per printed row.

    *edges* are ``(src_row, dst_row, lane, kind)`` tuples; lanes come from the
    shared ``cfg_layout`` routing model so the nesting matches the web SVG.  The
    arrowhead (``▶``) lands on the destination row, immediately left of the
    block text.  Returns ``[""] * n_rows`` when there are no edges (so an
    edgeless function prints exactly as before, with no left margin).
    """
    if not edges:
        return [""] * n_rows
    width = max(lane for _, _, lane, _ in edges) + 1
    grid = [[0] * width for _ in range(n_rows)]
    arrow_rows: set[int] = set()
    for src_row, dst_row, lane, _kind in edges:
        col = width - 1 - lane
        down = dst_row > src_row
        top, bot = (src_row, dst_row) if down else (dst_row, src_row)
        for r in range(top + 1, bot):
            grid[r][col] |= _N | _S
        # Corner at each endpoint: the horizontal stub always runs east (toward
        # the block text); the vertical runs toward the other endpoint.
        if down:
            grid[src_row][col] |= _E | _S
            grid[dst_row][col] |= _N | _E
        else:
            grid[src_row][col] |= _N | _E
            grid[dst_row][col] |= _E | _S
        for r in (src_row, dst_row):
            for c in range(col + 1, width):
                grid[r][c] |= _E | _W
        arrow_rows.add(dst_row)
    out: list[str] = []
    for r in range(n_rows):
        cells = [_GUTTER_GLYPH[b] for b in grid[r]]
        if r in arrow_rows:
            cells[width - 1] = "▶"
        out.append("".join(cells))
    return out


def _render_cfg(
    snapshots: list[FunctionSnapshot],
    *,
    line_index: LineIndex,
    use_colour: bool,
    post_ssa: bool,
) -> None:
    """Shared renderer for the pre- and post-SSA CFG views.

    Both build the same block listing (post-SSA adds phis, use/def sets, and an
    analysis summary) and prepend an ASCII edge gutter routed by ``cfg_layout``.
    """
    print()
    print(style("cfg-post-ssa" if post_ssa else "cfg-pre-ssa", Ansi.BOLD, use_colour))

    for snap in snapshots:
        print(style(f"function {snap.name}", Ansi.CYAN, use_colour))
        print(
            style(f"  entry={snap.cfg.entry} blocks={len(snap.cfg.blocks)}", Ansi.DIM, use_colour)
        )

        lines: list[str] = []
        header_row: dict[str, int] = {}
        term_row: dict[str, int] = {}

        for block_name, block in snap.cfg.blocks.items():
            ssa_block = snap.ssa.blocks.get(block_name) if post_ssa else None
            entry_suffix = " [entry]" if block_name == snap.cfg.entry else ""
            dead_suffix = ""
            if post_ssa and block_name in snap.analysis.unreachable_blocks:
                dead_suffix = " [unreachable]"
            header_row[block_name] = len(lines)
            lines.append(
                style(f"  block {block_name}{entry_suffix}{dead_suffix}", Ansi.BOLD, use_colour)
            )

            if ssa_block is not None and ssa_block.phis:
                for phi in ssa_block.phis:
                    incoming = ", ".join(
                        f"{pred}:{phi.name}#{ver}" for pred, ver in sorted(phi.incoming.items())
                    )
                    phi_type = snap.analysis.types.get((phi.name, phi.version))
                    type_suffix = ""
                    if phi_type is not None and phi_type.kind is not TypeKind.UNKNOWN:
                        type_suffix = f" : {format_type(phi_type)}"
                    lines.append(
                        style(
                            f"    phi {phi.name}#{phi.version} <- {incoming}{type_suffix}",
                            Ansi.MAGENTA,
                            use_colour,
                        )
                    )

            for idx, stmt in enumerate(block.statements):
                span = line_index.format_range(stmt.range)
                summary = stmt_summary(stmt)
                lines.append(
                    f"    [{idx}] {style(summary, _stmt_colour(stmt), use_colour)} "
                    f"{style(f'[{span}]', Ansi.DIM, use_colour)}"
                )
                if post_ssa:
                    uses = "{}"
                    defs = "{}"
                    if ssa_block is not None and idx < len(ssa_block.statements):
                        ssa_stmt = ssa_block.statements[idx]
                        uses = _format_uses(ssa_stmt.uses)
                        defs = _format_defs(
                            ssa_stmt.defs, snap.analysis.values, snap.analysis.types
                        )
                    lines.append(style(f"         uses={uses} defs={defs}", Ansi.DIM, use_colour))

            term = block.terminator
            term_summary = _terminator_summary(term)
            term_span = ""
            if isinstance(term, (CFGGoto, CFGBranch, CFGReturn)) and term.range is not None:
                term_span = f" [{line_index.format_range(term.range)}]"
            term_row[block_name] = len(lines)
            lines.append(style(f"    term {term_summary}{term_span}", Ansi.BLUE, use_colour))

        gutter_edges = [
            (term_row[e.src], header_row[e.dst], e.lane, e.kind)
            for e in build_cfg_edges(snap.cfg)
            if e.src in term_row and e.dst in header_row
        ]
        gutter = _cfg_gutter(len(lines), gutter_edges)
        for g, line in zip(gutter, lines):
            print(f"{style(g, Ansi.BLUE, use_colour)} {line}" if g else line)

        if post_ssa:
            _print_function_analysis_summary(snap, use_colour=use_colour)
        print()


def print_cfg_pre_ssa(
    snapshots: list[FunctionSnapshot],
    *,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    _render_cfg(snapshots, line_index=line_index, use_colour=use_colour, post_ssa=False)


def print_cfg_post_ssa(
    snapshots: list[FunctionSnapshot],
    *,
    line_index: LineIndex,
    use_colour: bool,
) -> None:
    _render_cfg(snapshots, line_index=line_index, use_colour=use_colour, post_ssa=True)


def _render_green_tokens(
    node, *, prefix: str, use_colour: bool, depth: int, max_depth: int
) -> None:
    """Draw a node's tokens as a box-drawing tree, nesting opaque regions.

    Each token is a tree entry tagged with its type and absolute byte range;
    an opaque ``{...}`` / ``[...]`` token descends into its re-lexed child
    region (a dim ``kind [mode]`` descriptor, then that region's own tokens).
    """
    from shared.tokens import TokenType

    tokens = node.tokens
    for idx, tok in enumerate(tokens):
        is_last = idx == len(tokens) - 1
        connector = _TREE_LAST if is_last else _TREE_BRANCH
        opaque = tok.type in (TokenType.STR, TokenType.CMD) and bool(tok.text)
        preview_text = repr(preview(tok.text.replace("\n", "⏎"), 40))
        print(
            f"{prefix}{connector}"
            f"{style(tok.type.name, Ansi.CYAN if opaque else Ansi.GRAY, use_colour)} "
            f"{preview_text} "
            f"{style(f'[{tok.start.offset}:{tok.end.offset}]', Ansi.DIM, use_colour)}"
        )
        if opaque and depth < max_depth:
            child = node.descend(tok)
            child_prefix = prefix + (_TREE_GAP if is_last else _TREE_VBAR)
            print(
                style(
                    f"{child_prefix}{child.kind.name.lower()} [{child.mode.name.lower()}] "
                    f"w={child.width}",
                    Ansi.CYAN if child.kind.name != "ERROR" else Ansi.RED,
                    use_colour,
                )
            )
            _render_green_tokens(
                child,
                prefix=child_prefix,
                use_colour=use_colour,
                depth=depth + 1,
                max_depth=max_depth,
            )


def print_greentree(source: str, *, use_colour: bool) -> None:
    """Render the lossless green token tree (Phase 0 lexer-layer visibility).

    Shows each region's structural kind and tokenisation :class:`Mode`
    (SCRIPT / QUOTED / EXPR / RAW), recursively descending opaque ``{...}`` /
    ``[...]`` leaves — the same lazy, memoised descent the analyser consumes.
    Unterminated/recovered regions render as ERROR nodes.
    """
    from compiler.parsing.green_tree import green_tree_scope, node_for

    print()
    print(style("green-tree", Ansi.BOLD, use_colour))
    with green_tree_scope():
        root = node_for(source)
        print(
            style(
                f"{root.kind.name.lower()} [{root.mode.name.lower()}] "
                f"@{root.base_offset} w={root.width}",
                Ansi.CYAN if root.kind.name != "ERROR" else Ansi.RED,
                use_colour,
            )
        )
        _render_green_tokens(root, prefix="", use_colour=use_colour, depth=0, max_depth=16)


def print_loops(snapshots: list[FunctionSnapshot], *, use_colour: bool) -> None:
    """Render the natural-loop forest per function (Phase 1 ``LoopForest``).

    Shows each loop's header, body-block count, latches and nesting depth so
    the loop structure the optimiser/LICM and (future) range-widening see is
    inspectable alongside the cfg/ssa views.
    """
    from compiler.loops import build_loop_forest, dominates

    print()
    print(style("loops", Ansi.BOLD, use_colour))
    for snap in snapshots:
        if snap.cfg is None or snap.ssa is None or snap.analysis is None:
            continue
        executable = set(snap.cfg.blocks) - snap.analysis.unreachable_blocks
        forest = build_loop_forest(snap.cfg, snap.ssa, executable)
        print(style(f"function {snap.name}", Ansi.CYAN, use_colour))
        if forest.is_empty():
            print(style("  (no loops)", Ansi.DIM, use_colour))
            continue
        headers = list(forest.by_header)
        for loop in forest.loops:
            depth = 1 + sum(
                1 for h in headers if h != loop.header and dominates(snap.ssa, h, loop.header)
            )
            latches = ", ".join(sorted(loop.latches)) or "-"
            print(
                style(
                    f"  header {loop.header} (depth {depth}): "
                    f"{len(loop.blocks)} block(s), latch(es): {latches}",
                    Ansi.BLUE,
                    use_colour,
                )
            )


def print_intervals(snapshots: list[FunctionSnapshot], *, use_colour: bool) -> None:
    """Render the integer-interval abstract domain per SSA value (Phase 3).

    Shows the ``[lo, hi]`` range computed for each ``name#version`` (``-inf`` /
    ``+inf`` for unbounded), so the numeric facts the bounds / divide-by-zero
    checks consult — and the loop-header widening behind them — are inspectable
    next to the ``types``/``dataflow`` views.  Only non-trivial (bounded)
    ranges are listed; everything else is ``[-inf, +inf]`` (top).
    """
    from compiler.intervals import compute_intervals

    print()
    print(style("intervals", Ansi.BOLD, use_colour))
    for snap in snapshots:
        if snap.cfg is None or snap.ssa is None or snap.analysis is None:
            continue
        intervals = compute_intervals(snap.cfg, snap.ssa, snap.analysis.values)
        bounded = {
            key: iv
            for key, iv in intervals.items()
            if not iv.is_top and (iv.lo is not None or iv.hi is not None)
        }
        print(style(f"function {snap.name}", Ansi.CYAN, use_colour))
        if not bounded:
            print(style("  (no bounded ranges)", Ansi.DIM, use_colour))
            continue
        for (name, ver), iv in sorted(bounded.items()):
            lo = "-inf" if iv.lo is None else str(iv.lo)
            hi = "+inf" if iv.hi is None else str(iv.hi)
            print(style(f"  {name}#{ver}: [{lo}, {hi}]", Ansi.BLUE, use_colour))


def print_bounds(snapshots: list[FunctionSnapshot], *, use_colour: bool) -> None:
    """Render the interval-driven dynamic bounds findings (Phase 3).

    Shows each provable out-of-range ``lindex``/``lset`` access (W230/W231) and
    divide-by-zero (W233) the interval domain finds — the resolved index range,
    the container length, and why it is out of range — so a finding (and the
    interval reasoning behind it) is explainable next to the ``intervals`` view.
    """
    from compiler.interval_bounds import find_interval_findings

    print()
    print(style("bounds", Ansi.BOLD, use_colour))
    for snap in snapshots:
        if snap.cfg is None or snap.ssa is None or snap.analysis is None:
            continue
        # Both passes share one interval fixpoint (find_interval_bounds and
        # find_divide_by_zero each recompute it otherwise — paid twice here).
        executable = set(snap.cfg.blocks) - snap.analysis.unreachable_blocks
        findings, divzero = find_interval_findings(
            snap.cfg, snap.ssa, snap.execution_intent, snap.analysis.values, executable
        )
        print(style(f"function {snap.name}", Ansi.CYAN, use_colour))
        if not findings and not divzero:
            print(style("  (no provable out-of-range / divide-by-zero)", Ansi.DIM, use_colour))
            continue
        for f in findings:
            iv = f.index_interval
            lo = "-inf" if iv.lo is None else str(iv.lo)
            hi = "+inf" if iv.hi is None else str(iv.hi)
            print(
                style(
                    f"  {f.code} {f.command} ${f.index_var}#? in [{lo}, {hi}] "
                    f"vs length {f.length} → {f.reason}",
                    Ansi.YELLOW,
                    use_colour,
                )
            )
        for dz in divzero:
            print(
                style(
                    f"  W233 '{dz.op}' divisor is provably 0 (divide by zero)",
                    Ansi.YELLOW,
                    use_colour,
                )
            )


def print_interprocedural(
    interproc: InterproceduralAnalysis,
    *,
    use_colour: bool,
) -> None:
    print(style("interprocedural", Ansi.BOLD, use_colour))

    if not interproc.procedures and not interproc.methods:
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
            f"return={format_return_shape(summary)}"
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

    # TclOO method summaries (SF-2 — consumed by the O126 my-dispatch gate).
    if interproc.methods:
        print(style("  methods", Ansi.BOLD, use_colour))
        for mqname in sorted(interproc.methods):
            msum = interproc.methods[mqname]
            calls = ", ".join(msum.calls) if msum.calls else "-"
            status_colour = Ansi.GREEN if msum.pure else Ansi.YELLOW
            ivars = " ".join(sorted(msum.writes_instance_vars)) or "-"
            print(
                style(
                    f"  {mqname} kind={msum.method_kind} pure={msum.pure}",
                    status_colour,
                    use_colour,
                )
            )
            print(style(f"    calls: {calls}", Ansi.DIM, use_colour))
            print(style(f"    writes-instance-vars: {ivars}", Ansi.DIM, use_colour))


def print_rendered_properties(snapshots: list[FunctionSnapshot], *, use_colour: bool) -> None:
    """Render the per-SSA-value rendered-property flags.

    Mirrors the GUI's Rendered Properties tab — each variable that has
    any may/must rendered-context flag set is listed with both sets.
    Variables with no flags are skipped (top-of-lattice).
    """
    print()
    print(style("rendered-properties", Ansi.BOLD, use_colour))
    any_entry = False
    for snap in snapshots:
        entries = []
        for (name, ver), rp in sorted(snap.analysis.rendered_props.items()):
            may = [f.name for f in type(rp.may).__members__.values() if f in rp.may and f.value]
            must = [f.name for f in type(rp.must).__members__.values() if f in rp.must and f.value]
            if not may and not must:
                continue
            entries.append((name, ver, may, must))
        if not entries:
            continue
        any_entry = True
        print(style(f"function {snap.name}", Ansi.CYAN, use_colour))
        for name, ver, may, must in entries:
            may_s = ",".join(may) if may else "-"
            must_s = ",".join(must) if must else "-"
            print(
                style(
                    f"  {name}#{ver}: may={{{may_s}}} must={{{must_s}}}",
                    Ansi.BLUE,
                    use_colour,
                )
            )
    if not any_entry:
        print(style("  (no rendered-property flags)", Ansi.DIM, use_colour))


def print_event_order(event_entries, *, use_colour: bool) -> None:
    """Render the iRules event-order list.

    The optimiser/iRules-flow extractor gives one entry per ``when``
    block with the resolved base/offset priority and multiplicity; the
    CLI just projects that to a sorted list.  No source parsing.
    """
    print()
    print(style("event-order", Ansi.BOLD, use_colour))
    if not event_entries:
        print(style("  (no event-order data)", Ansi.DIM, use_colour))
        return
    # Effective priority = base + offset (higher fires earlier).
    sorted_entries = sorted(
        event_entries,
        key=lambda e: (-(e.base_priority + e.priority_offset), e.event),
    )
    for e in sorted_entries:
        eff = e.base_priority + e.priority_offset
        # multiplicity is a string label ("once", "per-request", "init")
        # — render as a tag, not a count.
        mult = f" [{e.multiplicity}]" if e.multiplicity and e.multiplicity != "once" else ""
        print(
            style(
                f"  {e.event} effective={eff} (base={e.base_priority} "
                f"offset={e.priority_offset}){mult}",
                Ansi.BLUE,
                use_colour,
            )
        )


def print_dataflow(dataflow_graph, *, use_colour: bool) -> None:
    """Render the SSA dataflow graph (def → use edges) per function.

    The compiler's :class:`DataFlowGraph` ships nodes (defs) and edges
    (def → use) per function; the CLI walks each function and prints
    each node followed by its outgoing edges.  Purely a typed graph
    projection — no source text or IR re-derivation.
    """
    print()
    print(style("dataflow", Ansi.BOLD, use_colour))
    if dataflow_graph is None or not dataflow_graph.functions:
        print(style("  (no dataflow graph)", Ansi.DIM, use_colour))
        return
    for func in dataflow_graph.functions:
        print(style(f"function {func.function_name}", Ansi.CYAN, use_colour))
        print(
            style(
                f"  defs={func.total_defs} uses={func.total_uses} dead={func.dead_defs} "
                f"aliased={func.aliased_vars}",
                Ansi.DIM,
                use_colour,
            )
        )
        if not func.nodes:
            print(style("  (no defs)", Ansi.DIM, use_colour))
            continue
        # Group edges by source (def_name, def_version) for compact print.
        edges_by_src: dict[tuple[str, int], list] = {}
        for edge in func.edges:
            edges_by_src.setdefault((edge.from_name, edge.from_version), []).append(edge)
        for node in func.nodes:
            tag = " (dead)" if node.is_dead else ""
            type_part = f" : {node.type_info}" if node.type_info else ""
            lattice_part = f" = {node.lattice}" if node.lattice else ""
            print(
                style(
                    f"  {node.name}#{node.version} [{node.def_kind} @ {node.block}]"
                    f"{type_part}{lattice_part}{tag}",
                    Ansi.BLUE,
                    use_colour,
                )
            )
            for edge in edges_by_src.get((node.name, node.version), []):
                target = f"{edge.to_block}[{edge.to_statement_index}]"
                if edge.to_name:
                    target += f" → {edge.to_name}"
                kind = edge.edge_kind.name.lower()
                print(style(f"    └── {kind} → {target}", Ansi.DIM, use_colour))
        for alias in func.aliases:
            print(style(f"  alias: {alias!r}", Ansi.DIM, use_colour))


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
    """Render the full type lattice (peer to the SSA view) for each function.

    Includes ``UNKNOWN`` (``?``) and ``OVERDEFINED`` (``*``) slots so the
    full lattice progression is visible per SSA version, plus a synthetic
    ``(return)`` row at the top sourced from ``analysis.return_type`` --
    the latter shows a return type for procs whose body is just
    ``return [expr ...]`` and so has no SSA def-site for the analyser to
    attach the type to.
    """
    from compiler.intervals import compute_intervals

    print(style("type-inference", Ansi.BOLD, use_colour))

    any_types = False
    for snap in snapshots:
        rt = snap.analysis.return_type
        if not snap.analysis.types and rt is None:
            continue
        any_types = True
        intervals = compute_intervals(snap.cfg, snap.ssa, snap.analysis.values)
        print(style(f"  function {snap.name}", Ansi.CYAN, use_colour))
        if rt is not None:
            rt_colour = {
                TypeKind.KNOWN: Ansi.GREEN,
                TypeKind.SHIMMERED: Ansi.YELLOW,
                TypeKind.OVERDEFINED: Ansi.MAGENTA,
                TypeKind.UNKNOWN: Ansi.DIM,
            }.get(rt.kind, Ansi.DIM)
            print(style(f"    (return): {format_type(rt)}", rt_colour, use_colour))
        for (name, ver), tl in sorted(snap.analysis.types.items()):
            colour = {
                TypeKind.KNOWN: Ansi.GREEN,
                TypeKind.SHIMMERED: Ansi.YELLOW,
                TypeKind.OVERDEFINED: Ansi.MAGENTA,
                TypeKind.UNKNOWN: Ansi.DIM,
            }.get(tl.kind, Ansi.DIM)
            rng = ""
            iv = intervals.get((name, ver))
            if iv is not None and not iv.is_top and (iv.lo is not None or iv.hi is not None):
                lo = "-inf" if iv.lo is None else str(iv.lo)
                hi = "+inf" if iv.hi is None else str(iv.hi)
                rng = style(f"  range [{lo}, {hi}]", Ansi.DIM, use_colour)
            print(style(f"    {name}#{ver}: {format_type(tl)}", colour, use_colour) + rng)

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
    warnings: list[TaintWarning],
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
            details = f" ({w.variable} -> {w.sink_command})"
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
    from compiler.codegen.bytecode import codegen_module, format_module_asm

    if cfg_module is None:
        from compiler.cfg import build_cfg

        cfg_module = build_cfg(ir_module)
    module_asm = codegen_module(cfg_module, ir_module)
    text = format_module_asm(module_asm)
    _print_coloured_listing("bytecode-asm", text, _ASM_RULES, use_colour=use_colour)


def print_wasm(
    ir_module: IRModule, *, optimise: bool = False, cfg_module=None, use_colour: bool
) -> None:
    """Render WebAssembly text (WAT) for all functions in the module."""
    from compiler.codegen.wasm import wasm_codegen_module

    if cfg_module is None:
        from compiler.cfg import build_cfg

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
        help="Shortcut for a common --show group (compiler / optimiser / all).",
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
        "--opt",
        choices=("off", "on", "diff"),
        default="off",
        help="Optimisation lens for the IR/CFG/SSA/ASM/WASM views: render the "
        "original path (off), the optimised path (on), or a diff of the two "
        "(diff).  Other views ignore it.",
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
    fmt = parser.add_mutually_exclusive_group()
    fmt.add_argument(
        "--tui",
        dest="format",
        action="store_const",
        const="tui",
        help="Live Textual UI (default on an interactive terminal).",
    )
    fmt.add_argument(
        "--text",
        dest="format",
        action="store_const",
        const="text",
        help="Flat scrolling text (default when piped / non-interactive).",
    )
    fmt.add_argument(
        "--json",
        dest="format",
        action="store_const",
        const="json",
        help="Machine-readable JSON (same serialisation as the web explorer).",
    )
    parser.set_defaults(format=None)
    args = parser.parse_args(argv)

    show_raw = args.focus if args.focus is not None else args.show
    try:
        args.views = expand_show(show_raw)
    except argparse.ArgumentTypeError as exc:
        parser.error(str(exc))
    return args


# Ordered view list — the order in which views render in text mode and appear
# in the TUI sidebar.
_VIEW_ORDER: tuple[str, ...] = (
    "greentree",
    "ir",
    "cfg",
    "ssa",
    "loops",
    "types",
    "rendered",
    "intervals",
    "bounds",
    "dataflow",
    "interproc",
    "opt",
    "gvn",
    "shimmer",
    "taint",
    "irules",
    "event-order",
    "callouts",
    "asm",
    "wasm",
    "asm-opt",
    "wasm-opt",
)


def render_view(
    view: str,
    result: CompilerExplorerResult,
    source: str,
    *,
    use_colour: bool,
    line_index: LineIndex,
    views: frozenset[str] = frozenset(),
    show_optimised_source: bool = False,
    max_annotations: int = 80,
) -> None:
    """Render a single explorer *view* to stdout.

    The single source of truth for view rendering, shared by the flat-text CLI
    path and the Textual TUI (which captures this output per view).  Each view
    is self-contained so it can be rendered independently.
    """
    if view == "ir":
        print_ir_module(result.ir_module, line_index=line_index, use_colour=use_colour)
    elif view == "cfg":
        print_cfg_pre_ssa(result.snapshots, line_index=line_index, use_colour=use_colour)
    elif view == "ssa":
        print_cfg_post_ssa(result.snapshots, line_index=line_index, use_colour=use_colour)
    elif view == "greentree":
        print_greentree(source, use_colour=use_colour)
    elif view == "loops":
        print_loops(result.snapshots, use_colour=use_colour)
    elif view == "intervals":
        print_intervals(result.snapshots, use_colour=use_colour)
    elif view == "bounds":
        print_bounds(result.snapshots, use_colour=use_colour)
    elif view == "interproc":
        print_interprocedural(result.interproc, use_colour=use_colour)
    elif view == "types":
        print_types(result.snapshots, use_colour=use_colour)
    elif view == "opt":
        print_optimiser(result.optimisations, line_index=line_index, use_colour=use_colour)
        if show_optimised_source and result.optimised_source != source:
            print_source_listing("optimised-source", result.optimised_source, use_colour=use_colour)
    elif view == "gvn":
        print_gvn_warnings(result.gvn_warnings, line_index=line_index, use_colour=use_colour)
    elif view == "shimmer":
        print_shimmer_warnings(
            result.shimmer_warnings, line_index=line_index, use_colour=use_colour
        )
    elif view == "taint":
        print_taint(
            result.taint_warnings, result.snapshots, line_index=line_index, use_colour=use_colour
        )
    elif view == "irules":
        print_irules_flow(result.irules_flow_warnings, line_index=line_index, use_colour=use_colour)
    elif view in ("asm", "wasm"):
        from compiler.cfg import build_cfg

        cfg = build_cfg(result.ir_module)
        if view == "asm":
            print_asm(result.ir_module, cfg_module=cfg, use_colour=use_colour)
        else:
            try:
                print_wasm(result.ir_module, cfg_module=cfg, use_colour=use_colour)
            except Exception as exc:
                print(f"warning: wasm generation failed: {exc}", file=sys.stderr)
    elif view in ("asm-opt", "wasm-opt"):
        if result.optimised_source == result.source:
            print("(no optimisations applied — optimised output is identical to the default path)")
            return
        from compiler.cfg import build_cfg
        from compiler.lowering import lower_to_ir

        try:
            opt_ir = lower_to_ir(result.optimised_source)
            opt_cfg = build_cfg(opt_ir)
        except Exception as exc:
            print(f"warning: optimised IR lowering failed: {exc}", file=sys.stderr)
            return
        try:
            if view == "asm-opt":
                print_asm(opt_ir, cfg_module=opt_cfg, use_colour=use_colour)
            else:
                print_wasm(opt_ir, optimise=True, cfg_module=opt_cfg, use_colour=use_colour)
        except Exception as exc:
            print(f"warning: optimised {view} generation failed: {exc}", file=sys.stderr)
    elif view == "callouts":
        annotations, omitted = collect_annotations(
            result, views=views or frozenset(_VIEW_ORDER), max_annotations=max_annotations
        )
        print_source_callouts(
            source, annotations, use_colour=use_colour, omitted_annotations=omitted
        )
    elif view == "dataflow":
        print_dataflow(result.dataflow_graph, use_colour=use_colour)
    elif view == "rendered":
        print_rendered_properties(result.snapshots, use_colour=use_colour)
    elif view == "event-order":
        print_event_order(result.event_order, use_colour=use_colour)
    else:
        # Loud failure when a view in ALL_VIEWS / _VIEW_ORDER has no
        # rendering branch — beats the silent no-op the dataflow view
        # used to produce.  Catches view-registration drift between the
        # pipeline and this dispatcher early.
        raise ValueError(f"render_view: no renderer for view {view!r}")


# Views whose output reflects the lowered program, so they can be rendered
# against the optimised path (``--opt on``) or diffed against the original
# (``--opt diff``).  Other views (greentree, analysis facts, callouts, ...)
# have no meaningful optimised variant and ignore the opt mode.
OPT_VIEWS = frozenset({"ir", "cfg", "ssa", "asm", "wasm"})


def optimised_result(
    result: CompilerExplorerResult, dialect: str
) -> tuple[CompilerExplorerResult, str] | None:
    """Run the pipeline on the optimised source so opt-aware views can show it.

    Returns ``(opt_result, optimised_source)`` or ``None`` when the optimiser
    left the source unchanged (nothing to render / diff).
    """
    opt_source = result.optimised_source
    if not opt_source or opt_source == result.source:
        return None
    try:
        return run_pipeline(opt_source, dialect=dialect), opt_source
    except Exception as exc:  # pragma: no cover - optimised source should re-lower
        print(f"warning: optimised pipeline failed: {exc}", file=sys.stderr)
        return None


def _capture_view(
    view: str,
    result: CompilerExplorerResult,
    source: str,
    *,
    line_index: LineIndex,
    views: frozenset[str],
    max_annotations: int,
) -> list[str]:
    import contextlib
    import io

    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        render_view(
            view,
            result,
            source,
            use_colour=False,
            line_index=line_index,
            views=views,
            max_annotations=max_annotations,
        )
    return buf.getvalue().splitlines()


# The optimiser shifts byte offsets, source ranges, sequence indices, and the
# box-drawing connectors that encode tree depth / control-flow edges every time
# it adds or removes a node.  The opt-diff compares *nodes* (an IR statement, a
# bytecode instruction, a CFG block), so these position-only tokens are
# collapsed to a constant before diffing — otherwise a single rewrite shifts
# every following line and the localised change drowns in offset noise.  This
# mirrors the web GUI, which already diffs offset-free keys (``normaliseForDiff``
# / ``irToLines`` in ``static/explorer-core.js``).
_DIFF_RANGE_SPAN = re.compile(r"\[(?:\d+:\d+-\d+:\d+|\?:\?-\?:\?)\]")  # [1:1-1:7] source range
_DIFF_ASM_OFFSET = re.compile(r"^(\s*)\(\d+\)")  # (0) bytecode byte offset
_DIFF_CFG_INDEX = re.compile(r"\[\d+\]")  # [0] CFG statement sequence number
_DIFF_PC_TARGET = re.compile(r"\bpc \d+")  # absolute jump target (pc 33)
_DIFF_ASM_COUNTS = re.compile(r"\b\d+ (instructions|bytes|literals|variables)\b")  # header tallies
_DIFF_PUSH_LITERAL = re.compile(
    r"\b(push\d+) \d+"
)  # push1 0 -> push1 (the comment names the literal)
_DIFF_LITERAL_INDEX = re.compile(r"^(\s*)\d+: ")  # literal-pool table index
_DIFF_LVT_SLOT = re.compile(r"%v\d+")  # local-variable-table slot (the comment names the variable)
_DIFF_BOX_ARROWS = "▶◀▸◂▴▾"


def _normalise_diff_line(line: str) -> str:
    """Reduce a rendered view line to an offset-free comparison key.

    Maps every position-only token (tree/gutter glyph, source range, byte
    offset, sequence index, literal-pool index, header tally) to a constant so
    two renderings of the *same* node compare equal even when the optimiser
    shifted everything around it.  Operand values that carry meaning —
    instruction arities, increment immediates, variable slots, literal text —
    are left untouched so genuinely different nodes still differ.
    """
    # Box-drawing glyphs (U+2500–U+257F) and arrowheads encode position, not
    # content; map each to a single space so ``├──`` vs ``└──`` and the CFG
    # edge gutter never register as a change while column width (the tree
    # depth) is preserved.
    key = "".join(" " if "─" <= ch <= "╿" or ch in _DIFF_BOX_ARROWS else ch for ch in line)
    key = _DIFF_RANGE_SPAN.sub("[]", key)
    key = _DIFF_ASM_OFFSET.sub(r"\1()", key)
    key = _DIFF_CFG_INDEX.sub("[]", key)
    key = _DIFF_PC_TARGET.sub("pc", key)
    key = _DIFF_ASM_COUNTS.sub(r"N \1", key)
    key = _DIFF_PUSH_LITERAL.sub(r"\1", key)
    key = _DIFF_LITERAL_INDEX.sub(r"\1#: ", key)
    key = _DIFF_LVT_SLOT.sub("%v", key)
    return key.rstrip()


def _print_opt_diff(view: str, before: list[str], after: list[str], *, use_colour: bool) -> None:
    """Print a unified diff of two rendered views with offsets ignored.

    The hunks are computed over offset-free keys (:func:`_normalise_diff_line`)
    so only genuine node changes drive the diff, but the *original* lines —
    offsets and all — are what gets displayed, so the reader still sees the real
    instruction stream.  Lines that differ only in offsets fall into ``equal``
    runs and render as quiet context.
    """
    import difflib

    matcher = difflib.SequenceMatcher(
        None,
        [_normalise_diff_line(line) for line in before],
        [_normalise_diff_line(line) for line in after],
        autojunk=False,
    )
    # get_grouped_opcodes is a generator (always truthy) — materialise it so the
    # "no change" check below fires when the streams are offset-identical.
    groups = list(matcher.get_grouped_opcodes(n=3))
    if not groups:
        print(
            style(f"{view}: no change under the optimiser (offsets ignored)", Ansi.DIM, use_colour)
        )
        return
    print(style(f"--- {view} (original)", Ansi.RED, use_colour))
    print(style(f"+++ {view} (optimised)", Ansi.GREEN, use_colour))
    for group in groups:
        first, last = group[0], group[-1]
        print(
            style(
                f"@@ -{first[1] + 1},{last[2] - first[1]} +{first[3] + 1},{last[4] - first[3]} @@",
                Ansi.CYAN,
                use_colour,
            )
        )
        for tag, i1, i2, j1, j2 in group:
            if tag == "equal":
                for line in before[i1:i2]:
                    print(f" {line}")
                continue
            for line in before[i1:i2]:
                print(style(f"-{line}", Ansi.RED, use_colour))
            for line in after[j1:j2]:
                print(style(f"+{line}", Ansi.GREEN, use_colour))


def render_view_opt(
    view: str,
    result: CompilerExplorerResult,
    source: str,
    *,
    use_colour: bool,
    line_index: LineIndex,
    opt_mode: str = "off",
    opt: tuple[CompilerExplorerResult, str] | None = None,
    views: frozenset[str] = frozenset(),
    show_optimised_source: bool = False,
    max_annotations: int = 80,
) -> None:
    """Render *view* honouring the optimisation lens (off / on / diff).

    *opt* is the ``(opt_result, opt_source)`` pair from :func:`optimised_result`
    (or ``None``).  Only :data:`OPT_VIEWS` react to the mode; every other view
    renders exactly as :func:`render_view` would.  This is the shared entry
    point for the text path and the Textual TUI so the lens behaves identically.
    """
    if opt_mode == "off" or view not in OPT_VIEWS:
        render_view(
            view,
            result,
            source,
            use_colour=use_colour,
            line_index=line_index,
            views=views,
            show_optimised_source=show_optimised_source,
            max_annotations=max_annotations,
        )
        return

    if opt is None:
        # Nothing changed — fall back to the original and say so once.
        render_view(
            view,
            result,
            source,
            use_colour=use_colour,
            line_index=line_index,
            views=views,
            max_annotations=max_annotations,
        )
        print(style("  (optimiser left the source unchanged)", Ansi.DIM, use_colour))
        return

    opt_result, opt_source = opt
    if opt_mode == "on":
        render_view(
            view,
            opt_result,
            opt_source,
            use_colour=use_colour,
            line_index=LineIndex(opt_source),
            views=views,
            max_annotations=max_annotations,
        )
        return

    # diff: a node-level unified diff, surface-agnostic across all OPT_VIEWS.
    # Offsets are normalised away (see _normalise_diff_line) so the diff tracks
    # the node, not where the optimiser pushed it.
    before = _capture_view(
        view, result, source, line_index=line_index, views=views, max_annotations=max_annotations
    )
    after = _capture_view(
        view,
        opt_result,
        opt_source,
        line_index=LineIndex(opt_source),
        views=views,
        max_annotations=max_annotations,
    )
    _print_opt_diff(view, before, after, use_colour=use_colour)


def _summary_parts(result: CompilerExplorerResult, dialect: str) -> list[str]:
    total_dead_stores = sum(len(s.analysis.dead_stores) for s in result.snapshots)
    total_unreachable = sum(len(s.analysis.unreachable_blocks) for s in result.snapshots)
    parts = [
        f"dialect={dialect}",
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
    return parts


def _render_text(
    result: CompilerExplorerResult, source: str, args: argparse.Namespace, *, use_colour: bool
) -> int:
    line_index = LineIndex(source)
    _version = FULL_VERSION + (f" ({BUILD_TIMESTAMP})" if BUILD_TIMESTAMP else "")
    print(style(f"compiler-optimiser-explorer {_version}", Ansi.BOLD, use_colour))
    print(style(" ".join(_summary_parts(result, args.dialect)), Ansi.DIM, use_colour))
    opt_label = "" if args.opt == "off" else f"  opt={args.opt}"
    print(style(f"views: {','.join(sorted(args.views))}{opt_label}", Ansi.DIM, use_colour))
    print()
    opt = optimised_result(result, args.dialect) if args.opt != "off" else None
    for view in _VIEW_ORDER:
        if view not in args.views:
            continue
        if view == "callouts" and args.no_source_callouts:
            continue
        render_view_opt(
            view,
            result,
            source,
            use_colour=use_colour,
            line_index=line_index,
            opt_mode=args.opt,
            opt=opt,
            views=args.views,
            show_optimised_source=args.show_optimised_source,
            max_annotations=args.max_annotations,
        )
        print()
    return 0


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)

    try:
        source = load_source(args.path, args.source)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    # Resolve output mode: explicit flag wins; otherwise TUI on an interactive
    # terminal (and only if Textual is installed), else flat text.
    mode = args.format
    if mode is None:
        mode = "tui" if sys.stdout.isatty() and _textual_available() else "text"

    if mode == "json":
        from tooling.cli.serialise import serialise_result

        try:
            result = run_pipeline(source, dialect=args.dialect)
        except Exception as exc:
            print(f"error: compiler exploration failed: {exc}", file=sys.stderr)
            return 2
        print(json.dumps(serialise_result(result), indent=2))
        return 0

    if mode == "tui":
        from tooling.explorer.tui import run_tui

        return run_tui(source, args)

    # text
    use_colour = (not args.no_colour) and sys.stdout.isatty()
    try:
        result = run_pipeline(source, dialect=args.dialect)
    except Exception as exc:
        print(f"error: compiler exploration failed: {exc}", file=sys.stderr)
        return 2
    return _render_text(result, source, args, use_colour=use_colour)


def _textual_available() -> bool:
    try:
        import textual  # noqa: F401

        return True
    except ImportError:
        return False
