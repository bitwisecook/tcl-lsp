"""Code actions, end-to-end against the packaged server.

Full-parity port of the Tcl-dialect cases in ``tests/test_code_actions.py``.
The realistic flow drives diagnostics back into the ``codeAction`` context —
exactly what an editor does — so quick-fix offers are exercised against real
(or, where a specific range matters, fabricated) diagnostics.

The F5 iRules code actions (collect-bootstrap, taint quick-fixes, profile
headers) run against the dedicated iRules server in
``tests/lsp_e2e/test_irules_e2e.py``.
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


def _kinds(actions, kind_prefix):
    return [a for a in actions or [] if (a.get("kind") or "").startswith(kind_prefix)]


def _diag(code, message, start, end):
    return {
        "range": {
            "start": {"line": start[0], "character": start[1]},
            "end": {"line": end[0], "character": end[1]},
        },
        "code": code,
        "message": message,
        "source": "tcl-lsp",
    }


class TestQuickFixes:
    def test_w100_offers_brace_wrap(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "if $a {puts x}\n")
        w100 = [d for d in diags if d.get("code") == "W100"]
        assert w100, "expected a W100 diagnostic to drive the quick fix"
        actions = lsp_server.code_actions(uri, (0, 0), (0, 14), diagnostics=w100)
        assert any("brace" in t.lower() for t in _titles(actions))
        assert any("{$a}" in nt for nt in _new_texts(actions))

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


class TestRefactorActionsExtended:
    def test_extract_proc_snippets(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set value 42\nputs $value\nputs done\n")
        actions = lsp_server.code_actions(uri, (1, 0), (2, 0), only=["refactor.extract"])
        extract = _kinds(actions, "refactor.extract")
        assert extract
        snippets = _new_texts(extract)
        assert any("proc extracted_proc {value}" in s for s in snippets)
        assert any("extracted_proc $value" in s for s in snippets)

    def test_extract_proc_attaches_rename_command(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set value 42\nputs $value\nputs done\n")
        actions = lsp_server.code_actions(uri, (1, 0), (2, 0), only=["refactor.extract"])
        extract = _kinds(actions, "refactor.extract")[0]
        cmd = extract.get("command")
        assert cmd and cmd["command"] == "tclLsp.renameSymbolAtPosition"
        line, start, end = cmd["arguments"]
        assert line == 0
        assert start == len("proc ")
        assert end == start + len("extracted_proc")

    def test_inline_proc_available(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {name} { puts $name }\ngreet Bob\n")
        actions = lsp_server.code_actions(uri, (1, 1), (1, 1), only=["refactor.inline"])
        inline = _kinds(actions, "refactor.inline")
        assert inline
        assert any("puts Bob" in s for s in _new_texts(inline))

    def test_inline_proc_skips_returning_proc(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc wrap {x} { return $x }\nwrap value\n")
        actions = lsp_server.code_actions(uri, (1, 1), (1, 1), only=["refactor.inline"])
        assert _kinds(actions, "refactor.inline") == []


class TestExprRefactorActions:
    def _rewrite(self, lsp_server, uri, start, end):
        actions = lsp_server.code_actions(uri, start, end, only=["refactor.rewrite"])
        return _kinds(actions, "refactor.rewrite")

    def test_demorgan_forward(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "if {!($a && $b)} { puts yes }\n")
        dm = [
            a for a in self._rewrite(lsp_server, uri, (0, 4), (0, 15)) if "De Morgan" in a["title"]
        ]
        assert len(dm) == 1
        assert any("!$a || !$b" in s for s in _new_texts(dm))

    def test_demorgan_reverse(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "if {!$a || !$b} { puts yes }\n")
        dm = [
            a for a in self._rewrite(lsp_server, uri, (0, 4), (0, 14)) if "De Morgan" in a["title"]
        ]
        assert len(dm) == 1
        assert any("!($a && $b)" in s for s in _new_texts(dm))

    def test_invert_expression(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "if {$a == $b} { puts yes }\n")
        inv = [a for a in self._rewrite(lsp_server, uri, (0, 4), (0, 12)) if "Invert" in a["title"]]
        assert len(inv) == 1
        assert any("$a != $b" in s for s in _new_texts(inv))

    def test_no_actions_for_non_expression_selection(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts hello\n")
        acts = self._rewrite(lsp_server, uri, (0, 0), (0, 4))
        assert not [a for a in acts if "De Morgan" in a["title"] or "Invert" in a["title"]]

    def test_no_actions_for_empty_selection(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "if {$a == $b} { puts yes }\n")
        acts = self._rewrite(lsp_server, uri, (0, 5), (0, 5))
        assert not [a for a in acts if "De Morgan" in a["title"] or "Invert" in a["title"]]

    def test_invert_subexpression_in_compound(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "if {$a == $b && $c < $d} { puts yes }\n")
        inv = [
            a for a in self._rewrite(lsp_server, uri, (0, 16), (0, 23)) if "Invert" in a["title"]
        ]
        assert len(inv) == 1


class TestW115CommentContinuation:
    def test_simple_continuation_fix(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "# hello \\\nworld")
        diag = _diag("W115", "Backslash-newline in comment", (0, 0), (1, 4))
        actions = lsp_server.code_actions(uri, (0, 0), (1, 4), diagnostics=[diag])
        fixes = [a for a in actions if "per-line" in a.get("title", "")]
        assert len(fixes) == 1
        snippet = _new_texts(fixes)[0]
        assert "# hello" in snippet
        assert "# world" in snippet
        assert "\\" not in snippet

    def test_already_commented_continuation_not_doubled(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "# line1 \\\n# line2")
        diag = _diag("W115", "Backslash-newline in comment", (0, 0), (1, 6))
        actions = lsp_server.code_actions(uri, (0, 0), (1, 6), diagnostics=[diag])
        fixes = [a for a in actions if "per-line" in a.get("title", "")]
        lines = _new_texts(fixes)[0].split("\n")
        assert lines[0] == "# line1"
        assert lines[1] == "# line2"


class TestIPConversionActions:
    def test_ipv4_offers_ipv6_mapped(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set addr 10.0.0.1\n")
        actions = lsp_server.code_actions(uri, (0, 10), (0, 10))
        assert any("IPv6-mapped" in t for t in _titles(actions))
        assert any("::ffff:" in s for s in _new_texts(actions))

    def test_ipv6_mapped_offers_ipv4(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set addr ::ffff:192.168.1.1\n")
        actions = lsp_server.code_actions(uri, (0, 12), (0, 12))
        assert any("IPv4" in t for t in _titles(actions))
        assert any("192.168.1.1" in s for s in _new_texts(actions))

    def test_non_mapped_ipv6_no_ipv4_action(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set addr ::1\n")
        actions = lsp_server.code_actions(uri, (0, 10), (0, 10))
        assert not any("IPv4" in t for t in _titles(_kinds(actions, "refactor")))

    def test_ipv4_with_prefix_preserves_suffix(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set net 10.0.0.0/8\n")
        actions = lsp_server.code_actions(uri, (0, 10), (0, 10))
        assert any("/8" in s for s in _new_texts(_kinds(actions, "refactor")))

    def test_non_ip_word_no_action(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set name hello\n")
        assert _kinds(lsp_server.code_actions(uri, (0, 10), (0, 10)), "refactor") == []

    def test_cursor_past_end_of_line_does_not_crash(self, lsp_server, uri_factory):
        # The request must succeed (no server-side IndexError); an empty result
        # comes back as JSON null over the wire rather than [].
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\n")
        result = lsp_server.code_actions(uri, (0, 999), (0, 999))
        assert result is None or isinstance(result, list)

    def test_cursor_past_last_line_does_not_crash(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\n")
        result = lsp_server.code_actions(uri, (99, 0), (99, 0))
        assert result is None or isinstance(result, list)


class TestGenerateDocstringAction:
    def test_offered_for_undocumented_proc(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {name} { puts $name }\n")
        sa = _kinds(lsp_server.code_actions(uri, (0, 0), (0, 0)), "source")
        doc = [a for a in sa if "docstring" in a["title"].lower()]
        assert len(doc) == 1
        assert "'greet'" in doc[0]["title"]

    def test_not_offered_for_documented_proc(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "# Already documented\nproc greet {name} { puts $name }\n")
        sa = _kinds(lsp_server.code_actions(uri, (1, 0), (1, 0)), "source")
        assert [a for a in sa if "docstring" in a["title"].lower()] == []

    def test_generated_edit_contains_param_tags(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc add {a b} { expr {$a + $b} }\n")
        sa = _kinds(lsp_server.code_actions(uri, (0, 0), (0, 0)), "source")
        doc = [a for a in sa if "docstring" in a["title"].lower()]
        snippets = _new_texts(doc)
        assert any("@param a" in s for s in snippets)
        assert any("@param b" in s for s in snippets)


class TestEvalListQuickFix:
    def test_eval_string_to_list_action(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, 'eval "process $x"\n')
        w101 = [d for d in diags if d.get("code") == "W101"]
        assert w101
        actions = lsp_server.code_actions(uri, (0, 0), (0, 17), diagnostics=w101)
        assert any("[list process $x]" in s for s in _new_texts(actions))
