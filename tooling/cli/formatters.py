"""Shared formatting utilities for the compiler explorer.

Provides text formatters for lattice values, types, taints, IR statements,
and source-range helpers shared between the CLI and web explorers.
"""

from __future__ import annotations

from analyser.semantic_model import Range
from compiler.core_analyses import LatticeKind, LatticeValue
from compiler.expr_ast import ExprNode, render_expr
from compiler.interprocedural import ProcSummary
from compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRFor,
    IRIf,
    IRIncr,
    IRReturn,
    IRStatement,
    IRSwitch,
)
from compiler.registry.taint_hints import TaintColour
from compiler.taint import TaintLattice
from compiler.types import TypeKind, TypeLattice
from shared.ranges import widen_for_highlight
from shared.source_map import offset_to_line_col

# LineIndex — source offset mapping


class LineIndex:
    """Fast line/column lookup for source text."""

    def __init__(self, source: str) -> None:
        self.source = source
        self.lines = source.splitlines()
        if source.endswith("\n"):
            self.lines.append("")
        self.line_starts = [0]
        for i, ch in enumerate(source):
            if ch == "\n":
                self.line_starts.append(i + 1)
        if not self.lines:
            self.lines = [""]

    def line_count(self) -> int:
        return len(self.lines)

    def line_text(self, line: int) -> str:
        if 0 <= line < len(self.lines):
            return self.lines[line]
        return ""

    def line_start(self, line: int) -> int:
        if line < 0:
            return 0
        if line >= len(self.line_starts):
            return len(self.source)
        return self.line_starts[line]

    def line_end_exclusive(self, line: int) -> int:
        if line + 1 < len(self.line_starts):
            return self.line_starts[line + 1]
        return len(self.source)

    def offset_to_line_col(self, offset: int) -> tuple[int, int]:
        return offset_to_line_col(self.source, self.line_starts, offset)

    def format_range(self, r: Range) -> str:
        s_line, s_col = self.offset_to_line_col(r.start.offset)
        e_line, e_col = self.offset_to_line_col(r.end.offset)
        return f"{s_line + 1}:{s_col + 1}-{e_line + 1}:{e_col + 1}"


# Value formatters


def to_str(v) -> str:
    """Coerce value to string, handling ExprNode types."""
    if isinstance(v, str):
        return v
    if isinstance(v, ExprNode):
        return render_expr(v)
    return str(v)


def preview(text, limit: int = 64) -> str:
    """Escape and truncate text for display."""
    text = to_str(text)
    escaped = text.replace("\\", "\\\\").replace("\n", "\\n").replace("\t", "\\t")
    return escaped[: limit - 3] + "..." if len(escaped) > limit else escaped


def format_lattice(value: LatticeValue) -> str:
    if value.kind is LatticeKind.UNKNOWN:
        return "unknown"
    if value.kind is LatticeKind.OVERDEFINED:
        return "overdefined"
    return f"const({value.value!r})"


def format_type(tl: TypeLattice) -> str:
    match tl.kind:
        case TypeKind.UNKNOWN:
            return "?"
        case TypeKind.OVERDEFINED:
            return "*"
        case TypeKind.KNOWN if tl.tcl_type is not None:
            return tl.tcl_type.name.lower()
        case TypeKind.SHIMMERED if tl.from_type is not None and tl.tcl_type is not None:
            return f"shimmered({tl.from_type.name.lower()}/{tl.tcl_type.name.lower()})"
        case _:
            return "?"


def format_taint(tl: TaintLattice) -> str:
    if not tl.tainted:
        return "untainted"
    colours = []
    for c in TaintColour:
        if c is TaintColour.TAINTED:
            continue
        if c in tl.colour:
            colours.append(c.name.lower())
    if colours:
        return f"tainted({','.join(colours)})"
    return "tainted"


# IR statement helpers


