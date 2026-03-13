"""Diagnostic coverage tests derived from the Tcl 9.0.2 test suite.

Tests that idiomatic Tcl patterns trigger (or don't trigger) the correct
diagnostics: W100 (unbraced expr), W110 (== on strings), etc.
"""

from __future__ import annotations

import pytest

from core.analysis.analyser import analyse

# Helpers


def _diag_with_code(source: str, code: str):
    """Return all diagnostics matching a specific code."""
    result = analyse(source)
    return [d for d in result.diagnostics if d.code == code]


# W100: Unbraced expr


class TestUnbracedExprFromTcl9:
    """W100 patterns derived from Tcl 9.0.2 expr.test section 1."""

    @pytest.mark.parametrize(
        "source",
        [
            # Variable substitution in unbraced expr
            pytest.param("set x 5\nexpr $x + 1", id="unbraced-var"),
            # Command substitution in unbraced expr
            pytest.param('expr 4*[llength "6 2"]', id="expr-1.10"),
        ],
    )
    def test_unbraced_warns(self, source):
        diags = _diag_with_code(source, "W100")
        assert len(diags) > 0, f"Expected W100 for: {source}"

    @pytest.mark.parametrize(
        "source",
        [
            # Braced expressions (from expr-1.7, 1.8)
            pytest.param("expr {-0005}", id="expr-1.7"),
            pytest.param("expr {{-0x1234}}", id="expr-1.8"),
            pytest.param("expr {$x + 1}", id="braced-var"),
            pytest.param("expr {2 * $a}", id="braced-multiply"),
            # Braced conditions in control flow
            pytest.param("if {$x > 0} {puts yes}", id="braced-if"),
            pytest.param("while {$i < 10} {incr i}", id="braced-while"),
            pytest.param("for {set i 0} {$i < 10} {incr i} {puts $i}", id="braced-for"),
        ],
    )
    def test_braced_no_warning(self, source):
        diags = _diag_with_code(source, "W100")
        assert len(diags) == 0, f"Unexpected W100 for: {source}"

    @pytest.mark.parametrize(
        "source",
        [
            # Safe literal-only expressions (from expr-1.2, 1.5)
            pytest.param("expr -25", id="expr-1.2"),
            pytest.param("expr 42", id="safe-literal"),
            pytest.param("expr 62.0", id="safe-float"),
            # Multi-word literal-only exprs — no substitution risk (expr-1.3, 1.4)
            pytest.param("expr -8.2   -6", id="expr-1.3"),
            pytest.param("expr 20 - 5 +10 -7", id="expr-1.4"),
        ],
    )
    def test_literal_no_warning(self, source):
        diags = _diag_with_code(source, "W100")
        assert len(diags) == 0, f"Unexpected W100 for literal: {source}"


# W110: == on strings


class TestStringCompareFromTcl9:
    """W110 patterns -- == on strings should use eq."""

    @pytest.mark.parametrize(
        "source",
        [
            pytest.param('set x hello\nexpr {$x == "hello"}', id="eq-string"),
            pytest.param('set x hello\nexpr {$x != "world"}', id="ne-string"),
            pytest.param('set x hello\nif {$x == "yes"} {puts yes}', id="if-eq-string"),
        ],
    )
    def test_string_eq_warns(self, source):
        diags = _diag_with_code(source, "W110")
        assert len(diags) > 0, f"Expected W110 for: {source}"

    @pytest.mark.parametrize(
        "source",
        [
            pytest.param('set x hello\nexpr {$x eq "hello"}', id="eq-correct"),
            pytest.param('set x hello\nexpr {$x ne "world"}', id="ne-correct"),
            pytest.param("set x 5\nexpr {$x == 5}", id="numeric-eq"),
        ],
    )
    def test_string_eq_no_warning(self, source):
        diags = _diag_with_code(source, "W110")
        assert len(diags) == 0, f"Unexpected W110 for: {source}"


# No-crash tests on complex Tcl patterns


