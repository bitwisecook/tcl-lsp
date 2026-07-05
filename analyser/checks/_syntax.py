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

"""Syntax checks (E1xx)."""

from __future__ import annotations

from compiler.parsing.token_positions import classify_quoted_contexts
from shared.codes import diag
from shared.tokens import Token, TokenType

from ..semantic_model import CodeFix, Diagnostic, Range, Severity
from ._helpers import (
    _find_bracket_insertion_point,
    _stray_brace_fix,
)

# E100: Unmatched close bracket ']'


@diag("E100", "Syntax error — unclosed brace.", section="error", internal=True)
def check_unmatched_close_bracket(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """E100: Warn when an ESC token contains an unmatched ']'.

    A bare ']' outside of command substitution almost always indicates a
    missing '[' (e.g. after a bad edit removed it).  When a known command
    name precedes the ']', a CodeFix is attached suggesting where to
    insert the missing '['.

    A ']' inside a double-quoted string (e.g. ``puts "foo ]"``) is just a
    literal character and must not trigger this diagnostic.  Quoted-context
    detection is centralised in :func:`classify_quoted_contexts` —
    see its docstring for the underlying lexer contract.
    """
    from shared.tokens import SourcePosition

    diagnostics: list[Diagnostic] = []
    in_quoted = classify_quoted_contexts(all_tokens)
    for tok_idx, tok in enumerate(all_tokens):
        if tok.type is not TokenType.ESC:
            continue
        # Skip ESC tokens that are part of a quoted word — a ']' inside
        # a "..." string is a literal character, not an unmatched bracket.
        if in_quoted[tok_idx]:
            continue
        # Find the first unescaped ']' in the token text.
        idx = -1
        search_from = 0
        while True:
            pos = tok.text.find("]", search_from)
            if pos < 0:
                break
            # A ']' preceded by a backslash is an escape, not a bracket.
            if pos > 0 and tok.text[pos - 1] == "\\":
                search_from = pos + 1
                continue
            idx = pos
            break
        if idx < 0:
            continue
        # Compute position of the ']' within the token.
        bracket_pos = SourcePosition(
            line=tok.start.line,
            character=tok.start.character + idx,
            offset=tok.start.offset + idx,
        )
        fixes: tuple[CodeFix, ...] = ()
        insert_pos = _find_bracket_insertion_point(
            cmd_name,
            all_tokens,
            arg_tokens,
            tok_idx,
        )
        if insert_pos is not None:
            # Zero-width insertion: end must be one character before start
            # so that the +1 inclusive->exclusive conversion in code_actions.py
            # produces start==end in LSP coordinates.
            insert_end = SourcePosition(
                line=insert_pos.line,
                character=insert_pos.character - 1,
                offset=insert_pos.offset - 1,
            )
            fixes = (
                CodeFix(
                    range=Range(start=insert_pos, end=insert_end),
                    new_text="[",
                    description="Insert missing '['",
                ),
            )
        # When an insertion point was found, highlight from the insertion
        # point to the ']'.  When not, fall back to the command start so the
        # user still sees the full context of the problem.
        if insert_pos is not None:
            diag_start = insert_pos
        elif all_tokens:
            diag_start = all_tokens[0].start
        else:
            diag_start = bracket_pos
        diagnostics.append(
            Diagnostic(
                range=Range(start=diag_start, end=bracket_pos),
                message="Unmatched ']' \u2014 missing opening '['?",
                severity=Severity.ERROR,
                code="E100",
                fixes=fixes,
            )
        )
    return diagnostics


# E102: Unmatched close brace '}'


@diag("E102", "Syntax error — unclosed quote.", section="error", internal=True)
def check_unmatched_close_brace(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """E102: Warn when an ESC token is a bare ``}``.

    A bare ``}`` outside of a brace-quoted string almost always indicates a
    missing ``{`` (e.g. a switch body that closed an enclosing scope).
    Attaches a CodeFix to remove the stray ``}`` line.

    A ``}`` inside a double-quoted string (e.g. ``append result "}"``) is
    just a string character and must not trigger this diagnostic — quoted
    context is determined by :func:`classify_quoted_contexts`.
    """
    diagnostics: list[Diagnostic] = []
    in_quoted = classify_quoted_contexts(all_tokens)
    for i, tok in enumerate(all_tokens):
        if tok.type is TokenType.ESC and tok.text == "}" and not in_quoted[i]:
            fix = _stray_brace_fix(tok, source)
            diagnostics.append(
                Diagnostic(
                    range=Range(start=tok.start, end=tok.end),
                    message="Unmatched '}' \u2014 missing opening '{'?",
                    severity=Severity.ERROR,
                    code="E102",
                    fixes=(fix,) if fix else (),
                )
            )
    return diagnostics
