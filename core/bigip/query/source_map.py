"""Offset → ``(line, column)`` lookup for the source-text errors.

The bigger document buffer in :mod:`core.parsing.document_buffer` does
the same job for the full LSP pipeline, but it pulls in tokeniser
state we do not need here.  This is the trimmed-down version: build a
prefix-sum of newline offsets once, binary-search per lookup.
"""

from __future__ import annotations

import bisect
from dataclasses import dataclass


@dataclass
class SourceMap:
    """Map a byte offset back to a one-based ``(line, column)``."""

    source: str
    _line_starts: list[int]

    @classmethod
    def build(cls, source: str) -> "SourceMap":
        starts = [0]
        for i, ch in enumerate(source):
            if ch == "\n":
                starts.append(i + 1)
        return cls(source=source, _line_starts=starts)

    def line_col(self, offset: int) -> tuple[int, int]:
        if offset < 0:
            offset = 0
        if offset > len(self.source):
            offset = len(self.source)
        # Find the largest line-start <= offset.
        idx = bisect.bisect_right(self._line_starts, offset) - 1
        if idx < 0:
            idx = 0
        line = idx + 1
        col = offset - self._line_starts[idx] + 1
        return line, col

    def line_text(self, line: int) -> str:
        if line < 1 or line > len(self._line_starts):
            return ""
        start = self._line_starts[line - 1]
        end = self._line_starts[line] if line < len(self._line_starts) else len(self.source)
        return self.source[start:end].rstrip("\n")
