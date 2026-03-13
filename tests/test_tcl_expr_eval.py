"""Tests for the Tcl expression evaluator (Tcl 9.0.2 semantics)."""

import math

from core.compiler.tcl_expr_eval import (
    eval_tcl_expr_str,
    format_tcl_value,
)


def _eval(expr, variables=None):
    return eval_tcl_expr_str(expr, variables)


class TestLiterals:
    def test_integer(self):
        assert _eval("42") == 42
        assert isinstance(_eval("42"), int)

    def test_negative_integer(self):
        assert _eval("-7") == -7

    def test_hex(self):
        assert _eval("0xFF") == 255

    def test_octal(self):
        assert _eval("0o17") == 15

    def test_binary(self):
        assert _eval("0b1010") == 10

    def test_float(self):
        assert _eval("3.14") == 3.14
        assert isinstance(_eval("3.14"), float)

    def test_scientific(self):
        assert _eval("1e10") == 1e10

    def test_bool_true(self):
        assert _eval("true") == 1
        assert isinstance(_eval("true"), int)

    def test_bool_false(self):
        assert _eval("false") == 0

    def test_bool_yes_no(self):
        assert _eval("yes") == 1
        assert _eval("no") == 0

    def test_bool_on_off(self):
        assert _eval("on") == 1
        assert _eval("off") == 0


class TestArithmetic:
    def test_add_int(self):
        assert _eval("1 + 2") == 3
        assert isinstance(_eval("1 + 2"), int)

    def test_add_float_promotion(self):
        assert _eval("1.0 + 2") == 3.0
        assert isinstance(_eval("1.0 + 2"), float)

    def test_sub(self):
        assert _eval("10 - 3") == 7

    def test_mul(self):
        assert _eval("6 * 7") == 42

    def test_div_int_floor(self):
        """Tcl integer division: floor toward -inf."""
        assert _eval("7 / 2") == 3
        assert isinstance(_eval("7 / 2"), int)

    def test_div_int_negative_floor(self):
        assert _eval("-7 / 2") == -4  # floor, not truncate

    def test_div_positive_negative(self):
        assert _eval("7 / -2") == -4

    def test_div_float(self):
        r = _eval("7.0 / 2")
        assert r == 3.5
        assert isinstance(r, float)

    def test_div_by_zero(self):
        assert _eval("1 / 0") is None

    def test_mod_int(self):
        assert _eval("7 % 3") == 1

    def test_mod_sign_follows_divisor(self):
        """Tcl modulo: result sign follows divisor."""
        assert _eval("-7 % 3") == 2  # not -1
        assert _eval("7 % -3") == -2  # not 1

    def test_mod_by_zero(self):
        assert _eval("1 % 0") is None


class TestExponentiation:
    def test_int_positive(self):
        assert _eval("2 ** 10") == 1024
        assert isinstance(_eval("2 ** 10"), int)

    def test_zero_exponent(self):
        assert _eval("5 ** 0") == 1

    def test_one_exponent(self):
        assert _eval("5 ** 1") == 5

    def test_negative_exponent_large_base(self):
        """Tcl: |base| > 1 with negative exp → integer 0."""
        assert _eval("2 ** -1") == 0

    def test_negative_exponent_neg_one(self):
        assert _eval("(-1) ** 3") == -1
        assert _eval("(-1) ** 4") == 1

    def test_negative_exponent_one(self):
        assert _eval("1 ** -5") == 1

    def test_negative_exponent_zero(self):
        assert _eval("0 ** -1") is None

    def test_float_base(self):
        r = _eval("2.0 ** 3")
        assert r == 8.0
        assert isinstance(r, float)

    def test_float_negative_exponent(self):
        assert _eval("2.0 ** -1") == 0.5

    def test_huge_exponent_guard(self):
        assert _eval("2 ** 100000") is None


class TestComparisons:
    def test_lt(self):
        assert _eval("3 < 5") == 1
        assert isinstance(_eval("3 < 5"), int)  # int, not bool

    def test_lt_false(self):
        assert _eval("5 < 3") == 0

    def test_le(self):
        assert _eval("3 <= 3") == 1

    def test_gt(self):
        assert _eval("5 > 3") == 1

    def test_ge(self):
        assert _eval("3 >= 5") == 0

    def test_eq(self):
        assert _eval("5 == 5") == 1
        assert _eval("5 == 6") == 0

    def test_ne(self):
        assert _eval("5 != 6") == 1

    def test_mixed_type_eq(self):
        assert _eval("5 == 5.0") == 1

    def test_str_eq(self):
        assert _eval("5 eq 5") == 1

    def test_str_ne(self):
        assert _eval("5 ne 6") == 1


