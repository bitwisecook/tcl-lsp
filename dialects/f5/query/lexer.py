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

"""Tokeniser for the query DSL.

The grammar is small enough that a hand-rolled scanner is shorter than
the regex it would replace.  Tokens carry their offset so parse errors
can underline the offending character.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto

from .errors import LexError


class TokenKind(Enum):
    DOT = auto()
    LBRACKET = auto()
    RBRACKET = auto()
    LPAREN = auto()
    RPAREN = auto()
    LBRACE = auto()
    RBRACE = auto()
    COMMA = auto()
    COLON = auto()
    SEMICOLON = auto()
    PIPE = auto()  # |
    PIPE_EQ = auto()  # |=
    PLUS = auto()
    PLUS_EQ = auto()
    MINUS = auto()
    MINUS_EQ = auto()
    STAR = auto()
    SLASH = auto()
    EQ = auto()  # =
    EQEQ = auto()  # ==
    NEQ = auto()  # !=
    LT = auto()
    LE = auto()
    GT = auto()
    GE = auto()
    STRING = auto()
    NUMBER = auto()
    IDENT = auto()  # bareword (may contain '-' after the first char)
    AND = auto()
    OR = auto()
    NOT = auto()
    TRUE = auto()
    FALSE = auto()
    NULL = auto()
    AS = auto()  # `expr as $name | body` — jq's let-binding form
    IF = auto()  # `if COND then BODY [elif ...] [else ...] end`
    THEN = auto()
    ELIF = auto()
    ELSE = auto()
    END = auto()
    DOLLAR_IDENT = auto()  # $name — variable reference to a named source
    QUESTION = auto()  # `?` — optional-marker suffix on a path step
    EOF = auto()


@dataclass(frozen=True, slots=True)
class Token:
    kind: TokenKind
    text: str
    offset: int

    # Convenience for string/number literals: the parsed value.
    value: object = None


_KEYWORDS = {
    "and": TokenKind.AND,
    "or": TokenKind.OR,
    "not": TokenKind.NOT,
    "true": TokenKind.TRUE,
    "false": TokenKind.FALSE,
    "null": TokenKind.NULL,
    "as": TokenKind.AS,
    "if": TokenKind.IF,
    "then": TokenKind.THEN,
    "elif": TokenKind.ELIF,
    "else": TokenKind.ELSE,
    "end": TokenKind.END,
}


def _is_ident_start(ch: str) -> bool:
    return ch.isalpha() or ch == "_"


def _is_ident_cont(ch: str) -> bool:
    # Hyphens are valid mid-identifier so that TMSH-spelt names like
    # ``data-group`` and ``source-address-translation`` lex as a single
    # token.  A leading hyphen is *not* an identifier — it is the minus
    # operator.
    return ch.isalnum() or ch in "_-"


def tokenise(source: str) -> list[Token]:
    out: list[Token] = []
    i = 0
    n = len(source)

    while i < n:
        ch = source[i]

        if ch.isspace():
            i += 1
            continue

        # Comments: ``# ...`` to end of line.  Useful for multi-line
        # queries loaded with ``-f``.
        if ch == "#":
            while i < n and source[i] != "\n":
                i += 1
            continue

        start = i

        # Single-character punctuation that has no two-char form.
        simple = {
            ".": TokenKind.DOT,
            "[": TokenKind.LBRACKET,
            "]": TokenKind.RBRACKET,
            "(": TokenKind.LPAREN,
            ")": TokenKind.RPAREN,
            "{": TokenKind.LBRACE,
            "}": TokenKind.RBRACE,
            ",": TokenKind.COMMA,
            ":": TokenKind.COLON,
            ";": TokenKind.SEMICOLON,
            "*": TokenKind.STAR,
            "/": TokenKind.SLASH,
            "?": TokenKind.QUESTION,
        }
        if ch in simple:
            out.append(Token(simple[ch], ch, start))
            i += 1
            continue

        # Two-char-or-one operators.
        if ch == "|":
            if i + 1 < n and source[i + 1] == "=":
                out.append(Token(TokenKind.PIPE_EQ, "|=", start))
                i += 2
            else:
                out.append(Token(TokenKind.PIPE, "|", start))
                i += 1
            continue

        if ch == "+":
            if i + 1 < n and source[i + 1] == "=":
                out.append(Token(TokenKind.PLUS_EQ, "+=", start))
                i += 2
            else:
                out.append(Token(TokenKind.PLUS, "+", start))
                i += 1
            continue

        if ch == "-":
            if i + 1 < n and source[i + 1] == "=":
                out.append(Token(TokenKind.MINUS_EQ, "-=", start))
                i += 2
            else:
                out.append(Token(TokenKind.MINUS, "-", start))
                i += 1
            continue

        if ch == "=":
            if i + 1 < n and source[i + 1] == "=":
                out.append(Token(TokenKind.EQEQ, "==", start))
                i += 2
            else:
                out.append(Token(TokenKind.EQ, "=", start))
                i += 1
            continue

        if ch == "!":
            if i + 1 < n and source[i + 1] == "=":
                out.append(Token(TokenKind.NEQ, "!=", start))
                i += 2
                continue
            raise LexError("unexpected '!' (did you mean '!='?)", start)

        if ch == "<":
            if i + 1 < n and source[i + 1] == "=":
                out.append(Token(TokenKind.LE, "<=", start))
                i += 2
            else:
                out.append(Token(TokenKind.LT, "<", start))
                i += 1
            continue

        if ch == ">":
            if i + 1 < n and source[i + 1] == "=":
                out.append(Token(TokenKind.GE, ">=", start))
                i += 2
            else:
                out.append(Token(TokenKind.GT, ">", start))
                i += 1
            continue

        # Variable references: ``$name``.  Used to address one specific
        # source when more than one config was loaded — e.g.
        # ``$gtm.gtm.wideip[]`` reads from the source bound under
        # ``gtm``.  The name follows the same identifier rules as a
        # bareword (hyphens allowed mid-name, underscores anywhere).
        if ch == "$":
            j = i + 1
            if j >= n or not _is_ident_start(source[j]):
                raise LexError(
                    "expected an identifier after '$' (e.g. $ltm)",
                    start,
                )
            j += 1
            while j < n and _is_ident_cont(source[j]):
                j += 1
            name = source[i + 1 : j]
            out.append(Token(TokenKind.DOLLAR_IDENT, source[i:j], start, value=name))
            i = j
            continue

        # Strings: double-quoted, with the usual backslash escapes.
        # A leading ``~`` is *not* a token kind of its own — the parser
        # checks for it when the string appears as the body of a
        # subscript ``[ ... ]``.  That keeps ``sub(.x, "~lit", "y")``
        # working: the literal happens to start with ``~``, but it is
        # not a regex subscript and is passed through unchanged.
        if ch == '"':
            j = i + 1
            buf: list[str] = []
            while j < n and source[j] != '"':
                if source[j] == "\\" and j + 1 < n:
                    esc = source[j + 1]
                    buf.append(
                        {
                            "n": "\n",
                            "t": "\t",
                            "r": "\r",
                            '"': '"',
                            "\\": "\\",
                        }.get(esc, esc)
                    )
                    j += 2
                    continue
                buf.append(source[j])
                j += 1
            if j >= n:
                raise LexError("unterminated string literal", start)
            text = "".join(buf)
            j += 1  # consume closing quote
            out.append(Token(TokenKind.STRING, source[start:j], start, value=text))
            i = j
            continue

        # Numbers.  Integer or float (no scientific notation in v1).
        if ch.isdigit():
            j = i
            while j < n and source[j].isdigit():
                j += 1
            if j < n and source[j] == "." and j + 1 < n and source[j + 1].isdigit():
                j += 1
                while j < n and source[j].isdigit():
                    j += 1
                value: object = float(source[i:j])
            else:
                value = int(source[i:j])
            out.append(Token(TokenKind.NUMBER, source[i:j], start, value=value))
            i = j
            continue

        # Identifiers / keywords.
        if _is_ident_start(ch):
            j = i + 1
            while j < n and _is_ident_cont(source[j]):
                j += 1
            text = source[i:j]
            kw = _KEYWORDS.get(text)
            if kw is not None:
                value = {
                    TokenKind.TRUE: True,
                    TokenKind.FALSE: False,
                    TokenKind.NULL: None,
                }.get(kw)
                out.append(Token(kw, text, start, value=value))
            else:
                out.append(Token(TokenKind.IDENT, text, start, value=text))
            i = j
            continue

        raise LexError(f"unexpected character {ch!r}", start)

    out.append(Token(TokenKind.EOF, "", n))
    return out
