"""Shared command text shape matchers."""

from __future__ import annotations

from .lexer import TclLexer
from .tokens import Token, TokenType


def _word_piece(tok: Token) -> str:
    """Return source-faithful text for one token when rebuilding a Tcl word."""
    if tok.type is TokenType.VAR:
        is_braced = (tok.end.offset - tok.start.offset) > len(tok.text)
        if "}" in tok.text:
            return "$" + tok.text
        if is_braced:
            return f"${{{tok.text}}}"
        return "$" + tok.text
    if tok.type is TokenType.CMD:
        return f"[{tok.text}]"
    return tok.text


def extract_single_expr_argument(
    cmd_text: str,
    *,
    expr_aliases: frozenset[str] | None = None,
) -> str | None:
    """Return expr argument if command text is exactly: ``expr <one-arg>``.

    When *expr_aliases* is provided, command names in that set are also
    accepted as equivalent to ``expr`` (e.g. for ``interp alias`` aliases).

    Returns ``None`` for multi-command bodies (``expr X; cmd2``) so the
    caller falls back to a generic value path — without the trailing-EOL
    check the lexer breaks at the first ``;`` and we'd silently discard
    the second command, lowering ``set r [expr {…};expr {…}]`` as if
    only the first ``expr`` existed.
    """
    lexer = TclLexer(cmd_text)
    argv_texts: list[str] = []
    argv_single: list[bool] = []
    prev_type = TokenType.EOL
    saw_eol = False

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is TokenType.EOL:
            # ``;`` / ``\n`` ends a command.  If anything follows, the
            # body holds multiple commands — bail out so the caller
            # routes through the generic eval-fallback path.
            saw_eol = True
            prev_type = tok.type
            continue
        if saw_eol:
            # Real word AFTER the first command ended → this is a
            # multi-command body, not a single ``expr ARG``.
            return None
        if tok.type in (TokenType.SEP, TokenType.COMMENT):
            prev_type = tok.type
            continue

        piece = _word_piece(tok)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv_texts.append(piece)
            argv_single.append(True)
        else:
            argv_texts[-1] += piece
            argv_single[-1] = False
        prev_type = tok.type

    if len(argv_texts) != 2:
        return None
    cmd_name = argv_texts[0]
    is_expr = cmd_name == "expr" or (expr_aliases is not None and cmd_name in expr_aliases)
    if not is_expr or not argv_single[1]:
        return None
    return argv_texts[1]
