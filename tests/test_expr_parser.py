"""Tests for the Tcl expression Pratt parser."""

from __future__ import annotations

from core.compiler.expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
    ExprVar,
    UnaryOp,
    render_expr,
    vars_in_expr_node,
)
from core.parsing.expr_parser import parse_expr


class TestLiterals:
    def test_integer(self):
        node = parse_expr("42")
        assert isinstance(node, ExprLiteral)
        assert node.text == "42"

    def test_negative_integer(self):
        node = parse_expr("-7")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NEG
        assert isinstance(node.operand, ExprLiteral)
        assert node.operand.text == "7"

    def test_float(self):
        node = parse_expr("3.14")
        assert isinstance(node, ExprLiteral)
        assert node.text == "3.14"

    def test_hex(self):
        node = parse_expr("0xFF")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0xFF"

    def test_boolean_true(self):
        node = parse_expr("true")
        assert isinstance(node, ExprLiteral)
        assert node.text == "true"

    def test_boolean_false(self):
        node = parse_expr("false")
        assert isinstance(node, ExprLiteral)
        assert node.text == "false"

    def test_string_literal(self):
        node = parse_expr('"hello"')
        assert isinstance(node, ExprString)
        assert node.text == '"hello"'


class TestVariables:
    def test_simple_var(self):
        node = parse_expr("$x")
        assert isinstance(node, ExprVar)
        assert node.text == "$x"
        assert node.name == "x"

    def test_braced_var(self):
        node = parse_expr("${my_var}")
        assert isinstance(node, ExprVar)
        assert node.name == "my_var"

    def test_namespaced_var(self):
        node = parse_expr("$ns::count")
        assert isinstance(node, ExprVar)
        assert node.name == "ns::count"

    def test_array_var(self):
        node = parse_expr("$arr(idx)")
        assert isinstance(node, ExprVar)
        assert node.name == "arr"


class TestCommandSubstitution:
    def test_simple_command(self):
        node = parse_expr("[llength $list]")
        assert isinstance(node, ExprCommand)
        assert node.text == "[llength $list]"


