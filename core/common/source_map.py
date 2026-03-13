"""Shared source text offset/position conversion helpers."""

from __future__ import annotations

from bisect import bisect_right
from dataclasses import dataclass, field

from ..analysis.semantic_model import Range
from ..parsing.tokens import SourcePosition


def offset_to_line_col(
    source: str,
    line_starts: tuple[int, ...] | list[int],
    offset: int,
) -> tuple[int, int]:
    """Convert a source offset to (line, column), clamping to bounds.

    Standalone function for callers that already have a ``line_starts``
    array and don't want to construct a full :class:`SourceMap`.
    """
    safe_offset = max(0, min(offset, len(source)))
    line = bisect_right(line_starts, safe_offset) - 1
    line = max(0, line)
    col = safe_offset - line_starts[line]
    return line, col


@dataclass(frozen=True, slots=True)
class SourceMap:
    """Efficient offset/position mapping over a source string."""

    source: str
    _line_starts: tuple[int, ...] = field(init=False, repr=False)

    def __post_init__(self) -> None:
        line_starts = [0]
        for idx, ch in enumerate(self.source):
            if ch == "\n":
                line_starts.append(idx + 1)
        object.__setattr__(self, "_line_starts", tuple(line_starts))

    def position_to_offset(self, line: int, character: int) -> int:
        """Convert (line, character) to a clamped source offset."""
        if not self._line_starts:
            return 0

        safe_line = max(0, min(line, len(self._line_starts) - 1))
        line_start = self._line_starts[safe_line]
        if safe_line + 1 < len(self._line_starts):
            line_end = self._line_starts[safe_line + 1] - 1
        else:
            line_end = len(self.source)
        line_length = max(0, line_end - line_start)
        safe_char = max(0, min(character, line_length))
        return line_start + safe_char

    def offset_to_position(self, offset: int) -> SourcePosition:
        """Convert an offset to (line, character, offset), clamping to source bounds."""
        safe_offset = max(0, min(offset, len(self.source)))
        line = bisect_right(self._line_starts, safe_offset) - 1
        line = max(0, line)
        line_start = self._line_starts[line]
        return SourcePosition(
            line=line,
            character=safe_offset - line_start,
            offset=safe_offset,
        )

    def range_from_offsets(self, start: int, end_inclusive: int) -> Range:
        """Build a Range from inclusive source offsets."""
        if not self.source:
            pos = SourcePosition(line=0, character=0, offset=0)
            return Range(start=pos, end=pos)

        max_end = len(self.source) - 1
        safe_start = max(0, min(start, max_end))
        safe_end = max(0, min(end_inclusive, max_end))
        if safe_end < safe_start:
            safe_end = safe_start

        return Range(
            start=self.offset_to_position(safe_start),
            end=self.offset_to_position(safe_end),
        )
