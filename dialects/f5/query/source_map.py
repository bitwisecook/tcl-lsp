# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Offset → ``(line, column)`` lookup for the source-text errors.

The bigger document buffer in :mod:`compiler.parsing.document_buffer` does
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
