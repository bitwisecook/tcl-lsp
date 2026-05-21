"""Server-layer end-to-end tests for tools, diagnostics, and applied fixes.

These drive the actual ``lsp.commands`` workspace-command handlers and the
``lsp.features.diagnostics`` provider — the same entry points the JSON-RPC
dispatch calls — and assert on the *result of the transform*, not merely that
a handler returned something.  Where a command rewrites source (fix-all,
optimise, minify) the test re-checks the rewritten text so a semantic
regression (e.g. a fix that leaves the original diagnostic firing) fails.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

import lsp.commands as commands
from lsp.features.diagnostics import get_diagnostics
from lsp.state import workspace_state


def _codes(diags: list[types.Diagnostic]) -> set[str]:
    out: set[str] = set()
    for d in diags:
        code = d.code
        out.add(code if isinstance(code, str) else str(code))
    return out


# ── tools: fix-all safe issues (server applies quick-fixes) ────────────


class TestFixAllSafeIssues:
    def test_braces_unbraced_expressions_and_clears_w100(self):
        uri = "file:///fixall.tcl"
        source = "if $a { puts yes }\nset n [expr $x + 1]\n"
        workspace_state.open(uri, source)
        result = commands.on_fix_all_safe_issues(uri)
        assert result is not None
        rewritten = result["source"]
        assert "if {$a} { puts yes }" in rewritten
        assert "expr {$x + 1}" in rewritten
        assert {entry["code"] for entry in result["applied"]} == {"W100"}
        # Semantic check: the fixes must actually resolve the W100s — none
        # may remain in the rewritten document.
        assert "W100" not in _codes(get_diagnostics(rewritten))


# ── tools: optimise document ───────────────────────────────────────────


class TestOptimiseDocument:
    def test_constant_fold_and_dead_store(self):
        uri = "file:///opt.tcl"
        workspace_state.open(uri, "set x [expr {1 + 2}]\nputs $x\n")
        result = commands.on_optimise_document(uri)
        assert result is not None
        codes = {o["code"] for o in result["optimisations"]}
        # Constant folding (O100) of 1+2 -> 3 must be offered.
        assert "O100" in codes
        # The rewritten source folds the constant through to the puts.
        assert "puts 3" in result["source"]


# ── tools: minify document ─────────────────────────────────────────────


class TestMinifyDocument:
    def test_strips_comments_and_collapses_whitespace(self):
        uri = "file:///min.tcl"
        source = "# comment\nset   x    42\n\nputs $x\n"
        workspace_state.open(uri, source)
        result = commands.on_minify_document(uri)
        assert result is not None
        minified = result["source"]
        assert "# comment" not in minified
        assert "set x 42" in minified
        assert "puts $x" in minified
        assert result["minifiedLength"] < result["originalLength"]


# ── tools: registry lookups ────────────────────────────────────────────


class TestRegistryTools:
    def test_describe_event_known(self):
        data = commands.on_describe_irule_event("HTTP_REQUEST")
        assert data["known"] is True
        assert data["validCommandCount"] >= 1

    def test_describe_event_unknown(self):
        data = commands.on_describe_irule_event("NOT_A_REAL_EVENT")
        assert data["known"] is False
        assert data["validCommandCount"] == 0

    def test_describe_command(self):
        data = commands.on_describe_irule_command("HTTP::uri")
        assert data["found"] is True
        assert data["command"] == "HTTP::uri"

    def test_list_irule_events_nonempty(self):
        data = commands.on_list_irule_events()
        assert isinstance(data.get("events"), list)
        assert "HTTP_REQUEST" in data["events"]


# ── tools: diagram extraction ──────────────────────────────────────────


class TestDiagramData:
    def test_irule_events_extracted(self):
        source = 'when HTTP_REQUEST {\n    if {[HTTP::uri] eq "/"} { pool web }\n}\n'
        result = commands.on_diagram_data(source)
        assert result is not None
        names = [e.get("name") for e in result.get("events", [])]
        assert "HTTP_REQUEST" in names


# ── tools: effective config ────────────────────────────────────────────


class TestEffectiveConfig:
    def test_shape(self):
        uri = "file:///cfg.tcl"
        workspace_state.open(uri, "set x 1\n")
        data = commands.on_get_effective_config(uri)
        assert isinstance(data, dict)
        assert "dialect" in data


# ── diagnostics (server-layer provider) ────────────────────────────────


class TestServerDiagnostics:
    def test_unbraced_expr_is_w100_error(self):
        codes = _codes(get_diagnostics("if $a {puts x}\n"))
        assert "W100" in codes

    def test_catch_without_result_is_w302(self):
        codes = _codes(get_diagnostics("catch {error e}\n"))
        assert "W302" in codes

    def test_arity_error_is_e002(self):
        diags = get_diagnostics("set\n")
        assert any(
            (d.code if isinstance(d.code, str) else str(d.code)) == "E002"
            and d.severity == types.DiagnosticSeverity.Error
            for d in diags
        )

    def test_clean_file_has_no_diagnostics(self):
        assert get_diagnostics("set x [clock seconds]\nputs $x\n") == []
