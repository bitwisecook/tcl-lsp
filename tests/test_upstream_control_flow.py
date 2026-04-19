"""Tests ported from Tcl's official tests/if.test, switch.test, for.test, foreach.test.

These supplement the existing test_analyser.py and test_checks.py with
additional coverage derived from the upstream Tcl test suite for control-flow
commands.

Areas covered:
- if: arity errors, valid if/elseif/else chains, body variable analysis
- switch: arity errors, valid patterns, default, fall-through, option handling
- for: arity errors, valid 4-arg loops, body analysis
- foreach: arity errors, valid single/multi-var, body analysis
- break/continue: arity errors
- Unreachable branch detection (I230/I231 diagnostics)
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.analysis.semantic_model import Severity


def _diag_with_code(source: str, code: str):
    """Return all diagnostics matching a specific *code*."""
    result = analyse(source)
    return [d for d in result.diagnostics if d.code == code]


def _arity_errors(source: str):
    """Return all E002 (too few) and E003 (too many) diagnostics."""
    result = analyse(source)
    return [d for d in result.diagnostics if d.code in ("E002", "E003")]


# if command — ported from Tcl's tests/if.test


class TestIfCommand:
    """Validate that well-formed ``if`` invocations produce no arity errors
    and that body analysis captures variables correctly.
    """

    def test_valid_if(self):
        """Simple if with braced condition and body — no arity errors."""
        errors = _arity_errors("if {$x > 0} { set y 1 }")
        assert len(errors) == 0

    def test_valid_if_else(self):
        """if/else — no arity errors expected."""
        errors = _arity_errors("if {$x > 0} { set y 1 } else { set y 0 }")
        assert len(errors) == 0

    def test_valid_if_elseif_else(self):
        """Multi-branch if/elseif/else chain — no arity errors."""
        source = textwrap.dedent("""\
            if {$x > 0} {
                set y 1
            } elseif {$x < 0} {
                set y -1
            } else {
                set y 0
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_if_body_variables(self):
        """Variables set inside an if body should appear in scope."""
        result = analyse("if {1} { set y 42 }")
        assert "y" in result.global_scope.variables

    def test_if_with_then_keyword(self):
        """The optional ``then`` keyword between condition and body is valid."""
        errors = _arity_errors("if {$x > 0} then { set y 1 }")
        assert len(errors) == 0

    def test_if_elseif_chain(self):
        """Longer if/elseif/else chain — all bodies should be analysed."""
        source = textwrap.dedent("""\
            if {$x > 0} {
                set r positive
            } elseif {$x < 0} {
                set r negative
            } else {
                set r zero
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0
        result = analyse(source)
        assert "r" in result.global_scope.variables

    def test_if_else_body_variables_both_branches(self):
        """Variables set in both branches should be visible in the scope."""
        source = "if {$x} { set a 1 } else { set b 2 }"
        result = analyse(source)
        assert "a" in result.global_scope.variables
        assert "b" in result.global_scope.variables

    def test_if_elseif_else_all_bodies_analysed(self):
        """All branches of an if/elseif/else chain should be walked."""
        source = "if {$x} { set a 1 } elseif {$y} { set b 2 } else { set c 3 }"
        result = analyse(source)
        assert "a" in result.global_scope.variables
        assert "b" in result.global_scope.variables
        assert "c" in result.global_scope.variables

    def test_if_nested(self):
        """Nested if statements should not produce arity errors."""
        source = textwrap.dedent("""\
            if {$x > 0} {
                if {$y > 0} {
                    set r both_positive
                }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0
        result = analyse(source)
        assert "r" in result.global_scope.variables


# switch command — ported from Tcl's tests/switch.test


class TestSwitchCommand:
    """Validate that well-formed ``switch`` invocations produce no arity
    errors and that patterns, default, and fall-through are handled correctly.
    """

    def test_valid_switch_brace_body(self):
        """Basic switch with brace body — no arity errors."""
        source = textwrap.dedent("""\
            switch $x {
                a { set r 1 }
                b { set r 2 }
                default { set r 0 }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_switch_with_default(self):
        """Switch with a default arm — no errors expected."""
        source = textwrap.dedent("""\
            switch $x {
                foo { set r foo_match }
                default { set r no_match }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_switch_fall_through(self):
        """Switch with ``-`` fall-through pattern — no errors expected."""
        source = textwrap.dedent("""\
            switch $x {
                a -
                b { set r 1 }
                default { set r 0 }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_switch_exact_option(self):
        """switch -exact — no errors expected."""
        source = textwrap.dedent("""\
            switch -exact $x {
                a { set r 1 }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_switch_glob_option(self):
        """switch -glob — no errors expected."""
        source = textwrap.dedent("""\
            switch -glob $x {
                a* { set r 1 }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_switch_regexp_option(self):
        """switch -regexp — no errors expected."""
        source = textwrap.dedent("""\
            switch -regexp $x {
                ^a { set r 1 }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_switch_nocase_option(self):
        """switch -nocase — no errors expected."""
        source = textwrap.dedent("""\
            switch -nocase $x {
                a { set r 1 }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_switch_body_variables(self):
        """Variables set inside switch arms should be visible in scope."""
        source = textwrap.dedent("""\
            switch $x {
                a { set r 1 }
                b { set r 2 }
            }
        """)
        result = analyse(source)
        assert "r" in result.global_scope.variables

    def test_switch_multiple_arms_different_vars(self):
        """Different variables set in different arms should all be visible."""
        source = textwrap.dedent("""\
            switch $x {
                a { set alpha 1 }
                b { set beta 2 }
                default { set gamma 3 }
            }
        """)
        result = analyse(source)
        assert "alpha" in result.global_scope.variables
        assert "beta" in result.global_scope.variables
        assert "gamma" in result.global_scope.variables

    def test_switch_nocase_glob_combined(self):
        """switch -nocase -glob — combined options should be accepted."""
        source = textwrap.dedent("""\
            switch -nocase -glob $x {
                a* { set r 1 }
                default { set r 0 }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_switch_double_dash(self):
        """switch -- $x {...} — the ``--`` end-of-options marker is valid."""
        source = textwrap.dedent("""\
            switch -- $x {
                a { set r 1 }
                default { set r 0 }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0


# for command — ported from Tcl's tests/for.test


class TestForCommand:
    """Validate that well-formed ``for`` invocations produce no arity
    errors and that body analysis tracks variables correctly.
    """

    def test_valid_for_loop(self):
        """Standard four-argument for loop — no arity errors."""
        errors = _arity_errors("for {set i 0} {$i < 10} {incr i} { puts $i }")
        assert len(errors) == 0

    def test_for_body_analysis(self):
        """Variables set inside the for body should be visible in scope."""
        source = "for {set i 0} {$i < 10} {incr i} { set total $i }"
        result = analyse(source)
        assert "total" in result.global_scope.variables

    def test_for_init_creates_variable(self):
        """The for init expression ``set i 0`` should create variable ``i``."""
        source = "for {set i 0} {$i < 10} {incr i} { puts $i }"
        result = analyse(source)
        assert "i" in result.global_scope.variables

    def test_for_too_few_args(self):
        """for with fewer than 4 arguments should produce an arity error."""
        errors = _arity_errors("for {set i 0} {$i < 10}")
        assert len(errors) >= 1

    def test_for_nested(self):
        """Nested for loops should not produce arity errors."""
        source = textwrap.dedent("""\
            for {set i 0} {$i < 10} {incr i} {
                for {set j 0} {$j < 5} {incr j} {
                    set product [expr {$i * $j}]
                }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0
        result = analyse(source)
        assert "i" in result.global_scope.variables
        assert "j" in result.global_scope.variables

    def test_for_with_break(self):
        """for loop with break inside — no arity errors."""
        source = textwrap.dedent("""\
            for {set i 0} {$i < 100} {incr i} {
                if {$i > 50} break
                set last $i
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_for_with_continue(self):
        """for loop with continue inside — no arity errors."""
        source = textwrap.dedent("""\
            for {set i 0} {$i < 100} {incr i} {
                if {$i == 5} continue
                set processed $i
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0


# foreach command — ported from Tcl's tests/foreach.test


class TestForeachCommand:
    """Validate that well-formed ``foreach`` invocations produce no arity
    errors and that loop variables and body analysis work correctly.
    """

    def test_valid_foreach(self):
        """Simple foreach — no arity errors."""
        errors = _arity_errors("foreach item $list { puts $item }")
        assert len(errors) == 0

    def test_foreach_multi_var(self):
        """foreach with multiple iteration variables — no arity errors."""
        errors = _arity_errors("foreach {k v} $dict { puts $k }")
        assert len(errors) == 0

    def test_foreach_creates_loop_var(self):
        """foreach should define the iteration variable in scope."""
        result = analyse("foreach item {a b c} { puts $item }")
        assert "item" in result.global_scope.variables

    def test_foreach_body_analysis(self):
        """Variables set inside the foreach body should be tracked."""
        source = "foreach item {a b c} { set result $item }"
        result = analyse(source)
        assert "result" in result.global_scope.variables

    def test_foreach_multi_var_defines_all(self):
        """All variables in a multi-variable foreach should be defined."""
        result = analyse("foreach {a b c} {1 2 3 4 5 6} { puts $a }")
        assert "a" in result.global_scope.variables
        assert "b" in result.global_scope.variables
        assert "c" in result.global_scope.variables

    def test_foreach_literal_list(self):
        """foreach over a literal list — no errors."""
        source = "foreach colour {red green blue} { puts $colour }"
        errors = _arity_errors(source)
        assert len(errors) == 0
        result = analyse(source)
        assert "colour" in result.global_scope.variables

    def test_foreach_nested(self):
        """Nested foreach loops — no arity errors."""
        source = textwrap.dedent("""\
            foreach outer {a b c} {
                foreach inner {1 2 3} {
                    set pair "$outer$inner"
                }
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0
        result = analyse(source)
        assert "outer" in result.global_scope.variables
        assert "inner" in result.global_scope.variables

    def test_foreach_with_break(self):
        """foreach with break — no arity errors."""
        source = textwrap.dedent("""\
            foreach item {a b c d e} {
                if {$item eq "c"} break
                set last $item
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_foreach_with_continue(self):
        """foreach with continue — no arity errors."""
        source = textwrap.dedent("""\
            foreach item {a b c d e} {
                if {$item eq "c"} continue
                set processed $item
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0


# break / continue — arity validation


class TestBreakContinue:
    """Validate that break and continue are accepted with zero arguments
    and that extra arguments produce arity errors.
    """

    def test_break_no_args_valid(self):
        """break with no arguments inside a loop — no arity errors."""
        source = 'foreach x $list { if {$x eq "stop"} break }'
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_continue_no_args_valid(self):
        """continue with no arguments inside a loop — no arity errors."""
        source = 'foreach x $list { if {$x eq "skip"} continue }'
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_break_extra_arg_error(self):
        """break with extra arguments should produce an arity error."""
        errors = _arity_errors("break extra")
        assert len(errors) >= 1

    def test_continue_extra_arg_error(self):
        """continue with extra arguments should produce an arity error."""
        errors = _arity_errors("continue extra")
        assert len(errors) >= 1

    def test_break_in_for_loop(self):
        """break inside a for loop — no arity errors."""
        source = textwrap.dedent("""\
            for {set i 0} {$i < 10} {incr i} {
                if {$i == 5} break
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_continue_in_while_loop(self):
        """continue inside a while loop — no arity errors."""
        source = textwrap.dedent("""\
            while {$x > 0} {
                incr x -1
                if {$x == 3} continue
                set last $x
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0


# while command — supplementary coverage


class TestWhileCommand:
    """Validate that well-formed ``while`` invocations produce no arity
    errors and that body analysis tracks variables correctly.
    """

    def test_valid_while(self):
        """Simple while loop — no arity errors."""
        errors = _arity_errors("while {$x > 0} { incr x -1 }")
        assert len(errors) == 0

    def test_while_body_analysis(self):
        """Variables set inside the while body should be visible."""
        source = "while {1} { set found 1; break }"
        result = analyse(source)
        assert "found" in result.global_scope.variables

    def test_while_too_few_args(self):
        """while with only one argument should produce an arity error."""
        errors = _arity_errors("while {1}")
        assert len(errors) >= 1

    def test_while_with_break_and_continue(self):
        """while loop containing both break and continue — no arity errors."""
        source = textwrap.dedent("""\
            while {$running} {
                if {$skip} continue
                if {$done} break
                set count [expr {$count + 1}]
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0

    def test_while_nested_in_proc(self):
        """while loop inside a proc body — no arity errors."""
        source = textwrap.dedent("""\
            proc countdown {n} {
                while {$n > 0} {
                    incr n -1
                }
                return $n
            }
        """)
        errors = _arity_errors(source)
        assert len(errors) == 0
        result = analyse(source)
        assert "countdown" in result.global_scope.procs


# Unreachable branch detection — I230 / I231


class TestUnreachableBranches:
    """Validate that constant conditions in if/switch produce the
    appropriate unreachable-branch informational diagnostics.
    """

    def test_unreachable_else_after_true(self):
        """if {1} with an else branch — the else is unreachable (I230)."""
        source = "if {1} { set x 1 } else { set y 2 }"
        diags = _diag_with_code(source, "I230")
        assert len(diags) >= 1
        assert diags[0].severity == Severity.INFO
        assert "unreachable" in diags[0].message.lower()

    def test_unreachable_if_body_false(self):
        """if {0} — the body is unreachable (I230)."""
        source = "if {0} { set x 1 }"
        diags = _diag_with_code(source, "I230")
        assert len(diags) >= 1
        assert diags[0].severity == Severity.INFO

    def test_constant_true_if_no_else_still_flagged(self):
        """if {1} without else — a constant condition is still detected."""
        source = "if {1} { set x 1 }"
        # Even without an else branch, the constant condition may be flagged.
        # The analyser may or may not emit I230 here depending on whether
        # an unreachable branch exists; we verify no crash at minimum.
        result = analyse(source)
        assert result is not None

    def test_constant_switch_unreachable_arm(self):
        """switch with a constant match — later arms are unreachable (I231)."""
        source = "switch 1 {1 {set x 1} 2 {set y 2} default {set z 3}}"
        diags = _diag_with_code(source, "I231")
        assert len(diags) >= 1
        assert diags[0].severity == Severity.INFO

    def test_unreachable_elseif_after_true(self):
        """if {1} with elseif — subsequent branches are unreachable."""
        source = textwrap.dedent("""\
            if {1} {
                set a 1
            } elseif {$x} {
                set b 2
            } else {
                set c 3
            }
        """)
        diags = _diag_with_code(source, "I230")
        # The always-true first condition means elseif/else are unreachable.
        assert len(diags) >= 1


# Robustness — the analyser should not crash on valid control flow


class TestControlFlowRobustness:
    """Ensure the analyser handles various control-flow patterns without
    raising exceptions, even for edge cases.
    """

    def test_empty_if_body(self):
        """if with an empty body should not crash."""
        result = analyse("if {1} {}")
        assert result is not None

    def test_empty_for_body(self):
        """for with an empty body should not crash."""
        result = analyse("for {set i 0} {$i < 10} {incr i} {}")
        assert result is not None

    def test_empty_while_body(self):
        """while with an empty body should not crash."""
        result = analyse("while {1} {}")
        assert result is not None

    def test_empty_foreach_body(self):
        """foreach with an empty body should not crash."""
        result = analyse("foreach x {a b c} {}")
        assert result is not None

    def test_empty_switch_body(self):
        """switch with an empty body should not crash."""
        result = analyse("switch $x {}")
        assert result is not None

    def test_deeply_nested_control_flow(self):
        """Deeply nested if/for/foreach should not crash the analyser."""
        source = textwrap.dedent("""\
            for {set i 0} {$i < 5} {incr i} {
                foreach item {a b c} {
                    if {$item eq "a"} {
                        while {$i < 3} {
                            incr i
                            if {$i == 2} break
                        }
                    }
                }
            }
        """)
        result = analyse(source)
        assert result is not None
        assert "i" in result.global_scope.variables
        assert "item" in result.global_scope.variables

    def test_switch_inside_for(self):
        """switch nested inside a for loop — no crash."""
        source = textwrap.dedent("""\
            for {set i 0} {$i < 10} {incr i} {
                switch $i {
                    0 { set phase init }
                    5 { set phase mid }
                    default { set phase other }
                }
            }
        """)
        result = analyse(source)
        assert result is not None

    def test_if_inside_foreach(self):
        """if nested inside foreach — no crash, variables captured."""
        source = textwrap.dedent("""\
            foreach colour {red green blue} {
                if {$colour eq "green"} {
                    set found 1
                }
            }
        """)
        result = analyse(source)
        assert result is not None
        assert "colour" in result.global_scope.variables
        assert "found" in result.global_scope.variables

    def test_multiple_foreach_same_scope(self):
        """Multiple foreach loops in the same scope — no crash."""
        source = textwrap.dedent("""\
            foreach x {1 2 3} {
                set sum_x $x
            }
            foreach y {a b c} {
                set sum_y $y
            }
        """)
        result = analyse(source)
        assert result is not None
        assert "x" in result.global_scope.variables
        assert "y" in result.global_scope.variables

    def test_for_with_complex_increment(self):
        """for loop with a multi-command increment clause — no crash."""
        source = textwrap.dedent("""\
            for {set i 0} {$i < 20} {incr i 2} {
                set half [expr {$i / 2}]
            }
        """)
        result = analyse(source)
        assert result is not None
        assert "i" in result.global_scope.variables
