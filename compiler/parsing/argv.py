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

"""Helpers for argv token span reconstruction."""

from __future__ import annotations

from shared.tokens import Token


def widen_argv_tokens_to_word_spans(argv: list[Token], all_tokens: list[Token]) -> list[Token]:
    """Return argv tokens widened to each Tcl word's full token span."""
    if not argv or not all_tokens:
        return argv

    widened: list[Token] = []
    arg_i = 0
    word_start = argv[arg_i]
    word_end = word_start
    next_start = argv[arg_i + 1].start.offset if arg_i + 1 < len(argv) else None

    for tok in all_tokens:
        while next_start is not None and tok.start.offset >= next_start:
            widened.append(
                Token(
                    type=word_start.type,
                    text=word_start.text,
                    start=word_start.start,
                    end=word_end.end,
                )
            )
            arg_i += 1
            if arg_i >= len(argv):
                return widened
            word_start = argv[arg_i]
            word_end = word_start
            next_start = argv[arg_i + 1].start.offset if arg_i + 1 < len(argv) else None
        word_end = tok

    widened.append(
        Token(
            type=word_start.type,
            text=word_start.text,
            start=word_start.start,
            end=word_end.end,
        )
    )
    return widened
