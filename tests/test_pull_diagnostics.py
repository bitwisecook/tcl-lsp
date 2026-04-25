"""Tests for pull-model diagnostics (textDocument/diagnostic, workspace/diagnostic)."""

from __future__ import annotations

import itertools
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

import lsp.diagnostics_pipeline as _dp
import lsp.server as server_module


def _reset_cache() -> None:
    _dp._pull_diag_cache.clear()
    _dp._pull_diag_result_ids.clear()
    _dp._pull_diag_counter = itertools.count(1)


def _mk_diag(line: int, msg: str) -> types.Diagnostic:
    return types.Diagnostic(
        range=types.Range(
            start=types.Position(line=line, character=0),
            end=types.Position(line=line, character=1),
        ),
        message=msg,
    )


class TestCacheWriteThrough:
    def test_publish_populates_cache(self, monkeypatch):
        _reset_cache()
        uri = "file:///a.tcl"
        monkeypatch.setattr(
            server_module.server,
            "text_document_publish_diagnostics",
            lambda params: None,
        )
        _dp._publish_diags_to_client(uri, [_mk_diag(0, "e")], version=1)
        assert uri in _dp._pull_diag_cache
        assert len(_dp._pull_diag_cache[uri]) == 1
        assert _dp._pull_diag_result_ids[uri] != ""

    def test_result_ids_increment(self, monkeypatch):
        _reset_cache()
        uri = "file:///b.tcl"
        monkeypatch.setattr(
            server_module.server,
            "text_document_publish_diagnostics",
            lambda params: None,
        )
        _dp._publish_diags_to_client(uri, [], version=1)
        first = _dp._pull_diag_result_ids[uri]
        _dp._publish_diags_to_client(uri, [_mk_diag(1, "e")], version=2)
        second = _dp._pull_diag_result_ids[uri]
        assert first != second

    def test_push_still_called(self, monkeypatch):
        """The always-push path must still invoke publishDiagnostics."""
        _reset_cache()
        calls: list[types.PublishDiagnosticsParams] = []
        monkeypatch.setattr(
            server_module.server,
            "text_document_publish_diagnostics",
            lambda params: calls.append(params),
        )
        _dp._publish_diags_to_client("file:///e.tcl", [_mk_diag(0, "err")], version=7)
        assert len(calls) == 1
        assert calls[0].uri == "file:///e.tcl"
        assert len(calls[0].diagnostics) == 1


class TestDocumentDiagnosticHandler:
    def test_full_report_for_fresh_client(self):
        _reset_cache()
        uri = "file:///c.tcl"
        _dp._pull_diag_cache[uri] = [_mk_diag(0, "err")]
        _dp._pull_diag_result_ids[uri] = "rid-1"
        params = types.DocumentDiagnosticParams(
            text_document=types.TextDocumentIdentifier(uri=uri),
            previous_result_id=None,
        )
        report = _dp.on_document_diagnostic(params)
        assert isinstance(report, types.RelatedFullDocumentDiagnosticReport)
        assert len(report.items) == 1
        assert report.result_id == "rid-1"

    def test_unchanged_report_when_result_id_matches(self):
        _reset_cache()
        uri = "file:///d.tcl"
        _dp._pull_diag_cache[uri] = [_mk_diag(0, "err")]
        _dp._pull_diag_result_ids[uri] = "rid-42"
        params = types.DocumentDiagnosticParams(
            text_document=types.TextDocumentIdentifier(uri=uri),
            previous_result_id="rid-42",
        )
        report = _dp.on_document_diagnostic(params)
        assert isinstance(report, types.RelatedUnchangedDocumentDiagnosticReport)
        assert report.result_id == "rid-42"

    def test_empty_report_for_unknown_uri(self):
        _reset_cache()
        params = types.DocumentDiagnosticParams(
            text_document=types.TextDocumentIdentifier(uri="file:///unknown.tcl"),
            previous_result_id=None,
        )
        report = _dp.on_document_diagnostic(params)
        assert isinstance(report, types.RelatedFullDocumentDiagnosticReport)
        assert report.items == []


class TestWorkspaceDiagnosticHandler:
    def test_report_includes_all_cached_uris(self):
        _reset_cache()
        _dp._pull_diag_cache["file:///x.tcl"] = [_mk_diag(0, "a")]
        _dp._pull_diag_cache["file:///y.tcl"] = []
        _dp._pull_diag_result_ids["file:///x.tcl"] = "x-1"
        _dp._pull_diag_result_ids["file:///y.tcl"] = "y-1"
        params = types.WorkspaceDiagnosticParams(previous_result_ids=[])
        report = _dp.on_workspace_diagnostic(params)
        assert isinstance(report, types.WorkspaceDiagnosticReport)
        assert len(report.items) == 2

    def test_unchanged_when_previous_id_matches(self):
        _reset_cache()
        _dp._pull_diag_cache["file:///z.tcl"] = [_mk_diag(0, "a")]
        _dp._pull_diag_result_ids["file:///z.tcl"] = "z-7"
        params = types.WorkspaceDiagnosticParams(
            previous_result_ids=[
                types.PreviousResultId(uri="file:///z.tcl", value="z-7"),
            ]
        )
        report = _dp.on_workspace_diagnostic(params)
        assert len(report.items) == 1
        assert isinstance(report.items[0], types.WorkspaceUnchangedDocumentDiagnosticReport)
