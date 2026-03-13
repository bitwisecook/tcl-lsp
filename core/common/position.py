"""Shared position-based lookup helpers for editor features.

These helpers centralise common "find the thing at position" logic that
was previously duplicated across feature modules (code_actions,
selection_range, etc.).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..analysis.semantic_model import Range
from ..parsing.tokens import Token
from .source_map import SourceMap

if TYPE_CHECKING:
    from ..parsing.command_segmenter import SegmentedCommand


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


def find_command_at_position(
    source: str,
    line: int,
    character: int,
    body_token: Token | None = None,
) -> SegmentedCommand | None:
    """Find the :class:`SegmentedCommand` whose range contains the position."""
    from ..parsing.command_segmenter import segment_commands

    for cmd in segment_commands(source, body_token):
        if position_in_range(line, character, cmd.range):
            return cmd
    return None


def find_token_in_command(
    cmd: SegmentedCommand,
    line: int,
    character: int,
) -> Token | None:
    """Find the argument token in *cmd* that contains the position."""
    for tok in cmd.argv:
        if position_in_range(line, character, Range(start=tok.start, end=tok.end)):
            return tok
    return None


def offset_at_position(source: str, line: int, character: int) -> int:
    """Convert an LSP (*line*, *character*) position to a byte offset."""
    return SourceMap(source).position_to_offset(line, character)
