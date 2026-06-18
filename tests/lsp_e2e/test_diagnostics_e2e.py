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


class TestIndirectArrayIdiom:
    """FP-STY-12: ``${var}(idx)`` in a varname position is the indirect-array-
    element idiom (``var`` holds the array name), not a broken ``$var(idx)`` —
    so neither W216 nor W212 fire through the server pipeline.  A value-position
    ``${arr}(x)`` still fires W216."""

    def test_set_indirect_array_silent(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set token ::http::1\nset ${token}(status) eof\n")
        assert "W216" not in _codes(diags), _codes(diags)
        assert "W212" not in _codes(diags), _codes(diags)

    def test_info_exists_indirect_array_silent(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "info exists ${token}(-pipeline)\n")
        assert "W216" not in _codes(diags)
        assert "W212" not in _codes(diags)

    def test_unset_and_vwait_indirect_array_silent(self, lsp_server, uri_factory):
        for src in ("unset ${tok}(socketcoro)\n", "vwait ${token}(status)\n"):
            uri = uri_factory()
            assert "W216" not in _codes(lsp_server.open_ready(uri, src)), src

    def test_value_position_still_fires_w216(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "puts ${arr}(x)\n")
        assert "W216" in _codes(diags), _codes(diags)
        assert 0 in _on_line(diags, "W216")

    def test_bare_dollar_name_still_fires_w212(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert "W212" in _codes(lsp_server.open_ready(uri, "set $x v\n"))


class TestOverridableLibraryProcs:
    """FP-STY-13: redefining an overridable Tcl *library* proc (``unknown``,
    ``history``, ``auto_*`` …) is not shadowing a C built-in — no W113.
    Redefining a genuine built-in (``set``/``clock``) still fires."""

    def test_unknown_override_silent(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert "W113" not in _codes(lsp_server.open_ready(uri, "proc unknown args { return }\n"))

    def test_library_procs_silent(self, lsp_server, uri_factory):
        for name in ("history", "auto_execok", "tcl_findLibrary", "pkg_mkIndex"):
            uri = uri_factory()
            src = f"proc {name} {{args}} {{ return }}\n"
            assert "W113" not in _codes(lsp_server.open_ready(uri, src)), name

    def test_c_builtin_override_still_fires(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert "W113" in _codes(lsp_server.open_ready(uri, "proc set {a b} { return }\n"))

    def test_non_bytecompiled_c_command_still_fires(self, lsp_server, uri_factory):
        # clock/after/socket/glob are C commands (not byte-compiled) but must
        # still fire — the library-proc exemption must not over-reach.
        for name in ("clock", "after", "socket", "glob"):
            uri = uri_factory()
            src = f"proc {name} {{a}} {{ return }}\n"
            assert "W113" in _codes(lsp_server.open_ready(uri, src)), name


class TestSingleVarBodyW105:
    """FP-STY-14: a body argument that is a single bare variable substitution
    (``eval $cmd``, ``$state(-command)``, ``after 0 $coroName``) is a
    script-valued reference, not an inline block — no W105 through the server
    pipeline.  A quoted/composite interpolated body still fires."""

    def test_eval_single_var_body_silent(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert "W105" not in _codes(lsp_server.open_ready(uri, "eval $cmd\n"))

    def test_callback_dispatch_body_silent(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "namespace eval :: $state(-command) $token\n"
        assert "W105" not in _codes(lsp_server.open_ready(uri, src))

    def test_after_and_dynamic_proc_silent(self, lsp_server, uri_factory):
        for src in ("after 0 $coroName\n", "proc $fakeName $arglist $body\n"):
            uri = uri_factory()
            assert "W105" not in _codes(lsp_server.open_ready(uri, src)), src

    def test_quoted_interpolated_body_still_fires(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, 'eval "do $script"\n')
        assert "W105" in _codes(diags), _codes(diags)
        assert 0 in _on_line(diags, "W105")

    def test_composite_body_still_fires(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert "W105" in _codes(lsp_server.open_ready(uri, "eval $cmd$args\n"))


class TestDollarBeforeCloseQuoteW306:
    """FP-STY-15: a ``$`` immediately before a closing ``"`` (the regex
    end-anchor ``"^foo$"`` / ``"\\n$"``) is literal — the lexer must not merge
    the quoted word with the next, so no E002/E205 and no spurious W306.  A
    live ``$bar`` in a quoted pattern still fires W306."""

    def test_regsub_end_anchor_no_errors(self, lsp_server, uri_factory):
        uri = uri_factory()
        codes = _codes(lsp_server.open_ready(uri, 'regsub "\\n$" $msg "" out\n'))
        assert "E002" not in codes, codes
        assert "E205" not in codes, codes
        assert "W306" not in codes, codes

    def test_string_match_end_anchor_no_arity(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert "E002" not in _codes(lsp_server.open_ready(uri, 'string match "abc$" $x\n'))

    def test_regex_end_anchor_no_w306(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert "W306" not in _codes(lsp_server.open_ready(uri, 'regexp -- "^foo$" $text\n'))

    def test_live_var_in_quoted_pattern_still_fires(self, lsp_server, uri_factory):
        uri = uri_factory()
        assert "W306" in _codes(lsp_server.open_ready(uri, 'regexp -- "^foo$bar" $text\n'))


class TestControlFlowRBSFamilyE2E:
    """W210 read-before-set, control-flow modelling family (PR #634).

    PR #634 fixed a family of false W210s rooted in imprecise control-flow
    modelling: loop bodies that provably run at least once, branches ended by a
    terminator (``tailcall``), and opaque ``switch`` arms that cannot complete
    normally.  The in-process precision battery is ``tests/test_fp_rbs.py``
    (locked to real tclsh 9.0.3); these certify a representative slice reaches
    the wire through the *server* pipeline, each as a matched
    must-stay-silent / must-fire pair so a blanket suppression can't pass.
    """

    # -- tailcall ends straight-line flow (FP-RBS-13) ---------------------- #

    def test_tailcall_terminated_branch_silent(self, lsp_server, uri_factory):
        # ``tailcall g`` returns from the proc, so ``return $result`` is reached
        # only via the else branch where ``result`` is set.
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc f {cond} {
                if {$cond} {
                    tailcall g
                } else {
                    set result 1
                }
                return $result
            }
        """)
        assert "W210" not in _codes(lsp_server.open_ready(uri, src))

    def test_non_terminating_branch_still_fires(self, lsp_server, uri_factory):
        # Replace the tailcall with a completing command → ``result`` is
        # maybe-unset on the then-path, so W210 must fire.
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc f {cond} {
                if {$cond} {
                    puts hi
                } else {
                    set result 1
                }
                return $result
            }
        """)
        diags = lsp_server.open_ready(uri, src)
        assert "W210" in _codes(diags), _codes(diags)

    # -- non-empty-literal foreach runs its body (FP-RBS-17) --------------- #

    def test_foreach_non_empty_literal_silent(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc f {} {\n    foreach x {1 2 3} { set y $x }\n    puts $y\n}\n"
        assert "W210" not in _codes(lsp_server.open_ready(uri, src))

    def test_foreach_empty_literal_still_fires(self, lsp_server, uri_factory):
        # An empty literal never runs the body → ``y`` is unset.
        uri = uri_factory()
        src = "proc f {} {\n    foreach x {} { set y $x }\n    puts $y\n}\n"
        assert "W210" in _codes(lsp_server.open_ready(uri, src))

    # -- for whose condition is true on entry runs its body (FP-RBS-18) ---- #

    def test_for_true_on_entry_silent(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc f {} {\n    for {set i 0} {$i < 3} {incr i} { set y $i }\n    puts $y\n}\n"
        assert "W210" not in _codes(lsp_server.open_ready(uri, src))

    def test_for_false_on_entry_still_fires(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc f {} {\n    for {set i 5} {$i < 3} {incr i} { set y $i }\n    puts $y\n}\n"
        assert "W210" in _codes(lsp_server.open_ready(uri, src))

    # -- while 1 only exits via break, where the var is set (FP-RBS-16) ---- #

    def test_while1_break_set_silent(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc f {} {\n    while 1 { set y 1; break }\n    puts $y\n}\n"
        assert "W210" not in _codes(lsp_server.open_ready(uri, src))

    def test_normal_while_still_fires(self, lsp_server, uri_factory):
        # A non-constant condition may run zero times → maybe-unset read.
        uri = uri_factory()
        src = "proc f {n} {\n    while {$n > 0} { set y 1; incr n -1 }\n    puts $y\n}\n"
        assert "W210" in _codes(lsp_server.open_ready(uri, src))

    # -- opaque switch whose every arm exits is a terminator (FP-RBS-15) --- #

    def test_all_arms_return_makes_trailing_read_unreachable(self, lsp_server, uri_factory):
        # Every arm returns, so ``puts $y`` is unreachable dead code → no W210.
        uri = uri_factory()
        src = (
            "proc f {x} {\n"
            "    switch -glob -- $x { a* { return 1 } default { return 2 } }\n"
            "    puts $y\n"
            "}\n"
        )
        assert "W210" not in _codes(lsp_server.open_ready(uri, src))

    def test_no_default_switch_falls_through_fires(self, lsp_server, uri_factory):
        # Without a ``default`` an unmatched subject falls through to the read,
        # so the switch is not a terminator and ``y`` is maybe-unset.
        uri = uri_factory()
        src = (
            "proc f {x} {\n"
            "    switch -glob -- $x { a* { return 1 } b* { return 2 } }\n"
            "    puts $y\n"
            "}\n"
        )
        assert "W210" in _codes(lsp_server.open_ready(uri, src))

    # -- opaque switch must-define excludes non-completing arms (FP-RBS-14) - #

    def test_returning_arm_excluded_from_must_define(self, lsp_server, uri_factory):
        # The ``a*`` arm returns before reaching ``puts $y``; the only path that
        # does (default) sets ``y`` → definitely defined, no W210.
        uri = uri_factory()
        src = (
            "proc f {x} {\n"
            "    switch -glob -- $x { a* { return 0 } default { set y 2 } }\n"
            "    puts $y\n"
            "}\n"
        )
        assert "W210" not in _codes(lsp_server.open_ready(uri, src))

    def test_break_arm_escaping_loop_still_fires(self, lsp_server, uri_factory):
        # Codex regression on #634: ``break``/``continue`` are loop-jumps, not
        # proc-exits, so a break arm does NOT define the other arm's var on the
        # path that escapes the loop — ``y`` is maybe-unset, W210 must fire.
        uri = uri_factory()
        src = (
            "proc f {} {\n"
            "    foreach x {a} { switch -glob -- $x { a* { break } default { set y 1 } } }\n"
            "    puts $y\n"
            "}\n"
        )
        assert "W210" in _codes(lsp_server.open_ready(uri, src))


class TestWhenBodyDialectGatingE2E:
    """``when`` is an iRules-only builtin (PR #640).

    Under plain Tcl it is an unknown would-be user command whose braced argument
    is opaque *data*, not a handler script — so its body must not be analysed:
    no W123 on a body command, no spurious W210 read-before-set.  (The iRules
    side, where the body IS analysed, lives in ``test_irules_e2e.py``.)
    """

    def test_when_body_not_analysed_under_plain_tcl(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "when HTTP_REQUEST {\n    boguscmd $undefvar\n}\n")
        # ``when`` itself is unknown under Tcl, but the opaque body must not be
        # recursed into: no W123 naming the body command, no W210 on its var.
        body_w123 = [
            d
            for d in diags
            if str(d.get("code")) == "W123" and "boguscmd" in (d.get("message") or "")
        ]
        assert body_w123 == [], diags
        assert "W210" not in _codes(diags), _codes(diags)


class TestConstantStringConditionFoldE2E:
    """I230 (always-true/false condition; alternate branch unreachable) now
    folds ``==``/``!=`` on string operands, matching Tcl's polymorphic compare
    (``expr {"foo" == "foo"}`` -> 1) — previously only the ``eq``/``ne``
    spelling folded (PR #640)."""

    def test_double_equals_string_condition_folds(self, lsp_server, uri_factory):
        for op in ("==", "eq"):
            uri = uri_factory()
            src = f'set x foo\nif {{$x {op} "foo"}} {{ puts hi }}\n'
            assert "I230" in _codes(lsp_server.open_ready(uri, src)), op

    def test_bang_equals_string_condition_folds(self, lsp_server, uri_factory):
        for op in ("!=", "ne"):
            uri = uri_factory()
            src = f'set x foo\nif {{$x {op} "foo"}} {{ puts hi }}\n'
            assert "I230" in _codes(lsp_server.open_ready(uri, src)), op


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


class TestSoundnessRegressionsE2E:
    """End-to-end coverage for three latent W210 soundness bugs surfaced
    while reviewing the Rust port and fixed in the analyser.  Ground truth is
    real tclsh 9.0.3; these drive the packaged server's ``publishDiagnostics``
    over the full pipeline, not just the in-process checker.
    """

    # -- Omitted-arg call-site constants are poisoned (interproc) ---------- #

    def test_omitted_default_arg_does_not_hide_read_before_set(self, lsp_server, uri_factory):
        # ``p`` (slot 0 omitted → default ``x == 0``) leaves ``y`` unset, so
        # ``puts $y`` is a real read-before-set; the literal ``1`` passed by
        # ``p 1`` must not be bound as a constant for ``x``.
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc p {{x 0}} {
                if {$x} {
                    set y 5
                }
                puts $y
            }
            p
            p 1
        """)
        diags = lsp_server.open_ready(uri, src)
        assert "W210" in _codes(diags), _codes(diags)
        assert 4 in _on_line(diags, "W210"), [d.get("range") for d in diags]

    def test_uniform_literal_arg_still_binds_silent(self, lsp_server, uri_factory):
        # Every caller passes ``1`` at slot 0 → the constant binding holds, the
        # ``if {$x}`` body is provably taken, ``y`` is always set: stay silent.
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc q {{x 0}} {
                if {$x} {
                    set y 5
                }
                puts $y
            }
            q 1
            q 1
        """)
        assert "W210" not in _codes(lsp_server.open_ready(uri, src))

    # -- regexp ``-expanded`` is not unconditionally literal-safe ---------- #

    def test_regexp_expanded_whitespace_pattern_silent(self, lsp_server, uri_factory):
        # ``-expanded`` ignores unescaped whitespace, so ``{a b}`` matches the
        # substring ``ab`` and writes ``v``; reading ``v`` must not fire W210.
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc g {input} {
                regexp -expanded {a b} $input v
                puts $v
            }
        """)
        assert "W210" not in _codes(lsp_server.open_ready(uri, src))

    def test_regexp_expanded_clean_literal_still_fires(self, lsp_server, uri_factory):
        # A whitespace/``#``-free literal stays provably literal: ``{x}`` never
        # matches ``X``, never writes ``w`` → reading ``w`` is read-before-set.
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc g {} {
                regexp -expanded {x} X w
                puts $w
            }
        """)
        assert "W210" in _codes(lsp_server.open_ready(uri, src))

    # -- try body throw keeps its handler exception edge ------------------- #

    def test_try_body_throw_keeps_handler_defs_silent(self, lsp_server, uri_factory):
        # ``x`` is set before the only throw, so the handler always sees it
        # (tclsh prints ``1``): stay silent.
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc f {} {
                try {
                    set x 1
                    error boom
                } on error {} {
                    puts $x
                }
            }
        """)
        assert "W210" not in _codes(lsp_server.open_ready(uri, src))

    def test_try_body_earlier_conditional_throw_fires(self, lsp_server, uri_factory):
        # The handler is reachable from every throw point; ``x`` is unset on the
        # earlier ``if {$c} {error a}`` path, so the read is maybe-unset.
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc f {c} {
                try {
                    if {$c} { error a }
                    set x 1
                    error b
                } on error {} {
                    puts $x
                }
            }
        """)
        assert "W210" in _codes(lsp_server.open_ready(uri, src))
