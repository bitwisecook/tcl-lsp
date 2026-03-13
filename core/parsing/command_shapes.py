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


def extract_single_expr_argument(cmd_text: str) -> str | None:
    """Return expr argument if command text is exactly: ``expr <one-arg>``."""
    lexer = TclLexer(cmd_text)
    argv_texts: list[str] = []
    argv_single: list[bool] = []
    prev_type = TokenType.EOL

    while True:
        tok = lexer.get_token()
        if tok is None or tok.type is TokenType.EOL:
            break
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
    if argv_texts[0] != "expr" or not argv_single[1]:
        return None
    return argv_texts[1]
