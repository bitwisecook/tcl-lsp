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
