"""Push diagnostics, end-to-end against the packaged server.

Ported from the server-layer diagnostic tests in
``tests/test_lsp_server_actions_e2e.py`` and ``tests/test_diagnostics.py``.
The server advertises no pull provider, so these assert on the
``publishDiagnostics`` the server pushes after analysis, keyed by version.
"""

from __future__ import annotations


def _codes(diags) -> set[str]:
    return {str(d.get("code")) for d in diags}


class TestPushDiagnostics:
    def test_unbraced_expr_is_w100(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert "W100" in _codes(lsp_server.open_ready(uri, "if $a {puts x}\n"))

    def test_catch_without_result_is_w302(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert "W302" in _codes(lsp_server.open_ready(uri, "catch {error e}\n"))

    def test_arity_error_is_e002_with_error_severity(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set\n")
        e002 = [d for d in diags if d.get("code") == "E002"]
        assert e002
        assert e002[0].get("severity") == 1  # DiagnosticSeverity.Error

    def test_clean_file_has_no_diagnostics(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert lsp_server.open_ready(uri, "set x [clock seconds]\nputs $x\n") == []

    def test_renamed_away_command_is_w128(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "proc a {} {return 1}\na\nrename a b\na\n")
        assert "W128" in _codes(diags)


class TestDiagnosticsTrackEdits:
    def test_fixing_the_source_clears_the_diagnostic(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "if $a {puts x}\n")
        assert "W100" in _codes(diags)
        # Wrap the expression in braces via an incremental edit: `$a` -> `{$a}`.
        lsp_server.change_document(
            uri,
            2,
            [
                {
                    "range": {
                        "start": {"line": 0, "character": 3},
                        "end": {"line": 0, "character": 3},
                    },
                    "text": "{",
                },
                {
                    "range": {
                        "start": {"line": 0, "character": 6},
                        "end": {"line": 0, "character": 6},
                    },
                    "text": "}",
                },
            ],
        )
        cleared = lsp_server.await_diagnostics(uri, version=2)
        assert "W100" not in _codes(cleared)

    def test_introducing_an_error_publishes_it(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert lsp_server.open_ready(uri, "puts hello\n") == []
        # Replace the whole line with an arity error.
        lsp_server.replace_document(uri, 2, "set\n")
        diags = lsp_server.await_diagnostics(uri, version=2)
        assert "E002" in _codes(diags)
