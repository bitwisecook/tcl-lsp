"""End-to-end regression coverage for the VS Code parity gaps.

Every test here corresponds to a VS Code extension test that failed against the
native Rust server (the extension suite exercises the packaged server over the
full LSP protocol, but the backend-neutral ``lsp_e2e`` suite previously did not
cover these paths — so a Rust-only regression slipped through green).  Each test
drives the *same* server behaviour directly over JSON-RPC so the gap is caught
here too, without needing the VS Code test host.

Grouped by the fix area:
  * diagnostics — W100 inside command substitutions, W216 brace-then-paren
  * completion — command-context var binders, ``::``-qualified globals, dialect
  * commands   — ``setDialect`` / ``compilerExplorer``
  * config     — folding / optimiser / diagnostics-master toggles
  * capabilities — typeHierarchy advertised, pull-diagnostics opt-in
  * navigation — method-parameter go-to-definition
  * code actions — code-lens showReferences wiring, brace-expr refactor
"""

from __future__ import annotations


def _codes(diags):
    return sorted(str(d.get("code")) for d in diags)


def _labels(items):
    if isinstance(items, dict):
        items = items.get("items", [])
    return [i.get("label") for i in (items or [])]


# ── diagnostics ──────────────────────────────────────────────────────────────


class TestDiagnosticsParity:
    def test_w100_fires_for_expr_in_command_substitution(self, lsp_server, uri_factory):
        # `set x [expr $a + $b]` — the unbraced expr lives inside a `[…]`
        # command substitution (an argument value), which the analyser must
        # descend for the W100 check.
        uri = uri_factory()
        assert "W100" in _codes(lsp_server.open_ready(uri, "set x [expr $a + $b]\n"))

    def test_w216_fires_inside_command_substitution(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = (
            "set arr(name) hello\n"
            "set foo bar\n"
            "set indirect [puts ${arr}(name)]\n"
            "set indirect2 [puts ${arr($foo)}]\n"
            'set "funny name" 1\n'
            "set indirect3 [puts ${funny name($foo)}]\n"
        )
        diags = lsp_server.open_ready(uri, src)
        w216 = [d for d in diags if str(d.get("code")) == "W216"]
        assert len(w216) >= 2
        msgs = " / ".join(d.get("message", "") for d in w216)
        assert "scalar" in msgs and "literal" in msgs  # ${arr}(name) message
        assert "does not substitute" in msgs  # ${arr($foo)} message

    def test_w216_quick_fixes(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "set arr(name) 1\nset foo bar\nset r [puts ${arr}(name)]\n"
        diags = lsp_server.open_ready(uri, src)
        w216 = [d for d in diags if str(d.get("code")) == "W216" and "scalar" in d.get("message", "")]
        assert w216
        rng = w216[0]["range"]
        actions = lsp_server.code_actions(
            uri,
            (rng["start"]["line"], rng["start"]["character"]),
            (rng["end"]["line"], rng["end"]["character"]),
            diagnostics=diags,
        )
        new_texts = [e.get("newText") for a in (actions or []) for e in _edits_of(a)]
        assert any(t == "$arr(name)" for t in new_texts)


# ── completion ───────────────────────────────────────────────────────────────


class TestCompletionParity:
    COMMAND_CONTEXTS = "\n".join(
        [
            "proc t {} {",
            "    lassign {1 2 3} la lb lc",
            "    regexp {(\\w+)=(\\w+)} foo=bar rf rk rv",
            "    regsub {foo} foobar baz rout",
            '    scan "1 2 3" "%d %d %d" sx sy sz',
            "    puts $",
            "}",
        ]
    )

    def test_command_context_binders_are_completed(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = self.COMMAND_CONTEXTS + "\n"
        lsp_server.open_ready(uri, src)
        labels = set(_labels(lsp_server.completion(uri, 5, len("    puts $"))))
        for v in ["$la", "$lb", "$lc", "$rf", "$rk", "$rv", "$rout", "$sx", "$sy", "$sz"]:
            assert v in labels, f"missing {v}: {sorted(labels)}"

    def test_global_is_colon_qualified_from_namespace_proc(self, lsp_server, uri_factory):
        # From inside `::myns::helper`, the global `foo-bar` is only reachable as
        # `$::foo-bar` (a bare `$foo-bar` there is a local lookup), and the
        # hyphen forces the brace form.
        uri = uri_factory()
        lines = [
            'set ::globalvar 9',
            'set "foo-bar" 1',
            'namespace eval ::myns {',
            '    proc helper {arg} {',
            '        puts $',
            '    }',
            '}',
        ]
        lsp_server.open_ready(uri, "\n".join(lines) + "\n")
        items = lsp_server.completion(uri, 4, len('        puts $'))
        if isinstance(items, dict):
            items = items.get("items", [])
        item = next((i for i in items if i.get("label") == "$::foo-bar"), None)
        assert item is not None, f"missing $::foo-bar: {_labels(items)}"
        inserted = (item.get("textEdit") or {}).get("newText") or item.get("insertText")
        assert inserted == "${::foo-bar}"

    def test_shebang_dialect_hides_newer_commands(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "#!/usr/bin/env tclsh8.5\ntr\n")
        assert "try" not in set(_labels(lsp_server.completion(uri, 1, 2)))

    def test_directive_dialect_hides_newer_commands(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "# tcl-dialect: tcl8.4\ntr\n")
        assert "try" not in set(_labels(lsp_server.completion(uri, 1, 2)))


# ── commands ─────────────────────────────────────────────────────────────────


class TestCommandParity:
    def test_set_dialect_returns_success(self, lsp_server, uri_factory):
        r = lsp_server.execute_command("tcl-lsp.setDialect", ["tcl8.6"])
        assert isinstance(r, dict) and isinstance(r.get("success"), bool) and r["success"]

    def test_compiler_explorer_valid_source(self, lsp_server, uri_factory):
        r = lsp_server.execute_command(
            "tcl-lsp.compilerExplorer", ["set x 10\nputs $x\n", "tcl8.6"]
        )
        assert isinstance(r, dict) and not r.get("error") and "ir" in r

    def test_compiler_explorer_empty_source_is_error(self, lsp_server, uri_factory):
        r = lsp_server.execute_command("tcl-lsp.compilerExplorer", ["", "tcl8.6"])
        assert isinstance(r, dict) and r.get("error")


# ── config toggles ───────────────────────────────────────────────────────────


class TestConfigToggleParity:
    def test_folding_toggle_suppresses_ranges(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc f {} {\n  set x 1\n  set y 2\n}\n")
        assert lsp_server.folding_range(uri)  # present by default
        with lsp_server.config_session({"features": {"folding": False}}, settle_uri=uri):
            # An authoritative empty set (not `None`) so the client does not fall
            # back to built-in indentation folding.
            assert lsp_server.folding_range(uri) == []

    def test_optimiser_toggle_suppresses_o_codes(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = 'if {$x == "foo"} { puts yes }\n'
        base = lsp_server.open_ready(uri, src)
        assert any(c.startswith("O") for c in _codes(base))
        lsp_server.clear_diagnostics_log()
        with lsp_server.config_session({"optimiser": {"enabled": False}}, settle_uri=uri):
            diags = lsp_server.await_diagnostics(uri, version=1, timeout=15)
            assert not [c for c in _codes(diags) if c.startswith("O")]

    def test_diagnostics_master_switch_clears_all(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert lsp_server.open_ready(uri, "catch {error e}\n")  # non-empty by default
        lsp_server.clear_diagnostics_log()
        with lsp_server.config_session({"features": {"diagnostics": False}}, settle_uri=uri):
            assert lsp_server.await_diagnostics(uri, version=1, timeout=15) == []

    def test_optimiser_code_override_does_not_leak(self, lsp_server, uri_factory):
        # A one-off `optimiser.O100 = true` must not persist after it is cleared:
        # rebuilding the override map authoritatively reverts to the profile.
        clean = "set x 10\nset y 20\nset sum [expr {$x + $y}]\n"
        with lsp_server.config_session({"optimiser": {"O100": True}}, settle_uri=""):
            pass
        uri = uri_factory()
        assert "O100" not in _codes(lsp_server.open_ready(uri, clean))


# ── capabilities ─────────────────────────────────────────────────────────────


class TestCapabilityParity:
    def test_type_hierarchy_advertised(self, lsp_server, uri_factory):
        caps = (lsp_server.initialize_result or {}).get("capabilities", {})
        assert caps.get("typeHierarchyProvider")

    def test_pull_diagnostics_not_advertised_by_default(self, lsp_server, uri_factory):
        caps = (lsp_server.initialize_result or {}).get("capabilities", {})
        assert caps.get("diagnosticProvider") is None


# ── navigation ───────────────────────────────────────────────────────────────


class TestNavigationParity:
    def test_method_parameter_definition_resolves_to_name(self, lsp_server, uri_factory):
        lines = [
            "oo::class create C {",
            "    method greet {name greeting} {",
            '        return "$greeting, $name"',
            "    }",
            "}",
        ]
        uri = uri_factory()
        lsp_server.open_ready(uri, "\n".join(lines) + "\n")
        idx = lines[2].rfind("name")
        res = lsp_server.definition(uri, 2, idx + 1)
        locs = res if isinstance(res, list) else [res]
        assert locs
        rng = locs[0]["range"]
        assert rng["start"]["line"] == 1  # the `name` param on the method line
        assert rng["start"]["line"] == rng["end"]["line"]
        assert rng["end"]["character"] - rng["start"]["character"] <= len("greeting")


# ── code actions / lenses ────────────────────────────────────────────────────


class TestCodeActionParity:
    def test_code_lens_resolves_show_references_command(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} { return 1 }\ngreet\ngreet\n")
        lenses = lsp_server.code_lens(uri) or []
        # Reference-count lenses are returned WITHOUT a command so the client
        # resolves them; resolve then wires the clickable showReferences command.
        unresolved = [lz for lz in lenses if not lz.get("command") and lz.get("data")]
        assert unresolved, f"expected an unresolved reference lens, got {lenses}"
        resolved = lsp_server.code_lens_resolve(unresolved[0])
        cmd = resolved.get("command") or {}
        assert cmd.get("command") == "tcl-lsp.showReferences"
        args = cmd.get("arguments")
        assert isinstance(args, list) and len(args) == 3 and isinstance(args[0], str)

    def test_brace_expr_refactor_offered(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "expr $a + $b\n")
        # VS Code invokes refactors at the caret (column 0 on the `expr` word).
        actions = lsp_server.code_actions(uri, (0, 0), (0, 0), diagnostics=diags)
        brace = next(
            (a for a in (actions or []) if "Brace expr" in (a.get("title") or "")), None
        )
        assert brace is not None, f"no Brace expr action: {[a.get('title') for a in (actions or [])]}"
        assert brace.get("kind") == "refactor.rewrite"
        assert any(e.get("newText") == "{$a + $b}" for e in _edits_of(brace))


def _edits_of(action):
    edit = action.get("edit") or {}
    changes = edit.get("changes") or {}
    out = []
    for edits in changes.values():
        out.extend(edits)
    for dc in edit.get("documentChanges") or []:
        out.extend(dc.get("edits") or [])
    return out
