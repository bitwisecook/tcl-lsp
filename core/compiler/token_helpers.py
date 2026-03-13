"""Shared token-processing helpers for the compiler pipeline.

Small utilities for extracting word text from tokens and parsing
decimal integer literals.  Used by lowering, optimiser, and other
compiler modules that walk raw token streams.
"""

from __future__ import annotations

from ..parsing.tokens import Token, TokenType
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
        # Detect braced ${...} vs bare $name form from token span.
        # ${name} has span > len(name) ($ + { + ... + }), while bare
        # $name has span == len(name) (just the $ prefix, end lands
        # on the last char of the name).
        is_braced = (tok.end.offset - tok.start.offset) > len(tok.text)
        if is_braced and "(" in tok.text and tok.text.endswith(")"):
            # Braced form with array-like name: ${a(1)} is a scalar,
            # NOT an array access.  Mark with $= prefix so codegen
            # emits push + loadStk instead of array load.
            return f"$={{{tok.text}}}"
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