class TestLogical:
    def test_and_true(self):
        assert _eval("1 && 1") == 1

    def test_and_false(self):
        assert _eval("1 && 0") == 0

    def test_or_true(self):
        assert _eval("0 || 1") == 1

    def test_or_false(self):
        assert _eval("0 || 0") == 0

    def test_and_short_circuit(self):
        """Right side has command subst but left is false → short-circuit."""
        assert _eval("0 && [cmd]") == 0

    def test_or_short_circuit(self):
        assert _eval("1 || [cmd]") == 1

    def test_not(self):
        assert _eval("!0") == 1
        assert _eval("!1") == 0
        assert _eval("!5") == 0


class TestBitwise:
    def test_and(self):
        assert _eval("0xFF & 0x0F") == 0x0F

    def test_or(self):
        assert _eval("0xF0 | 0x0F") == 0xFF

    def test_xor(self):
        assert _eval("0xFF ^ 0x0F") == 0xF0

    def test_not(self):
        assert _eval("~0") == -1

    def test_lshift(self):
        assert _eval("1 << 4") == 16

    def test_rshift(self):
        assert _eval("16 >> 2") == 4

    def test_require_int(self):
        assert _eval("1.0 & 2") is None
        assert _eval("1.0 | 2") is None
        assert _eval("1.0 << 2") is None


class TestTernary:
    def test_true_branch(self):
        assert _eval("1 ? 42 : 99") == 42

    def test_false_branch(self):
        assert _eval("0 ? 42 : 99") == 99

    def test_short_circuit(self):
        """Only the taken branch is evaluated."""
        assert _eval("1 ? 42 : [cmd]") == 42
        assert _eval("0 ? [cmd] : 99") == 99


class TestMathFunctions:
    def test_abs_int(self):
        assert _eval("abs(-5)") == 5
        assert isinstance(_eval("abs(-5)"), int)

    def test_abs_float(self):
        assert _eval("abs(-5.0)") == 5.0
        assert isinstance(_eval("abs(-5.0)"), float)

    def test_int_truncate(self):
        assert _eval("int(3.7)") == 3
        assert _eval("int(-3.7)") == -3

    def test_double(self):
        r = _eval("double(5)")
        assert r == 5.0
        assert isinstance(r, float)

    def test_bool(self):
        assert _eval("bool(0)") == 0
        assert _eval("bool(42)") == 1

    def test_round_ties_away_from_zero(self):
        """Tcl round: ties away from zero (not Python's even)."""
        assert _eval("round(2.5)") == 3  # Python would give 2
        assert _eval("round(3.5)") == 4  # same in both
        assert _eval("round(-2.5)") == -3  # Python would give -2

    def test_ceil_returns_float(self):
        r = _eval("ceil(2.1)")
        assert r == 3.0
        assert isinstance(r, float)  # Tcl ceil returns double

    def test_floor_returns_float(self):
        r = _eval("floor(2.9)")
        assert r == 2.0
        assert isinstance(r, float)

    def test_sqrt(self):
        assert _eval("sqrt(16)") == 4.0

    def test_sqrt_negative(self):
        assert _eval("sqrt(-1)") is None

    def test_isqrt(self):
        assert _eval("isqrt(16)") == 4
        assert isinstance(_eval("isqrt(16)"), int)

    def test_min(self):
        assert _eval("min(3, 1, 2)") == 1

    def test_max(self):
        assert _eval("max(3, 1, 2)") == 3

    def test_sin(self):
        assert _eval("sin(0)") == 0.0

    def test_cos(self):
        assert _eval("cos(0)") == 1.0

    def test_log(self):
        assert _eval("log(1)") == 0.0

    def test_pow(self):
        assert _eval("pow(2, 3)") == 8.0  # math.pow returns float

    def test_hypot(self):
        assert _eval("hypot(3, 4)") == 5.0

    def test_isinf(self):
        assert _eval("isinf(1)") == 0

    def test_isnan(self):
        assert _eval("isnan(1)") == 0

    def test_rand_not_foldable(self):
        assert _eval("rand()") is None

    def test_entier(self):
        assert _eval("entier(3.7)") == 3

    def test_wide(self):
        assert _eval("wide(42)") == 42


