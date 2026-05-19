"""Shared token-processing helpers for the compiler pipeline.

Small utilities for extracting word text from tokens and parsing
decimal integer literals.  Used by lowering, optimiser, and other
compiler modules that walk raw token streams.
"""

from __future__ import annotations

from compiler.parsing.lexer import TclLexer
from shared.tokens import Token, TokenType

from .eval_helpers import DECIMAL_INT_RE


def word_piece(tok: Token) -> str:
    """Return the source-level text fragment for a single token.

    Variables are prefixed with ``$`` and command substitutions are
    wrapped in ``[...]`` so that the result mirrors what the user wrote.

    For VAR tokens with array-like names (containing ``(`` and ending
    with ``)``) where the original source used braced ``${a(1)}`` form,
    a ``$={name}`` marker is emitted.  In Tcl, braces prevent array
    interpretation so ``${a(1)}`` refers to a scalar named ``a(1)``,
    while bare ``$a(1)`` refers to array element ``a`` key ``1``.
    Codegen uses this marker to emit ``push + loadStk`` instead of
    ``loadArray1``.
    """
    if tok.type is TokenType.VAR:
        # Detect braced ``${...}`` vs bare ``$name`` form from the
        # token span.  ``Token.end.offset`` is the position of the
        # *last* source byte covered by the token (inclusive — see
        # ``core/parsing/lexer.py``); ``tok.text`` excludes the
        # leading ``$`` for bare and excludes both braces for
        # ``${…}``.  Net delta is 0 for bare and 1 for braced
        # (the leading ``$`` and the closing ``}`` cancel against
        # the omitted ``${``+``}`` so only one source char is
        # un-accounted-for in the braced form).  An earlier
        # ``>= 2`` test never fired and silently flipped braced
        # array-shaped names into bare array-element reads
        # (Copilot review on PR #382).
        span_extra = (tok.end.offset - tok.start.offset) - len(tok.text)
        is_braced = span_extra >= 1
        if is_braced and "(" in tok.text and tok.text.endswith(")"):
            # Braced form with array-like name: ${a(1)} is a scalar,
            # NOT an array access.  Mark with $= prefix so codegen
            # emits push + loadStk instead of array load.
            return f"$={{{tok.text}}}"
        # Bare ``$arr(idx)`` with a *substituted* index (``$x`` or ``[cmd]``
        # inside the parens) must round-trip verbatim — the ``${…}`` wrapper
        # below would collapse the recursive substitution into a literal
        # scalar lookup (cmdAH-1.4 / 1.5 ``$numargErrors($cmd)``).  When
        # the index is fully literal we can still normalise to ``${a(1)}``
        # since no substitution is at risk.
        if (
            not is_braced
            and "(" in tok.text
            and tok.text.endswith(")")
            and ("$" in tok.text or "[" in tok.text)
        ):
            return "$" + tok.text
        # Use ${name} form only when the name doesn't contain '}'.
        # Names with '}' (e.g. array indices with braced expressions
        # like ``a(1[expr {3 - 1}])``) would cause the first '}' to
        # prematurely close the ${...} form during runtime substitution.
        if "}" in tok.text:
            return "$" + tok.text
        return f"${{{tok.text}}}"
    if tok.type is TokenType.CMD:
        return f"[{tok.text}]"
    return tok.text


def parse_command_words(
    text: str,
) -> tuple[list[str], list[Token], list[bool]] | None:
    """Parse a single Tcl command into ``(argv_texts, argv_tokens, argv_single)``.

    Returns ``None`` if *text* contains zero commands or more than one.
    Each word's text is reconstructed via :func:`word_piece`, so variable
    references are normalised to ``${name}`` form.
    """
    lexer = TclLexer(text)
    commands: list[tuple[list[str], list[Token], list[bool]]] = []
    argv_texts: list[str] = []
    argv_tokens: list[Token] = []
    argv_single: list[bool] = []
    prev_type = TokenType.EOL

    def flush() -> None:
        nonlocal argv_texts, argv_tokens, argv_single
        if argv_texts:
            commands.append((argv_texts, argv_tokens, argv_single))
        argv_texts = []
        argv_tokens = []
        argv_single = []

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is TokenType.COMMENT:
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        if tok.type is TokenType.EOL:
            flush()
            prev_type = tok.type
            continue

        piece = word_piece(tok)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv_texts.append(piece)
            argv_tokens.append(tok)
            argv_single.append(True)
        else:
            if argv_texts:
                argv_texts[-1] += piece
                argv_single[-1] = False
            else:
                argv_texts.append(piece)
                argv_tokens.append(tok)
                argv_single.append(True)
        prev_type = tok.type

    flush()
    if len(commands) != 1:
        return None
    return commands[0]


def parse_decimal_int(text: str) -> str | None:
    """Parse *text* as a decimal integer, returning its canonical ``str`` form.

    Returns ``None`` if *text* is not a valid decimal integer.
    """
    value = text.strip()
    if not DECIMAL_INT_RE.fullmatch(value):
        return None
    try:
        return str(int(value))
    except ValueError:
        return None
