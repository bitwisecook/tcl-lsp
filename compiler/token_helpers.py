"""Shared token-processing helpers for the compiler pipeline.

A thin façade over :mod:`compiler.parsing.token_scanning` (where ``word_piece``
and the single-command / word-scanning helpers now live canonically) plus the
decimal-int literal parser.  The token-scanning helpers were moved under
``compiler.parsing`` so every concern — including ``dialects`` — can import them
without an import-linter carve-out; the re-exports here keep the historical
``compiler.token_helpers`` import paths (incl. the dialects XC carve-out and the
``contains_braced_scalar_marker`` use in ``core_analyses``) stable.
"""

from __future__ import annotations

from compiler.parsing.token_scanning import (  # noqa: F401  (re-exported façade)
    BRACED_SCALAR_MARKER_PREFIX,
    contains_braced_scalar_marker,
    word_piece,
)
from compiler.parsing.token_scanning import (  # noqa: F401  (re-exported façade)
    parse_single_command as parse_command_words,
)

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
