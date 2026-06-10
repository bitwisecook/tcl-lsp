"""Existence-check handling for ``info exists`` / ``array exists``.

These commands test whether a variable is set rather than reading its value, so:

* they never trigger ``W210`` ("read before set");
* when the target is provably absent or present, SCCP folds the check to a
  constant, the existing ``I230`` constant-branch diagnostic fires, and DCE can
  drop the dead arm;
* in the true region of a guard the variable is known to exist (reads are
  safe), while the false region still flags a read-before-set.

Regression coverage for issue #500.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.core_analyses import analyse_source
from server.features.diagnostics import get_diagnostics


def _codes(src: str, *keep: str) -> list[str]:
    wanted = set(keep) if keep else None
    return [str(d.code) for d in get_diagnostics(src) if wanted is None or str(d.code) in wanted]


def _messages(src: str, code: str) -> list[str]:
    return [d.message for d in get_diagnostics(src) if str(d.code) == code]


class TestNoReadBeforeSet:
    def test_info_exists_is_not_read_before_set(self):
        # The original issue: the check itself must not be W210.
        assert "W210" not in _codes("proc p {} { if {[info exists FOO]} { set y 1 } }")

    def test_array_exists_is_not_read_before_set(self):
        assert "W210" not in _codes("proc p {} { if {[array exists FOO]} { set y 1 } }")

    def test_check_in_value_is_not_read_before_set(self):
        assert "W210" not in _codes("proc p {} { set r [info exists FOO]; puts $r }")

    def test_check_as_argument_is_not_read_before_set(self):
        assert "W210" not in _codes("proc p {} { puts [info exists FOO] }")

    def test_plain_read_still_flagged(self):
        # A genuine value read of an unset variable is still W210.
        assert "W210" in _codes("proc p {} { puts $BAR }", "W210")

    def test_array_get_is_a_value_read_still_flagged(self):
        # ``array get`` reads the array contents — not an existence check.
        assert "W210" in _codes("proc p {} { puts [array get FOO] }", "W210")

    def test_dynamic_target_is_a_real_read(self):
        # ``info exists $name`` reads ``name`` to form the variable name being
        # tested, so an unset ``name`` is still a genuine read-before-set and
        # must not be suppressed (PR #502 review).
        assert "W210" in _codes("proc p {} { if {[info exists $name]} { puts hi } }", "W210")
        assert "W210" in _codes("proc p {} { set r [info exists $name]; puts $r }", "W210")

    def test_vwait_var_is_not_read_before_set(self):
        # ``vwait varName`` blocks waiting for the variable to be written; the
        # variable is monitored, not read, so it does NOT need to be set
        # beforehand (``vwait forever`` is the canonical infinite-wait idiom —
        # tclsh 9.0.3-verified).
        assert "W210" not in _codes("proc p {} { vwait forever }", "W210")
        assert "W210" not in _codes("proc p {} { vwait sig }", "W210")
        # Top-level vwait too.
        assert "W210" not in _codes("vwait forever", "W210")

    def test_vwait_inside_command_sub_is_not_read_before_set(self):
        # A ``[vwait sig]`` embedded in an outer command (return value, set
        # RHS, …) must also be exempt — the cmd-sub scanner has to recognise
        # the safe-on-unset reference inside the brackets.
        assert "W210" not in _codes("proc p {} { return [vwait sig] }", "W210")

    def test_dict_for_body_local_set_is_not_read_before_set(self):
        # ``dict for`` lowers to an opaque IRBarrier (Tcl 9 compiles it as a
        # generic ensemble invoke, not an inlined loop), so the body's writes
        # are recovered name-level — exactly like a frozen ``while`` body or a
        # qualified ``::foreach``.  A ``set x ...`` inside the body must not
        # leave the ``$x`` read looking unset.
        src = (
            "proc p {} {\n"
            "    dict for {k v} {a 1 b 2} {\n"
            "        set fileData $k\n"
            "        puts $fileData\n"
            "    }\n"
            "}\n"
        )
        assert "W210" not in _codes(src, "W210")

    def test_dict_for_loop_vars_are_not_read_before_set(self):
        # The loop variables ``k``/``v`` are bound by ``dict for`` — they are
        # not read-before-set.
        src = 'proc p {} { dict for {k v} {a 1 b 2} { puts "$k=$v" } }\n'
        assert "W210" not in _codes(src, "W210")

    def test_dict_map_body_local_set_is_not_read_before_set(self):
        # ``dict map`` is the lmap analogue — same body-recovery contract.
        src = (
            "proc p {} {\n"
            "    dict map {k v} {a 1 b 2} {\n"
            "        set out $k\n"
            "        list $out\n"
            "    }\n"
            "}\n"
        )
        assert "W210" not in _codes(src, "W210")

    def test_dict_for_body_genuine_rbs_still_fires(self):
        # TP control: a body read of a variable that is truly never set must
        # still fire — the body-recovery is suppress-only, not blanket.
        src = "proc p {} {\n    dict for {k v} {a 1 b 2} { puts $never_set }\n}\n"
        assert "W210" in _codes(src, "W210")

    def test_cmd_sub_set_in_return_value_is_recognised(self):
        # tcllib installer.tcl idiom: ``return [read [set x [open $f r]]][close $x]``.
        # The ``[set x ...]`` lives inside a command substitution of the return
        # value, and the trailing ``[close $x]`` reads it.  The cmd-sub-writes
        # recovery in _read_before_set now scans block terminators (not just
        # block.statements), so the ``set x`` is recognised and ``$x`` is no
        # longer a spurious read-before-set.
        src = "proc p {f} { return [read [set x [open $f r]]][close $x] }\n"
        assert "W210" not in _codes(src, "W210")

    def test_cmd_sub_set_in_branch_condition_is_recognised(self):
        # Same idiom applied to a branch condition: ``if {[set x 1]} { puts $x }``
        # — the ``set x`` inside the cmd-sub of the branch condition must define
        # x for the body's read.  (The branch-cmd-sub-writes scan exempts the
        # name function-wide; main's narrowing handles existence-check shapes
        # separately, but this is a literal set, not an existence check.)
        src = "proc p {} { if {[set x 1]} { puts $x } }\n"
        assert "W210" not in _codes(src, "W210")

    def test_genuine_unset_in_return_still_fires(self):
        # TP control: a return that reads a variable never defined anywhere
        # must still fire — the terminator-scan is suppress-only.
        src = "proc p {} { return $never_set }\n"
        assert "W210" in _codes(src, "W210")


class TestProvablyAbsentFoldsFalse:
    def test_issue_example_body_never_executes(self):
        src = (
            "proc myProc {} {\n"
            "    if {[info exists ACCESS_RPC_HANDLE]} {\n"
            "        set y 1\n"
            "    }\n"
            "}"
        )
        msgs = _messages(src, "I230")
        assert any("always false" in m for m in msgs), msgs
        assert "W210" not in _codes(src)

    def test_lazy_init_reuse_branch_is_dead_for_local(self):
        # Mirrors the issue's real-world ILX pattern: a *local* never persists
        # across calls, so the re-use branch is unreachable.
        src = (
            "proc authorize {} {\n"
            "    if {[info exists H]} {\n"
            "        set reuse 1\n"
            "    } else {\n"
            "        set H 1\n"
            "    }\n"
            "}"
        )
        assert any("always false" in m for m in _messages(src, "I230"))

    def test_array_exists_absent_folds_false(self):
        assert any(
            "always false" in m
            for m in _messages("proc p {} { if {[array exists FOO]} { puts dead } }", "I230")
        )

    def test_negated_check_folds(self):
        src = "proc p {} { if {![info exists FOO]} { puts runs } else { puts dead } }"
        assert any("always true" in m for m in _messages(src, "I230"))


class TestProvablyPresentFoldsTrue:
    def test_set_before_check_folds_true(self):
        src = "proc p {} { set X 1; if {[info exists X]} { puts ok } else { puts dead } }"
        assert any("always true" in m for m in _messages(src, "I230"))

    def test_parameter_always_exists(self):
        src = "proc p {x} { if {[info exists x]} { puts ok } else { puts dead } }"
        assert any("always true" in m for m in _messages(src, "I230"))

    def test_unset_then_check_is_not_true(self):
        # After ``unset`` the variable is gone again — must not fold to true.
        src = "proc p {} { set X 1; unset X; if {[info exists X]} { puts a } else { puts b } }"
        assert not any("always true" in m for m in _messages(src, "I230"))


class TestSoundnessGates:
    """The fold must bail whenever a local could be created/destroyed invisibly."""

    def test_global_is_not_folded(self):
        # A global may exist independently of this frame.
        src = "proc p {} { global H; if {[info exists H]} { puts a } else { puts b } }"
        assert "I230" not in _codes(src)

    def test_namespace_variable_is_not_folded(self):
        src = "proc p {} { variable H; if {[info exists H]} { puts a } else { puts b } }"
        assert "I230" not in _codes(src)

    def test_eval_barrier_disables_fold(self):
        src = "proc p {} { eval $cmd; if {[info exists X]} { puts a } else { puts b } }"
        assert "I230" not in _codes(src)

    def test_namespace_qualified_name_is_not_folded(self):
        src = "proc p {} { if {[info exists ::ns::X]} { puts a } else { puts b } }"
        assert "I230" not in _codes(src)

    def test_array_element_is_not_folded(self):
        src = "proc p {} { if {[info exists A(k)]} { puts a } else { puts b } }"
        assert "I230" not in _codes(src)

    def test_nested_command_sub_assignment_is_not_folded(self):
        # `set y [set X 1]` creates X with no SSA definition, so the folder must
        # not treat X as absent (PR #502 review).
        src = "proc p {} { set y [set X 1]; if {[info exists X]} { puts a } else { puts b } }"
        assert "I230" not in _codes(src)

    def test_command_sub_in_call_arg_is_not_folded(self):
        src = "proc p {} { puts [set X 1]; if {[info exists X]} { puts a } else { puts b } }"
        assert "I230" not in _codes(src)

    def test_mutation_free_value_command_still_folds(self):
        # A value whose command sub cannot create a local must not block folding.
        src = "proc p {} { set n [string length abc]; if {[info exists X]} { puts a } else { puts b } }"
        assert any("always false" in m for m in _messages(src, "I230"))


class TestFlowSensitiveNarrowing:
    """In the true region a guarded variable exists; the false region does not."""

    def _src(self, body: str) -> str:
        # An ``eval`` barrier keeps both arms live (no fold) so the narrowing,
        # not DCE, is what governs the result.
        return f"proc p {{}} {{ eval $cmd; {body} }}"

    def test_true_branch_read_is_safe(self):
        src = self._src("if {[info exists X]} { puts $X }")
        # ``cmd`` is the genuine read-before-set; ``X`` is guarded.
        assert "X" not in [
            d.message.split("'")[1] for d in get_diagnostics(src) if str(d.code) == "W210"
        ]

    def test_false_branch_read_is_flagged(self):
        src = self._src("if {[info exists X]} { puts $X } else { puts $X }")
        flagged = [d.message.split("'")[1] for d in get_diagnostics(src) if str(d.code) == "W210"]
        assert "X" in flagged

    def test_negated_guard_narrows_else_branch(self):
        src = self._src("if {![info exists X]} { puts no } else { puts $X }")
        flagged = [d.message.split("'")[1] for d in get_diagnostics(src) if str(d.code) == "W210"]
        assert "X" not in flagged

    def test_nested_true_branch_read_is_safe(self):
        src = self._src("if {[info exists X]} { if {[string length a]} { puts $X } }")
        flagged = [d.message.split("'")[1] for d in get_diagnostics(src) if str(d.code) == "W210"]
        assert "X" not in flagged

    def test_and_pure_right_keeps_both_facts(self):
        # `[info exists X] && [info exists Y]` — both pure → both narrowed.
        src = self._src("if {[info exists X] && [info exists Y]} { puts $X$Y }")
        flagged = [d.message.split("'")[1] for d in get_diagnostics(src) if str(d.code) == "W210"]
        assert "X" not in flagged and "Y" not in flagged

    def test_and_impure_right_drops_left_fact(self):
        # An impure right operand can mutate X before the branch, so the left
        # `info exists X` fact must not narrow the body (PR #502 review).
        src = self._src("if {[info exists X] && [otherproc]} { puts $X }")
        flagged = [d.message.split("'")[1] for d in get_diagnostics(src) if str(d.code) == "W210"]
        assert "X" in flagged


class TestInfoVarsNarrowing:
    """``info vars`` / ``info locals`` membership idioms narrow reads too.

    The leading ``eval`` barrier keeps both arms live (no fold), so narrowing —
    not DCE — governs whether the guarded read is flagged.
    """

    def _flagged(self, body: str) -> set[str]:
        src = f"proc p {{}} {{ eval $cmd; {body} }}"
        return {d.message.split("'")[1] for d in get_diagnostics(src) if str(d.code) == "W210"}

    def test_info_vars_ne_empty_true_branch(self):
        assert "X" not in self._flagged('if {[info vars X] ne ""} { puts $X }')

    def test_info_vars_ne_empty_false_branch_flagged(self):
        assert "X" in self._flagged('if {[info vars X] ne ""} { puts a } else { puts $X }')

    def test_info_vars_eq_empty_else_branch(self):
        assert "X" not in self._flagged('if {[info vars X] eq ""} { puts a } else { puts $X }')

    def test_info_locals_ne_empty(self):
        assert "X" not in self._flagged('if {[info locals X] ne ""} { puts $X }')

    def test_llength_info_vars(self):
        assert "X" not in self._flagged("if {[llength [info vars X]]} { puts $X }")

    def test_lsearch_gt_minus_one(self):
        assert "X" not in self._flagged("if {[lsearch [info vars] X] > -1} { puts $X }")

    def test_lsearch_exact_ge_zero(self):
        assert "X" not in self._flagged("if {[lsearch -exact [info vars] X] >= 0} { puts $X }")

    def test_lsearch_eq_minus_one_else_branch(self):
        assert "X" not in self._flagged(
            "if {[lsearch [info vars] X] == -1} { puts a } else { puts $X }"
        )

    def test_info_globals_is_not_narrowed(self):
        # ``info globals`` proves the *global* exists, not the local ``$X``.
        assert "X" in self._flagged('if {[info globals X] ne ""} { puts $X }')

    def test_lsearch_regexp_is_not_narrowed(self):
        # ``-regexp`` matches a regex, not an exact name — unsound to narrow.
        assert "X" in self._flagged("if {[lsearch -regexp [info vars] X] > -1} { puts $X }")

    def test_glob_pattern_is_not_narrowed(self):
        # A glob pattern is not a single exact name.
        assert "X" in self._flagged('if {[info vars X*] ne ""} { puts $X }')


class TestCatchNarrowing:
    """``catch {set _ $X}`` signals existence on its succeeded (false) branch."""

    def _flagged(self, body: str) -> set[str]:
        src = f"proc p {{}} {{ eval $cmd; {body} }}"
        return {d.message.split("'")[1] for d in get_diagnostics(src) if str(d.code) == "W210"}

    def test_false_branch_knows_it_exists(self):
        assert "X" not in self._flagged("if {[catch {set _ $X}]} { puts a } else { puts $X }")

    def test_negated_true_branch_use_is_safe(self):
        assert "X" not in self._flagged("if {![catch {set _ $X}]} { puts $X }")

    def test_result_var_form(self):
        assert "X" not in self._flagged("if {[catch {set _ $X} e]} { puts a } else { puts $X }")

    def test_failed_branch_is_ambiguous_and_still_flagged(self):
        # catch != 0 could mean missing *or* an array read as a scalar.
        assert "X" in self._flagged("if {[catch {set _ $X}]} { puts $X }")

    def test_non_pure_body_is_not_narrowed(self):
        assert "X" in self._flagged("if {[catch {someproc $X}]} { puts a } else { puts $X }")


class TestAnalysisLevel:
    """Direct assertions on the core analysis result."""

    def test_constant_branch_recorded_for_absent_local(self):
        module = analyse_source("proc p {} { if {[info exists FOO]} { puts dead } }")
        fn = module.procedures["::p"]
        assert any(not b.value for b in fn.constant_branches)
        assert fn.unreachable_blocks

    def test_no_constant_branch_for_global(self):
        module = analyse_source(
            "proc p {} { global FOO; if {[info exists FOO]} { puts x } else { puts y } }"
        )
        fn = module.procedures["::p"]
        assert not fn.constant_branches
