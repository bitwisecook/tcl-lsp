"""Folding range provider -- proc bodies, namespaces, comment blocks, control structures."""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, Scope
from core.commands.registry.runtime import iter_body_arguments
from core.parsing.command_segmenter import segment_commands
from core.parsing.tokens import Token, TokenType


def _collect_scope_folds(
    scope: Scope,
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
) -> None:
    """Emit folding ranges from the scope tree (procs and namespaces)."""
    for proc_def in scope.procs.values():
        br = proc_def.body_range
        if br.start.line < br.end.line:
            key = (br.start.line, br.end.line)
            if key not in seen:
                seen.add(key)
                ranges.append(
                    types.FoldingRange(
                        start_line=br.start.line,
                        end_line=br.end.line,
                        kind=types.FoldingRangeKind.Region,
                    )
                )

    for child in scope.children:
        if child.kind == "namespace" and child.body_range is not None:
            br = child.body_range
            if br.start.line < br.end.line:
                key = (br.start.line, br.end.line)
                if key not in seen:
                    seen.add(key)
                    ranges.append(
                        types.FoldingRange(
                            start_line=br.start.line,
                            end_line=br.end.line,
                            kind=types.FoldingRangeKind.Region,
                        )
                    )
        _collect_scope_folds(child, seen, ranges)


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
                key = (body.token.start.line, body.token.end.line)
                if key not in seen:
                    seen.add(key)
                    ranges.append(
                        types.FoldingRange(
                            start_line=body.token.start.line,
                            end_line=body.token.end.line,
                            kind=types.FoldingRangeKind.Region,
                        )
                    )
                # Recurse into the body
                _collect_body_folds(
                    body.text,
                    seen,
                    ranges,
                    body_token=body.token,
                    depth=depth + 1,
                )


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

    _collect_scope_folds(analysis.global_scope, seen, ranges)
    _collect_comment_folds(source, seen, ranges, lines=lines)
    _collect_body_folds(source, seen, ranges)

    return ranges
