"""Shared position predicates and offset conversion.

`position_in_range` and `offset_at_position` are leaf-safe helpers that
only depend on `shared.diagnostic.Range` and `shared.document_buffer`.
The richer "find the command/token at a position" helpers — which need
the compiler's lexer/segmenter — live in `compiler.position_lookup`.
"""

from __future__ import annotations

from shared.diagnostic import Range

from .document_buffer import DocumentBuffer


def position_in_range(line: int, character: int, r: Range) -> bool:
    """Check if (*line*, *character*) falls within an analysis *Range*.

    The range end is treated as inclusive (matching the semantic model
    convention where ``end.character`` is the index of the last character).
    """
    if line < r.start.line or line > r.end.line:
        return False
    if line == r.start.line and character < r.start.character:
        return False
    if line == r.end.line and character > r.end.character:
        return False
    return True


def offset_at_position(source: str, line: int, character: int) -> int:
    """Convert an LSP (*line*, *character*) position to a byte offset."""
    return DocumentBuffer.from_source(source).position_to_offset(line, character)