class TestNoCrashOnTcl9Patterns:
    """Ensure analysis doesn't crash on complex patterns from the Tcl test suite."""

    @pytest.mark.parametrize(
        "source",
        [
            # Embedded variables (from expr-old-22)
            pytest.param("set a 16\nset x [expr {2*$a}]", id="expr-old-22.1"),
            pytest.param("set x -5\nset y 10\nset z [expr {$x + $y}]", id="expr-old-22.2"),
            pytest.param("set a 16\nset z [expr {[set a] - 14}]", id="expr-old-22.4"),
            # Double-quoted expressions
            pytest.param('expr {"abc"}', id="quoted-string"),
            # Boolean conditions
            pytest.param("if {true} {set a 2}", id="bool-if-true"),
            pytest.param("if {false} {set a 2}", id="bool-if-false"),
            pytest.param("if {yes} {set a 2}", id="bool-if-yes"),
            pytest.param("if {no} {set a 2}", id="bool-if-no"),
            pytest.param("if {on} {set a 2}", id="bool-if-on"),
            pytest.param("if {off} {set a 2}", id="bool-if-off"),
            # Complex ternary
            pytest.param("set x [expr {$a > 0 ? 1 : 0}]", id="ternary-simple"),
            pytest.param("set x [expr {$a ? 1 : $b ? 2 : 3}]", id="ternary-nested"),
            # Arithmetic with hex/octal
            pytest.param("set x [expr {0xff + 0o77}]", id="hex-plus-octal"),
            pytest.param("set x [expr {0b1010 & 0xff}]", id="binary-and-hex"),
            # Mixed operators
            pytest.param("set x [expr {$a > 0 && $b < 10 || $c == 0}]", id="mixed-logical"),
            pytest.param("set x [expr {($a + $b) * $c - $d / $e}]", id="chained-arith"),
            # Scientific notation in expr
            pytest.param("set x [expr {1.23e+1 + 2.34e-1}]", id="scientific-add"),
            # Math functions
            pytest.param("set x [expr {sin($a) + cos($b)}]", id="trig-sum"),
            pytest.param("set x [expr {pow(2.0 + 0.1, 3.0 + 0.1)}]", id="pow-with-exprs"),
            pytest.param("set x [expr {int(sin($a))}]", id="nested-func"),
            pytest.param("set x [expr {max($a, $b, $c)}]", id="variadic-func"),
            # Unary chains
            pytest.param("set x [expr {+--++36}]", id="unary-chain"),
            pytest.param("set x [expr {!!$flag}]", id="double-negation"),
            # String comparison ops
            pytest.param("set x [expr {$a lt $b}]", id="str-lt"),
            pytest.param("set x [expr {$a ge $b}]", id="str-ge"),
            # Array variable in expr
            pytest.param("set a(foo) 37\nset x [expr {$a(foo) + 1}]", id="array-in-expr"),
            # Namespace-qualified variable
            pytest.param("set x [expr {$::count + 1}]", id="global-var-in-expr"),
        ],
    )
    def test_no_crash(self, source):
        """Analysis must complete without exception."""
        result = analyse(source)
        assert result is not None


# Complex patterns from expr-old.test


class TestComplexPatterns:
    """Multi-line patterns derived from the Tcl 9.0.2 test suite."""

    def test_variable_in_loop_expr(self):
        """For loop with expr in condition."""
        source = """\
set sum 0
for {set i 0} {$i < 10} {incr i} {
    set sum [expr {$sum + $i}]
}
"""
        result = analyse(source)
        assert result is not None

    def test_nested_if_with_expr(self):
        """Nested if/else with expr conditions."""
        source = """\
set x 5
if {$x > 0} {
    set y [expr {$x * 2}]
} else {
    set y [expr {-$x}]
}
"""
        result = analyse(source)
        assert result is not None

    def test_switch_with_expr(self):
        """Switch with computed values."""
        source = """\
set x 5
switch $x {
    1 { set y one }
    5 { set y five }
    default { set y other }
}
"""
        result = analyse(source)
        assert result is not None

    def test_proc_with_expr_return(self):
        """Proc that returns an expr result."""
        source = """\
proc add {a b} {
    return [expr {$a + $b}]
}
set x [add 3 4]
"""
        result = analyse(source)
        assert result is not None

    def test_while_with_incr(self):
        """While loop with incr."""
        source = """\
set i 0
while {$i < 100} {
    incr i
}
"""
        result = analyse(source)
        assert result is not None
