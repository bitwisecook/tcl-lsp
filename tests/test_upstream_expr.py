"""Tests ported from Tcl's official tests/expr.test and tests/parseExpr.test.

These supplement the existing test_expr_parser.py and test_expr_lexer.py with
additional coverage derived from the upstream Tcl test suite.

Areas covered:
- Numeric literal formats (hex, octal, binary, scientific, leading-dot)
- All binary and unary operators via parse_expr()
- Operator precedence verification via AST tree shapes
- Math function calls
- Boolean constants (true, false, yes, no, on, off)
- Expression type inference via compiler pipeline
- Error/fallback to ExprRaw
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprRaw,
    ExprTernary,
    ExprUnary,
    ExprVar,
    UnaryOp,
)
from core.compiler.types import TclType
from core.parsing.expr_lexer import ExprTokenType, tokenise_expr
from core.parsing.expr_parser import parse_expr

from .helpers import analyse_types as _analyse
from .helpers import var_type as _var_type

# Numeric literals — upstream expr.test §4 (literal parsing)


class TestNumericLiterals:
    """Verify that numeric literal formats produce ExprLiteral nodes."""

    def test_integer_literal(self):
        node = parse_expr("42")
        assert isinstance(node, ExprLiteral)
        assert node.text == "42"

    def test_negative_integer(self):
        """Negative integers are parsed as unary NEG applied to a literal."""
        node = parse_expr("-7")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NEG
        assert isinstance(node.operand, ExprLiteral)
        assert node.operand.text == "7"

    def test_float_literal(self):
        node = parse_expr("3.14")
        assert isinstance(node, ExprLiteral)
        assert node.text == "3.14"

    def test_leading_dot_float(self):
        """Tcl accepts ``.5`` as a valid floating-point literal."""
        node = parse_expr(".5")
        assert isinstance(node, ExprLiteral)
        assert node.text == ".5"

    def test_scientific_notation(self):
        node = parse_expr("1.5e10")
        assert isinstance(node, ExprLiteral)
        assert node.text == "1.5e10"

    def test_scientific_negative_exponent(self):
        node = parse_expr("2.5e-3")
        assert isinstance(node, ExprLiteral)
        assert node.text == "2.5e-3"

    def test_hex_literal(self):
        node = parse_expr("0xFF")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0xFF"

    def test_octal_literal(self):
        node = parse_expr("0o77")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0o77"

    def test_binary_literal(self):
        node = parse_expr("0b1010")
        assert isinstance(node, ExprLiteral)
        assert node.text == "0b1010"


# Arithmetic operators — upstream expr.test §5


class TestArithmeticOperators:
    """Binary arithmetic operators produce ExprBinary with the correct BinOp."""

    def test_addition(self):
        node = parse_expr("1 + 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD

    def test_subtraction(self):
        node = parse_expr("5 - 3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.SUB

    def test_multiplication(self):
        node = parse_expr("2 * 3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MUL

    def test_division(self):
        node = parse_expr("10 / 3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.DIV

    def test_modulo(self):
        node = parse_expr("10 % 3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MOD

    def test_exponentiation(self):
        node = parse_expr("2 ** 8")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.POW


# Comparison operators — upstream expr.test §6


class TestComparisonOperators:
    """Numeric comparison operators."""

    def test_eq(self):
        node = parse_expr("$x == 1")
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
        node = parse_expr("$x >= 0")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.GE


# String comparison operators — upstream expr.test §7


class TestStringComparisonOperators:
    """String comparison operators: eq, ne, lt, le, gt, ge."""

    def test_str_eq(self):
        node = parse_expr('"a" eq "b"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_EQ

    def test_str_ne(self):
        node = parse_expr('"a" ne "b"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_NE

    def test_str_lt(self):
        node = parse_expr('"a" lt "b"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_LT

    def test_str_le(self):
        node = parse_expr('"a" le "b"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_LE

    def test_str_gt(self):
        node = parse_expr('"a" gt "b"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_GT

    def test_str_ge(self):
        node = parse_expr('"a" ge "b"')
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.STR_GE


# Logical operators — upstream expr.test §8


class TestLogicalOperators:
    """Logical conjunction, disjunction, and negation."""

    def test_and(self):
        node = parse_expr("$a && $b")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.AND

    def test_or(self):
        node = parse_expr("$a || $b")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.OR

    def test_not(self):
        node = parse_expr("!$a")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NOT


# Bitwise operators — upstream expr.test §9


class TestBitwiseOperators:
    """Bitwise AND, OR, XOR, NOT, and shifts."""

    def test_bitwise_and(self):
        node = parse_expr("$x & 0xFF")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_AND

    def test_bitwise_or(self):
        node = parse_expr("$x | 0x0F")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_OR

    def test_bitwise_xor(self):
        node = parse_expr("$x ^ 0xFF")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.BIT_XOR

    def test_bitwise_not(self):
        node = parse_expr("~$x")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.BIT_NOT

    def test_left_shift(self):
        node = parse_expr("$x << 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.LSHIFT

    def test_right_shift(self):
        node = parse_expr("$x >> 2")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.RSHIFT


# List membership operators — upstream expr.test §10 (Tcl 8.5+)


class TestListMembership:
    """The ``in`` and ``ni`` list membership operators."""

    def test_in(self):
        node = parse_expr("$x in $list")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.IN

    def test_ni(self):
        node = parse_expr("$x ni $list")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.NI


# Unary operators — upstream expr.test §11


class TestUnaryOperators:
    """Unary minus and plus applied to variables."""

    def test_unary_minus(self):
        node = parse_expr("-$x")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NEG
        assert isinstance(node.operand, ExprVar)

    def test_unary_plus(self):
        node = parse_expr("+$x")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.POS
        assert isinstance(node.operand, ExprVar)


# Operator precedence — upstream parseExpr.test


class TestOperatorPrecedence:
    """Verify that precedence and associativity produce the expected AST shapes."""

    def test_mul_binds_tighter_than_add(self):
        """``1 + 2 * 3`` should parse as ``1 + (2 * 3)``."""
        node = parse_expr("1 + 2 * 3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.MUL

    def test_add_left_associative(self):
        """``1 + 2 + 3`` should parse as ``(1 + 2) + 3``."""
        node = parse_expr("1 + 2 + 3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.ADD
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.ADD

    def test_power_right_associative(self):
        """``2 ** 3 ** 4`` should parse as ``2 ** (3 ** 4)``."""
        node = parse_expr("2 ** 3 ** 4")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.POW
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.POW

    def test_comparison_lower_than_arithmetic(self):
        """``$x + 1 > $y`` should parse as ``($x + 1) > $y``."""
        node = parse_expr("$x + 1 > $y")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.GT
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.ADD

    def test_logical_lower_than_comparison(self):
        """``$x > 0 && $y < 10`` should parse as ``($x > 0) && ($y < 10)``."""
        node = parse_expr("$x > 0 && $y < 10")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.AND
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.GT
        assert isinstance(node.right, ExprBinary)
        assert node.right.op is BinOp.LT

    def test_parentheses_override_precedence(self):
        """``(1 + 2) * 3`` should parse as ``(1 + 2) * 3``."""
        node = parse_expr("(1 + 2) * 3")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MUL
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.ADD


# Ternary operator — upstream parseExpr.test §ternary


class TestTernary:
    """Ternary conditional ``cond ? true_val : false_val``."""

    def test_simple_ternary(self):
        node = parse_expr("$x > 0 ? 1 : 0")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.condition, ExprBinary)
        assert isinstance(node.true_branch, ExprLiteral)
        assert isinstance(node.false_branch, ExprLiteral)

    def test_nested_ternary(self):
        """``$a ? $b ? 1 : 2 : 3`` — the false branch of outer is ``3``."""
        node = parse_expr("$a ? $b ? 1 : 2 : 3")
        assert isinstance(node, ExprTernary)
        # The true branch should itself be a ternary
        assert isinstance(node.true_branch, ExprTernary)
        assert isinstance(node.false_branch, ExprLiteral)
        assert node.false_branch.text == "3"


# Math function calls — upstream expr.test §functions


class TestMathFunctions:
    """Math function calls: sin, pow, abs, int, double, rand, etc."""

    def test_sin(self):
        node = parse_expr("sin(3.14)")
        assert isinstance(node, ExprCall)
        assert node.function == "sin"
        assert len(node.args) == 1
        assert isinstance(node.args[0], ExprLiteral)

    def test_pow(self):
        node = parse_expr("pow(2, 8)")
        assert isinstance(node, ExprCall)
        assert node.function == "pow"
        assert len(node.args) == 2

    def test_abs(self):
        node = parse_expr("abs(-1)")
        assert isinstance(node, ExprCall)
        assert node.function == "abs"
        assert len(node.args) == 1

    def test_int_function(self):
        node = parse_expr("int(3.14)")
        assert isinstance(node, ExprCall)
        assert node.function == "int"

    def test_double_function(self):
        node = parse_expr("double(42)")
        assert isinstance(node, ExprCall)
        assert node.function == "double"

    def test_nested_function_calls(self):
        """``abs(sin($x))`` — nested calls produce ExprCall wrapping ExprCall."""
        node = parse_expr("abs(sin($x))")
        assert isinstance(node, ExprCall)
        assert node.function == "abs"
        assert len(node.args) == 1
        inner = node.args[0]
        assert isinstance(inner, ExprCall)
        assert inner.function == "sin"

    def test_rand_no_args(self):
        node = parse_expr("rand()")
        assert isinstance(node, ExprCall)
        assert node.function == "rand"
        assert node.args == ()


# Boolean constants — upstream expr.test §booleans


class TestBooleanConstants:
    """Tcl boolean constants are tokenised as BOOL tokens."""

    def test_true(self):
        tokens = tokenise_expr("true")
        bools = [t for t in tokens if t.type is ExprTokenType.BOOL]
        assert len(bools) == 1
        assert bools[0].text == "true"

    def test_false(self):
        tokens = tokenise_expr("false")
        bools = [t for t in tokens if t.type is ExprTokenType.BOOL]
        assert len(bools) == 1
        assert bools[0].text == "false"

    def test_yes(self):
        tokens = tokenise_expr("yes")
        bools = [t for t in tokens if t.type is ExprTokenType.BOOL]
        assert len(bools) == 1
        assert bools[0].text == "yes"

    def test_no(self):
        tokens = tokenise_expr("no")
        bools = [t for t in tokens if t.type is ExprTokenType.BOOL]
        assert len(bools) == 1
        assert bools[0].text == "no"

    def test_on(self):
        tokens = tokenise_expr("on")
        bools = [t for t in tokens if t.type is ExprTokenType.BOOL]
        assert len(bools) == 1
        assert bools[0].text == "on"

    def test_off(self):
        tokens = tokenise_expr("off")
        bools = [t for t in tokens if t.type is ExprTokenType.BOOL]
        assert len(bools) == 1
        assert bools[0].text == "off"


# Variables in expressions — upstream parseExpr.test §variables


class TestVariablesInExpressions:
    """Variable references inside expressions."""

    def test_simple_var(self):
        node = parse_expr("$x + 1")
        assert isinstance(node, ExprBinary)
        assert isinstance(node.left, ExprVar)

    def test_namespace_var(self):
        node = parse_expr("$ns::val > 0")
        assert isinstance(node, ExprBinary)
        assert isinstance(node.left, ExprVar)
        assert node.left.text == "$ns::val"

    def test_braced_var(self):
        node = parse_expr("${my var}")
        assert isinstance(node, ExprVar)
        assert node.text == "${my var}"

    def test_array_var(self):
        node = parse_expr("$arr(key)")
        assert isinstance(node, ExprVar)
        assert node.text == "$arr(key)"


# Command substitution — upstream parseExpr.test §commands


class TestCommandSubstitution:
    """Command substitutions ``[cmd ...]`` inside expressions."""

    def test_command_in_expr(self):
        node = parse_expr("[llength $list] > 0")
        assert isinstance(node, ExprBinary)
        assert isinstance(node.left, ExprCommand)
        assert "[llength $list]" in node.left.text


# Expression type inference — upstream expr.test §types


class TestExpressionTypeInference:
    """End-to-end type inference through the full compiler pipeline."""

    def test_integer_expr_type(self):
        """``set x [expr {1 + 2}]`` should infer x as INT."""
        analysis = _analyse("set x [expr {1 + 2}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_float_expr_type(self):
        """``set x [expr {1.0 + 2.0}]`` should infer x as DOUBLE."""
        analysis = _analyse("set x [expr {1.0 + 2.0}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_comparison_type(self):
        """``set x [expr {$a > 0}]`` should infer x as BOOLEAN."""
        analysis = _analyse("set a 1\nset x [expr {$a > 0}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_math_function_type(self):
        """``set x [expr {sin(1.0)}]`` should infer x as DOUBLE."""
        analysis = _analyse("set x [expr {sin(1.0)}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE


# Fallback to ExprRaw — upstream parseExpr.test §errors


class TestFallbackToExprRaw:
    """Malformed or empty expressions should fall back to ExprRaw."""

    def test_empty_expression(self):
        """An empty string cannot be parsed into a structured node."""
        node = parse_expr("")
        assert isinstance(node, ExprRaw)

    def test_invalid_syntax(self):
        """A trailing operator with no right-hand side should fall back."""
        node = parse_expr("1 +")
        assert isinstance(node, ExprRaw)
