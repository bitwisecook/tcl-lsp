"""Position-based lookup helpers that need the compiler's lexer / segmenter.

Lives in `compiler/` rather than `shared/` because the lookups consume
`SegmentedCommand` and `segment_commands` from `compiler.parsing` — a
dependency that would otherwise pull `compiler/` into a leaf `shared/`.
The plain-position predicate (`position_in_range`) and the
buffer-backed offset converter (`offset_at_position`) stay in
`shared.position`.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from compiler.parsing.command_segmenter import segment_commands
from shared.diagnostic import Range
from shared.position import position_in_range
from shared.tokens import Token

if TYPE_CHECKING:
    from compiler.parsing.command_segmenter import SegmentedCommand


def find_command_at_position(
    source: str,
    line: int,
    character: int,
    body_token: Token | None = None,
) -> SegmentedCommand | None:
    """Find the :class:`SegmentedCommand` whose range contains the position."""
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