def stmt_summary(stmt: IRStatement) -> str:
    """One-line summary of an IR statement.

    The single source of truth for both the text CLI / TUI (ANSI-coloured)
    and the JSON serialiser (web GUI).  Renderers consume the string
    verbatim; they don't re-format the IR.
    """
    if isinstance(stmt, IRAssignConst):
        return f"assign-const {stmt.name} = {stmt.value}"
    if isinstance(stmt, IRAssignExpr):
        return f"assign-expr {stmt.name} = [expr {{{preview(stmt.expr, 48)}}}]"
    if isinstance(stmt, IRAssignValue):
        return f"assign-value {stmt.name} = {preview(stmt.value, 48)}"
    if isinstance(stmt, IRIncr):
        return f"incr {stmt.name}" + (f" {preview(stmt.amount, 32)}" if stmt.amount else "")
    if isinstance(stmt, IRCall):
        rendered_args = " ".join(preview(a, 20) for a in stmt.args[:4])
        if len(stmt.args) > 4:
            rendered_args += " ..."
        return f"call {stmt.command}" + (f" {rendered_args}" if rendered_args else "")
    if isinstance(stmt, IRReturn):
        return "return" + (f" {preview(stmt.value, 48)}" if stmt.value else "")
    if isinstance(stmt, IRBarrier):
        return f"barrier {stmt.reason}" + (f" ({stmt.command})" if stmt.command else "")
    if isinstance(stmt, IRIf):
        return f"if ({len(stmt.clauses)} clause(s){', else' if stmt.else_body is not None else ''})"
    if isinstance(stmt, IRFor):
        return f"for ({preview(stmt.condition, 40)})"
    if isinstance(stmt, IRSwitch):
        return f"switch {preview(stmt.subject, 40)} ({len(stmt.arms)} arm(s))"
    return stmt.__class__.__name__


def stmt_color_class(stmt: IRStatement) -> str:
    """CSS class for an IR statement kind."""
    if isinstance(stmt, IRBarrier):
        return "ir-barrier"
    if isinstance(stmt, IRCall):
        return "ir-call"
    if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr)):
        return "ir-assign"
    if isinstance(stmt, IRReturn):
        return "ir-return"
    if isinstance(stmt, (IRIf, IRFor, IRSwitch)):
        return "ir-control"
    return "ir-other"


def stmt_kind(stmt: IRStatement) -> str:
    """Short name for an IR statement kind."""
    return stmt.__class__.__name__


# Interprocedural helpers


def format_return_shape(summary: ProcSummary) -> str:
    """Project a :class:`ProcSummary`'s return facts to a display string.

    Single source of truth for both the text CLI/TUI summary line and
    the JSON ``returnShape`` field the web GUI reads — neither side
    derives this from the raw summary fields.
    """
    if summary.returns_constant:
        return f"const({summary.constant_return!r})"
    if summary.return_passthrough_param is not None:
        return f"passthrough({summary.return_passthrough_param})"
    if summary.return_depends_on_params:
        deps = ",".join(summary.return_depends_on_params)
        return f"depends({deps})"
    return "unknown"


# Range serialisation helper


def range_dict(r: Range) -> dict:
    """Convert Range to a JSON-serialisable dict.

    Two adjustments make the serialised offsets match what the front-end
    highlights:

    * A braced/quoted/bracketed word's range stops on the last *inner*
      character (the closer is excluded by convention).  When the document
      source is known (set via ``shared.ranges.set_highlight_source``),
      widen the range to cover the closing ``}`` / ``"`` / ``]`` so
      ``{$condition}`` is not highlighted as ``{$conditi``.
    * The semantic-model :class:`Range` end is *inclusive*; the front-end
      slices ``src.substring(startOffset, endOffset)`` with an *exclusive*
      end (matching :func:`server._lsp_conv.to_lsp_range`), so add one to the
      end position.
    """
    r = widen_for_highlight(r)
    return {
        "startLine": r.start.line,
        "startCol": r.start.character,
        "startOffset": r.start.offset,
        "endLine": r.end.line,
        "endCol": r.end.character + 1,
        "endOffset": r.end.offset + 1,
    }
