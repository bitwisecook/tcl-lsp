"""Helpers for computing safe replacement spans for refactor edits."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..commands.registry.runtime import iter_body_arguments
from ..common.source_map import SourceMap
from ..parsing.command_segmenter import segment_commands
from ..parsing.tokens import TokenType

if TYPE_CHECKING:
    from ..parsing.command_segmenter import SegmentedCommand
    from ..parsing.tokens import Token

_CLOSER_BY_OPENER = {
    '"': '"',
    "{": "}",
    "[": "]",
}


def token_end_offset(source: str, token: Token) -> int:
    """Return an exclusive end offset for *token* in *source*.

    Lexer token end offsets are inclusive and omit closing delimiters for
    quoted/brace/bracket words, so this widens the span when needed.
    """
    end = token.end.offset + 1
    if token.type in (TokenType.STR, TokenType.CMD, TokenType.ESC):
        start = token.start.offset
        if 0 <= start < len(source):
            closer = _CLOSER_BY_OPENER.get(source[start])
            if closer and end < len(source) and source[end] == closer:
                end += 1
    return max(0, min(end, len(source)))


def command_span_offsets(source: str, cmd: SegmentedCommand) -> tuple[int, int]:
    """Return ``(start, end)`` offsets that cover the full command text."""
    if not cmd.all_tokens:
        return (0, 0)
    start = cmd.all_tokens[0].start.offset
    end = token_end_offset(source, cmd.all_tokens[-1])
    return (max(0, start), max(0, end))


def offsets_to_position(
    source: str,
    start: int,
    end: int,
) -> tuple[int, int, int, int]:
    """Convert offsets to ``(start_line, start_char, end_line, end_char)``."""
    source_map = SourceMap(source)
    start_pos = source_map.offset_to_position(start)
    end_pos = source_map.offset_to_position(end)
    return (
        start_pos.line,
        start_pos.character,
        end_pos.line,
        end_pos.character,
    )


def command_replacement_range(
    source: str,
    cmd: SegmentedCommand,
) -> tuple[int, int, int, int]:
    """Return a replacement range that covers the full command text."""
    start, end = command_span_offsets(source, cmd)
    return offsets_to_position(source, start, end)


def find_command_at(
    source: str,
    line: int,
    character: int,
    *,
    predicate: str | None = None,
    _body_token: Token | None = None,
    _depth: int = 0,
) -> SegmentedCommand | None:
    """Recursively find the innermost command at *(line, character)*.

    Unlike a flat ``segment_commands`` scan, this walks into body
    arguments (``proc``, ``when``, ``if``, ``while``, ``foreach``, etc.)
    so that refactorings work inside nested bodies.

    If *predicate* is given, only return a command whose first word
    matches that name.
    """
    if _depth > 20:
        return None
    for cmd in segment_commands(source, _body_token):
        if not (cmd.range.start.line <= line <= cmd.range.end.line):
            continue
        # Recurse into body arguments first (innermost match wins).
        if cmd.texts:
            for body in iter_body_arguments(cmd.name, cmd.args, cmd.arg_tokens):
                inner = find_command_at(
                    body.text,
                    line,
                    character,
                    predicate=predicate,
                    _body_token=body.token,
                    _depth=_depth + 1,
                )
                if inner is not None:
                    return inner
        # Check this command itself.
        if predicate is None or (cmd.texts and cmd.texts[0] == predicate):
            return cmd
    return None


def walk_all_commands(
    source: str,
    *,
    _body_token: Token | None = None,
    _depth: int = 0,
) -> list[SegmentedCommand]:
    """Recursively yield all commands, including those inside body arguments."""
    if _depth > 20:
        return []
    result: list[SegmentedCommand] = []
    for cmd in segment_commands(source, _body_token):
        result.append(cmd)
        if cmd.texts:
            for body in iter_body_arguments(cmd.name, cmd.args, cmd.arg_tokens):
                result.extend(
                    walk_all_commands(
                        body.text,
                        _body_token=body.token,
                        _depth=_depth + 1,
                    )
                )
    return result
