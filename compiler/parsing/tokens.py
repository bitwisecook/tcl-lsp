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
