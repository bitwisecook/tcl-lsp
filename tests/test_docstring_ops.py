"""Tests for ai/shared/docstring_ops.py shared helpers."""

from __future__ import annotations

from core.analysis.semantic_model import AnalysisResult, ParamDef, ProcDef
from lsprotocol.types import Position, Range

from ai.shared.docstring_ops import collect_proc_docs, insert_docstring_stubs


def _make_proc(name: str, line: int, doc: str = "", params: list | None = None) -> ProcDef:
    _body_range = Range(start=Position(line=0, character=0), end=Position(line=0, character=0))
    return ProcDef(
        name=name,
        qualified_name=f"::{name}",
        params=params or [],
        name_range=Range(start=Position(line=line, character=0), end=Position(line=line, character=len(name))),
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
            "f", 0, params=[ParamDef(name="x"), ParamDef(name="y", has_default=True, default_value="0")]
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