class TestVariables:
    def test_simple_var(self):
        assert _eval("$a + 1", {"a": 5}) == 6

    def test_two_vars(self):
        assert _eval("$a + $b", {"a": 3, "b": 4}) == 7

    def test_unbound_var(self):
        assert _eval("$a + 1") is None

    def test_string_constant(self):
        """String constants are auto-coerced to numbers."""
        assert _eval("$a + 1", {"a": "5"}) == 6

    def test_string_non_numeric(self):
        assert _eval("$a + 1", {"a": "hello"}) is None


class TestUnevaluable:
    def test_command_subst(self):
        assert _eval("[clock seconds]") is None

    def test_raw(self):
        # An expression the parser can't handle returns None
        assert _eval("") is None

    def test_nested_command(self):
        assert _eval("1 + [foo]") is None


class TestFormatTclValue:
    def test_int(self):
        assert format_tcl_value(42) == "42"

    def test_float_whole(self):
        assert format_tcl_value(3.0) == "3.0"

    def test_float_fractional(self):
        assert format_tcl_value(3.14) == "3.14"

    def test_negative(self):
        assert format_tcl_value(-7) == "-7"

    def test_zero(self):
        assert format_tcl_value(0) == "0"


# Tests ported from Tcl 9.0.2 test suite (expr.test, expr-old.test)


class TestTcl9ArithmeticExtended:
    """Arithmetic tests from Tcl 9.0.2 expr.test / expr-old.test."""

    def test_add_hex(self):
        assert _eval("0xff + 0x3") == 258

    def test_add_negative(self):
        assert _eval("4 + -2") == 2

    def test_sub_negative_hex(self):
        assert _eval("-0xf2 - -0x3") == -239

    def test_mul_hex(self):
        assert _eval("0xff * 0x3") == 765

    def test_mul_negative(self):
        assert _eval("4 * -2") == -8

    def test_div_exact(self):
        assert _eval("36 / 12") == 3

    def test_div_truncate(self):
        assert _eval("27 / 4") == 6

    def test_div_neg_dividend(self):
        assert _eval("-1 / 2") == -1

    def test_div_float_exact(self):
        assert _eval("36.0 / 12.0") == 3.0

    def test_div_mixed_float(self):
        assert _eval("27 / 4.0") == 6.75

    def test_mod_hex(self):
        assert _eval("0xff % 2") == 1

    def test_mod_octal(self):
        assert _eval("7891 % 0o123") == 6

    def test_mod_negative_both(self):
        assert _eval("-0xf2 % -0x3") == -2

    def test_mod_floor_pos_pos(self):
        assert _eval("36 % 5") == 1

    def test_mod_floor_neg_pos(self):
        assert _eval("-36 % 5") == 4

    def test_mod_floor_pos_neg(self):
        assert _eval("36 % -5") == -4

    def test_mod_floor_neg_neg(self):
        assert _eval("-36 % -5") == -1


class TestTcl9Exponentiation:
    """Exponentiation tests from Tcl 9.0.2."""

    def test_simple_power(self):
        assert _eval("4 ** 2") == 16

    def test_hex_base(self):
        assert _eval("0xff ** 2") == 65025

    def test_octal_exponent(self):
        assert _eval("18 ** 07") == 612220032

    def test_hex_both(self):
        assert _eval("0xff ** 0x3") == 16581375

    def test_zero_pow_one(self):
        assert _eval("0 ** 1") == 0

    def test_zero_pow_zero(self):
        assert _eval("0 ** 0") == 1

    def test_neg_exp_truncate(self):
        assert _eval("2 ** -2") == 0

    def test_neg_one_pow_neg_one(self):
        assert _eval("(-1) ** -1") == -1

    def test_one_pow_large(self):
        assert _eval("1 ** 1234567") == 1

    def test_float_base_neg_exp(self):
        assert _eval("2.0 ** -1") == 0.5

    def test_big_pow_17(self):
        assert _eval("10 ** 17") == 100000000000000000

    def test_big_pow_18(self):
        assert _eval("10 ** 18") == 1000000000000000000

    def test_big_pow_19(self):
        assert _eval("10 ** 19") == 10000000000000000000


