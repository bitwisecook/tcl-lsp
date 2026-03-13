"""Syntax checks (E1xx)."""

from __future__ import annotations

from ...parsing.tokens import Token, TokenType
from ..semantic_model import CodeFix, Diagnostic, Range, Severity
from ._helpers import (
    _find_bracket_insertion_point,
    _stray_brace_fix,
)

# E100: Unmatched close bracket ']'


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
    """
    from ...parsing.tokens import SourcePosition

    diagnostics: list[Diagnostic] = []
    for tok_idx, tok in enumerate(all_tokens):
        if tok.type is not TokenType.ESC:
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
    """
    diagnostics: list[Diagnostic] = []
    for tok in all_tokens:
        if tok.type is TokenType.ESC and tok.text == "}":
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
