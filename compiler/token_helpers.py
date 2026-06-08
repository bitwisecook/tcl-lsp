"""Shared token-processing helpers for the compiler pipeline.

A thin façade over :mod:`compiler.parsing.token_scanning` (the canonical home
for ``word_piece`` and the token/command scanners) plus the decimal-int literal
parser.  The scanners live under ``compiler.parsing`` so every concern —
including ``dialects`` — can import them without an import-linter carve-out; the
re-exports here keep the historical ``compiler.token_helpers`` import paths
stable.
"""

from __future__ import annotations

from compiler.parsing.token_scanning import (
    parse_single_command as parse_command_words,
)
from compiler.parsing.token_scanning import word_piece

__all__ = ["parse_command_words", "word_piece"]

from .eval_helpers import DECIMAL_INT_RE


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