class TestTcl9Comparisons:
    """Comparison tests from Tcl 9.0.2."""

    def test_hex_ge(self):
        assert _eval("0xff >= +0x3") == 1

    def test_neg_hex_lt(self):
        assert _eval("-0xf2 < 0x3") == 1

    def test_shift_then_compare(self):
        assert _eval("6 >> 1 >= 3") == 1

    def test_float_gt(self):
        assert _eval("3.1 > 2.1") == 1
        assert _eval("2.1 > 2.1") == 0

    def test_float_lt(self):
        assert _eval("1.1 < 2.1") == 1

    def test_float_ge(self):
        assert _eval("3.1 >= 2.2") == 1
        assert _eval("2.345 >= 2.345") == 1
        assert _eval("1.1 >= 2.2") == 0

    def test_float_le(self):
        assert _eval("2.1 <= 2.1") == 1

    def test_float_eq(self):
        assert _eval("2.2 == 2.2") == 1

    def test_float_ne(self):
        assert _eval("3.2 != 2.2") == 1

    def test_mixed_type_comparisons(self):
        assert _eval("2 > 2.5") == 0
        assert _eval("2.5 > 2") == 1
        assert _eval("2 < 2.5") == 1
        assert _eval("2 >= 2.5") == 0
        assert _eval("2 <= 2.5") == 1
        assert _eval("2 == 2.5") == 0
        assert _eval("2 != 2.5") == 1


class TestTcl9Logical:
    """Logical operator tests from Tcl 9.0.2."""

    def test_and_floats(self):
        assert _eval("1.3 && 3.3") == 1

    def test_and_zero_float(self):
        assert _eval("0.0 && 1.3") == 0

    def test_or_floats(self):
        assert _eval("0.0 || 1.3") == 1
        assert _eval("3.0 || 0.0") == 1

    def test_or_multiple(self):
        assert _eval("0 || 0 || 1") == 1

    def test_and_multiple(self):
        assert _eval("1 && 1 && 2") == 1

    def test_not_zero_float(self):
        assert _eval("!0.0") == 1

    def test_not_nonzero(self):
        assert _eval("!27") == 0

    def test_not_nonzero_float(self):
        assert _eval("!2.1") == 0


class TestTcl9Bitwise:
    """Bitwise operator tests from Tcl 9.0.2."""

    def test_and_hex(self):
        assert _eval("7 & 0x13") == 3
        assert _eval("0xf2 & 0x53") == 82

    def test_and_negative(self):
        assert _eval("-1 & -7") == -7

    def test_or_hex(self):
        assert _eval("7 | 0x13") == 23

    def test_or_zero(self):
        assert _eval("0 | 7") == 7

    def test_xor_hex(self):
        assert _eval("7 ^ 0x13") == 20
        assert _eval("3 ^ 0x10") == 19
        assert _eval("0 ^ 7") == 7

    def test_xor_negative(self):
        assert _eval("-1 ^ 7") == -8

    def test_not_small(self):
        assert _eval("~4") == -5

    def test_not_large(self):
        assert _eval("~0xff00ff") == -16711936

    def test_lshift_hex(self):
        assert _eval("1 << 3") == 8

    def test_lshift_negative_base(self):
        assert _eval("-0xf2 << 0x3") == -1936

    def test_rshift_hex(self):
        assert _eval("0xff >> 2") == 63

    def test_rshift_negative(self):
        assert _eval("-1 >> 2") == -1

    def test_shift_with_expr(self):
        assert _eval("7 + 1 << 2") == 32
        assert _eval("7 >> 3 - 2") == 3


class TestTcl9Unary:
    """Unary operator tests from Tcl 9.0.2."""

    def test_unary_plus(self):
        assert _eval("+36") == 36

    def test_unary_minus(self):
        assert _eval("-4") == -4

    def test_double_minus(self):
        assert _eval("--5") == 5

    def test_mixed_unary(self):
        assert _eval("+--++36") == 36

    def test_not_nonzero(self):
        assert _eval("!2") == 0

    def test_not_zero(self):
        assert _eval("!0") == 1

    def test_bitwise_not(self):
        assert _eval("~3") == -4

    def test_unary_octal(self):
        assert _eval("+0o00123") == 83

    def test_unary_neg_float(self):
        assert _eval("-4.2") == -4.2

    def test_unary_complex_signs(self):
        assert _eval("+--+-62.0") == -62.0


