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

"""Shared text-processing helpers for iApp/APL modules."""

from __future__ import annotations


def find_brace_end(source: str, start: int) -> int:
    """Return offset past the closing ``}`` matching the ``{`` at *start*.

    Handles braces inside double-quoted strings so that a ``"}"`` literal
    does not prematurely close the block.
    """
    assert source[start] == "{", f"expected '{{' at position {start}, got {source[start]!r}"
    pos = start + 1
    depth = 1
    while pos < len(source) and depth > 0:
        ch = source[pos]
        if ch == '"':
            # Skip entire quoted string (may contain braces)
            pos += 1
            while pos < len(source) and source[pos] != '"':
                if source[pos] == "\\":
                    pos += 1  # skip escaped char inside string
                pos += 1
            # Advance past closing quote
            pos += 1
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif ch == "\\":
            pos += 1  # skip escaped char
        pos += 1
    return pos


def offset_to_line_char(source: str, offset: int) -> tuple[int, int]:
    """Convert a byte offset to (line, character) pair."""
    line = source.count("\n", 0, offset)
    line_start = source.rfind("\n", 0, offset) + 1
    return (line, offset - line_start)
