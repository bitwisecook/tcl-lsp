"""Error-recovery correctness contract — end-to-end against the packaged server.

This suite is deliberately **implementation-agnostic**: it asserts the
*observable* recovery behaviour over the LSP protocol, never a Python internal or
a Python-specific diagnostic code.  It is the contract the recovery engine must
satisfy in *any* implementation — Python today, Rust after the port — so the
same assertions can be run unchanged against the Rust server to prove the port
is correct.

The stable contracts (each a property any correct recovery must have):

  C1  an unterminated ``[`` / ``"`` / ``{`` is flagged with an error diagnostic;
  C2  recovery is non-fatal and bounded — the rest of the document is still
      analysed (a proc after the break appears in document symbols) instead of
      being swallowed to EOF;
  C3  the recovered token stream is well-formed (5-int groups, in-document
      positions) and re-lexes commands after the break;
  C4  the published diagnostic set never contains exact duplicates;
  C5  edits that introduce a break raise the error and edits that fix it clear
      it, version by version;
  C6  well-formed code produces no recovery error.

Brittle, implementation-specific facts (the exact ``E200`` vs ``E201`` code, the
precise split offset) are intentionally NOT asserted here — they belong to the
language-internal unit/differential/fuzz suites.
"""

from __future__ import annotations

from ._lsp_helpers import decode_semantic_tokens

# Parser/recovery diagnostic family (unclosed/extra-delimiter).  A conforming
# implementation may pick any code in this family or word the message its own
# way; the contract is only that *some* error in this space is reported.
_RECOVERY_CODES = {"E200", "E201", "E202", "E203", "E204", "E205", "E206"}
_RECOVERY_WORDS = ("missing", "close", "unterminated", "unclosed", "unbalanced", "extra characters")


def _codes(diags) -> set[str]:
    return {str(d.get("code")) for d in (diags or [])}


def _is_recovery_error(d) -> bool:
    code = str(d.get("code"))
    msg = (d.get("message") or "").lower()
    sev = d.get("severity")
    if code in _RECOVERY_CODES:
        return True
    return sev == 1 and any(w in msg for w in _RECOVERY_WORDS)


def _has_recovery_error(diags) -> bool:
    return any(_is_recovery_error(d) for d in (diags or []))


def _symbol_names(syms) -> list[str]:
    out: list[str] = []

    def walk(items):
        for s in items or []:
            out.append(s.get("name"))
            walk(s.get("children"))

    walk(syms)
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


def _diag_identity(d) -> tuple:
    rng = d.get("range", {})
    return (
        str(d.get("code")),
        rng.get("start", {}).get("line"),
        rng.get("start", {}).get("character"),
        rng.get("end", {}).get("line"),
        rng.get("end", {}).get("character"),
        d.get("message"),
    )


# --------------------------------------------------------------------------- #
# C1 — an unterminated delimiter is flagged
# --------------------------------------------------------------------------- #