class TestTcl9TernaryExtended:
    """Ternary operator tests from Tcl 9.0.2."""

    def test_nested_ternary(self):
        assert _eval("1 ? 2 ? 3 : 4 : 0") == 3

    def test_nested_false_outer(self):
        assert _eval("0 ? 2 ? 3 : 4 : 0") == 0

    def test_with_comparison(self):
        assert _eval("3 > 2 ? 44 : 66") == 44

    def test_false_arm(self):
        assert _eval("2 > 3 ? 44 : 66") == 66

    def test_with_or(self):
        assert _eval("1 || 0 ? 3 : 4") == 3


class TestTcl9MathFunctionsExtended:
    """Math function tests from Tcl 9.0.2."""

    def test_abs_large_negative(self):
        assert _eval("abs(-2147483648)") == 2147483648

    def test_abs_zero(self):
        assert _eval("abs(-0)") == 0

    def test_abs_float_zero(self):
        assert _eval("abs(-0.0)") == 0.0

    def test_round_half_up(self):
        assert _eval("round(0.5)") == 1

    def test_round_1_5(self):
        assert _eval("round(1.5)") == 2

    def test_round_neg_half(self):
        assert _eval("round(-0.5)") == -1

    def test_round_neg_1_5(self):
        assert _eval("round(-1.5)") == -2

    def test_round_int_valued_float(self):
        assert _eval("round(3.0)") == 3

    def test_int_passthrough(self):
        assert _eval("int(5)") == 5

    def test_double_int(self):
        r = _eval("double(27)")
        assert r == 27.0
        assert isinstance(r, float)

    def test_bool_composition(self):
        assert _eval("bool(-1 + 1)") == 0
        assert _eval("bool(0 + 1)") == 1

    def test_log10(self):
        assert _eval("log10(100)") == 2.0

    def test_exp(self):
        assert abs(_eval("exp(1)") - math.e) < 1e-10

    def test_atan(self):
        assert _eval("atan(0)") == 0.0

    def test_tan(self):
        assert _eval("tan(0)") == 0.0

    def test_asin(self):
        assert _eval("asin(0)") == 0.0

    def test_acos(self):
        assert abs(_eval("acos(0)") - math.pi / 2) < 1e-10

    def test_sinh(self):
        assert _eval("sinh(0)") == 0.0

    def test_cosh(self):
        assert _eval("cosh(0)") == 1.0

    def test_tanh(self):
        assert _eval("tanh(0)") == 0.0

    def test_atan2(self):
        assert _eval("atan2(0, 1)") == 0.0

    def test_fmod(self):
        assert _eval("fmod(7.5, 3.0)") == 1.5

    def test_isfinite_int(self):
        assert _eval("isfinite(42)") == 1

    def test_isfinite_float(self):
        assert _eval("isfinite(3.14)") == 1


class TestTcl9OperatorPrecedence:
    """Operator precedence tests from Tcl 9.0.2 expr.test."""

    def test_neg_bitwise_not(self):
        assert _eval("-~3") == 4

    def test_neg_logical_not(self):
        assert _eval("-!3") == 0

    def test_neg_bitwise_not_zero(self):
        assert _eval("-~0") == 1

    def test_mul_div(self):
        assert _eval("2 * 4 / 6") == 1

    def test_div_mul(self):
        assert _eval("24 / 6 * 3") == 12

    def test_div_div(self):
        assert _eval("24 / 6 / 2") == 2

    def test_add_sub(self):
        assert _eval("-2 + 4") == 2

    def test_sub_sub(self):
        assert _eval("6 - 3 - 2") == 1

    def test_mul_before_add(self):
        assert _eval("2 * 3 + 4") == 10

    def test_div_before_add(self):
        assert _eval("8 / 2 + 4") == 8

    def test_mod_before_add(self):
        assert _eval("8 % 3 + 4") == 6

    def test_add_then_rshift(self):
        assert _eval("7 + 1 >> 2") == 2

    def test_add_then_lshift(self):
        assert _eval("7 + 1 << 2") == 32

    def test_sub_then_rshift(self):
        assert _eval("7 >> 3 - 2") == 3

    def test_sub_then_lshift(self):
        assert _eval("7 << 3 - 2") == 14

    def test_and_xor(self):
        assert _eval("7 & 3 ^ 0x10") == 19

    def test_xor_or(self):
        assert _eval("7 ^ 0x10 | 3") == 23

    def test_logical_and_or(self):
        assert _eval("0 && 1 || 1") == 1
        assert _eval("1 || 1 && 0") == 1

    def test_chained_comparisons(self):
        """Tcl chains comparisons left-to-right: (2 < 3) < 4 = 1 < 4 = 1."""
        assert _eval("2 < 3 < 4") == 1
        assert _eval("4 > 3 >= 2") == 0  # (4>3)=1, 1>=2 = 0
        assert _eval("1 == 4 > 3") == 1  # > higher prec: 1 == (4>3) = 1 == 1


