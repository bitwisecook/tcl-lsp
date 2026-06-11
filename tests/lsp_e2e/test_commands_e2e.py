"""Workspace ``executeCommand`` handlers, end-to-end against the packaged server.

Ported from ``tests/test_lsp_server_actions_e2e.py``.  These drive the real
``workspace/executeCommand`` dispatch the editors' command palette entries
invoke.  pygls binds each declared parameter positionally, so the argument
arrays here mirror the handler signatures exactly (a short array is rejected
with ``Invalid Params``).
"""

from __future__ import annotations


class TestDocumentTransforms:
    def test_fix_all_safe_issues_clears_w100(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "if $a { puts yes }\nset n [expr $x + 1]\n")
        result = lsp_server.execute_command("tcl-lsp.fixAllSafeIssues", [uri])
        assert result is not None
        assert "if {$a} { puts yes }" in result["source"]
        assert {entry["code"] for entry in result["applied"]} == {"W100"}

    def test_minify_strips_comments(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "# comment\nset   x    42\n\nputs $x\n")
        result = lsp_server.execute_command("tcl-lsp.minifyDocument", [uri, False, False, False])
        assert result is not None
        assert "# comment" not in result["source"]
        assert "set x 42" in result["source"]
        assert result["minifiedLength"] < result["originalLength"]

    def test_optimise_returns_optimisation_offers(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts [llength [list a b c]]\n")
        result = lsp_server.execute_command("tcl-lsp.optimiseDocument", [uri, "full"])
        assert result is not None
        assert "puts 3" in result["source"]

    def test_minify_preserves_switch_braced_quoted_pattern_closers(self, lsp_server, uri_factory):
        # Issue #540: ``iter_switch_case_list`` derived a braced ``{a b}`` /
        # quoted ``"c d"`` pattern's end one char short, so the minifier dropped
        # the closing ``}`` / ``"`` and re-emitted a truncated, unbalanced
        # pattern (tclsh rejected the result with "missing close-brace").  The
        # minified document must keep both patterns intact and stay balanced.
        uri = uri_factory()
        src = 'switch $x {\n  {a b} { puts one }\n  "c d" { puts two }\n  default { puts def }\n}\n'
        lsp_server.open_ready(uri, src)
        result = lsp_server.execute_command("tcl-lsp.minifyDocument", [uri, False, False, False])
        assert result is not None
        out = result["source"]
        assert "{a b}" in out, out
        assert '"c d"' in out, out
        assert out.count("{") == out.count("}"), out


class TestRegistryLookups:
    def test_describe_event_known(self, lsp_server):
        data = lsp_server.execute_command("tcl-lsp.describeIruleEvent", ["HTTP_REQUEST"])
        assert data["known"] is True
        assert data["validCommandCount"] >= 1

    def test_describe_event_unknown(self, lsp_server):
        data = lsp_server.execute_command("tcl-lsp.describeIruleEvent", ["NOT_A_REAL_EVENT"])
        assert data["known"] is False
        assert data["validCommandCount"] == 0

    def test_describe_command(self, lsp_server):
        data = lsp_server.execute_command("tcl-lsp.describeIruleCommand", ["HTTP::uri"])
        assert data["found"] is True
        assert data["command"] == "HTTP::uri"

    def test_list_irule_events_nonempty(self, lsp_server):
        data = lsp_server.execute_command("tcl-lsp.listIruleEvents", [])
        assert "HTTP_REQUEST" in data["events"]


class TestDiagramAndConfig:
    def test_diagram_extracts_irule_events(self, lsp_server):
        source = 'when HTTP_REQUEST {\n    if {[HTTP::uri] eq "/"} { pool web }\n}\n'
        result = lsp_server.execute_command("tcl-lsp.diagramData", [source])
        assert result is not None
        assert "HTTP_REQUEST" in [e.get("name") for e in result.get("events", [])]

    def test_effective_config_shape(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\n")
        data = lsp_server.execute_command("tcl-lsp.getEffectiveConfig", [uri])
        assert isinstance(data, dict)
        assert "dialect" in data
