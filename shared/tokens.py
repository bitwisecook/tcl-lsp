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

"""Token types and position-aware token dataclass for Tcl lexing."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto


class TokenType(Enum):
    """Types of tokens produced by the Tcl lexer."""

    ESC = auto()  # plain string fragment (possibly with escape sequences)
    STR = auto()  # braced string {…}
    CMD = auto()  # command substitution [… ]
    VAR = auto()  # variable substitution $name
    SEP = auto()  # whitespace separator
    EOL = auto()  # end-of-line / semicolon
    EOF = auto()  # end-of-input
    COMMENT = auto()  # comment (# to end of line)
    EXPAND = auto()  # {*} expansion prefix


@dataclass(frozen=True, slots=True)
class SourcePosition:
    """A position in source text (0-based line and character)."""

    line: int  # 0-based line number
    character: int  # 0-based column (UTF-16 code units per LSP spec)
    offset: int  # byte offset into the source string


@dataclass(frozen=True, slots=True)
class Token:
    """A token with its type, text, and source location."""

    type: TokenType
    text: str
    start: SourcePosition
    end: SourcePosition
    in_quote: bool = False
