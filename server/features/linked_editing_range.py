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

"""Linked editing range provider.

For a cursor positioned on a proc name (either the declaration or a
self-call site inside that proc's own body), returns all ranges that
should be edited in lock-step — the declaration plus any recursive
self-calls inside the body.
"""

from __future__ import annotations

from lsprotocol import types

from analyser import analyse
from analyser.semantic_model import AnalysisResult, ProcDef, Range
from server._lsp_conv import to_lsp_range
from shared.position import position_in_range

from .references import find_proc_call_sites
from .symbol_resolution import find_word_at_position

# Word pattern matching the character set used for Tcl proc names.
_WORD_PATTERN = r"[A-Za-z_][A-Za-z0-9_]*"


def _cursor_proc(
    analysis: AnalysisResult,
    line: int,
    character: int,
    word: str,
) -> ProcDef | None:
    """Return the ProcDef whose declaration or body contains the cursor."""
    for _qname, proc in analysis.all_procs.items():
        if proc.name != word and proc.qualified_name != word:
            continue
        # Declaration name range.
        if position_in_range(line, character, proc.name_range):
            return proc
        # Within the proc's body?
        if position_in_range(line, character, proc.body_range):
            return proc
    return None


def get_linked_editing_ranges(
    source: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> types.LinkedEditingRanges | None:
    """Return linked-editing ranges for the symbol at ``(line, character)``."""
    if analysis is None:
        analysis = analyse(source)
    word = find_word_at_position(source, line, character)
    if not word:
        return None
    proc = _cursor_proc(analysis, line, character, word)
    if proc is None:
        return None

    # Collect the declaration plus any self-calls inside the body.
    ranges: list[Range] = [proc.name_range]
    call_sites = find_proc_call_sites(proc.name, proc.qualified_name, analysis)
    for r in call_sites:
        if position_in_range(r.start.line, r.start.character, proc.body_range):
            ranges.append(r)

    # Deduplicate and convert to LSP ranges.
    seen: set[tuple[int, int, int, int]] = set()
    lsp_ranges: list[types.Range] = []
    for r in ranges:
        key = (r.start.line, r.start.character, r.end.line, r.end.character)
        if key in seen:
            continue
        seen.add(key)
        lsp_ranges.append(to_lsp_range(r))

    if len(lsp_ranges) < 2:
        # Nothing to link — a single range provides no benefit over direct edit.
        return None

    return types.LinkedEditingRanges(
        ranges=lsp_ranges,
        word_pattern=_WORD_PATTERN,
    )
