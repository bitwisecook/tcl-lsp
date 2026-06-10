"""Decimal-int literal parser for the compiler pipeline.

Provides :func:`parse_decimal_int`, which canonicalises a decimal integer
literal to its ``str`` form (returning ``None`` for non-integers).
"""

from __future__ import annotations

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
