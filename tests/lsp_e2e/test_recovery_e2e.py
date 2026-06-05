"""Error-recovery, end-to-end against the packaged server.

These pin the *user-visible* behaviour the single recovering-detection pass must
keep robust:

- An unterminated delimiter still produces a recovery diagnostic, and the
  insert-missing-delimiter quick-fix is still offered where the heuristics fire.
- Crucially, recovery *splits* the broken command so the code **after** it is
  still tokenised (semantic tokens) **and** analysed (diagnostics) rather than
  being swallowed to EOF — a bare ``set`` after the break still raises its arity
  error.

The recovered token stream and the recovery diagnostics flow from the same
``detect_recovery`` pass, so these guard that both surfaces stay correct through
the whole server pipeline.
"""

from __future__ import annotations

from ._lsp_helpers import decode_semantic_tokens


def _codes(diags) -> set[str]:
    return {str(d.get("code")) for d in (diags or [])}


def _new_texts(actions) -> list[str]:
    out: list[str] = []
    for action in actions or []:
        edit = action.get("edit") or {}
        for edits in (edit.get("changes") or {}).values():
            out.extend(e["newText"] for e in edits)
        for change in edit.get("documentChanges") or []:
            out.extend(e["newText"] for e in change.get("edits") or [])
    return out


def _legend(lsp_server):
    return lsp_server.initialize_result["capabilities"]["semanticTokensProvider"]["legend"][
        "tokenTypes"
    ]


def _typed(lsp_server, uri):
    legend = _legend(lsp_server)
    tokens = decode_semantic_tokens(lsp_server.semantic_tokens(uri))
    for tok in tokens:
        tok["type"] = legend[tok["type"]]
    return tokens


class TestRecoveryEnablesTailAnalysis:
    """The most important robustness property: a broken delimiter must not
    swallow the rest of the document — analysis continues past it."""

    def test_bare_set_after_unterminated_bracket_still_arity_errors(
        self, lsp_server, uri_factory
    ):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set x [foo bar\nset\n")
        codes = _codes(diags)
        assert "E002" in codes, f"tail `set` should still arity-error; got {codes}"
        assert "E200" in codes, f"unterminated `[` should emit a recovery diagnostic; got {codes}"

    def test_bare_set_after_nested_unterminated_bracket_still_analysed(
        self, lsp_server, uri_factory
    ):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "proc p {} {\n  set y [foo\n}\nset\n")
        codes = _codes(diags)
        assert "E002" in codes, f"tail `set` past a recovered nested body; got {codes}"

    def test_well_formed_has_no_recovery_diagnostic(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set x [foo bar]\nputs hi\n")
        assert not ({"E200", "E201", "E202", "E203"} & _codes(diags))


class TestRecoverySuggestionPublished:
    """Where a recovery heuristic fires (here a nested unterminated ``[``), the
    E201 recovery suggestion must still be published as a diagnostic — this is
    the surface the editor renders and drives quick-fixes from."""

    def test_nested_unterminated_bracket_publishes_e201(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "proc p {} {\n  set y [foo\n}\nset\n")
        e201 = [d for d in diags if d.get("code") == "E201"]
        assert e201, f"expected an E201 recovery suggestion; got {_codes(diags)}"
        assert "bracket" in e201[0].get("message", "").lower()


class TestRecoverySemanticTokens:
    def test_command_after_unterminated_bracket_is_recovered(self, lsp_server, uri_factory):
        # Recovery closes `[foo bar` at the line break so the following
        # `puts hi` is lexed as a real command rather than swallowed into the
        # unterminated command substitution.
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x [foo bar\nputs hi\n")
        toks = _typed(lsp_server, uri)
        assert any(t["line"] == 1 and t["type"] == "function" for t in toks), (
            f"expected `puts` on line 1 recovered as a function; got {toks}"
        )

    def test_command_after_unterminated_quote_is_recovered(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'puts "\nputs hi\n')
        toks = _typed(lsp_server, uri)
        assert any(t["line"] == 1 and t["type"] == "function" for t in toks), (
            f"expected recovered `puts` on line 1; got {toks}"
        )

    def test_nested_unterminated_bracket_recovers_following_lines(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc p {} {\n  set x [foo\n}\nputs done\n")
        toks = _typed(lsp_server, uri)
        assert any(t["line"] == 3 and t["type"] == "function" for t in toks), (
            f"expected `puts` on line 3 after a recovered nested body; got {toks}"
        )


class TestRecoveryTracksEdits:
    def test_introducing_then_fixing_unterminated_bracket(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set x [foo bar]\nset\n")
        assert "E200" not in _codes(diags)
        # Delete the closing `]` (line 0, char 14) -> unterminated.
        lsp_server.change_document(
            uri,
            2,
            [
                {
                    "range": {
                        "start": {"line": 0, "character": 14},
                        "end": {"line": 0, "character": 15},
                    },
                    "text": "",
                }
            ],
        )
        diags = lsp_server.await_diagnostics(uri, version=2)
        assert "E200" in _codes(diags), f"expected recovery diagnostic after edit; got {_codes(diags)}"
        # Put it back -> the recovery diagnostic clears.
        lsp_server.change_document(
            uri,
            3,
            [
                {
                    "range": {
                        "start": {"line": 0, "character": 14},
                        "end": {"line": 0, "character": 14},
                    },
                    "text": "]",
                }
            ],
        )
        diags = lsp_server.await_diagnostics(uri, version=3)
        assert "E200" not in _codes(diags), f"recovery diagnostic should clear; got {_codes(diags)}"
