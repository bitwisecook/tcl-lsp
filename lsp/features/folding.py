"""Folding range provider -- proc bodies, namespaces, comment blocks, control structures."""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, Scope
from core.commands.registry.runtime import iter_body_arguments
from core.parsing.command_segmenter import segment_commands
from core.parsing.tokens import Token, TokenType


def _adjust_body_end_line(source: str, end_offset: int, end_line: int) -> int:
    """Return a fold end line that leaves the closing ``}`` visible.

    A braced body token ends at the character immediately before its closing
    ``}``.  When that character is a newline, ``}`` is on ``end_line + 1`` and
    ``end_line`` is already the correct fold end (VS Code keeps the line after
    ``end_line`` visible).  When ``}`` sits on the same line as the last byte
    of content -- e.g. ``} else {`` or a trailing ``}`` on the same line as
    inner text -- the unadjusted fold would cover the separator line and
    collide with the next sibling body's fold range, producing the
    non-hierarchical overlap VS Code's folding tree-builder rejects.  Moving
    the fold end up one line in that case keeps siblings disjoint.
    """
    if 0 <= end_offset < len(source) and source[end_offset] != "\n":
        return end_line - 1
    return end_line


def _collect_scope_folds(
    scope: Scope,
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
    source: str,
) -> None:
    """Emit folding ranges from the scope tree (procs and namespaces)."""
    for proc_def in scope.procs.values():
        br = proc_def.body_range
        if br.start.line < br.end.line:
            end_line = _adjust_body_end_line(source, br.end.offset, br.end.line)
            if end_line > br.start.line:
                key = (br.start.line, end_line)
                if key not in seen:
                    seen.add(key)
                    ranges.append(
                        types.FoldingRange(
                            start_line=br.start.line,
                            end_line=end_line,
                            kind=types.FoldingRangeKind.Region,
                        )
                    )

    for child in scope.children:
        if child.kind == "namespace" and child.body_range is not None:
            br = child.body_range
            if br.start.line < br.end.line:
                end_line = _adjust_body_end_line(source, br.end.offset, br.end.line)
                if end_line > br.start.line:
                    key = (br.start.line, end_line)
                    if key not in seen:
                        seen.add(key)
                        ranges.append(
                            types.FoldingRange(
                                start_line=br.start.line,
                                end_line=end_line,
                                kind=types.FoldingRangeKind.Region,
                            )
                        )
        _collect_scope_folds(child, seen, ranges, source)


def _collect_comment_folds(
    source: str,
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
    *,
    lines: list[str] | None = None,
) -> None:
    """Emit folding ranges for consecutive comment-line blocks."""
    if lines is None:
        lines = source.split("\n")
    block_start: int | None = None

    for i, line in enumerate(lines):
        stripped = line.lstrip()
        if stripped.startswith("#"):
            if block_start is None:
                block_start = i
        else:
            if block_start is not None and i - block_start >= 2:
                key = (block_start, i - 1)
                if key not in seen:
                    seen.add(key)
                    ranges.append(
                        types.FoldingRange(
                            start_line=block_start,
                            end_line=i - 1,
                            kind=types.FoldingRangeKind.Comment,
                        )
                    )
            block_start = None

    # Handle trailing comment block at end of file
    if block_start is not None:
        end = len(lines) - 1
        if end - block_start >= 1:
            key = (block_start, end)
            if key not in seen:
                seen.add(key)
                ranges.append(
                    types.FoldingRange(
                        start_line=block_start,
                        end_line=end,
                        kind=types.FoldingRangeKind.Comment,
                    )
                )


def _collect_body_folds(
    source: str,
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
    *,
    original_source: str,
    body_token: Token | None = None,
    depth: int = 0,
) -> None:
    """Recursively segment commands and emit folds for multi-line BODY args."""
    if depth > 20:
        return

    for cmd in segment_commands(source, body_token):
        if not cmd.argv:
            continue
        for body in iter_body_arguments(cmd.name, cmd.args, cmd.arg_tokens):
            if body.token.type is not TokenType.STR:
                continue
            if body.token.start.line < body.token.end.line:
                end_line = _adjust_body_end_line(
                    original_source,
                    body.token.end.offset,
                    body.token.end.line,
                )
                if end_line > body.token.start.line:
                    key = (body.token.start.line, end_line)
                    if key not in seen:
                        seen.add(key)
                        ranges.append(
                            types.FoldingRange(
                                start_line=body.token.start.line,
                                end_line=end_line,
                                kind=types.FoldingRangeKind.Region,
                            )
                        )
                # Recurse into the body regardless of whether the outer fold
                # was adjusted away -- inner multi-line bodies may still be
                # foldable even when the enclosing token collapses to a
                # single effective line.
                _collect_body_folds(
                    body.text,
                    seen,
                    ranges,
                    original_source=original_source,
                    body_token=body.token,
                    depth=depth + 1,
                )


def _normalise_overlaps(
    ranges: list[types.FoldingRange],
) -> list[types.FoldingRange]:
    """Ensure folding ranges are disjoint or properly nested.

    VS Code builds a folding tree from the returned ranges and silently drops
    or misplaces ranges that partially overlap (share a boundary line without
    one containing the other).  The collectors already try to avoid this via
    ``_adjust_body_end_line``, but a belt-and-suspenders post-pass keeps the
    output well-formed even if a new collector forgets the invariant.
    """
    if not ranges:
        return ranges

    # Sort by start ascending, end descending so that parents come before
    # children and equal-start ranges with larger spans come first.
    ordered = sorted(ranges, key=lambda r: (r.start_line, -r.end_line))

    # Walk with a stack of open ranges; when a new range overlaps the top of
    # the stack without being fully contained, shrink it so it nests properly.
    stack: list[types.FoldingRange] = []
    result: list[types.FoldingRange] = []
    for r in ordered:
        # A stack entry is closed when it ends strictly before r starts OR
        # exactly at r.start — in the latter case r is r's sibling, not a
        # child, even though LSP spec permits either interpretation.
        while stack and stack[-1].end_line <= r.start_line:
            stack.pop()
        if stack and stack[-1].end_line < r.end_line:
            # r extends past its would-be parent — trim it.
            new_end = stack[-1].end_line
            if new_end > r.start_line:
                r = types.FoldingRange(
                    start_line=r.start_line,
                    end_line=new_end,
                    kind=r.kind,
                )
            else:
                continue
        result.append(r)
        stack.append(r)
    return result


def get_folding_ranges(
    source: str,
    analysis: AnalysisResult | None = None,
    *,
    lines: list[str] | None = None,
) -> list[types.FoldingRange]:
    """Return folding ranges for a Tcl source file."""
    if analysis is None:
        analysis = analyse(source)

    ranges: list[types.FoldingRange] = []
    seen: set[tuple[int, int]] = set()

    _collect_scope_folds(analysis.global_scope, seen, ranges, source)
    _collect_comment_folds(source, seen, ranges, lines=lines)
    _collect_body_folds(source, seen, ranges, original_source=source)

    return _normalise_overlaps(ranges)
