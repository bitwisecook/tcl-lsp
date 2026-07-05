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

"""Suffix array and LCP (Longest Common Prefix) construction.

Used by the minifier's substring aliasing pass and potentially by
other analyses that need to find repeated substrings efficiently.
"""

from __future__ import annotations


def build_suffix_array(text: str) -> list[int]:
    """Build a suffix array for *text* using the built-in sort."""
    n = len(text)
    return sorted(range(n), key=lambda i: text[i:])


def build_lcp_array(text: str, sa: list[int]) -> list[int]:
    """Kasai's algorithm — compute LCP array from suffix array.

    Returns an array where ``lcp[i]`` is the length of the longest
    common prefix between ``text[sa[i]:]`` and ``text[sa[i+1]:]``.
    """
    n = len(text)
    rank = [0] * n
    for i, s in enumerate(sa):
        rank[s] = i
    lcp = [0] * n
    k = 0
    for i in range(n):
        if rank[i] == n - 1:
            k = 0
            continue
        j = sa[rank[i] + 1]
        while i + k < n and j + k < n and text[i + k] == text[j + k]:
            k += 1
        lcp[rank[i]] = k
        if k:
            k -= 1
    return lcp