class TestTcl9TypeConversions:
    """Mixed type expression tests from Tcl 9.0.2."""

    def test_int_plus_float(self):
        assert _eval("2 + 2.5") == 4.5

    def test_float_plus_int(self):
        assert _eval("2.5 + 2") == 4.5

    def test_int_minus_float(self):
        assert _eval("2 - 2.5") == -0.5

    def test_int_div_float(self):
        assert _eval("2 / 2.5") == 0.8

    def test_scientific_notation(self):
        assert _eval("2.0e2") == 200.0

    def test_scientific_large(self):
        assert _eval("2.0e15") == 2000000000000000.0


class TestTcl9NumberFormats:
    """Number format tests from Tcl 9.0.2."""

    def test_octal(self):
        assert _eval("0o15") == 13

    def test_hex(self):
        assert _eval("0x20") == 32

    def test_negative_hex(self):
        assert _eval("-0xff") == -255

    def test_hex_add(self):
        assert _eval("0xff + 0x3") == 258


class TestTcl9BoolLiterals:
    """Boolean literal tests from Tcl 9.0.2."""

    def test_not_true(self):
        assert _eval("!true") == 0

    def test_not_false(self):
        assert _eval("!false") == 1

    def test_not_off(self):
        assert _eval("!off") == 1

    def test_not_on(self):
        assert _eval("!on") == 0

    def test_not_yes(self):
        assert _eval("!yes") == 0

    def test_not_no(self):
        assert _eval("!no") == 1

    def test_true_and_true(self):
        assert _eval("true && true") == 1

    def test_false_or_true(self):
        assert _eval("false || true") == 1

    def test_true_eq_one(self):
        assert _eval("true == 1") == 1

    def test_false_eq_zero(self):
        assert _eval("false == 0") == 1


class TestTcl9EdgeCases:
    """Edge cases from Tcl 9.0.2 test suite."""

    def test_div_by_zero_int(self):
        assert _eval("1 / 0") is None

    def test_div_by_zero_float(self):
        assert _eval("1.0 / 0") is None

    def test_mod_by_zero(self):
        assert _eval("1 % 0") is None

    def test_zero_pow_negative(self):
        assert _eval("0 ** -1") is None

    def test_huge_exponent(self):
        assert _eval("2 ** 100000") is None

    def test_rshift_large(self):
        """Large shift amounts should be handled safely."""
        assert _eval("5 >> 32") == 0

    def test_parenthesized_expr(self):
        assert _eval("(1 + 2) * 3") == 9

    def test_nested_parens(self):
        assert _eval("((1 + 2) * (3 + 4))") == 21

    def test_complex_expr(self):
        assert _eval("2 + 3 * 4 - 1") == 13

    def test_float_div_preserves_type(self):
        r = _eval("10.0 / 3.0")
        assert isinstance(r, float)
        assert abs(r - 3.3333333333333335) < 1e-10


class TestTcl9OptimiserFolding:
    """Tests verifying that the new evaluator folds expressions the old
    safe_eval_expr could not (division, exponentiation, math functions, floats).
    """

    def test_fold_division(self):
        """Division was unfoldable in the old whitelist-based evaluator."""
        assert _eval("100 / 3") == 33
        assert _eval("10 / 4") == 2

    def test_fold_exponentiation(self):
        """Exponentiation was unfoldable in the old evaluator."""
        assert _eval("2 ** 10") == 1024
        assert _eval("3 ** 3") == 27

    def test_fold_math_functions(self):
        """Math functions were completely unfoldable."""
        assert _eval("abs(-42)") == 42
        assert _eval("min(5, 3, 7)") == 3
        assert _eval("max(5, 3, 7)") == 7

    def test_fold_float_expressions(self):
        """Float arithmetic was unfoldable."""
        assert _eval("3.14 * 2") == 6.28
        assert _eval("1.5 + 2.5") == 4.0

    def test_fold_trig(self):
        """Trig functions were unfoldable."""
        assert _eval("sin(0)") == 0.0
        assert _eval("cos(0)") == 1.0

    def test_fold_combined(self):
        """Complex expressions with mixed types and functions."""
        assert _eval("abs(-5) + 3") == 8
        assert _eval("int(3.7) * 2") == 6
        assert _eval("round(2.5) + 1") == 4


