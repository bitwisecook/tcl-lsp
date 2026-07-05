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

"""Tests for shared proc-lookup helpers used by LSP features."""

from __future__ import annotations

from analyser.proc_lookup import find_proc_by_reference, iter_procs_by_reference
from analyser.semantic_model import AnalysisResult, ProcDef, Range


def _proc(name: str, qname: str) -> ProcDef:
    zero = Range.zero()
    return ProcDef(
        name=name,
        qualified_name=qname,
        params=[],
        name_range=zero,
        body_range=zero,
    )


def test_iter_procs_by_reference_matches_supported_forms() -> None:
    analysis = AnalysisResult()
    analysis.all_procs = {
        "::foo": _proc("foo", "::foo"),
        "::ns::foo": _proc("foo", "::ns::foo"),
        "::bar": _proc("bar", "::bar"),
    }

    matches = iter_procs_by_reference(analysis, "foo")
    assert [qname for qname, _proc_def in matches] == ["::foo", "::ns::foo"]

    explicit = iter_procs_by_reference(analysis, "::ns::foo")
    assert [qname for qname, _proc_def in explicit] == ["::ns::foo"]


def test_find_proc_by_reference_prefers_first_match_order() -> None:
    analysis = AnalysisResult()
    analysis.all_procs = {
        "::alpha::foo": _proc("foo", "::alpha::foo"),
        "::beta::foo": _proc("foo", "::beta::foo"),
    }

    match = find_proc_by_reference(analysis, "foo")
    assert match is not None
    assert match[0] == "::alpha::foo"


def test_find_proc_by_reference_returns_none_for_missing_name() -> None:
    analysis = AnalysisResult()
    analysis.all_procs = {"::foo": _proc("foo", "::foo")}
    assert find_proc_by_reference(analysis, "missing") is None
