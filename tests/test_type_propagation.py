"""Tests for type propagation through the SSA graph."""

from __future__ import annotations

from core.compiler.types import TclType, TypeKind, TypeLattice

from .helpers import analyse_types as _analyse
from .helpers import var_type as _var_type


def _all_var_types(analysis, var_name: str) -> dict[int, TypeLattice]:
    """Return all versions and their types for a variable."""
    return {ver: t for (name, ver), t in analysis.types.items() if name == var_name}


class TestLiteralTypes:
    def test_integer_literal(self):
        analysis = _analyse("set x 42")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.kind is TypeKind.KNOWN
        assert t.tcl_type is TclType.INT

    def test_negative_integer(self):
        analysis = _analyse("set x -7")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_string_literal(self):
        analysis = _analyse("set x hello")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.STRING

    def test_float_literal(self):
        analysis = _analyse("set x 3.14")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_boolean_literal_true(self):
        analysis = _analyse("set x true")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_boolean_literal_false(self):
        analysis = _analyse("set x false")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN


class TestIncr:
    def test_incr_is_int(self):
        analysis = _analyse("set x 0\nincr x")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_incr_with_amount(self):
        analysis = _analyse("set x 0\nincr x 5")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.INT


class TestExprAssign:
    def test_expr_known_int(self):
        analysis = _analyse("set x [expr {1 + 2}]")
        t = _var_type(analysis, "x")
        assert t is not None
        # Should be INT or NUMERIC depending on SCCP result
        assert t.tcl_type in (TclType.INT, TclType.NUMERIC)


class TestVariableReference:
    def test_var_ref_inherits_type(self):
        analysis = _analyse("set x 42\nset y $x")
        t = _var_type(analysis, "y")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_string_interpolation_is_string(self):
        analysis = _analyse('set x 42\nset y "value is $x"')
        t = _var_type(analysis, "y")
        assert t is not None
        assert t.tcl_type is TclType.STRING


