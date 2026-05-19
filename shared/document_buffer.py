"""Shared per-document position infrastructure.

``DocumentBuffer`` is the single source of truth for source text, version,
and line-start metadata.  Every LSP feature handler should use it instead
of recomputing ``source.split("\\n")`` or constructing ad-hoc ``SourceMap``
instances.
"""

from __future__ import annotations

from bisect import bisect_right
from dataclasses import dataclass, field

from compiler.parsing.tokens import SourcePosition
from core.analysis.semantic_model import Range


def compute_line_starts(source: str) -> tuple[int, ...]:
    """Build a line-starts index from scratch — O(len(source))."""
    starts = [0]
    for i, ch in enumerate(source):
        if ch == "\n":
            starts.append(i + 1)
    return tuple(starts)


@dataclass(slots=True)
class DocumentBuffer:
    """Shared per-document position infrastructure.

    Replaces scattered ``source.split("\\n")``, ``SourceMap(source)``,
    ``_chunk_line_range(source, chunk)``, and ``position_from_relative()``
    calls with a single cached object.
    """

    source: str
    version: int | None
    line_starts: tuple[int, ...]

    # Lazily cached derived data.
    _lines: list[str] | None = field(default=None, repr=False)

    # Constructors

    @classmethod
    def from_source(
        cls,
        source: str,
        version: int | None = None,
    ) -> DocumentBuffer:
        """Create a buffer with a freshly computed line-starts index."""
        return cls(
            source=source,
            version=version,
            line_starts=compute_line_starts(source),
        )

    # Cached properties

    @property
    def lines(self) -> list[str]:
        """Source split by ``\\n``, cached for the buffer's lifetime."""
        if self._lines is None:
            self._lines = self.source.split("\n")
        return self._lines

    # Position conversion

    def offset_to_position(self, offset: int) -> SourcePosition:
        """O(log n) offset → (line, character, offset) via bisect."""
        safe = max(0, min(offset, len(self.source)))
        line = bisect_right(self.line_starts, safe) - 1
        line = max(0, line)
        col = safe - self.line_starts[line]
        return SourcePosition(line=line, character=col, offset=safe)

    def position_to_offset(self, line: int, character: int) -> int:
        """O(1) (line, character) → offset, with clamping."""
        if not self.line_starts:
            return 0
        safe_line = max(0, min(line, len(self.line_starts) - 1))
        line_start = self.line_starts[safe_line]
        # Clamp character to line length.
        if safe_line + 1 < len(self.line_starts):
            line_end = self.line_starts[safe_line + 1] - 1  # exclude '\n'
        else:
            line_end = len(self.source)
        line_length = max(0, line_end - line_start)
        safe_char = max(0, min(character, line_length))
        return line_start + safe_char

    def offset_to_line_col(self, offset: int) -> tuple[int, int]:
        """O(log n) offset → (line, col) tuple (no SourcePosition alloc)."""
        safe = max(0, min(offset, len(self.source)))
        line = bisect_right(self.line_starts, safe) - 1
        line = max(0, line)
        return line, safe - self.line_starts[line]

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

    def chunk_line_range(
        self,
        start_offset: int,
        end_offset: int,
    ) -> tuple[int, int, int, int]:
        """O(log n) replacement for the O(offset) ``_chunk_line_range()``.

        Returns ``(start_line, start_col, end_line, end_col)``.
        """
        sl, sc = self.offset_to_line_col(start_offset)
        el, ec = self.offset_to_line_col(end_offset)
        return sl, sc, el, ec
