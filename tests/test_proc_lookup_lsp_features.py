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

"""Regression checks for proc-lookup usage across LSP features."""

from __future__ import annotations

from analyser import analyse
from analyser.proc_lookup import find_proc_by_reference
from server.features.call_hierarchy import prepare_call_hierarchy
from server.features.definition import get_definition
from server.features.references import get_references
from server.features.rename import get_rename_edits
from server.features.signature_help import get_signature_help

TEST_URI = "file:///proc-lookup.tcl"


def test_proc_lookup_consistent_across_lsp_features() -> None:
    source = (
        "namespace eval a {\n"
        "    proc foo {x} { return $x }\n"
        "}\n"
        "namespace eval b {\n"
        "    proc foo {y} { return $y }\n"
        "}\n"
        "foo 1\n"
    )
    analysis = analyse(source)

    proc_match = find_proc_by_reference(analysis, "foo")
    assert proc_match is not None
    expected_qname, expected_proc = proc_match
    assert expected_qname == "::a::foo"

    definition = get_definition(source, TEST_URI, 6, 1, analysis)
    assert len(definition) == 1
    assert definition[0].range.start.line == expected_proc.name_range.start.line

    references = get_references(
        source,
        TEST_URI,
        6,
        1,
        include_declaration=True,
        analysis=analysis,
    )
    assert any(loc.range.start.line == expected_proc.name_range.start.line for loc in references)

    rename_edits = get_rename_edits(source, TEST_URI, 6, 1, "foo_new", analysis)
    assert rename_edits is not None
    changes = rename_edits.changes
    assert changes is not None
    assert TEST_URI in changes
    assert any(
        edit.range.start.line == expected_proc.name_range.start.line for edit in changes[TEST_URI]
    )

    signature = get_signature_help(source, 6, 5, analysis)
    assert signature is not None
    assert signature.signatures[0].label.startswith("foo x")

    call_items = prepare_call_hierarchy(source, TEST_URI, 6, 1, analysis)
    assert len(call_items) == 1
    assert call_items[0].selection_range.start.line == expected_proc.name_range.start.line
