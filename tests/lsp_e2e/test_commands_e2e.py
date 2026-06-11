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


# Commands advertised in ``executeCommandProvider`` that every conforming
# backend must expose.  Side-effecting / environment-coupled commands
# (tclpkg.*, bigipCleanup, writeRuleBack, renamePartition, tkPreview,
# compilerExplorer, searchHelp, listRules — which read external files) are
# exercised elsewhere or out of scope for a pure-protocol probe.
_CORE_COMMANDS = {
    "tcl-lsp.optimiseDocument",
    "tcl-lsp.minifyDocument",
    "tcl-lsp.fixAllSafeIssues",
    "tcl-lsp.getEffectiveConfig",
    "tcl-lsp.listSubcommands",
    "tcl-lsp.describeIruleEvent",
    "tcl-lsp.describeIruleCommand",
    "tcl-lsp.listIruleEvents",
    "tcl-lsp.diagramData",
}


class TestCommandSurface:
    """The advertised ``executeCommand`` surface is present and well-shaped."""

    def test_core_commands_are_advertised(self, lsp_server):
        provider = lsp_server.initialize_result["capabilities"].get("executeCommandProvider") or {}
        advertised = set(provider.get("commands") or [])
        missing = _CORE_COMMANDS - advertised
        assert not missing, f"executeCommandProvider missing core commands: {missing}"

    def test_minify_compact_round_trip(self, lsp_server, uri_factory):
        # Compact renaming shortens identifiers and reports the reverse map, so a
        # name in the minified source resolves back to the original.
        uri = uri_factory()
        lsp_server.open_ready(
            uri, "proc addNumbers {a b} { return [expr {$a + $b}] }\naddNumbers 1 2\n"
        )
        result = lsp_server.execute_command("tcl-lsp.minifyDocument", [uri, True, False, False])
        assert result is not None
        assert result["minifiedLength"] < result["originalLength"]
        assert "addNumbers" not in result["source"], result["source"]
        assert "addNumbers" in result["symbolMap"], result["symbolMap"]

    def test_list_subcommands_shape(self, lsp_server):
        data = lsp_server.execute_command("tcl-lsp.listSubcommands", ["string"])
        assert data["command"] == "string"
        names = {s["name"] for s in data["subcommands"]}
        assert {"length", "range", "map"} <= names, names

    def test_list_subcommands_unknown_is_empty(self, lsp_server):
        data = lsp_server.execute_command(
            "tcl-lsp.listSubcommands", ["definitely_not_a_command_zzz"]
        )
        assert data["subcommands"] == []

    def test_list_known_packages_shape(self, lsp_server):
        data = lsp_server.execute_command("tcl-lsp.listKnownPackages", [])
        assert isinstance(data.get("packages"), list)

    def test_suggest_packages_for_symbol_shape(self, lsp_server):
        data = lsp_server.execute_command("tcl-lsp.suggestPackagesForSymbol", ["json::write"])
        assert data["symbol"] == "json::write"
        assert isinstance(data.get("suggestions"), list)

    def test_export_config_writes_a_file(self, lsp_server):
        data = lsp_server.execute_command("tcl-lsp.exportConfig", [])
        assert data.get("success") is True
        assert data.get("path")