class TestC1UnterminatedIsFlagged:
    def test_unterminated_bracket(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set x [foo bar\nputs hi\n")
        assert _has_recovery_error(diags), f"unterminated [ should be flagged; got {_codes(diags)}"

    def test_unterminated_brace(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "proc p {} {\n  set y [foo\n}\nset\n")
        assert _has_recovery_error(diags), f"unterminated delimiter expected; got {_codes(diags)}"

    def test_well_formed_is_not_flagged(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set x [foo bar]\nputs hi\n")
        assert not _has_recovery_error(diags), f"well-formed flagged a recovery error: {_codes(diags)}"


# --------------------------------------------------------------------------- #
# C2 — recovery is non-fatal: the tail is still analysed
# --------------------------------------------------------------------------- #


class TestC2RecoveryContinues:
    def test_proc_after_unterminated_bracket_is_a_symbol(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x [foo\nproc recovered_after_bracket {} {}\n")
        names = _symbol_names(lsp_server.document_symbols(uri))
        assert "recovered_after_bracket" in names, f"tail proc not recovered; symbols={names}"

    def test_proc_after_unterminated_brace_body_is_a_symbol(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "namespace eval n {\nproc recovered_in_ns {} {}\n")
        names = _symbol_names(lsp_server.document_symbols(uri))
        assert "recovered_in_ns" in names, f"tail proc not recovered; symbols={names}"

    def test_proc_after_multiple_breaks_is_a_symbol(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "set a [foo\nset b 2\nset c [bar\nproc recovered_after_two {} {}\n"
        lsp_server.open_ready(uri, src)
        names = _symbol_names(lsp_server.document_symbols(uri))
        assert "recovered_after_two" in names, f"tail proc not recovered; symbols={names}"

    def test_bare_set_after_break_still_arity_errors(self, lsp_server, uri_factory):
        # A bare `set` after a single break must be analysed as a command (and so
        # raise its arity error) — proof the tail is parsed as code, not text.
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set x [foo bar\nset\n")
        assert "E002" in _codes(diags), f"tail `set` should arity-error; got {_codes(diags)}"

    def test_bare_set_after_unterminated_expr_brace_still_analysed(self, lsp_server, uri_factory):
        # An unterminated braced *expression* (`if {…`) must also recover so the
        # following command is analysed — a bare `set` still arity-errors.
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "if {$x > 5\nset\n")
        assert "E002" in _codes(diags), f"tail `set` after `if {{` should arity-error; got {_codes(diags)}"


# --------------------------------------------------------------------------- #
# C3 — recovered token stream is well-formed
# --------------------------------------------------------------------------- #


class TestC3RecoveredTokens:
    def test_command_after_break_is_tokenised(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x [foo bar\nputs hi\n")
        toks = _typed(lsp_server, uri)
        assert any(t["line"] == 1 and t["type"] == "function" for t in toks), (
            f"expected `puts` recovered as a function on line 1; got {toks}"
        )

    def test_nested_break_recovers_following_lines(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc p {} {\n  set x [foo\n}\nputs done\n")
        toks = _typed(lsp_server, uri)
        assert any(t["line"] == 3 and t["type"] == "function" for t in toks), toks

    def test_tokens_well_formed_for_pathological_input(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = 'proc p {} {\n  set x [foo "bar\n  if {1} {\n    puts [baz\n}\nputs end\n'
        lsp_server.open_ready(uri, src)
        raw = lsp_server.semantic_tokens(uri)
        data = raw.get("data") if isinstance(raw, dict) else None
        assert data is not None and len(data) % 5 == 0, "token data must be 5-int groups"
        n_lines = src.count("\n") + 1
        for t in _typed(lsp_server, uri):
            assert 0 <= t["line"] < n_lines, f"token line out of range: {t}"
            assert t["length"] >= 0

    def test_deeply_nested_unterminated_does_not_hang(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set x [a [b [c [d [e\nputs tail\n")
        assert isinstance(diags, list)  # returned promptly, no hang/crash

    def test_crlf_document_recovers(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x [foo\r\nproc recovered_crlf {} {}\r\n")
        names = _symbol_names(lsp_server.document_symbols(uri))
        assert "recovered_crlf" in names, f"CRLF tail not recovered; symbols={names}"


# --------------------------------------------------------------------------- #
# C4 — no duplicate diagnostics
# --------------------------------------------------------------------------- #


class TestC4NoDuplicateDiagnostics:
    def test_no_exact_duplicate_published(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, 'set x "\nif {1} {\n  puts [foo\n}\nset\n')
        seen: set[tuple] = set()
        for d in diags:
            ident = _diag_identity(d)
            assert ident not in seen, f"duplicate diagnostic published: {ident}"
            seen.add(ident)


# --------------------------------------------------------------------------- #
# C5 — edits toggle recovery
# --------------------------------------------------------------------------- #


def _del_close_bracket():
    return [{"range": {"start": {"line": 0, "character": 14},
                       "end": {"line": 0, "character": 15}}, "text": ""}]


def _insert_close_bracket():
    return [{"range": {"start": {"line": 0, "character": 14},
                       "end": {"line": 0, "character": 14}}, "text": "]"}]


class TestC5EditsToggleRecovery:
    def test_break_then_fix(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set x [foo bar]\nset\n")
        assert not _has_recovery_error(diags)
        lsp_server.change_document(uri, 2, _del_close_bracket())
        diags = lsp_server.await_diagnostics(uri, version=2)
        assert _has_recovery_error(diags), f"break should flag; got {_codes(diags)}"
        lsp_server.change_document(uri, 3, _insert_close_bracket())
        diags = lsp_server.await_diagnostics(uri, version=3)
        assert not _has_recovery_error(diags), f"fix should clear; got {_codes(diags)}"

    def test_rapid_toggling_stays_consistent(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x [foo bar]\nset\n")
        version = 1
        for _ in range(3):
            version += 1
            lsp_server.change_document(uri, version, _del_close_bracket())
            diags = lsp_server.await_diagnostics(uri, version=version)
            assert _has_recovery_error(diags), f"v{version} broken; got {_codes(diags)}"
            version += 1
            lsp_server.change_document(uri, version, _insert_close_bracket())
            diags = lsp_server.await_diagnostics(uri, version=version)
            assert not _has_recovery_error(diags), f"v{version} clean; got {_codes(diags)}"