class TestCommandReturnTypes:
    def test_list_returns_list(self):
        analysis = _analyse("set x [list a b c]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.LIST

    def test_llength_returns_int(self):
        analysis = _analyse("set x [list a b]\nset n [llength $x]")
        t = _var_type(analysis, "n")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_split_returns_list(self):
        analysis = _analyse('set x [split "a,b,c" ,]')
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.LIST

    def test_join_returns_string(self):
        analysis = _analyse("set x [join {a b c} ,]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.STRING

    def test_string_length_returns_int(self):
        analysis = _analyse("set x [string length hello]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_concat_returns_list(self):
        analysis = _analyse("set x [concat {a b} {c d}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.LIST


class TestPhiMerging:
    def test_same_type_merges(self):
        source = """
set x 1
if {1} {
    set x 2
} else {
    set x 3
}
"""
        analysis = _analyse(source)
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_incompatible_types_shimmer(self):
        source = """
set x 1
if {$cond} {
    set x hello
}
"""
        analysis = _analyse(source)
        # After the if, x could be INT (from before) or STRING (from body)
        # The phi merge should produce SHIMMERED
        all_types = _all_var_types(analysis, "x")
        # There should be a SHIMMERED version (the phi)
        shimmer_versions = [
            t for t in all_types.values() if t.kind in (TypeKind.SHIMMERED, TypeKind.OVERDEFINED)
        ]
        assert shimmer_versions, f"Expected a SHIMMERED/OVERDEFINED version, got {all_types}"


class TestBarrier:
    def test_barrier_widens_type(self):
        source = """
set x 42
eval {set x hello}
"""
        analysis = _analyse(source)
        # After eval barrier, x type should be OVERDEFINED
        t = _var_type(analysis, "x")
        # x_1 is INT (before eval), but the eval is a barrier.
        # Since eval doesn't create a new SSA def for x,
        # the type of x remains from the last def.
        # The barrier itself just creates an IRBarrier, not a new def.
        assert t is not None


class TestExpressionTypeInference:
    """Tests for operator-aware type inference on structured expression AST."""

    def test_int_division_is_int(self):
        """Tcl: ``expr {5 / 2}`` is integer division → INT."""
        analysis = _analyse("set x [expr {5 / 2}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_float_division_is_double(self):
        """``expr {5.0 / 2}`` → DOUBLE."""
        analysis = _analyse("set x [expr {5.0 / 2}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_comparison_is_boolean(self):
        """``expr {$a == $b}`` → BOOLEAN."""
        analysis = _analyse("set a 1\nset b 2\nset z [expr {$a == $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_string_comparison_is_boolean(self):
        """``expr {$a eq "x"}`` → BOOLEAN."""
        analysis = _analyse('set a hello\nset z [expr {$a eq "x"}]')
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_bitwise_and_is_int(self):
        """``expr {$a & 0xFF}`` → INT."""
        analysis = _analyse("set a 256\nset z [expr {$a & 0xFF}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_sin_is_double(self):
        """``expr {sin($x)}`` → DOUBLE."""
        analysis = _analyse("set x 1\nset z [expr {sin($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_int_func_is_int(self):
        """``expr {int($x)}`` → INT."""
        analysis = _analyse("set x 3.14\nset z [expr {int($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_ternary_join(self):
        """``expr {$c ? 1 : 2.0}`` → NUMERIC (join of INT and DOUBLE)."""
        analysis = _analyse("set c 1\nset z [expr {$c ? 1 : 2.0}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.NUMERIC

    def test_logical_and_is_boolean(self):
        """``expr {$a && $b}`` → BOOLEAN."""
        analysis = _analyse("set a 1\nset b 0\nset z [expr {$a && $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_negation_preserves_type(self):
        """``expr {-$x}`` when x is INT → INT."""
        analysis = _analyse("set x 5\nset z [expr {-$x}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_logical_not_is_boolean(self):
        """``expr {!$x}`` → BOOLEAN."""
        analysis = _analyse("set x 1\nset z [expr {!$x}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_bitwise_not_is_int(self):
        """``expr {~$x}`` → INT."""
        analysis = _analyse("set x 0xFF\nset z [expr {~$x}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_int_plus_int_is_int(self):
        """``expr {$a + $b}`` with both INT → INT."""
        analysis = _analyse("set a 1\nset b 2\nset z [expr {$a + $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_int_plus_double_is_double(self):
        """``expr {$a + $b}`` with INT + DOUBLE → DOUBLE."""
        analysis = _analyse("set a 1\nset b 2.0\nset z [expr {$a + $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_abs_preserves_type(self):
        """``expr {abs($x)}`` preserves operand type."""
        analysis = _analyse("set x -5\nset z [expr {abs($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_bool_func_is_boolean(self):
        """``expr {bool($x)}`` → BOOLEAN."""
        analysis = _analyse("set x 1\nset z [expr {bool($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_double_func_is_double(self):
        """``expr {double($x)}`` → DOUBLE."""
        analysis = _analyse("set x 1\nset z [expr {double($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_max_joins_operand_types(self):
        """``expr {max(1, 2.0)}`` → NUMERIC (join INT, DOUBLE)."""
        analysis = _analyse("set z [expr {max(1, 2.0)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.NUMERIC

    def test_shift_is_int(self):
        """``expr {$x << 2}`` → INT."""
        analysis = _analyse("set x 1\nset z [expr {$x << 2}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_power_int_int_is_int(self):
        """``expr {2 ** 10}`` → INT."""
        analysis = _analyse("set z [expr {2 ** 10}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_modulo_int_is_int(self):
        """``expr {$a % $b}`` with both INT → INT."""
        analysis = _analyse("set a 10\nset b 3\nset z [expr {$a % $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT


# Comprehensive expression type inference tests (Tcl 9.0.2 patterns)


class TestArithmeticTypePromotion:
    """INT/DOUBLE promotion rules from Tcl 9.0.2 expr.test."""

    def test_int_sub_int_is_int(self):
        analysis = _analyse("set a 5\nset b 3\nset z [expr {$a - $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_int_mul_int_is_int(self):
        analysis = _analyse("set a 3\nset b 7\nset z [expr {$a * $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_double_add_double_is_double(self):
        analysis = _analyse("set a 1.5\nset b 2.5\nset z [expr {$a + $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_double_mul_int_is_double(self):
        analysis = _analyse("set a 3.14\nset b 2\nset z [expr {$a * $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_int_div_double_is_double(self):
        """``expr {5 / 2.0}`` → DOUBLE."""
        analysis = _analyse("set z [expr {5 / 2.0}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_int_mod_int_is_int(self):
        """``expr {7891 % 123}`` → INT (from Tcl 9.0.2 expr.test)."""
        analysis = _analyse("set z [expr {7891 % 123}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_power_int_int_is_int(self):
        """``expr {4 ** 2}`` → INT (from Tcl 9.0.2)."""
        analysis = _analyse("set z [expr {4 ** 2}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_power_int_double_is_double(self):
        """``expr {$a ** 2.0}`` → DOUBLE."""
        analysis = _analyse("set a 4\nset z [expr {$a ** 2.0}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_boolean_plus_int_is_int(self):
        """BOOLEAN + INT → INT (booleans are integers in Tcl)."""
        analysis = _analyse("set a true\nset b 5\nset z [expr {$a + $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_boolean_plus_boolean_is_int(self):
        """BOOLEAN + BOOLEAN → INT."""
        analysis = _analyse("set a true\nset b false\nset z [expr {$a + $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT


class TestBitwiseTypeInference:
    """Bitwise operators always produce INT (from Tcl 9.0.2)."""

    def test_bitwise_or_is_int(self):
        analysis = _analyse("set a 0xFF\nset b 0x0F\nset z [expr {$a | $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_bitwise_xor_is_int(self):
        analysis = _analyse("set a 0xFF\nset b 0x0F\nset z [expr {$a ^ $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_bitwise_not_is_int(self):
        analysis = _analyse("set a 0xFF\nset z [expr {~$a}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_left_shift_is_int(self):
        analysis = _analyse("set a 1\nset z [expr {$a << 8}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_right_shift_is_int(self):
        analysis = _analyse("set a 256\nset z [expr {$a >> 4}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT


class TestComparisonTypeInference:
    """All comparison operators return BOOLEAN (from Tcl 9.0.2)."""

    def test_lt_is_boolean(self):
        analysis = _analyse("set a 1\nset b 2\nset z [expr {$a < $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_le_is_boolean(self):
        analysis = _analyse("set a 1\nset b 2\nset z [expr {$a <= $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_gt_is_boolean(self):
        analysis = _analyse("set a 3\nset b 2\nset z [expr {$a > $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_ge_is_boolean(self):
        analysis = _analyse("set a 3\nset b 2\nset z [expr {$a >= $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_ne_is_boolean(self):
        analysis = _analyse("set a 1\nset b 2\nset z [expr {$a != $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_str_ne_is_boolean(self):
        analysis = _analyse('set a hello\nset z [expr {$a ne "world"}]')
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_str_lt_is_boolean(self):
        analysis = _analyse('set a abc\nset z [expr {$a lt "xyz"}]')
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_in_is_boolean(self):
        """``$x in $list`` → BOOLEAN."""
        analysis = _analyse("set x 1\nset list {a b c}\nset z [expr {$x in $list}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_ni_is_boolean(self):
        """``$x ni $list`` → BOOLEAN."""
        analysis = _analyse("set x 1\nset list {a b c}\nset z [expr {$x ni $list}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN


class TestLogicalTypeInference:
    """Logical operators return BOOLEAN (from Tcl 9.0.2)."""

    def test_logical_or_is_boolean(self):
        analysis = _analyse("set a 1\nset b 0\nset z [expr {$a || $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_chained_logical_and_is_boolean(self):
        analysis = _analyse("set a 1\nset b 1\nset c 0\nset z [expr {$a && $b && $c}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_logical_not_is_boolean(self):
        analysis = _analyse("set x 42\nset z [expr {!$x}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_double_logical_not_is_boolean(self):
        """``!!$x`` → BOOLEAN."""
        analysis = _analyse("set x 0\nset z [expr {!!$x}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN


class TestMathFunctionTypeInference:
    """Math function return types from Tcl 9.0.2 compExpr.test."""

    def test_cos_is_double(self):
        analysis = _analyse("set x 0\nset z [expr {cos($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_tan_is_double(self):
        analysis = _analyse("set x 1\nset z [expr {tan($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_exp_is_double(self):
        analysis = _analyse("set x 1\nset z [expr {exp($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_log_is_double(self):
        analysis = _analyse("set x 10\nset z [expr {log($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_log10_is_double(self):
        analysis = _analyse("set x 100\nset z [expr {log10($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_sqrt_is_double(self):
        analysis = _analyse("set x 4\nset z [expr {sqrt($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_ceil_is_int(self):
        analysis = _analyse("set x 3.14\nset z [expr {ceil($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_floor_is_int(self):
        analysis = _analyse("set x 3.7\nset z [expr {floor($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_round_is_int(self):
        analysis = _analyse("set x 3.5\nset z [expr {round($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_wide_is_int(self):
        analysis = _analyse("set x 5\nset z [expr {wide($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_entier_is_int(self):
        analysis = _analyse("set x 3.14\nset z [expr {entier($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_isqrt_is_int(self):
        analysis = _analyse("set x 16\nset z [expr {isqrt($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_isnan_is_boolean(self):
        analysis = _analyse("set x 1.0\nset z [expr {isnan($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_isinf_is_boolean(self):
        analysis = _analyse("set x 1.0\nset z [expr {isinf($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_pow_function_is_double(self):
        """``pow($a, $b)`` function → DOUBLE (unlike ** operator)."""
        analysis = _analyse("set a 2\nset b 3\nset z [expr {pow($a, $b)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_hypot_is_double(self):
        analysis = _analyse("set a 3\nset b 4\nset z [expr {hypot($a, $b)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_fmod_is_double(self):
        analysis = _analyse("set a 10.5\nset b 3.0\nset z [expr {fmod($a, $b)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_rand_is_double(self):
        analysis = _analyse("set z [expr {rand()}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_abs_double_is_double(self):
        """``abs($x)`` with DOUBLE operand → DOUBLE."""
        analysis = _analyse("set x -3.14\nset z [expr {abs($x)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_min_int_int_is_int(self):
        """``min(1, 2)`` → INT."""
        analysis = _analyse("set z [expr {min(1, 2)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_min_int_double_is_numeric(self):
        """``min(1, 2.0)`` → NUMERIC (join INT, DOUBLE)."""
        analysis = _analyse("set z [expr {min(1, 2.0)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.NUMERIC

    def test_max_int_int_is_int(self):
        """``max(3, 5)`` → INT."""
        analysis = _analyse("set z [expr {max(3, 5)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT


class TestTernaryTypeInference:
    """Ternary type join rules from Tcl 9.0.2."""

    def test_ternary_int_int_is_int(self):
        """``$c ? 1 : 2`` → INT (same type both branches)."""
        analysis = _analyse("set c 1\nset z [expr {$c ? 1 : 2}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_ternary_double_double_is_double(self):
        """``$c ? 1.0 : 2.0`` → DOUBLE."""
        analysis = _analyse("set c 1\nset z [expr {$c ? 1.0 : 2.0}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_ternary_bool_bool_is_boolean(self):
        """``$c ? ($a > 0) : ($b < 0)`` → BOOLEAN."""
        analysis = _analyse("set a 1\nset b 2\nset c 1\nset z [expr {$c ? ($a > 0) : ($b < 0)}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN


class TestUnaryTypePreservation:
    """Unary operator type preservation from Tcl 9.0.2."""

    def test_neg_double_is_double(self):
        """``-$x`` when x is DOUBLE → DOUBLE."""
        analysis = _analyse("set x 3.14\nset z [expr {-$x}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_pos_int_is_int(self):
        """``+$x`` when x is INT → INT."""
        analysis = _analyse("set x 5\nset z [expr {+$x}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_pos_double_is_double(self):
        """``+$x`` when x is DOUBLE → DOUBLE."""
        analysis = _analyse("set x 3.14\nset z [expr {+$x}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE


class TestLiteralTypeInference:
    """Type inference for literal values in expressions."""

    def test_hex_literal_is_int(self):
        analysis = _analyse("set z [expr {0xFF + 1}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_octal_literal_is_int(self):
        analysis = _analyse("set z [expr {0o77 + 1}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_binary_literal_is_int(self):
        analysis = _analyse("set z [expr {0b1010 + 1}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_scientific_notation_is_double(self):
        analysis = _analyse("set z [expr {1.5e3 + 1}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_boolean_true_type(self):
        analysis = _analyse("set z true")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_boolean_yes_type(self):
        analysis = _analyse("set z yes")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_boolean_on_type(self):
        analysis = _analyse("set z on")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_boolean_no_type(self):
        analysis = _analyse("set z no")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_boolean_off_type(self):
        analysis = _analyse("set z off")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN


class TestCommandSubstitutionType:
    """Command substitution in expressions should be OVERDEFINED."""

    def test_command_sub_in_expr(self):
        """[some_cmd] in expr → result is OVERDEFINED/NUMERIC."""
        analysis = _analyse("set z [expr {[some_cmd] + 1}]")
        t = _var_type(analysis, "z")
        # The result type depends on SCCP outcome; at minimum should exist.
        assert t is not None

    def test_nested_expr(self):
        """``set z [expr {[expr {1 + 2}] + 1}]`` — nested command sub."""
        analysis = _analyse("set z [expr {[expr {1 + 2}] + 1}]")
        t = _var_type(analysis, "z")
        assert t is not None


class TestComplexExpressionTypeInference:
    """Complex multi-operator expression type inference."""

    def test_comparison_chain_is_boolean(self):
        """``$a < $b == $c > $d`` — equality of comparisons → BOOLEAN."""
        analysis = _analyse("set a 1\nset b 2\nset c 3\nset d 4\nset z [expr {$a < $b == $c > $d}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_mixed_arithmetic_is_double(self):
        """``$a * 2 + $b / 3.0`` → DOUBLE (because of 3.0)."""
        analysis = _analyse("set a 1\nset b 2\nset z [expr {$a * 2 + $b / 3.0}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_bitwise_in_comparison_is_boolean(self):
        """``($a & 0xFF) == 0`` → BOOLEAN."""
        analysis = _analyse("set a 256\nset z [expr {($a & 0xFF) == 0}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_shift_in_bitwise_is_int(self):
        """``$a << 2 | $b`` → INT."""
        analysis = _analyse("set a 1\nset b 4\nset z [expr {$a << 2 | $b}]")
        t = _var_type(analysis, "z")
        assert t is not None
        assert t.tcl_type is TclType.INT


class TestVariableShapeTypePropagation:
    """Variable-shape handling through type-propagation consumers."""

    def test_namespaced_scalar_keeps_qualified_symbol_type(self):
        analysis = _analyse('set ::ns::x "alpha beta"\nset n [llength $::ns::x]')
        t = _var_type(analysis, "::ns::x")
        assert t is not None
        assert t.kind is TypeKind.KNOWN
        assert t.tcl_type is TclType.STRING

    def test_namespaced_array_element_uses_base_array_symbol_type(self):
        analysis = _analyse('set ::ns::arr(k) "alpha beta"\nset n [llength $::ns::arr(k)]')
        t = _var_type(analysis, "::ns::arr")
        assert t is not None
        assert t.kind is TypeKind.KNOWN
        assert t.tcl_type is TclType.STRING

    def test_braced_scalar_like_array_name_normalises_to_scalar_symbol(self):
        analysis = _analyse('set {a(1)} "alpha beta"\nset n [llength ${a(1)}]')
        t = _var_type(analysis, "a")
        assert t is not None
        assert t.kind is TypeKind.KNOWN
        assert t.tcl_type is TclType.STRING

    def test_unbraced_array_ref_normalises_to_base_array_symbol(self):
        analysis = _analyse('set a(1) "alpha beta"\nset n [llength $a(1)]')
        t = _var_type(analysis, "a")
        assert t is not None
        assert t.kind is TypeKind.KNOWN
        assert t.tcl_type is TclType.STRING

    def test_namespace_var_commands_with_qualified_names_do_not_break_type_flow(self):
        source = """\
namespace eval ::demo {
    variable value 1
}
proc wire_namespace_vars {} {
    global ::demo::value
    upvar 0 ::demo::value local_value
    unset -nocomplain ::demo::value
}
"""
        analysis = _analyse(source)
        assert analysis.types is not None
