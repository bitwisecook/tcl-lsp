"""Shared text-editing and name-generation helpers.

These are used by both the minifier and optimiser to apply positional
rewrites and generate short identifier names.
"""

from __future__ import annotations

import itertools
import string
from collections.abc import Iterator

_LETTERS = string.ascii_lowercase


def name_generator() -> Iterator[str]:
    """Yield short identifier names: a, b, ..., z, aa, ab, ..., zz, aaa, ...

    Names are generated lazily so cost is proportional to the number
    of names actually consumed, even for lengths > 3.
    """
    length = 1
    while True:
        for combo in itertools.product(_LETTERS, repeat=length):
            yield "".join(combo)
        length += 1


def apply_edits(source: str, edits: list[tuple[int, int, str]]) -> str:
    """Apply non-overlapping text edits to *source*.

    Each edit is ``(offset, length, new_text)``.  Edits are applied in
    reverse offset order so earlier edits don't shift later positions.
    Duplicate ``(offset, length)`` pairs are silently deduplicated.
    """
    if not edits:
        return source
    sorted_edits = sorted(edits, key=lambda e: e[0], reverse=True)
    seen: set[tuple[int, int]] = set()
    result = source
    for offset, length, new_text in sorted_edits:
        key = (offset, length)
        if key in seen:
            continue
        seen.add(key)
        result = result[:offset] + new_text + result[offset + length :]
    return result
