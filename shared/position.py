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

"""Shared position predicates and offset conversion.

`position_in_range` and `offset_at_position` are leaf-safe helpers that
only depend on `shared.diagnostic.Range` and `shared.document_buffer`.
The richer "find the command/token at a position" helpers — which need
the compiler's lexer/segmenter — live in `compiler.position_lookup`.
"""

from __future__ import annotations

from shared.diagnostic import Range

from .document_buffer import DocumentBuffer


def position_in_range(line: int, character: int, r: Range) -> bool:
    """Check if (*line*, *character*) falls within an analysis *Range*.

    The range end is treated as inclusive (matching the semantic model
    convention where ``end.character`` is the index of the last character).
    """
    if line < r.start.line or line > r.end.line:
        return False
    if line == r.start.line and character < r.start.character:
        return False
    if line == r.end.line and character > r.end.character:
        return False
    return True


def offset_at_position(source: str, line: int, character: int) -> int:
    """Convert an LSP (*line*, *character*) position to a byte offset."""
    return DocumentBuffer.from_source(source).position_to_offset(line, character)
