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

"""Shared text-editing and name-generation helpers.

These are used by both the minifier and optimiser to apply positional
rewrites and generate short identifier names.
"""

from __future__ import annotations

import itertools
import re
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


# Path-join refactor helper — shared by the compiler taint pass
# (compiler/taint/_path_concat.py) and analyser path checks.  Pure
# string→string, no layer-specific types, so it lives in shared/.
_SIMPLE_PATH_SEGMENT_RE = re.compile(r"^[A-Za-z0-9_.-]+$")
_SIMPLE_PATH_VAR_RE = re.compile(
    r"^\$(?:\{[A-Za-z_][A-Za-z0-9_:]*\}|[A-Za-z_][A-Za-z0-9_:]*)$",
)


def build_file_join_fix(path_expr: str) -> str | None:
    """Build a conservative ``[file join ...]`` replacement when safe."""
    text = path_expr.strip()
    if not text:
        return None
    if (text.startswith('"') and text.endswith('"')) or (
        text.startswith("{") and text.endswith("}")
    ):
        text = text[1:-1]
    if not text or any(ch in text for ch in "[];"):
        return None
    if " " in text:
        return None

    parts = [part for part in re.split(r"[/\\]+", text) if part]
    if len(parts) < 2:
        return None

    for part in parts:
        if _SIMPLE_PATH_VAR_RE.fullmatch(part):
            continue
        if _SIMPLE_PATH_SEGMENT_RE.fullmatch(part):
            continue
        return None

    return "[file join " + " ".join(parts) + "]"
