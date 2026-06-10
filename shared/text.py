"""Shared text-similarity utilities.

Used by the analyser (W123 unknown command suggestions) and the compiler
(W001 unknown subcommand suggestions) to offer "did you mean?" hints.
"""

from __future__ import annotations

import heapq
from collections.abc import Iterable


def edit_distance(a: str, b: str) -> int:
    """Compute Levenshtein edit distance between two strings."""
    if len(a) < len(b):
        return edit_distance(b, a)
    if not b:
        return len(a)
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a):
        curr = [i + 1]
        for j, cb in enumerate(b):
            cost = 0 if ca == cb else 1
            curr.append(min(prev[j + 1] + 1, curr[j] + 1, prev[j] + cost))
        prev = curr
    return prev[-1]


def suggest_similar(
    attempted: str,
    candidates: Iterable[str],
    *,
    max_suggestions: int = 3,
    max_distance: int = 3,
) -> list[str]:
    """Suggest similar strings ranked by edit distance.

    Returns up to *max_suggestions* candidates within *max_distance*.
    """
    scored = ((edit_distance(attempted, name), name) for name in candidates)
    smallest = heapq.nsmallest(max_suggestions, scored)
    return [name for dist, name in smallest if dist <= max_distance]