# iRules expression operators (dialect="f5-irules")


def _irule_eval(expr, variables=None):
    return eval_tcl_expr_str(expr, variables, dialect="f5-irules")


class TestIRulesContains:
    def test_substring_found(self):
        assert _irule_eval('"hello world" contains "world"') == 1

    def test_substring_not_found(self):
        assert _irule_eval('"hello world" contains "xyz"') == 0

    def test_empty_needle(self):
        assert _irule_eval('"hello" contains ""') == 1

    def test_empty_haystack(self):
        assert _irule_eval('"" contains "a"') == 0

    def test_case_sensitive(self):
        assert _irule_eval('"Hello" contains "hello"') == 0

    def test_contains_with_variable(self):
        assert _irule_eval('$uri contains "/admin"', {"uri": "/admin/login"}) == 1
        assert _irule_eval('$uri contains "/admin"', {"uri": "/public"}) == 0

    def test_full_string(self):
        assert _irule_eval('"abc" contains "abc"') == 1


class TestIRulesStartsWith:
    def test_prefix_match(self):
        assert _irule_eval('"hello world" starts_with "hello"') == 1

    def test_prefix_no_match(self):
        assert _irule_eval('"hello world" starts_with "world"') == 0

    def test_empty_prefix(self):
        assert _irule_eval('"hello" starts_with ""') == 1

    def test_exact_match(self):
        assert _irule_eval('"abc" starts_with "abc"') == 1

    def test_longer_prefix(self):
        assert _irule_eval('"ab" starts_with "abc"') == 0

    def test_with_variable(self):
        assert _irule_eval('$path starts_with "/api/"', {"path": "/api/v2/users"}) == 1


class TestIRulesEndsWith:
    def test_suffix_match(self):
        assert _irule_eval('"hello.txt" ends_with ".txt"') == 1

    def test_suffix_no_match(self):
        assert _irule_eval('"hello.txt" ends_with ".jpg"') == 0

    def test_empty_suffix(self):
        assert _irule_eval('"hello" ends_with ""') == 1

    def test_exact_match(self):
        assert _irule_eval('"abc" ends_with "abc"') == 1

    def test_longer_suffix(self):
        assert _irule_eval('"bc" ends_with "abc"') == 0

    def test_with_variable(self):
        assert _irule_eval('$host ends_with ".example.com"', {"host": "www.example.com"}) == 1


class TestIRulesEquals:
    def test_equal_strings(self):
        assert _irule_eval('"hello" equals "hello"') == 1

    def test_unequal_strings(self):
        assert _irule_eval('"hello" equals "world"') == 0

    def test_case_sensitive(self):
        assert _irule_eval('"Hello" equals "hello"') == 0

    def test_empty_strings(self):
        assert _irule_eval('"" equals ""') == 1

    def test_with_variable(self):
        assert _irule_eval('$method equals "GET"', {"method": "GET"}) == 1
        assert _irule_eval('$method equals "GET"', {"method": "POST"}) == 0


class TestIRulesMatchesGlob:
    def test_star_pattern(self):
        assert _irule_eval('"hello.txt" matches_glob "*.txt"') == 1

    def test_star_no_match(self):
        assert _irule_eval('"hello.txt" matches_glob "*.jpg"') == 0

    def test_question_mark(self):
        assert _irule_eval('"cat" matches_glob "c?t"') == 1
        assert _irule_eval('"cart" matches_glob "c?t"') == 0

    def test_exact_match(self):
        assert _irule_eval('"hello" matches_glob "hello"') == 1

    def test_complex_pattern(self):
        assert _irule_eval('"/api/v2/users" matches_glob "/api/*/users"') == 1

    def test_bracket_range(self):
        assert _irule_eval('"a1" matches_glob "a[0-9]"') == 1
        assert _irule_eval('"ax" matches_glob "a[0-9]"') == 0

    def test_with_variable(self):
        assert _irule_eval('$uri matches_glob "/images/*"', {"uri": "/images/logo.png"}) == 1


