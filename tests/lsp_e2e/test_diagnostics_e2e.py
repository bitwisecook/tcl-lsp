"""Push diagnostics, end-to-end against the packaged server.

Ported from the server-layer diagnostic tests in
``tests/test_lsp_server_actions_e2e.py`` and ``tests/test_diagnostics.py``.
The server advertises no pull provider, so these assert on the
``publishDiagnostics`` the server pushes after analysis, keyed by version.
"""

from __future__ import annotations

import textwrap


def _codes(diags) -> set[str]:
    return {str(d.get("code")) for d in diags}


def _on_line(diags, code: str) -> set[int]:
    """The set of start lines for diagnostics carrying ``code``."""
    return {
        d["range"]["start"]["line"] for d in diags if str(d.get("code")) == code and d.get("range")
    }


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


class TestSubcommandOptionArity:
    """End-to-end arity checks for per-subcommand option flags (issue #581).

    ``file link -symbolic linkName target`` is valid Tcl — the optional
    ``-linktype`` flag precedes the two positionals — but the packaged
    server used to emit ``Too many arguments for 'file link'`` because the
    subcommand's declared options were never skipped before the positional
    count.  These assert the real server's ``publishDiagnostics`` over the
    full pipeline, not just the in-process checker.
    """

    def test_file_link_symbolic_has_no_arity_error(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "file link -symbolic $dst $src\n")
        assert "E003" not in _codes(diags)

    def test_file_link_hard_has_no_arity_error(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "file link -hard $dst $src\n")
        assert "E003" not in _codes(diags)

    def test_file_link_too_many_positionals_is_e003(self, lsp_server, uri_factory):
        # No option flag: three positionals genuinely exceed the max of 2,
        # so the arity error must still fire (the fix skips options, not
        # real arguments).
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "file link $a $b $c\n")
        e003 = [d for d in diags if d.get("code") == "E003"]
        assert e003
        assert e003[0].get("severity") == 1  # DiagnosticSeverity.Error

    def test_string_match_nocase_has_no_arity_error(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "string match -nocase $pat $str\n")
        assert "E003" not in _codes(diags)


class TestDiagnosticCanaries:
    """One canary per analysis family, locked to the server's published output.

    ``test_diagnostics_e2e`` previously asserted only E002/W100/W128/W302 — far
    too thin to notice a whole dataflow family silently regressing.  The
    authoritative precision battery (``tests/test_fp_*.py`` +
    ``tests/test_ground_truth_tn_fn.py``, locked to real tclsh 9.0.3) still
    certifies the in-process analyser; these canaries certify that the analysis
    actually reaches the wire through the *server* pipeline, with a matched
    must-fire / must-stay-silent pair per family so a blanket suppression can't
    pass.
    """

    # -- W210: read of a possibly-unset variable --------------------------- #

    def test_w210_read_before_set_on_path_merge(self, lsp_server, uri_factory):
        # ``y`` is defined only inside the ``if`` arm; the read at the merge
        # point may see it unset.
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc p {x} {
                if {$x} {
                    set y 1
                }
                return $y
            }
        """)
        diags = lsp_server.open_ready(uri, src)
        assert "W210" in _codes(diags), _codes(diags)
        # The diagnostic anchors on the read site (`return $y`), not the def.
        assert 4 in _on_line(diags, "W210"), [d.get("range") for d in diags]

    def test_w210_use_after_unset(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set a 1\nunset a\nputs $a\n")
        assert "W210" in _codes(diags), _codes(diags)
        assert 2 in _on_line(diags, "W210")

    def test_w210_silent_when_set_on_all_paths(self, lsp_server, uri_factory):
        # Defined on both branches → no read-before-set (must-stay-silent).
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc p {x} {
                if {$x} {
                    set y 1
                } else {
                    set y 2
                }
                return $y
            }
        """)
        assert "W210" not in _codes(lsp_server.open_ready(uri, src))

    # -- W307: a variable used in command position ------------------------- #

    def test_w307_known_literal_non_command_fires(self, lsp_server, uri_factory):
        # ``cmd`` resolves to the literal ``foo`` which is not a command.
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "proc p {} {\n    set cmd foo\n    $cmd bar\n}\n")
        assert "W307" in _codes(diags), _codes(diags)

    def test_w307_silent_for_opaque_dispatch_target(self, lsp_server, uri_factory):
        # ``$self`` is an opaque parameter (a method-dispatch idiom); with no
        # known non-command literal value, W307 must not fire.
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "proc p {self} {\n    $self configure -x 1\n}\n")
        assert "W307" not in _codes(diags), _codes(diags)

    # -- W220: dead store -------------------------------------------------- #

    def test_w220_dead_store_fires(self, lsp_server, uri_factory):
        # ``x`` is set, then overwritten before any read → the first store is dead.
        uri = uri_factory()
        diags = lsp_server.open_ready(
            uri, "proc p {} {\n    set x 1\n    set x 2\n    return $x\n}\n"
        )
        assert "W220" in _codes(diags), _codes(diags)
        assert 1 in _on_line(diags, "W220")

    def test_w220_silent_when_value_is_read_between(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc p {} {\n    set x 1\n    puts $x\n    set x 2\n    return $x\n}\n"
        assert "W220" not in _codes(lsp_server.open_ready(uri, src))

    # -- Clean code stays clean (cross-family negative control) ------------ #

    def test_clean_dataflow_has_no_diagnostics(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc p {x} {
                set d [dict create a 1]
                if {[dict exists $d a]} {
                    return [dict get $d a]
                }
                return $x
            }
        """)
        assert lsp_server.open_ready(uri, src) == []


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
