# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
