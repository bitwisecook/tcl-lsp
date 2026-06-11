"""Universal invariants + adversarial robustness, end-to-end against the server.

Most suites assert *what* a provider returns for a specific input.  These assert
properties that must hold for **every** provider on **every** document — the
contracts a client relies on and silently mis-renders (or crashes on) when
broken:

  * every ``Range`` a provider emits is well-formed — non-negative, ``start <=
    end``, on real lines, ``start`` column within its line in UTF-16 units;
  * ``rename`` / code-action ``WorkspaceEdit`` arrays contain no overlapping or
    duplicate edits (the client applies them as a batch);
  * adversarial documents — unterminated delimiters, deep nesting, multibyte /
    emoji, CRLF, a BOM, an empty file, a large file — never make the server
    hang, crash, or emit a malformed span.

The robustness corpus is the second oracle: a server that wedges or throws on
hostile input fails here even if its happy-path tests pass.
"""

from __future__ import annotations

import pytest

from ._lsp_helpers import range_violations, workspace_edit_violations

# A corpus spanning clean code, every multibyte/EOL hazard, and the recovery
# paths.  Reused by the range-invariant and robustness batteries.
_CORPUS = {
    "clean": 'proc greet {name} {\n    puts "Hello $name"\n}\ngreet World\n',
    "vars": "set x 1\nset y $x\nputs [expr {$x + $y}]\n",
    "multibyte": 'set s "café résumé naïve"\nputs $s\nproc f {s} { return $s }\nf $s\n',
    "emoji": 'set e "😀 🚀 🐫 done"\nputs $e\nset n 1\n',
    "unicode_ident": "set 日本語 1\nputs ${日本語}\n",
    "crlf": "proc p {} {\r\n    set x 1\r\n}\r\np\r\n",
    "bom": "﻿puts hello\nset x 1\n",
    "unterminated_bracket": "set x [foo bar\nproc recovered {} {}\nputs hi\n",
    "unterminated_brace": "proc p {} {\n  set y [foo\n}\nset z 1\n",
    "unterminated_quote": 'set s "open\nputs done\n',
    "deep_nesting": "set x [a [b [c [d [e [f [g\nputs tail\n",
    "empty": "",
    "blank_lines": "\n\n\n",
    "only_comment": "# just a comment\n",
    "large": "".join(f"proc p{i} {{a b}} {{ return [expr {{$a + $b}}] }}\n" for i in range(400)),
}


def _exercise_all_providers(lsp_server, uri: str):
    """Run a broad request battery against ``uri``; return ``{name: result}``.

    Every request must return (not raise / not time out).  Positions are chosen
    to land on real tokens in the smaller corpus entries and are harmless
    elsewhere (providers return empty for an uninteresting position).
    """
    results: dict[str, object] = {}
    results["documentSymbol"] = lsp_server.document_symbols(uri)
    results["foldingRange"] = lsp_server.folding_range(uri)
    results["semanticTokens"] = lsp_server.semantic_tokens(uri)
    results["hover@0,1"] = lsp_server.hover(uri, 0, 1)
    results["definition@0,1"] = lsp_server.definition(uri, 0, 1)
    results["references@0,5"] = lsp_server.references(uri, 0, 5)
    results["highlight@0,5"] = lsp_server.document_highlight(uri, 0, 5)
    results["completion@0,1"] = lsp_server.completion(uri, 0, 1)
    results["signatureHelp@1,8"] = lsp_server.signature_help(uri, 1, 8)
    results["selectionRange@0,1"] = lsp_server.selection_range(uri, [(0, 1)])
    results["codeAction@0,0-1,0"] = lsp_server.code_actions(uri, (0, 0), (1, 0))
    results["documentLink"] = lsp_server.document_links(uri)
    results["codeLens"] = lsp_server.code_lens(uri)
    return results


class TestRangeInvariants:
    """Every Range from every provider is well-formed, over the whole corpus."""

    @pytest.mark.parametrize("name", sorted(_CORPUS))
    def test_all_provider_ranges_well_formed(self, name, lsp_server, uri_factory):
        source = _CORPUS[name]
        uri = uri_factory()
        lsp_server.open_ready(uri, source)
        results = _exercise_all_providers(lsp_server, uri)
        violations: list[str] = []
        for req, res in results.items():
            violations += range_violations(res, source, label=f"{name}/{req}")
        assert not violations, "malformed ranges:\n" + "\n".join(violations)


class TestWorkspaceEditInvariants:
    def test_rename_edits_do_not_overlap(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc greet {name} { puts $name }\ngreet a\ngreet b\ngreet c\n"
        lsp_server.open_ready(uri, src)
        edit = lsp_server.rename(uri, 0, 6, "welcome")
        assert not workspace_edit_violations(edit, label="rename")

    def test_multisite_variable_rename_edits_are_disjoint(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc p {} {\n    set count 0\n    incr count\n    incr count\n    return $count\n}\n"
        lsp_server.open_ready(uri, src)
        edit = lsp_server.rename(uri, 1, 8, "counter")
        assert not workspace_edit_violations(edit, label="var-rename")

    def test_code_action_edits_do_not_overlap(self, lsp_server, uri_factory):
        uri = uri_factory()
        # W100 (unbraced expr) offers a safe wrap fix; its edits must be disjoint.
        lsp_server.open_ready(uri, "if $a { puts yes }\nif $b { puts no }\n")
        actions = lsp_server.code_actions(uri, (0, 0), (2, 0)) or []
        for action in actions:
            assert not workspace_edit_violations(action.get("edit"), label="code-action")


class TestRobustnessAdversarial:
    """Hostile inputs never hang/crash the server and never yield bad spans."""

    @pytest.mark.parametrize("name", sorted(_CORPUS))
    def test_server_survives_and_responds(self, name, lsp_server, uri_factory):
        source = _CORPUS[name]
        uri = uri_factory()
        # ``open_ready`` itself proves the analysis pipeline didn't wedge.
        lsp_server.open_ready(uri, source)
        # The full battery must return for every adversarial input (the request
        # helpers raise on a JSON-RPC error or time out, so reaching the end is
        # the assertion) and every span must be well-formed.
        results = _exercise_all_providers(lsp_server, uri)
        violations: list[str] = []
        for req, res in results.items():
            violations += range_violations(res, source, label=f"{name}/{req}")
        assert not violations, "malformed ranges on adversarial input:\n" + "\n".join(violations)

    def test_server_still_responsive_after_adversarial_burst(self, lsp_server, uri_factory):
        # After hammering hostile documents, a normal request on a fresh doc must
        # still answer correctly — proves no document poisoned global state.
        for name in ("deep_nesting", "unterminated_quote", "emoji", "large"):
            u = uri_factory()
            lsp_server.open_ready(u, _CORPUS[name])
            _exercise_all_providers(lsp_server, u)
        sane = uri_factory()
        lsp_server.open_ready(sane, "puts hello\n")
        from ._lsp_helpers import hover_text

        assert "puts" in hover_text(lsp_server.hover(sane, 0, 2))
