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

"""Tests for ai/shared/docstring_ops.py shared helpers."""

from __future__ import annotations

from ai.shared.docstring_ops import collect_proc_docs, insert_docstring_stubs
from analyser.semantic_model import (
    AnalysisResult,
    ParamDef,
    ProcDef,
)
from shared.diagnostic import Range
from shared.tokens import SourcePosition


def _make_proc(name: str, line: int, doc: str = "", params: list | None = None) -> ProcDef:
    _body_range = Range.zero()
    return ProcDef(
        name=name,
        qualified_name=f"::{name}",
        params=params or [],
        name_range=Range(
            start=SourcePosition(line=line, character=0, offset=0),
            end=SourcePosition(line=line, character=len(name), offset=len(name)),
        ),
        body_range=_body_range,
        doc=doc,
    )


class TestCollectProcDocs:
    def test_basic(self):
        result = AnalysisResult()
        result.all_procs["::foo"] = _make_proc("foo", 0, doc="Hello")
        result.all_procs["::bar"] = _make_proc("bar", 2)

        docs = collect_proc_docs(result)
        assert len(docs) == 2
        foo_doc = next(d for d in docs if d["name"] == "foo")
        bar_doc = next(d for d in docs if d["name"] == "bar")
        assert foo_doc["doc_raw"] == "Hello"
        assert foo_doc["doc"] is not None
        assert bar_doc["doc"] is None

    def test_params_included(self):
        result = AnalysisResult()
        result.all_procs["::f"] = _make_proc(
            "f",
            0,
            params=[ParamDef(name="x"), ParamDef(name="y", has_default=True, default_value="0")],
        )
        docs = collect_proc_docs(result)
        assert docs[0]["params"] == [{"name": "x"}, {"name": "y", "default": "0"}]


class TestInsertDocstringStubs:
    def test_inserts_stubs(self):
        source = "proc foo {} { puts hi }\nproc bar {} { puts bye }\n"
        result = AnalysisResult()
        result.all_procs["::foo"] = _make_proc("foo", 0)
        result.all_procs["::bar"] = _make_proc("bar", 1)

        modified, count = insert_docstring_stubs(source, result)
        assert count == 2
        assert "# @brief TODO: describe foo" in modified
        assert "# @brief TODO: describe bar" in modified

    def test_skips_documented_procs(self):
        source = "# Existing doc\nproc foo {} { puts hi }\nproc bar {} { puts bye }\n"
        result = AnalysisResult()
        result.all_procs["::foo"] = _make_proc("foo", 1, doc="Existing doc")
        result.all_procs["::bar"] = _make_proc("bar", 2)

        modified, count = insert_docstring_stubs(source, result)
        assert count == 1
        assert "TODO: describe bar" in modified
        assert "TODO: describe foo" not in modified
