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
