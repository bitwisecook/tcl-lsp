"""Code actions, end-to-end against the packaged server.

Ported from ``tests/test_code_actions.py``.  The realistic flow drives the
*published* diagnostics back into the ``codeAction`` context — exactly what an
editor does — so the quick-fix offers are exercised against real diagnostic
ranges from the live pipeline.
"""

from __future__ import annotations


def _new_texts(actions) -> list[str]:
    out: list[str] = []
    for action in actions or []:
        edit = action.get("edit") or {}
        for edits in (edit.get("changes") or {}).values():
            out.extend(e["newText"] for e in edits)
        for change in edit.get("documentChanges") or []:
            out.extend(e["newText"] for e in change.get("edits") or [])
    return out


def _titles(actions) -> list[str]:
    return [a.get("title", "") for a in actions or []]


class TestQuickFixes:
    def test_w100_offers_brace_wrap(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "if $a {puts x}\n")
        w100 = [d for d in diags if d.get("code") == "W100"]
        assert w100, "expected a W100 diagnostic to drive the quick fix"
        actions = lsp_server.code_actions(uri, (0, 0), (0, 14), diagnostics=w100)
        assert any("brace" in t.lower() for t in _titles(actions))
        assert any("{$a}" in nt for nt in _new_texts(actions))

    def test_w304_offers_option_terminator(self, lsp_server, uri_factory):
        # Ported from editors/vscode/src/test/codeActions.test.ts: an option-
        # bearing command fed substituted input gets a `--` terminator fix.
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "regexp -nocase $re $s\n")
        w304 = [d for d in diags if d.get("code") == "W304"]
        assert w304, "expected a W304 diagnostic to drive the quick fix"
        actions = lsp_server.code_actions(uri, (0, 0), (0, 20), diagnostics=w304, only=["quickfix"])
        assert any("option terminator" in t.lower() for t in _titles(actions))

    def test_w302_adds_result_capture_actions(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "catch {error oops}\n")
        w302 = [d for d in diags if d.get("code") == "W302"]
        assert w302
        actions = lsp_server.code_actions(uri, (0, 0), (0, 4), diagnostics=w302, only=["quickfix"])
        snippets = _new_texts(actions)
        assert " result" in snippets
        assert " result opts" in snippets

    def test_w302_no_fix_when_result_present(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "catch {error oops} result\n")
        # Fabricate the W302 range an editor would send; the server must still
        # decline because the source already captures a result variable.
        diag = {
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 4}},
            "code": "W302",
            "message": "catch without a result variable silently swallows errors.",
            "source": "tcl-lsp",
        }
        actions = lsp_server.code_actions(
            uri, (0, 0), (0, 4), diagnostics=[diag], only=["quickfix"]
        )
        assert _new_texts(actions) == []


class TestRefactorActions:
    def test_extract_proc_available_without_diagnostics(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set value 42\nputs $value\nputs done\n")
        actions = lsp_server.code_actions(
            uri, (1, 0), (2, 0), diagnostics=[], only=["refactor.extract"]
        )
        assert any("extract" in t.lower() for t in _titles(actions))
