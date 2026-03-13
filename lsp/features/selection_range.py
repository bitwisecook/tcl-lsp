"""Selection range provider -- smart expand/shrink selection (⌥⇧→ / ⌥⇧←)."""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, Range, Scope
from core.common.lsp import to_lsp_range
from core.common.position import find_command_at_position, find_token_in_command

from .symbol_resolution import find_scope_at_line, find_word_span_at_position


def _lsp_ranges_equal(a: types.Range, b: types.Range) -> bool:
    """Check if two LSP ranges are identical."""
    return (
        a.start.line == b.start.line
        and a.start.character == b.start.character
        and a.end.line == b.end.line
        and a.end.character == b.end.character
    )


def _build_chain(ranges: list[types.Range]) -> types.SelectionRange | None:
    """Build a SelectionRange linked list from a list of ranges (inner to outer).

    Skips duplicate consecutive ranges.
    """
    if not ranges:
        return None

    # Deduplicate consecutive identical ranges
    deduped: list[types.Range] = [ranges[0]]
    for r in ranges[1:]:
        if not _lsp_ranges_equal(r, deduped[-1]):
            deduped.append(r)

    # Build linked list from outermost to innermost
    node: types.SelectionRange | None = None
    for r in reversed(deduped):
        node = types.SelectionRange(range=r, parent=node)
    return node


def _file_range(source: str, *, lines: list[str] | None = None) -> types.Range:
    """Return an LSP range covering the entire file."""
    if lines is None:
        lines = source.split("\n")
    last_line = max(0, len(lines) - 1)
    last_char = len(lines[last_line]) if lines else 0
    return types.Range(
        start=types.Position(line=0, character=0),
        end=types.Position(line=last_line, character=last_char),
    )


def _selection_range_for_position(
    source: str,
    line: int,
    character: int,
    analysis: AnalysisResult,
    *,
    lines: list[str] | None = None,
) -> types.SelectionRange:
    """Build the selection range chain for a single cursor position."""
    ranges: list[types.Range] = []

    # 1. Word range
    word_span = find_word_span_at_position(source, line, character, lines=lines)
    if word_span is not None:
        _word, start_col, end_col = word_span
        ranges.append(
            types.Range(
                start=types.Position(line=line, character=start_col),
                end=types.Position(line=line, character=end_col),
            )
        )

    # 2-3. Token and command ranges
    cmd = find_command_at_position(source, line, character)
    if cmd is not None:
        # Token range (individual argument)
        tok = find_token_in_command(cmd, line, character)
        if tok is not None:
            tok_range = to_lsp_range(Range(start=tok.start, end=tok.end))
            ranges.append(tok_range)

        # Full command range
        cmd_range = to_lsp_range(cmd.range)
        ranges.append(cmd_range)

    # 4-5. Scope chain: proc body → namespace body → ...
    scope = find_scope_at_line(analysis.global_scope, line)
    current: Scope | None = scope
    while current is not None:
        if current.kind == "proc":
            # Find the ProcDef in the parent scope
            parent = current.parent
            if parent is not None:
                proc_def = parent.procs.get(current.name)
                if proc_def is not None:
                    ranges.append(to_lsp_range(proc_def.body_range))
        elif current.kind == "namespace" and current.body_range is not None:
            ranges.append(to_lsp_range(current.body_range))
        current = current.parent

    # 6. Whole file
    ranges.append(_file_range(source, lines=lines))

    # Sort from innermost (smallest) to outermost (largest).
    # Ranges that start later are inner; for same start, those ending earlier
    # are inner.
    ranges.sort(
        key=lambda r: (
            -r.start.line,
            -r.start.character,
            r.end.line,
            r.end.character,
        )
    )

    chain = _build_chain(ranges)
    # Should always have at least the file range
    assert chain is not None
    return chain


def get_selection_ranges(
    source: str,
    positions: list[types.Position],
    analysis: AnalysisResult | None = None,
    *,
    lines: list[str] | None = None,
) -> list[types.SelectionRange]:
    """Return selection ranges for each requested cursor position."""
    if analysis is None:
        analysis = analyse(source)

    return [
        _selection_range_for_position(
            source,
            pos.line,
            pos.character,
            analysis,
            lines=lines,
        )
        for pos in positions
    ]
