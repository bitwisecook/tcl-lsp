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

"""Source ranges and diagnostic primitives.

`Range`, `Severity`, `CodeFix`, and `Diagnostic` are used everywhere —
from the parser surfacing recovery errors, through the analyser
producing checks, into the compiler's pass-internal diagnostics, and
out to the LSP server's `textDocument/publishDiagnostics`. They are
pure data types with no dependencies on any concern beyond `shared.tokens`,
which keeps `shared/` a graph leaf.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto

from shared.tokens import SourcePosition


@dataclass(frozen=True, slots=True)
class Range:
    """A span in source text."""

    start: SourcePosition
    end: SourcePosition

    @classmethod
    def zero(cls) -> Range:
        """Return a zero-length range at position (0, 0, 0)."""
        pos = SourcePosition(line=0, character=0, offset=0)
        return cls(start=pos, end=pos)


class Severity(Enum):
    ERROR = auto()
    WARNING = auto()
    INFO = auto()
    HINT = auto()


@dataclass(frozen=True, slots=True)
class CodeFix:
    """A suggested fix for a diagnostic -- maps to an LSP TextEdit."""

    range: Range
    new_text: str
    description: str = ""


@dataclass(frozen=True, slots=True)
class Diagnostic:
    """A single error, warning, or info message attached to a source range."""

    range: Range
    message: str
    severity: Severity = Severity.ERROR
    code: str = ""
    fixes: tuple[CodeFix, ...] = ()
    related_ranges: tuple[tuple[Range, str], ...] = ()
