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

"""Shared docstring operations for AI consumers (MCP server, tcl_ai CLI).

Provides high-level helpers that combine core analysis with docstring
utilities, so both the CLI and MCP entry points can share the same logic.
"""

from __future__ import annotations

from typing import Any

from analyser.semantic_model import AnalysisResult
from tooling.formatter.docstring import (
    DocstringTagStyle,
    generate_stub_for_proc,
    parse_docstring,
)


def collect_proc_docs(result: AnalysisResult) -> list[dict[str, Any]]:
    """Build a list of structured documentation dicts for all procs.

    Each entry contains ``name``, ``qualified_name``, ``params``,
    ``doc_raw``/``doc`` (parsed), and ``param_traits`` where available.
    """
    procs: list[dict[str, Any]] = []
    for _qname, proc_def in result.all_procs.items():
        entry: dict[str, Any] = {
            "name": proc_def.name,
            "qualified_name": proc_def.qualified_name,
            "params": [
                {"name": p.name, "default": p.default_value} if p.has_default else {"name": p.name}
                for p in proc_def.params
            ],
        }
        if proc_def.doc:
            entry["doc_raw"] = proc_def.doc
            parsed = parse_docstring(proc_def.doc)
            entry["doc"] = parsed.to_dict()
        else:
            entry["doc"] = None
        if proc_def.param_traits:
            entry["param_traits"] = {
                name: sorted(t.name for t in traits)
                for name, traits in proc_def.param_traits.items()
            }
        procs.append(entry)
    return procs


def insert_docstring_stubs(
    source: str,
    result: AnalysisResult,
    *,
    tag_style: DocstringTagStyle = DocstringTagStyle.DOXYGEN,
    decoration: bool = False,
) -> tuple[str, int]:
    """Insert docstring stubs for all undocumented procs.

    Returns ``(modified_source, documented_count)``.  Inserts from bottom
    to top so that line offsets are not shifted by earlier insertions.
    """
    procs_to_doc = [pd for pd in result.all_procs.values() if not pd.doc]
    procs_to_doc.sort(key=lambda p: p.name_range.start.line, reverse=True)

    lines = source.splitlines(keepends=True)
    for proc_def in procs_to_doc:
        stub = generate_stub_for_proc(proc_def, tag_style=tag_style, decoration=decoration)
        lines.insert(proc_def.name_range.start.line, stub + "\n")

    return "".join(lines), len(procs_to_doc)