class TestBinaryOperators:
    def test_addition(self):
        node = parse_expr("$a + $b")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD
        assert isinstance(node.left, ExprVar)
        assert node.left.name == "a"
        assert isinstance(node.right, ExprVar)
        assert node.right.name == "b"

    def test_subtraction(self):
        node = parse_expr("$a - 1")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.SUB

    def test_multiplication(self):
        node = parse_expr("$x * $y")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MUL

    def test_division(self):
        node = parse_expr("$x / 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.DIV

    def test_modulo(self):
        node = parse_expr("$x % 3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MOD

    def test_power(self):
        node = parse_expr("$x ** 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.POW

    def test_left_shift(self):
        node = parse_expr("$x << 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LSHIFT

    def test_right_shift(self):
        node = parse_expr("$x >> 3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.RSHIFT

    def test_bitwise_and(self):
        node = parse_expr("$x & 0xFF")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_AND

    def test_bitwise_or(self):
        node = parse_expr("$x | $y")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_OR

    def test_bitwise_xor(self):
        node = parse_expr("$x ^ $y")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_XOR

    def test_logical_and(self):
        node = parse_expr("$x && $y")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.AND

    def test_logical_or(self):
        node = parse_expr("$x || $y")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.OR

    def test_eq(self):
        node = parse_expr("$x == 5")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.EQ

    def test_ne(self):
        node = parse_expr("$x != 0")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.NE

    def test_lt(self):
        node = parse_expr("$x < 10")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LT

    def test_le(self):
        node = parse_expr("$x <= 10")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LE

    def test_gt(self):
        node = parse_expr("$x > 0")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.GT

    def test_ge(self):
        node = parse_expr("$x >= 1")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.GE

    def test_str_eq(self):
        node = parse_expr('$x eq "hello"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_EQ

    def test_str_ne(self):
        node = parse_expr('$x ne "world"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_NE

    def test_in_operator(self):
        node = parse_expr("$x in $list")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.IN

    def test_ni_operator(self):
        node = parse_expr("$x ni $list")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.NI


class TestUnaryOperators:
    def test_negate(self):
        node = parse_expr("-$x")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NEG
        assert isinstance(node.operand, ExprVar)

    def test_positive(self):
        node = parse_expr("+$x")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.POS

    def test_logical_not(self):
        node = parse_expr("!$flag")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NOT

    def test_bitwise_not(self):
        node = parse_expr("~$mask")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.BIT_NOT


class TestPrecedence:
    def test_mul_before_add(self):
        # $a + $b * $c  →  $a + ($b * $c)
        node = parse_expr("$a + $b * $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD
        assert isinstance(node.left, ExprVar)
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.MUL

    def test_add_left_associative(self):
        # $a + $b + $c  →  ($a + $b) + $c
        node = parse_expr("$a + $b + $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.ADD

    def test_power_right_associative(self):
        # 2 ** 3 ** 2  →  2 ** (3 ** 2)
        node = parse_expr("2 ** 3 ** 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.POW
        assert isinstance(node.left, ExprLiteral)
        assert node.left.text == "2"
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.POW

    def test_comparison_lower_than_arithmetic(self):
        # $a + 1 < $b * 2  →  ($a + 1) < ($b * 2)
        node = parse_expr("$a + 1 < $b * 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LT
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.ADD
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.MUL

    def test_logical_lower_than_comparison(self):
        # $a > 0 && $b < 10  →  ($a > 0) && ($b < 10)
        node = parse_expr("$a > 0 && $b < 10")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.AND
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.GT
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.LT

    def test_or_lower_than_and(self):
        # $a && $b || $c  →  ($a && $b) || $c
        node = parse_expr("$a && $b || $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.OR
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.AND

    def test_unary_higher_than_binary(self):
        # -$a + $b  →  (-$a) + $b
        node = parse_expr("-$a + $b")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD
        assert isinstance(node.left, ExprUnary)
        assert node.left.op is UnaryOp.NEG

    def test_parenthesised_override(self):
        # ($a + $b) * $c
        node = parse_expr("($a + $b) * $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MUL
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.ADD

    def test_bitwise_precedence(self):
        # $a | $b & $c  →  $a | ($b & $c)
        node = parse_expr("$a | $b & $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_OR
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.BIT_AND

    def test_shift_between_add_and_comparison(self):
        # $a + $b << 2 < $c  →  (($a + $b) << 2) < $c
        node = parse_expr("$a + $b << 2 < $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LT
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.LSHIFT


class TestTernary:
    def test_simple_ternary(self):
        node = parse_expr("$x ? 1 : 0")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.condition, ExprVar)
        assert isinstance(node.true_branch, ExprLiteral)
        assert isinstance(node.false_branch, ExprLiteral)

    def test_nested_ternary_right_assoc(self):
        # $a ? 1 : $b ? 2 : 3  →  $a ? 1 : ($b ? 2 : 3)
        node = parse_expr("$a ? 1 : $b ? 2 : 3")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.false_branch, ExprTernary)

    def test_ternary_with_expressions(self):
        node = parse_expr("$a > 0 ? $a : -$a")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.condition, ExprBinary)
        assert node.condition.op is BinOp.GT
        assert isinstance(node.false_branch, ExprUnary)


class TestFunctionCalls:
    def test_single_arg(self):
        node = parse_expr("sin($x)")
        assert isinstance(node, ExprCall)
        assert node.function == "sin"
        assert len(node.args) == 1
        assert isinstance(node.args[0], ExprVar)

    def test_two_args(self):
        node = parse_expr("pow($a, $b)")
        assert isinstance(node, ExprCall)
        assert node.function == "pow"
        assert len(node.args) == 2

    def test_nested_function(self):
        node = parse_expr("int(sin($x))")
        assert isinstance(node, ExprCall)
        assert node.function == "int"
        assert len(node.args) == 1
        assert isinstance(node.args[0], ExprCall)
        assert node.args[0].function == "sin"

    def test_function_with_expression_arg(self):
        node = parse_expr("abs($x - $y)")
        assert isinstance(node, ExprCall)
        assert node.function == "abs"
        assert len(node.args) == 1
        assert isinstance(node.args[0], ExprBinary)

    def test_no_args(self):
        node = parse_expr("rand()")
        assert isinstance(node, ExprCall)
        assert node.function == "rand"
        assert len(node.args) == 0

    def test_max_multiple_args(self):
        node = parse_expr("max($a, $b, $c)")
        assert isinstance(node, ExprCall)
        assert node.function == "max"
        assert len(node.args) == 3


class TestComplexExpressions:
    def test_chained_arithmetic(self):
        node = parse_expr("$a + $b * $c - $d / $e")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.SUB

    def test_nested_parens(self):
        node = parse_expr("(($a + $b)) * $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MUL

    def test_mixed_operators(self):
        node = parse_expr("$a > 0 && $b < 10 || $c == 0")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.OR

    def test_command_in_arithmetic(self):
        node = parse_expr("[llength $list] + 1")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD
        assert isinstance(node.left, ExprCommand)

    def test_unary_chain(self):
        node = parse_expr("!!$flag")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NOT
        assert isinstance(node.operand, ExprUnary)


class TestFallback:
    def test_empty_expression(self):
        node = parse_expr("")
        assert isinstance(node, ExprRaw)

    def test_whitespace_only(self):
        node = parse_expr("   ")
        assert isinstance(node, ExprRaw)


class TestRenderExpr:
    def test_round_trip_simple(self):
        node = parse_expr("$a + $b")
        text = render_expr(node)
        assert text == "$a + $b"

    def test_round_trip_unary(self):
        node = parse_expr("-$x")
        text = render_expr(node)
        assert text == "-$x"

    def test_round_trip_ternary(self):
        node = parse_expr("$x ? 1 : 0")
        text = render_expr(node)
        assert text == "$x ? 1 : 0"

    def test_round_trip_function(self):
        node = parse_expr("sin($x)")
        text = render_expr(node)
        assert text == "sin($x)"

    def test_raw_round_trip(self):
        node = ExprRaw(text="malformed {}")
        assert render_expr(node) == "malformed {}"


class TestVarsInExprNode:
    def test_simple_var(self):
        node = parse_expr("$x + 1")
        assert vars_in_expr_node(node) == {"x"}

    def test_multiple_vars(self):
        node = parse_expr("$a + $b * $c")
        assert vars_in_expr_node(node) == {"a", "b", "c"}

    def test_no_vars(self):
        node = parse_expr("1 + 2")
        assert vars_in_expr_node(node) == set()

    def test_ternary_vars(self):
        node = parse_expr("$cond ? $a : $b")
        assert vars_in_expr_node(node) == {"cond", "a", "b"}

    def test_function_vars(self):
        node = parse_expr("sin($angle)")
        assert vars_in_expr_node(node) == {"angle"}

    def test_raw_no_vars(self):
        node = ExprRaw(text="$x + $y")
        # ExprRaw is opaque — no variable extraction
        assert vars_in_expr_node(node) == set()

    def test_repeated_var(self):
        node = parse_expr("$x + $x")
        assert vars_in_expr_node(node) == {"x"}


# Comprehensive tests based on Tcl 9.0.2 test suite patterns


class TestBooleanLiterals:
    """Test all Tcl boolean literal keywords (from Tcl 9.0.2 expr.test)."""

    def test_yes(self):
        node = parse_expr("yes")
        assert isinstance(node, ExprLiteral)
        assert node.text == "yes"

    def test_no(self):
        node = parse_expr("no")
        assert isinstance(node, ExprLiteral)
        assert node.text == "no"

    def test_on(self):
        node = parse_expr("on")
        assert isinstance(node, ExprLiteral)
        assert node.text == "on"

    def test_off(self):
        node = parse_expr("off")
        assert isinstance(node, ExprLiteral)
        assert node.text == "off"

    def test_not_false(self):
        node = parse_expr("!false")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NOT
        assert isinstance(node.operand, ExprLiteral)
        assert node.operand.text == "false"

    def test_not_true(self):
        node = parse_expr("!true")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NOT
        assert isinstance(node.operand, ExprLiteral)

    def test_not_on(self):
        node = parse_expr("!on")
        assert isinstance(node, ExprUnary)
        assert isinstance(node.operand, ExprLiteral)
        assert node.operand.text == "on"

    def test_not_off(self):
        node = parse_expr("!off")
        assert isinstance(node, ExprUnary)
        assert isinstance(node.operand, ExprLiteral)

    def test_not_yes(self):
        node = parse_expr("!yes")
        assert isinstance(node, ExprUnary)
        assert isinstance(node.operand, ExprLiteral)

    def test_not_no(self):
        node = parse_expr("!no")
        assert isinstance(node, ExprUnary)
        assert isinstance(node.operand, ExprLiteral)

    def test_boolean_in_comparison(self):
        node = parse_expr("true == false")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.EQ
        assert isinstance(node.left, ExprLiteral)
        assert isinstance(node.right, ExprLiteral)

    def test_boolean_in_logical_and(self):
        node = parse_expr("true && on")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.AND

    def test_boolean_in_ternary(self):
        node = parse_expr("yes ? 1 : 0")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.condition, ExprLiteral)
        assert node.condition.text == "yes"


class TestNumericLiterals:
    """Test various numeric literal formats from Tcl 9.0.2 expr.test."""

    def test_octal_literal(self):
        node = parse_expr("0o10")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0o10"

    def test_octal_upper(self):
        node = parse_expr("0O77")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0O77"

    def test_binary_literal(self):
        node = parse_expr("0b1010")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0b1010"

    def test_binary_upper(self):
        node = parse_expr("0B1100")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0B1100"

    def test_hex_lower(self):
        node = parse_expr("0xff")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0xff"

    def test_hex_upper(self):
        node = parse_expr("0XFF")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0XFF"

    def test_scientific_notation(self):
        node = parse_expr("1.5e10")
        assert isinstance(node, ExprLiteral)
        assert node.text == "1.5e10"

    def test_scientific_positive_exp(self):
        node = parse_expr("2.3e+5")
        assert isinstance(node, ExprLiteral)
        assert node.text == "2.3e+5"

    def test_scientific_negative_exp(self):
        node = parse_expr("1.0e-3")
        assert isinstance(node, ExprLiteral)
        assert node.text == "1.0e-3"

    def test_scientific_upper_e(self):
        node = parse_expr("1.0E5")
        assert isinstance(node, ExprLiteral)
        assert node.text == "1.0E5"

    def test_leading_dot_float(self):
        node = parse_expr(".5")
        assert isinstance(node, ExprLiteral)
        assert node.text == ".5"

    def test_trailing_dot_float(self):
        node = parse_expr("5.")
        assert isinstance(node, ExprLiteral)
        assert node.text == "5."

    def test_zero(self):
        node = parse_expr("0")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0"

    def test_large_integer(self):
        node = parse_expr("9223372036854775807")
        assert isinstance(node, ExprLiteral)
        assert node.text == "9223372036854775807"


class TestOperatorPrecedenceComprehensive:
    """Full Tcl operator precedence chain from Tcl 9.0.2 parseExpr.test.

    Precedence (highest to lowest):
    **, unary, * / %, + -, << >>, < > <= >= in ni, == != eq ne,
    &, ^, |, &&, ||, ? :
    """

    def test_power_higher_than_mul(self):
        # $a * $b ** $c  →  $a * ($b ** $c)
        node = parse_expr("$a * $b ** $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MUL
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.POW

    def test_div_before_add(self):
        # $a + $b / $c  →  $a + ($b / $c)
        node = parse_expr("$a + $b / $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.DIV

    def test_mod_before_add(self):
        # $a + $b % $c  →  $a + ($b % $c)
        node = parse_expr("$a + $b % $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.MOD

    def test_add_before_shift(self):
        # $a << $b + $c  →  $a << ($b + $c)
        # From parseExpr.test: {1+2 << 3} → << at top, + inside
        node = parse_expr("$a + $b << $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LSHIFT
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.ADD

    def test_shift_before_relational(self):
        # $a << $b < $c  →  ($a << $b) < $c
        # From parseExpr.test: {1<<2 < 3} → < at top, << inside
        node = parse_expr("$a << $b < $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LT
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.LSHIFT

    def test_relational_before_equality(self):
        # $a < $b == $c  →  ($a < $b) == $c
        # From parseExpr.test: {1<2 == 3} → == at top, < inside
        node = parse_expr("$a < $b == $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.EQ
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.LT

    def test_equality_before_bitwise_and(self):
        # $a == $b & $c  →  ($a == $b) & $c
        # From parseExpr.test: {1==2 & 3} → & at top, == inside
        node = parse_expr("$a == $b & $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_AND
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.EQ

    def test_bitwise_and_before_xor(self):
        # $a & $b ^ $c  →  ($a & $b) ^ $c
        # From parseExpr.test: {1&2 ^ 3} → ^ at top, & inside
        node = parse_expr("$a & $b ^ $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_XOR
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.BIT_AND

    def test_xor_before_bitwise_or(self):
        # $a ^ $b | $c  →  ($a ^ $b) | $c
        # From parseExpr.test: {1^2 | 3} → | at top, ^ inside
        node = parse_expr("$a ^ $b | $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_OR
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.BIT_XOR

    def test_bitwise_or_before_logical_and(self):
        # $a | $b && $c  →  ($a | $b) && $c
        # From parseExpr.test: {1|2 && 3} → && at top, | inside
        node = parse_expr("$a | $b && $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.AND
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.BIT_OR

    def test_logical_and_before_logical_or(self):
        # $a && $b || $c  →  ($a && $b) || $c
        # From parseExpr.test: {1&&2 || 3} → || at top, && inside
        node = parse_expr("$a && $b || $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.OR
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.AND

    def test_logical_or_before_ternary(self):
        # $a || $b ? $c : $d  →  ($a || $b) ? $c : $d
        node = parse_expr("$a || $b ? $c : $d")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.condition, ExprBinary)
        assert node.condition.op is BinOp.OR

    def test_full_precedence_chain(self):
        """Complex expression exercising many precedence levels.

        From Tcl 9.0.2 expr.test:
        {27||3? 3<<(1+4) : 4&&9} → 96
        Parsed as: (27||3) ? (3<<(1+4)) : (4&&9)
        """
        node = parse_expr("27 || 3 ? 3 << (1 + 4) : 4 && 9")
        assert isinstance(node, ExprTernary)
        # Condition: 27 || 3
        assert isinstance(node.condition, ExprBinary)
        assert node.condition.op is BinOp.OR
        # True branch: 3 << (1 + 4)
        assert isinstance(node.true_branch, ExprBinary)
        assert node.true_branch.op is BinOp.LSHIFT
        # False branch: 4 && 9
        assert isinstance(node.false_branch, ExprBinary)
        assert node.false_branch.op is BinOp.AND


class TestExponentiationEdgeCases:
    """Exponentiation (**) edge cases from Tcl 9.0.2 expr.test."""

    def test_right_associative_three(self):
        """2 ** 3 ** 4  →  2 ** (3 ** 4) (not (2**3)**4)."""
        node = parse_expr("2 ** 3 ** 4")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.POW
        assert isinstance(node.left, ExprLiteral)
        assert node.left.text == "2"
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.POW
        assert isinstance(node.right.left, ExprLiteral)
        assert node.right.left.text == "3"

    def test_power_with_unary_neg(self):
        """-$x ** 2  →  (-x) ** 2  because unary binds tighter than ** in Tcl."""
        node = parse_expr("-$x ** 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.POW
        assert isinstance(node.left, ExprUnary)
        assert node.left.op is UnaryOp.NEG

    def test_power_with_parens(self):
        """(-$x) ** 2  same as without parens (unary already binds tighter)."""
        node = parse_expr("(-$x) ** 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.POW
        assert isinstance(node.left, ExprUnary)
        assert node.left.op is UnaryOp.NEG

    def test_power_zero_exponent(self):
        node = parse_expr("$x ** 0")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.POW
        assert isinstance(node.right, ExprLiteral)
        assert node.right.text == "0"

    def test_hex_power(self):
        """0xff ** 2 — from Tcl 9.0.2 expr.test."""
        node = parse_expr("0xff ** 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.POW
        assert isinstance(node.left, ExprLiteral)
        assert node.left.text == "0xff"

    def test_hex_power_hex(self):
        """0xff ** 0x3 — from Tcl 9.0.2 expr.test."""
        node = parse_expr("0xff ** 0x3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.POW
        assert isinstance(node.left, ExprLiteral)
        assert isinstance(node.right, ExprLiteral)


class TestShiftOperatorEdgeCases:
    """Shift operator patterns from Tcl 9.0.2 expr.test."""

    def test_left_shift_literal(self):
        node = parse_expr("1 << 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LSHIFT

    def test_right_shift_literal(self):
        node = parse_expr("0xff >> 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.RSHIFT
        assert isinstance(node.left, ExprLiteral)
        assert node.left.text == "0xff"

    def test_shift_with_hex(self):
        """0xff << 0x3 — from Tcl 9.0.2 expr.test."""
        node = parse_expr("0xff << 0x3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LSHIFT
        assert isinstance(node.right, ExprLiteral)
        assert node.right.text == "0x3"

    def test_shift_left_associative(self):
        """$a << $b << $c  →  ($a << $b) << $c."""
        node = parse_expr("$a << $b << $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LSHIFT
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.LSHIFT

    def test_mixed_shifts(self):
        """$a << 2 >> 1  →  ($a << 2) >> 1  (same precedence, left-assoc)."""
        node = parse_expr("$a << 2 >> 1")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.RSHIFT
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.LSHIFT


class TestInNiOperators:
    """List membership operators from Tcl 9.0.2 expr.test."""

    def test_in_with_variables(self):
        node = parse_expr("$x in $list")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.IN

    def test_ni_with_variables(self):
        node = parse_expr("$x ni $list")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.NI

    def test_in_with_string_literal(self):
        node = parse_expr('"a" in $list')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.IN
        assert isinstance(node.left, ExprString)

    def test_in_combined_with_logical(self):
        """$x in $list && $y ni $other."""
        node = parse_expr("$x in $list && $y ni $other")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.AND
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.IN
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.NI

    def test_in_vs_variable_prefix(self):
        """``in`` should not match as operator if followed by alnum."""
        # "inary" should NOT be parsed as "in" operator + "ary"
        # The lexer requires 'in' not followed by alnum/underscore.
        node = parse_expr("$x + $inary")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD


class TestStringComparisonOperators:
    """String comparison operators (lt, le, gt, ge) from Tcl 9.0.2."""

    def test_str_lt(self):
        node = parse_expr('$a lt "z"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_LT

    def test_str_le(self):
        node = parse_expr('$a le "z"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_LE

    def test_str_gt(self):
        node = parse_expr('$a gt "a"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_GT

    def test_str_ge(self):
        node = parse_expr('$a ge "a"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_GE

    def test_str_ne_not_prefix(self):
        """``ne`` should not match if followed by alnum."""
        # "next" should NOT be parsed as "ne" operator + "xt"
        node = parse_expr("$x + $next")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD


class TestTernaryEdgeCases:
    """Ternary operator edge cases from Tcl 9.0.2."""

    def test_nested_ternary_three_levels(self):
        """$a ? 1 : $b ? 2 : $c ? 3 : 4"""
        node = parse_expr("$a ? 1 : $b ? 2 : $c ? 3 : 4")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.false_branch, ExprTernary)
        assert isinstance(node.false_branch.false_branch, ExprTernary)
        assert isinstance(node.false_branch.false_branch.false_branch, ExprLiteral)

    def test_ternary_with_complex_condition(self):
        """($a > 0 && $b < 10) ? $x : $y"""
        node = parse_expr("$a > 0 && $b < 10 ? $x : $y")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.condition, ExprBinary)
        assert node.condition.op is BinOp.AND

    def test_ternary_with_expr_branches(self):
        """$c ? $a + 1 : $b * 2"""
        node = parse_expr("$c ? $a + 1 : $b * 2")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.true_branch, ExprBinary)
        assert node.true_branch.op is BinOp.ADD
        assert isinstance(node.false_branch, ExprBinary)
        assert node.false_branch.op is BinOp.MUL

    def test_ternary_with_function_call(self):
        """$c ? sin($x) : cos($x)"""
        node = parse_expr("$c ? sin($x) : cos($x)")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.true_branch, ExprCall)
        assert isinstance(node.false_branch, ExprCall)

    def test_ternary_with_command_sub(self):
        """$c ? [cmd1] : [cmd2]"""
        node = parse_expr("$c ? [cmd1] : [cmd2]")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.true_branch, ExprCommand)
        assert isinstance(node.false_branch, ExprCommand)


class TestFunctionCallEdgeCases:
    """Math function edge cases from Tcl 9.0.2 compExpr.test."""

    def test_nested_math_functions(self):
        """sqrt(sin($x) ** 2 + cos($x) ** 2)"""
        node = parse_expr("sqrt(sin($x) ** 2 + cos($x) ** 2)")
        assert isinstance(node, ExprCall)
        assert node.function == "sqrt"
        assert len(node.args) == 1
        assert isinstance(node.args[0], ExprBinary)
        assert node.args[0].op is BinOp.ADD

    def test_atan2_two_args(self):
        node = parse_expr("atan2($y, $x)")
        assert isinstance(node, ExprCall)
        assert node.function == "atan2"
        assert len(node.args) == 2

    def test_max_with_expression_args(self):
        node = parse_expr("max($a + 1, $b * 2)")
        assert isinstance(node, ExprCall)
        assert node.function == "max"
        assert len(node.args) == 2
        assert isinstance(node.args[0], ExprBinary)
        assert isinstance(node.args[1], ExprBinary)

    def test_min_with_literals(self):
        node = parse_expr("min(3, 5, 1, 7)")
        assert isinstance(node, ExprCall)
        assert node.function == "min"
        assert len(node.args) == 4

    def test_wide_function(self):
        """wide() is a valid Tcl math function."""
        node = parse_expr("wide($x)")
        assert isinstance(node, ExprCall)
        assert node.function == "wide"
        assert len(node.args) == 1

    def test_entier_function(self):
        node = parse_expr("entier($x)")
        assert isinstance(node, ExprCall)
        assert node.function == "entier"

    def test_isqrt_function(self):
        node = parse_expr("isqrt($x)")
        assert isinstance(node, ExprCall)
        assert node.function == "isqrt"

    def test_hypot_function(self):
        node = parse_expr("hypot($x, $y)")
        assert isinstance(node, ExprCall)
        assert node.function == "hypot"
        assert len(node.args) == 2

    def test_fmod_function(self):
        node = parse_expr("fmod($x, $y)")
        assert isinstance(node, ExprCall)
        assert node.function == "fmod"
        assert len(node.args) == 2

    def test_srand_function(self):
        node = parse_expr("srand($seed)")
        assert isinstance(node, ExprCall)
        assert node.function == "srand"

    def test_isnan_function(self):
        node = parse_expr("isnan($x)")
        assert isinstance(node, ExprCall)
        assert node.function == "isnan"

    def test_isinf_function(self):
        node = parse_expr("isinf($x)")
        assert isinstance(node, ExprCall)
        assert node.function == "isinf"

    def test_function_in_binary_expr(self):
        """sin($x) + cos($x)"""
        node = parse_expr("sin($x) + cos($x)")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD
        assert isinstance(node.left, ExprCall)
        assert node.left.function == "sin"
        assert isinstance(node.right, ExprCall)
        assert node.right.function == "cos"


class TestComplexExpressionsFromTcl:
    """Complex expression patterns from Tcl 9.0.2 test suite."""

    def test_chained_logical_or(self):
        node = parse_expr("$a || $b || $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.OR
        # Left-associative: ($a || $b) || $c
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.OR

    def test_chained_logical_and(self):
        node = parse_expr("$a && $b && $c")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.AND
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.AND

    def test_bitwise_ops_chain(self):
        """$a & $b | $c ^ $d → ($a & $b) | ($c ^ $d)? No — check precedence."""
        # & (12) > ^ (10) > | (8)
        # $a & $b | $c ^ $d → (($a & $b) | ($c ^ $d))
        # But | is lower than ^, so: (a&b) | (c^d)
        node = parse_expr("$a & $b | $c ^ $d")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_OR
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.BIT_AND
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.BIT_XOR

    def test_multiple_comparisons(self):
        """$a < $b == $c > $d — relational before equality."""
        node = parse_expr("$a < $b == $c > $d")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.EQ
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.LT
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.GT

    def test_deeply_nested_parens(self):
        node = parse_expr("((($a + $b)))")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD

    def test_unary_not_chain(self):
        """!!!$x → !(!(!$x))"""
        node = parse_expr("!!!$x")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NOT
        assert isinstance(node.operand, ExprUnary)
        assert node.operand.op is UnaryOp.NOT
        assert isinstance(node.operand.operand, ExprUnary)

    def test_unary_bitwise_not_then_neg(self):
        """~-$x → ~(-$x)"""
        node = parse_expr("~-$x")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.BIT_NOT
        assert isinstance(node.operand, ExprUnary)
        assert node.operand.op is UnaryOp.NEG

    def test_mixed_arithmetic_and_comparison(self):
        """$a * $b + $c > $d - $e"""
        node = parse_expr("$a * $b + $c > $d - $e")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.GT
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.ADD
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.SUB

    def test_command_sub_in_function(self):
        """int([llength $list])"""
        node = parse_expr("int([llength $list])")
        assert isinstance(node, ExprCall)
        assert node.function == "int"
        assert isinstance(node.args[0], ExprCommand)

    def test_hex_in_bitwise(self):
        """0xf2 & 0x53 — from Tcl 9.0.2."""
        node = parse_expr("0xf2 & 0x53")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_AND
        assert isinstance(node.left, ExprLiteral)
        assert isinstance(node.right, ExprLiteral)

    def test_octal_in_modulo(self):
        """7891 % 0o123 — from Tcl 9.0.2."""
        node = parse_expr("7891 % 0o123")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MOD

    def test_unary_positive(self):
        """+$x — unary positive."""
        node = parse_expr("+$x")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.POS

    def test_multiple_unary_positive(self):
        """++$x → +(+$x)"""
        node = parse_expr("++$x")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.POS
        assert isinstance(node.operand, ExprUnary)
        assert node.operand.op is UnaryOp.POS


class TestRenderExprComprehensive:
    """Comprehensive round-trip rendering tests."""

    def test_render_boolean(self):
        node = parse_expr("true")
        assert render_expr(node) == "true"

    def test_render_hex(self):
        node = parse_expr("0xFF")
        assert render_expr(node) == "0xFF"

    def test_render_scientific(self):
        node = parse_expr("1.5e10")
        assert render_expr(node) == "1.5e10"

    def test_render_bitwise_ops(self):
        node = parse_expr("$a & $b | $c")
        text = render_expr(node)
        assert "$a" in text and "$b" in text and "$c" in text

    def test_render_in_operator(self):
        node = parse_expr("$x in $list")
        assert render_expr(node) == "$x in $list"

    def test_render_ni_operator(self):
        node = parse_expr("$x ni $list")
        assert render_expr(node) == "$x ni $list"

    def test_render_function_multiple_args(self):
        node = parse_expr("max($a, $b, $c)")
        text = render_expr(node)
        assert text == "max($a, $b, $c)"

    def test_render_complex_ternary(self):
        node = parse_expr("$a > 0 ? $b : $c")
        text = render_expr(node)
        assert "?" in text and ":" in text

    def test_render_nested_unary(self):
        node = parse_expr("!!$x")
        assert render_expr(node) == "!!$x"

    def test_render_shift(self):
        node = parse_expr("$x << 2")
        assert render_expr(node) == "$x << 2"

    def test_render_right_shift(self):
        node = parse_expr("$x >> 3")
        assert render_expr(node) == "$x >> 3"

    def test_render_power(self):
        node = parse_expr("$x ** 2")
        assert render_expr(node) == "$x ** 2"

    def test_render_str_eq(self):
        node = parse_expr('$x eq "hello"')
        assert render_expr(node) == '$x eq "hello"'

    def test_render_bitwise_not(self):
        node = parse_expr("~$x")
        assert render_expr(node) == "~$x"

    def test_render_logical_not(self):
        node = parse_expr("!$x")
        assert render_expr(node) == "!$x"


class TestVarsInExprComprehensive:
    """Comprehensive variable extraction tests."""

    def test_vars_in_ternary_all_branches(self):
        node = parse_expr("$a ? $b + $c : $d * $e")
        assert vars_in_expr_node(node) == {"a", "b", "c", "d", "e"}

    def test_vars_in_function_args(self):
        node = parse_expr("max($a, $b, $c)")
        assert vars_in_expr_node(node) == {"a", "b", "c"}

    def test_vars_in_nested_functions(self):
        node = parse_expr("int(sin($x) + cos($y))")
        assert vars_in_expr_node(node) == {"x", "y"}

    def test_vars_in_complex_expression(self):
        node = parse_expr("$a > 0 && $b < 10 || $c == 0 ? $d : $e")
        assert vars_in_expr_node(node) == {"a", "b", "c", "d", "e"}

    def test_vars_in_bitwise_expression(self):
        node = parse_expr("$mask & $val | $flag")
        assert vars_in_expr_node(node) == {"mask", "val", "flag"}

    def test_vars_in_shift_expression(self):
        node = parse_expr("$x << $shift")
        assert vars_in_expr_node(node) == {"x", "shift"}

    def test_vars_in_negated_expression(self):
        node = parse_expr("-$x + ~$y")
        assert vars_in_expr_node(node) == {"x", "y"}

    def test_no_vars_in_all_literals(self):
        node = parse_expr("42 + 3.14 * 2")
        assert vars_in_expr_node(node) == set()

    def test_vars_with_namespaces(self):
        node = parse_expr("$ns::x + $ns::y")
        assert vars_in_expr_node(node) == {"ns::x", "ns::y"}


class TestIRulesOperators:
    """Parse iRules-specific operators in f5-irules dialect."""

    def test_contains(self):
        node = parse_expr('$uri contains "/api"', dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.CONTAINS
        assert isinstance(node.left, ExprVar)
        assert isinstance(node.right, ExprString)

    def test_starts_with(self):
        node = parse_expr('$path starts_with "/v2"', dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STARTS_WITH

    def test_ends_with(self):
        node = parse_expr('$host ends_with ".com"', dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ENDS_WITH

    def test_equals(self):
        node = parse_expr('$method equals "GET"', dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_EQUALS

    def test_matches_glob(self):
        node = parse_expr('$path matches_glob "/api/*"', dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MATCHES_GLOB

    def test_matches_regex(self):
        node = parse_expr('$uri matches_regex "^/v[0-9]+"', dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MATCHES_REGEX

    def test_word_and(self):
        node = parse_expr("$a and $b", dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.WORD_AND

    def test_word_or(self):
        node = parse_expr("$a or $b", dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.WORD_OR

    def test_word_not(self):
        node = parse_expr("not $flag", dialect="f5-irules")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.WORD_NOT
        assert isinstance(node.operand, ExprVar)

    def test_not_parsed_in_standard_tcl(self):
        """Without dialect, 'contains' is not an operator — falls back to ExprRaw."""
        node = parse_expr('$uri contains "/api"')
        assert isinstance(node, ExprRaw)

    def test_and_precedence_same_as_symbolic(self):
        """'and' binds tighter than 'or' (same as && vs ||)."""
        node = parse_expr("$a or $b and $c", dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.WORD_OR
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.WORD_AND

    def test_not_higher_than_and(self):
        """'not' binds tighter than 'and' (same as '!')."""
        node = parse_expr("not $a and $b", dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.WORD_AND
        assert isinstance(node.left, ExprUnary)
        assert node.left.op is UnaryOp.WORD_NOT

    def test_contains_at_equality_precedence(self):
        """String ops at equality level: chains with 'and'."""
        node = parse_expr('$a contains "x" and $b contains "y"', dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.WORD_AND
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.CONTAINS
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.CONTAINS

    def test_mixed_symbolic_and_word_logical(self):
        """Mixing && with 'or'."""
        node = parse_expr("$a && $b or $c", dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.WORD_OR
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.AND

    def test_render_contains(self):
        node = parse_expr('$uri contains "/api"', dialect="f5-irules")
        assert render_expr(node) == '$uri contains "/api"'

    def test_render_word_not(self):
        node = parse_expr("not $flag", dialect="f5-irules")
        assert render_expr(node) == "not $flag"

    def test_render_word_and(self):
        node = parse_expr("$a and $b", dialect="f5-irules")
        assert render_expr(node) == "$a and $b"

    def test_command_sub_with_contains(self):
        """Command substitution result used with contains."""
        node = parse_expr('[HTTP::uri] contains "/api"', dialect="f5-irules")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.CONTAINS
        assert isinstance(node.left, ExprCommand)

    def test_complex_irules_expression(self):
        """Realistic iRules condition: cmd_sub and cmd_sub."""
        node = parse_expr(
            "[HTTP::cookie exists Session1] and [ACCESS::session exists]",
            dialect="f5-irules",
        )
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.WORD_AND
        assert isinstance(node.left, ExprCommand)
        assert isinstance(node.right, ExprCommand)
