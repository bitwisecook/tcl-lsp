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

"""Helpers for computing safe replacement spans for refactor edits."""

from __future__ import annotations

from typing import TYPE_CHECKING

from compiler.parsing.command_segmenter import segment_commands
from compiler.registry.runtime import iter_body_arguments
from shared.document_buffer import DocumentBuffer
from shared.ranges import word_closer_offset

if TYPE_CHECKING:
    from compiler.parsing.command_segmenter import SegmentedCommand
    from shared.tokens import Token


def token_end_offset(source: str, token: Token) -> int:
    """Return an exclusive end offset for *token* in *source*.

    Lexer token ends are inclusive and omit the closing delimiter for
    quoted/brace/bracket words, so a whole-word span must cover the closer.
    The closer is located via :func:`shared.ranges.word_closer_offset` — the
    single authoritative accessor — rather than re-derived here, so an empty
    ``{}`` / ``[]`` / ``""`` is handled correctly (its inclusive end already
    sits on the closer, so ``end + 1`` would overshoot by one).
    """
    closer = word_closer_offset(token, source)
    end = closer + 1 if closer is not None else token.end.offset + 1
    return max(0, min(end, len(source)))


def command_span_offsets(source: str, cmd: SegmentedCommand) -> tuple[int, int]:
    """Return ``(start, end)`` offsets that cover the full command text.

    Trusts the segmenter's authoritative ``cmd.range`` — which now covers the
    final word's closing delimiter for braces, brackets, *and* quoted words, and
    never overshoots an empty ``{}`` / ``""`` (the segmenter source-verifies the
    closer).  The exclusive end is the inclusive range end plus one.
    """
    r = cmd.range
    return (max(0, r.start.offset), min(r.end.offset + 1, len(source)))


def offsets_to_position(
    source: str,
    start: int,
    end: int,
) -> tuple[int, int, int, int]:
    """Convert offsets to ``(start_line, start_char, end_line, end_char)``."""
    buf = DocumentBuffer.from_source(source)
    start_pos = buf.offset_to_position(start)
    end_pos = buf.offset_to_position(end)
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