class TestIRulesMatchesRegex:
    def test_basic_match(self):
        assert _irule_eval('"hello123" matches_regex "[a-z]+[0-9]+"') == 1

    def test_no_match(self):
        assert _irule_eval('"hello" matches_regex "^[0-9]+$"') == 0

    def test_partial_match(self):
        # matches_regex uses search (partial match), like Tcl regexp
        assert _irule_eval('"abc123def" matches_regex "[0-9]+"') == 1

    def test_anchored(self):
        assert _irule_eval('"hello" matches_regex "^hello$"') == 1
        assert _irule_eval('"hello world" matches_regex "^hello$"') == 0

    def test_invalid_regex(self):
        assert _irule_eval('"hello" matches_regex "[invalid"') is None

    def test_dot_star(self):
        assert _irule_eval('"anything" matches_regex ".*"') == 1

    def test_with_variable(self):
        assert (
            _irule_eval(
                '$uri matches_regex "^/api/v[0-9]+/"',
                {"uri": "/api/v3/users"},
            )
            == 1
        )


class TestIRulesIn:
    def test_element_in_list(self):
        assert _irule_eval('"apple" in "apple banana cherry"') == 1

    def test_element_not_in_list(self):
        assert _irule_eval('"grape" in "apple banana cherry"') == 0

    def test_single_element_list(self):
        assert _irule_eval('"apple" in "apple"') == 1

    def test_empty_list(self):
        assert _irule_eval('"apple" in ""') == 0

    def test_braced_elements(self):
        assert _irule_eval('"hello world" in "{hello world} simple"') == 1

    def test_partial_no_match(self):
        # "app" is not a list element even though "apple" starts with it
        assert _irule_eval('"app" in "apple banana"') == 0

    def test_with_variable(self):
        assert _irule_eval('$method in "GET HEAD OPTIONS"', {"method": "GET"}) == 1
        assert _irule_eval('$method in "GET HEAD OPTIONS"', {"method": "POST"}) == 0


class TestIRulesNi:
    def test_element_not_in_list(self):
        assert _irule_eval('"grape" ni "apple banana cherry"') == 1

    def test_element_in_list(self):
        assert _irule_eval('"apple" ni "apple banana cherry"') == 0

    def test_empty_list(self):
        assert _irule_eval('"apple" ni ""') == 1

    def test_with_variable(self):
        assert _irule_eval('$method ni "GET HEAD"', {"method": "POST"}) == 1
        assert _irule_eval('$method ni "GET HEAD"', {"method": "GET"}) == 0


class TestIRulesWordLogical:
    def test_and_true(self):
        assert _irule_eval("1 and 1") == 1

    def test_and_false(self):
        assert _irule_eval("1 and 0") == 0
        assert _irule_eval("0 and 1") == 0

    def test_and_short_circuit(self):
        # 0 and [cmd] — should not evaluate RHS
        assert _irule_eval("0 and [some_cmd]") == 0

    def test_or_true(self):
        assert _irule_eval("1 or 0") == 1
        assert _irule_eval("0 or 1") == 1

    def test_or_false(self):
        assert _irule_eval("0 or 0") == 0

    def test_or_short_circuit(self):
        assert _irule_eval("1 or [some_cmd]") == 1

    def test_not(self):
        assert _irule_eval("not 1") == 0
        assert _irule_eval("not 0") == 1


class TestIRulesCombinedExpressions:
    def test_contains_and_starts_with(self):
        assert (
            _irule_eval(
                '"http://example.com/api" contains "api" and "http://example.com/api" starts_with "http"'
            )
            == 1
        )

    def test_negated_contains(self):
        assert _irule_eval('not ("hello" contains "xyz")') == 1

    def test_or_with_string_ops(self):
        assert _irule_eval('"test.jpg" ends_with ".jpg" or "test.jpg" ends_with ".png"') == 1
        assert _irule_eval('"test.gif" ends_with ".jpg" or "test.gif" ends_with ".png"') == 0

    def test_in_with_logical(self):
        assert (
            _irule_eval(
                '$method in "GET HEAD" and $uri starts_with "/api"',
                {"method": "GET", "uri": "/api/data"},
            )
            == 1
        )

    def test_variable_in_both_sides(self):
        assert (
            _irule_eval(
                "$host contains $domain",
                {"host": "www.example.com", "domain": "example"},
            )
            == 1
        )

    def test_ternary_with_string_op(self):
        assert _irule_eval('"hello" contains "ell" ? 42 : 0') == 42

    def test_comparison_with_string_result(self):
        # Mix numeric and string operators
        assert (
            _irule_eval(
                '($x > 10) and ($uri contains "/admin")',
                {"x": 15, "uri": "/admin/panel"},
            )
            == 1
        )
