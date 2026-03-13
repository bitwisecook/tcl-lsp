"""Expression parser stress-tests derived from the Tcl 9.0.2 test suite.

Each test case is traceable to its source via pytest.param(id=...).
Source files: tcl9.0.2/tests/expr-old.test, tcl9.0.2/tests/expr.test
"""

from __future__ import annotations

import pytest

from core.compiler.expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprLiteral,
    ExprRaw,
    ExprTernary,
    ExprUnary,
    UnaryOp,
)
from core.parsing.expr_parser import parse_expr

# Helpers


def _parses_ok(expr_str: str) -> bool:
    """Return True if the expression parses to a structured node (not ExprRaw)."""
    return not isinstance(parse_expr(expr_str), ExprRaw)


# Integer operators (expr-old section 1)


class TestIntegerOperators:
    """Integer operator expressions from expr-old.test section 1."""

    @pytest.mark.parametrize(
        "expr_str,expected_type",
        [
            pytest.param("-4", ExprUnary, id="expr-old-1.1"),
            pytest.param("-(1+4)", ExprUnary, id="expr-old-1.2"),
            pytest.param("~3", ExprUnary, id="expr-old-1.3"),
            pytest.param("!2", ExprUnary, id="expr-old-1.4"),
            pytest.param("!0", ExprUnary, id="expr-old-1.5"),
            pytest.param("4*6", ExprBinary, id="expr-old-1.6"),
            pytest.param("36/12", ExprBinary, id="expr-old-1.7"),
            pytest.param("27/4", ExprBinary, id="expr-old-1.8"),
            pytest.param("27%4", ExprBinary, id="expr-old-1.9"),
            pytest.param("2+2", ExprBinary, id="expr-old-1.10"),
            pytest.param("2-6", ExprBinary, id="expr-old-1.11"),
            pytest.param("1<<3", ExprBinary, id="expr-old-1.12"),
            pytest.param("0xff>>2", ExprBinary, id="expr-old-1.13"),
            pytest.param("-1>>2", ExprBinary, id="expr-old-1.14"),
            pytest.param("3>2", ExprBinary, id="expr-old-1.15"),
            pytest.param("2>2", ExprBinary, id="expr-old-1.16"),
            pytest.param("1>2", ExprBinary, id="expr-old-1.17"),
            pytest.param("3<2", ExprBinary, id="expr-old-1.18"),
            pytest.param("2<2", ExprBinary, id="expr-old-1.19"),
            pytest.param("1<2", ExprBinary, id="expr-old-1.20"),
            pytest.param("3>=2", ExprBinary, id="expr-old-1.21"),
            pytest.param("2>=2", ExprBinary, id="expr-old-1.22"),
            pytest.param("1>=2", ExprBinary, id="expr-old-1.23"),
            pytest.param("3<=2", ExprBinary, id="expr-old-1.24"),
            pytest.param("2<=2", ExprBinary, id="expr-old-1.25"),
            pytest.param("1<=2", ExprBinary, id="expr-old-1.26"),
            pytest.param("3==2", ExprBinary, id="expr-old-1.27"),
            pytest.param("2==2", ExprBinary, id="expr-old-1.28"),
            pytest.param("3!=2", ExprBinary, id="expr-old-1.29"),
            pytest.param("2!=2", ExprBinary, id="expr-old-1.30"),
            pytest.param("7&0x13", ExprBinary, id="expr-old-1.31"),
            pytest.param("7^0x13", ExprBinary, id="expr-old-1.32"),
            pytest.param("7|0x13", ExprBinary, id="expr-old-1.33"),
            pytest.param("0&&1", ExprBinary, id="expr-old-1.34"),
            pytest.param("0&&0", ExprBinary, id="expr-old-1.35"),
            pytest.param("1&&3", ExprBinary, id="expr-old-1.36"),
            pytest.param("0||1", ExprBinary, id="expr-old-1.37"),
            pytest.param("3||0", ExprBinary, id="expr-old-1.38"),
            pytest.param("0||0", ExprBinary, id="expr-old-1.39"),
            pytest.param("3>2?44:66", ExprTernary, id="expr-old-1.40"),
            pytest.param("2>3?44:66", ExprTernary, id="expr-old-1.41"),
            pytest.param("36/5", ExprBinary, id="expr-old-1.42"),
            pytest.param("36%5", ExprBinary, id="expr-old-1.43"),
            pytest.param("-36/5", ExprBinary, id="expr-old-1.44"),
            pytest.param("-36%5", ExprBinary, id="expr-old-1.45"),
            pytest.param("36/-5", ExprBinary, id="expr-old-1.46"),
            pytest.param("36%-5", ExprBinary, id="expr-old-1.47"),
            pytest.param("-36/-5", ExprBinary, id="expr-old-1.48"),
            pytest.param("-36%-5", ExprBinary, id="expr-old-1.49"),
            pytest.param("+36", ExprUnary, id="expr-old-1.50"),
            pytest.param("+--++36", ExprUnary, id="expr-old-1.51"),
            pytest.param("+36%+5", ExprBinary, id="expr-old-1.52"),
        ],
    )
    def test_parse(self, expr_str, expected_type):
        node = parse_expr(expr_str)
        assert not isinstance(node, ExprRaw), f"Failed to parse: {expr_str}"
        assert isinstance(node, expected_type), (
            f"Expected {expected_type.__name__} for {expr_str!r}, got {type(node).__name__}"
        )


# Floating-point operators (expr-old section 2)


class TestFloatOperators:
    """Floating-point operator expressions from expr-old.test section 2."""

    @pytest.mark.parametrize(
        "expr_str,expected_type",
        [
            pytest.param("-4.2", ExprUnary, id="expr-old-2.1"),
            pytest.param("-(1.125+4.25)", ExprUnary, id="expr-old-2.2"),
            pytest.param("+5.7", ExprUnary, id="expr-old-2.3"),
            pytest.param("+--+-62.0", ExprUnary, id="expr-old-2.4"),
            pytest.param("!2.1", ExprUnary, id="expr-old-2.5"),
            pytest.param("!0.0", ExprUnary, id="expr-old-2.6"),
            pytest.param("4.2*6.3", ExprBinary, id="expr-old-2.7"),
            pytest.param("36.0/12.0", ExprBinary, id="expr-old-2.8"),
            pytest.param("27/4.0", ExprBinary, id="expr-old-2.9"),
            pytest.param("2.3+2.1", ExprBinary, id="expr-old-2.10"),
            pytest.param("2.3-6.5", ExprBinary, id="expr-old-2.11"),
            pytest.param("3.1>2.1", ExprBinary, id="expr-old-2.12"),
            pytest.param("2.1 > 2.1", ExprBinary, id="expr-old-2.13"),
            pytest.param("1.23>2.34e+1", ExprBinary, id="expr-old-2.14"),
            pytest.param("3.45<2.34", ExprBinary, id="expr-old-2.15"),
            pytest.param("0.002e3<--200e-2", ExprBinary, id="expr-old-2.16"),
            pytest.param("1.1<2.1", ExprBinary, id="expr-old-2.17"),
            pytest.param("3.1>=2.2", ExprBinary, id="expr-old-2.18"),
            pytest.param("2.345>=2.345", ExprBinary, id="expr-old-2.19"),
            pytest.param("1.1>=2.2", ExprBinary, id="expr-old-2.20"),
            pytest.param("3.0<=2.0", ExprBinary, id="expr-old-2.21"),
            pytest.param("2.2<=2.2", ExprBinary, id="expr-old-2.22"),
            pytest.param("2.2<=2.2001", ExprBinary, id="expr-old-2.23"),
            pytest.param("3.2==2.2", ExprBinary, id="expr-old-2.24"),
            pytest.param("2.2==2.2", ExprBinary, id="expr-old-2.25"),
            pytest.param("3.2!=2.2", ExprBinary, id="expr-old-2.26"),
            pytest.param("2.2!=2.2", ExprBinary, id="expr-old-2.27"),
            pytest.param("0.0&&0.0", ExprBinary, id="expr-old-2.28"),
            pytest.param("0.0&&1.3", ExprBinary, id="expr-old-2.29"),
            pytest.param("1.3&&0.0", ExprBinary, id="expr-old-2.30"),
            pytest.param("1.3&&3.3", ExprBinary, id="expr-old-2.31"),
            pytest.param("0.0||0.0", ExprBinary, id="expr-old-2.32"),
            pytest.param("0.0||1.3", ExprBinary, id="expr-old-2.33"),
            pytest.param("1.3||0.0", ExprBinary, id="expr-old-2.34"),
            pytest.param("3.3||0.0", ExprBinary, id="expr-old-2.35"),
            pytest.param("3.3>2.3?44.3:66.3", ExprTernary, id="expr-old-2.36"),
            pytest.param("2.3>3.3?44.3:66.3", ExprTernary, id="expr-old-2.37"),
        ],
    )
    def test_parse(self, expr_str, expected_type):
        node = parse_expr(expr_str)
        assert not isinstance(node, ExprRaw), f"Failed to parse: {expr_str}"
        assert isinstance(node, expected_type)


# String operators (expr-old section 4)


class TestStringOperators:
    """String comparison expressions from expr-old.test section 4."""

    @pytest.mark.parametrize(
        "expr_str,expected_type",
        [
            pytest.param('"abc" > "def"', ExprBinary, id="expr-old-4.1"),
            pytest.param('"def" > "def"', ExprBinary, id="expr-old-4.2"),
            pytest.param('"g" > "def"', ExprBinary, id="expr-old-4.3"),
            pytest.param('"abc" < "abd"', ExprBinary, id="expr-old-4.4"),
            pytest.param('"abd" < "abd"', ExprBinary, id="expr-old-4.5"),
            pytest.param('"abe" < "abd"', ExprBinary, id="expr-old-4.6"),
            pytest.param('"abc" >= "def"', ExprBinary, id="expr-old-4.7"),
            pytest.param('"def" >= "def"', ExprBinary, id="expr-old-4.8"),
            pytest.param('"g" >= "def"', ExprBinary, id="expr-old-4.9"),
            pytest.param('"abc" <= "abd"', ExprBinary, id="expr-old-4.10"),
            pytest.param('"abd" <= "abd"', ExprBinary, id="expr-old-4.11"),
            pytest.param('"abe" <= "abd"', ExprBinary, id="expr-old-4.12"),
            pytest.param('"abc" == "abd"', ExprBinary, id="expr-old-4.13"),
            pytest.param('"abd" == "abd"', ExprBinary, id="expr-old-4.14"),
            pytest.param('"abc" != "abd"', ExprBinary, id="expr-old-4.15"),
            pytest.param('"abd" != "abd"', ExprBinary, id="expr-old-4.16"),
            pytest.param('"abc" eq "abd"', ExprBinary, id="expr-old-4.19"),
            pytest.param('"abd" eq "abd"', ExprBinary, id="expr-old-4.20"),
            pytest.param('"abc" ne "abd"', ExprBinary, id="expr-old-4.21"),
            pytest.param('"abd" ne "abd"', ExprBinary, id="expr-old-4.22"),
            pytest.param('"" eq "abd"', ExprBinary, id="expr-old-4.23"),
            pytest.param('"" eq ""', ExprBinary, id="expr-old-4.24"),
            pytest.param('"abd" ne ""', ExprBinary, id="expr-old-4.25"),
            pytest.param('"" ne ""', ExprBinary, id="expr-old-4.26"),
            pytest.param('"longerstring" eq "shorter"', ExprBinary, id="expr-old-4.27"),
            pytest.param('"longerstring" ne "shorter"', ExprBinary, id="expr-old-4.28"),
            pytest.param('1?"foo":"bar"', ExprTernary, id="expr-old-4.31"),
            pytest.param('0?"foo":"bar"', ExprTernary, id="expr-old-4.32"),
        ],
    )
    def test_parse(self, expr_str, expected_type):
        node = parse_expr(expr_str)
        assert not isinstance(node, ExprRaw), f"Failed to parse: {expr_str}"
        assert isinstance(node, expected_type)


# String comparison operators lt/le/gt/ge (expr.test section 8)


class TestStringComparisonOps:
    """String comparison operators lt/le/gt/ge from expr.test 8.36-8.39."""

    @pytest.mark.parametrize(
        "expr_str,expected_op",
        [
            pytest.param("$x lt $y", BinOp.STR_LT, id="expr-8.36-lt"),
            pytest.param("$x le $y", BinOp.STR_LE, id="expr-8.37-le"),
            pytest.param("$x le $x", BinOp.STR_LE, id="expr-8.37-le-self"),
            pytest.param("$x gt $y", BinOp.STR_GT, id="expr-8.38-gt"),
            pytest.param("$x gt $x", BinOp.STR_GT, id="expr-8.38-gt-self"),
            pytest.param("$x ge $y", BinOp.STR_GE, id="expr-8.39-ge"),
            pytest.param("$x ge $x", BinOp.STR_GE, id="expr-8.39-ge-self"),
        ],
    )
    def test_parse(self, expr_str, expected_op):
        node = parse_expr(expr_str)
        assert isinstance(node, ExprBinary), f"Expected ExprBinary for {expr_str!r}"
        assert node.op is expected_op


# Precedence (expr-old sections 6–20)


class TestPrecedenceFromTcl9:
    """Operator precedence from expr-old.test sections 6-20."""

    @pytest.mark.parametrize(
        "expr_str,top_op",
        [
            # Unary precedence: -~3, -!3, -~0
            pytest.param("-~3", None, id="expr-old-6.1"),  # top-level is unary
            pytest.param("-!3", None, id="expr-old-6.2"),
            pytest.param("-~0", None, id="expr-old-6.3"),
            # Mul/div associativity
            pytest.param("2*4/6", BinOp.DIV, id="expr-old-7.1"),
            pytest.param("24/6*3", BinOp.MUL, id="expr-old-7.2"),
            pytest.param("24/6/2", BinOp.DIV, id="expr-old-7.3"),
            # Add vs unary
            pytest.param("-2+4", BinOp.ADD, id="expr-old-8.1"),
            pytest.param("-2-4", BinOp.SUB, id="expr-old-8.2"),
            pytest.param("+2-4", BinOp.SUB, id="expr-old-8.3"),
            # Mul before add
            pytest.param("2*3+4", BinOp.ADD, id="expr-old-9.1"),
            pytest.param("8/2+4", BinOp.ADD, id="expr-old-9.2"),
            pytest.param("8%3+4", BinOp.ADD, id="expr-old-9.3"),
            pytest.param("2*3-1", BinOp.SUB, id="expr-old-9.4"),
            pytest.param("8/2-1", BinOp.SUB, id="expr-old-9.5"),
            pytest.param("8%3-1", BinOp.SUB, id="expr-old-9.6"),
            # Subtraction left-associative
            pytest.param("6-3-2", BinOp.SUB, id="expr-old-10.1"),
            # Shift vs add
            pytest.param("7+1>>2", BinOp.RSHIFT, id="expr-old-11.1"),
            pytest.param("7+1<<2", BinOp.LSHIFT, id="expr-old-11.2"),
            pytest.param("7>>3-2", BinOp.RSHIFT, id="expr-old-11.3"),
            pytest.param("7<<3-2", BinOp.LSHIFT, id="expr-old-11.4"),
            # Shift vs relational
            pytest.param("6>>1>4", BinOp.GT, id="expr-old-12.1"),
            pytest.param("6>>1<2", BinOp.LT, id="expr-old-12.2"),
            pytest.param("6>>1>=3", BinOp.GE, id="expr-old-12.3"),
            pytest.param("6>>1<=2", BinOp.LE, id="expr-old-12.4"),
            pytest.param("6<<1>5", BinOp.GT, id="expr-old-12.5"),
            pytest.param("6<<1<5", BinOp.LT, id="expr-old-12.6"),
            pytest.param("5<=6<<1", BinOp.LE, id="expr-old-12.7"),
            pytest.param("5>=6<<1", BinOp.GE, id="expr-old-12.8"),
            # Relational chaining
            pytest.param("2<3<4", BinOp.LT, id="expr-old-13.1"),
            pytest.param("0<4>2", BinOp.GT, id="expr-old-13.2"),
            # Equality vs relational
            pytest.param("1==4>3", BinOp.EQ, id="expr-old-14.1"),
            pytest.param("0!=4>3", BinOp.NE, id="expr-old-14.2"),
            pytest.param("1==3<4", BinOp.EQ, id="expr-old-14.3"),
            pytest.param("0!=3<4", BinOp.NE, id="expr-old-14.4"),
            # eq/ne at same precedence as ==/!=
            pytest.param("1==3==3", BinOp.EQ, id="expr-old-15.1"),
            pytest.param("3==3!=2", BinOp.NE, id="expr-old-15.2"),
            # Bitwise: & vs == (& binds tighter)
            pytest.param("7&3^0x10", BinOp.BIT_XOR, id="expr-old-17.1"),
            pytest.param("7^0x10&3", BinOp.BIT_XOR, id="expr-old-17.2"),
            # Bitwise: ^ vs | (^ binds tighter)
            pytest.param("7^0x10|3", BinOp.BIT_OR, id="expr-old-18.1"),
            pytest.param("7|0x10^3", BinOp.BIT_OR, id="expr-old-18.2"),
            # Logical: | vs && (&& binds tighter)
            pytest.param("7|3&&1", BinOp.AND, id="expr-old-19.1"),
            pytest.param("1&&3|7", BinOp.AND, id="expr-old-19.2"),
            pytest.param("0&&1||1", BinOp.OR, id="expr-old-19.3"),
            pytest.param("1||1&&0", BinOp.OR, id="expr-old-19.4"),
            # Ternary vs logical
            pytest.param("1||0?3:4", ExprTernary, id="expr-old-20.1"),
        ],
    )
    def test_top_level_operator(self, expr_str, top_op):
        node = parse_expr(expr_str)
        assert not isinstance(node, ExprRaw), f"Failed to parse: {expr_str}"
        if top_op is None:
            # Expect unary at top level
            assert isinstance(node, ExprUnary)
        elif top_op is ExprTernary:
            assert isinstance(node, ExprTernary)
        else:
            assert isinstance(node, ExprBinary), f"Expected ExprBinary for {expr_str!r}"
            assert node.op is top_op, (
                f"Expected {top_op} as top-level op for {expr_str!r}, got {node.op}"
            )

    def test_parenthesised_override(self):
        """expr-old-21.1: (2+4)*6 → top-level is MUL, not ADD."""
        node = parse_expr("(2+4)*6")
        assert isinstance(node, ExprBinary)
        assert node.op is BinOp.MUL
        assert isinstance(node.left, ExprBinary)
        assert node.left.op is BinOp.ADD

    def test_parenthesised_unary(self):
        """expr-old-21.3: +(3-4)."""
        node = parse_expr("+(3-4)")
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.POS


# Boolean literals (expr.test section 21)


class TestBooleanLiterals:
    """Boolean literal handling from expr.test section 21."""

    @pytest.mark.parametrize(
        "expr_str",
        [
            pytest.param("false", id="expr-21.1"),
            pytest.param("true", id="expr-21.2"),
            pytest.param("off", id="expr-21.3"),
            pytest.param("on", id="expr-21.4"),
            pytest.param("no", id="expr-21.5"),
            pytest.param("yes", id="expr-21.6"),
        ],
    )
    def test_boolean_literal_parses(self, expr_str):
        node = parse_expr(expr_str)
        assert isinstance(node, ExprLiteral), f"Expected ExprLiteral for {expr_str!r}"
        assert node.text == expr_str

    @pytest.mark.parametrize(
        "expr_str",
        [
            pytest.param("!false", id="expr-21.7"),
            pytest.param("!true", id="expr-21.8"),
            pytest.param("!off", id="expr-21.9"),
            pytest.param("!on", id="expr-21.10"),
            pytest.param("!no", id="expr-21.11"),
            pytest.param("!yes", id="expr-21.12"),
        ],
    )
    def test_negated_boolean(self, expr_str):
        node = parse_expr(expr_str)
        assert isinstance(node, ExprUnary)
        assert node.op is UnaryOp.NOT
        assert isinstance(node.operand, ExprLiteral)


# Literal formats (expr.test section 14)


class TestLiteralFormats:
    """Literal format handling from expr.test section 14."""

    @pytest.mark.parametrize(
        "expr_str",
        [
            pytest.param("1", id="expr-14.1"),
            pytest.param("123", id="expr-14.2"),
            pytest.param("0xff", id="expr-14.3"),
            pytest.param("0o0010", id="expr-14.4"),
            pytest.param("62.0", id="expr-14.5"),
            pytest.param("3.1400000", id="expr-14.6"),
            pytest.param("0x123", id="expr-13.12"),
            pytest.param("0b1010", id="binary-literal"),
            pytest.param("1e10", id="scientific-notation"),
            pytest.param("2.34e+1", id="scientific-positive-exp"),
            pytest.param("200e-2", id="scientific-negative-exp"),
            pytest.param("0005", id="leading-zeroes"),
            pytest.param("0o123", id="octal-prefix"),
        ],
    )
    def test_literal_parses(self, expr_str):
        node = parse_expr(expr_str)
        assert isinstance(node, ExprLiteral), f"Expected ExprLiteral for {expr_str!r}"


# Math functions (expr.test sections 2, 14, 15)


class TestMathFunctions:
    """Math function calls from expr.test."""

    @pytest.mark.parametrize(
        "expr_str,func_name",
        [
            pytest.param("sin($x)", "sin", id="sin"),
            pytest.param("cos(1.0)", "cos", id="cos"),
            pytest.param("tan(0.5)", "tan", id="tan"),
            pytest.param("asin(0.5)", "asin", id="asin"),
            pytest.param("acos(0.5)", "acos", id="acos"),
            pytest.param("atan(1.0)", "atan", id="atan"),
            pytest.param("atan2(1.0, 2.0)", "atan2", id="atan2"),
            pytest.param("sinh(1.0)", "sinh", id="sinh"),
            pytest.param("cosh(1.0)", "cosh", id="cosh"),
            pytest.param("tanh(1.0)", "tanh", id="tanh"),
            pytest.param("exp(1.0)", "exp", id="expr-14.25"),
            pytest.param("pow(2.0, 3.0)", "pow", id="expr-14.26"),
            pytest.param("sqrt(4.0)", "sqrt", id="sqrt"),
            pytest.param("log(1.0)", "log", id="log"),
            pytest.param("log10(100.0)", "log10", id="log10"),
            pytest.param("abs(-5)", "abs", id="abs"),
            pytest.param("int(3.14)", "int", id="int"),
            pytest.param("double(5)", "double", id="double"),
            pytest.param("round(3.14)", "round", id="round"),
            pytest.param("ceil(3.14)", "ceil", id="ceil"),
            pytest.param("floor(3.14)", "floor", id="floor"),
            pytest.param("fmod(10.0, 3.0)", "fmod", id="fmod"),
            pytest.param("hypot(3.0, 4.0)", "hypot", id="hypot"),
            pytest.param("bool(1)", "bool", id="bool"),
            pytest.param("rand()", "rand", id="rand"),
            pytest.param("srand(42)", "srand", id="srand"),
            pytest.param("max($a, $b, $c)", "max", id="max"),
            pytest.param("min($a, $b)", "min", id="min"),
            pytest.param("wide(42)", "wide", id="wide"),
            pytest.param("entier(3.14)", "entier", id="entier"),
            pytest.param("isqrt(16)", "isqrt", id="isqrt"),
            pytest.param("isnan($x)", "isnan", id="isnan"),
            pytest.param("isinf($x)", "isinf", id="isinf"),
        ],
    )
    def test_function_call(self, expr_str, func_name):
        node = parse_expr(expr_str)
        assert isinstance(node, ExprCall), f"Expected ExprCall for {expr_str!r}"
        assert node.function == func_name

    def test_nested_function(self):
        """expr: int(sin($x))."""
        node = parse_expr("int(sin($x))")
        assert isinstance(node, ExprCall)
        assert node.function == "int"
        assert len(node.args) == 1
        assert isinstance(node.args[0], ExprCall)
        assert node.args[0].function == "sin"


# Ternary expressions


class TestTernaryFromTcl9:
    """Ternary expressions from expr.test and expr-old.test."""

    @pytest.mark.parametrize(
        "expr_str",
        [
            pytest.param("3>2?44:66", id="expr-old-1.40"),
            pytest.param("2>3?44:66", id="expr-old-1.41"),
            pytest.param("3.3>2.3?44.3:66.3", id="expr-old-2.36"),
            pytest.param("2.3>3.3?44.3:66.3", id="expr-old-2.37"),
            pytest.param('1?"foo":"bar"', id="expr-old-4.31"),
            pytest.param('0?"foo":"bar"', id="expr-old-4.32"),
            pytest.param("3>2?44:66", id="expr-3.3"),
            pytest.param("2>3?44:66", id="expr-3.5"),
        ],
    )
    def test_ternary_parses(self, expr_str):
        node = parse_expr(expr_str)
        assert isinstance(node, ExprTernary)

    def test_nested_ternary_right_assoc(self):
        """expr-old-20.3: 1?2:0?3:4 → 1 ? 2 : (0 ? 3 : 4)."""
        node = parse_expr("1?2:0?3:4")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.false_branch, ExprTernary)

    def test_ternary_in_ternary_true_arm(self):
        """expr-old-20.5: 1?2?3:4:0."""
        node = parse_expr("1?2?3:4:0")
        assert isinstance(node, ExprTernary)
        assert isinstance(node.true_branch, ExprTernary)


# Variables in expressions (expr-old section 22)


class TestVariablesInExpr:
    """Variable references from expr-old.test section 22."""

    @pytest.mark.parametrize(
        "expr_str",
        [
            pytest.param("2*$a", id="expr-old-22.1"),
            pytest.param("$x + $y", id="expr-old-22.2"),
            pytest.param("[set a] - 14", id="expr-old-22.4"),
            pytest.param("$a(foo)", id="array-var"),
            pytest.param("${my_var}", id="braced-var"),
            pytest.param("$ns::count", id="namespaced-var"),
            pytest.param("$i", id="expr-14.11"),
        ],
    )
    def test_parses_ok(self, expr_str):
        assert _parses_ok(expr_str), f"Failed to parse: {expr_str}"


# Expressions from expr.test sections 3–13


class TestExprTestSections:
    """Additional expressions from expr.test sections 3-13."""

    @pytest.mark.parametrize(
        "expr_str,expected_type",
        [
            # Conditional
            pytest.param("3||0", ExprBinary, id="expr-3.1"),
            # Logical
            pytest.param("1.3&&3.3", ExprBinary, id="expr-4.1"),
            pytest.param("0||1.0", ExprBinary, id="expr-4.3"),
            pytest.param("3.0||0.0", ExprBinary, id="expr-4.4"),
            pytest.param("0||0||1", ExprBinary, id="expr-4.5"),
            # Logical and
            pytest.param("0&&1.0", ExprBinary, id="expr-5.3"),
            pytest.param("0&&0", ExprBinary, id="expr-5.4"),
            pytest.param("3.0&&1.2", ExprBinary, id="expr-5.5"),
            pytest.param("1&&1&&2", ExprBinary, id="expr-5.6"),
            # Bitwise
            pytest.param("7&0x13", ExprBinary, id="expr-6.1"),
            pytest.param("7^0x13", ExprBinary, id="expr-6.3"),
            pytest.param("3^0x10", ExprBinary, id="expr-6.4"),
            pytest.param("0^7", ExprBinary, id="expr-6.5"),
            pytest.param("-1^7", ExprBinary, id="expr-6.6"),
            # Equality
            pytest.param("3==2", ExprBinary, id="expr-7.1"),
            pytest.param("2.0==2", ExprBinary, id="expr-7.2"),
            pytest.param("3.2!=2.2", ExprBinary, id="expr-7.3"),
            pytest.param('"abc" == "abd"', ExprBinary, id="expr-7.4"),
            pytest.param("0xf2&0x53", ExprBinary, id="expr-7.7"),
            pytest.param("3&6", ExprBinary, id="expr-7.8"),
            pytest.param("-1&-7", ExprBinary, id="expr-7.9"),
            # eq/ne — word operators now tokenise even without whitespace
            pytest.param("3eq2", ExprBinary, id="expr-7.14"),
            pytest.param('"abc" eq "abd"', ExprBinary, id="expr-7.18"),
            # Relational
            pytest.param("3>=2", ExprBinary, id="expr-8.1"),
            pytest.param("2<=2.1", ExprBinary, id="expr-8.2"),
            pytest.param('3.2>"2.2"', ExprBinary, id="expr-8.3"),
            pytest.param("7==0x13", ExprBinary, id="expr-8.6"),
            pytest.param("-0xf2!=0x53", ExprBinary, id="expr-8.7"),
            # Shift
            pytest.param("3<<2", ExprBinary, id="expr-9.1"),
            pytest.param("0xff>>2", ExprBinary, id="expr-9.2"),
            pytest.param("-1>>2", ExprBinary, id="expr-9.3"),
            # Add/subtract
            pytest.param("4+-2", ExprBinary, id="expr-10.1"),
            pytest.param("0xff-2", ExprBinary, id="expr-10.2"),
            pytest.param("-1--2", ExprBinary, id="expr-10.3"),
            pytest.param("1-0o123", ExprBinary, id="expr-10.4"),
            pytest.param("0xff++0x3", ExprBinary, id="expr-11.6"),
            pytest.param("-0xf2--0x3", ExprBinary, id="expr-11.7"),
            # Multiply
            pytest.param("4*-2", ExprBinary, id="expr-11.1"),
            pytest.param("0xff%2", ExprBinary, id="expr-11.2"),
            pytest.param("-1/2", ExprBinary, id="expr-11.3"),
            pytest.param("0xff*0x3", ExprBinary, id="expr-12.6"),
            pytest.param("-0xf2%-0x3", ExprBinary, id="expr-12.7"),
            # Unary
            pytest.param("~4", ExprUnary, id="expr-12.1"),
            pytest.param("--5", ExprUnary, id="expr-12.2"),
            pytest.param("!27", ExprUnary, id="expr-12.3"),
            pytest.param("~0xff00ff", ExprUnary, id="expr-12.4"),
            pytest.param("-0xff", ExprUnary, id="expr-13.1"),
            pytest.param("+0o00123", ExprUnary, id="expr-13.2"),
            pytest.param("+--++36", ExprUnary, id="expr-13.3"),
            pytest.param("!2", ExprUnary, id="expr-13.4"),
            pytest.param("+--+-62.0", ExprUnary, id="expr-13.5"),
            pytest.param("!0.0", ExprUnary, id="expr-13.6"),
            pytest.param("!0xef", ExprUnary, id="expr-13.7"),
            # Subexpression
            pytest.param("2+(3*4)", ExprBinary, id="expr-14.28"),
        ],
    )
    def test_parse(self, expr_str, expected_type):
        node = parse_expr(expr_str)
        assert not isinstance(node, ExprRaw), f"Failed to parse: {expr_str}"
        assert isinstance(node, expected_type)


# Edge cases: expressions that should fail → ExprRaw


class TestEdgeCases:
    """Edge cases that should produce ExprRaw (unparseable)."""

    @pytest.mark.parametrize(
        "expr_str",
        [
            pytest.param("", id="empty"),
            pytest.param("   ", id="whitespace"),
        ],
    )
    def test_fallback_to_raw(self, expr_str):
        node = parse_expr(expr_str)
        assert isinstance(node, ExprRaw)

    @pytest.mark.parametrize(
        "expr_str",
        [
            # Inline: compact expressions without spaces that still parse
            pytest.param("0xff>=+0x3", id="expr-9.7"),
            pytest.param("-0xf2<0x3", id="expr-9.8"),
            pytest.param("0xff>>0x3", id="expr-10.6"),
            pytest.param("-0xf2<<0x3", id="expr-10.7"),
        ],
    )
    def test_compact_hex_expressions(self, expr_str):
        """Hex expressions without spaces should still parse."""
        node = parse_expr(expr_str)
        assert not isinstance(node, ExprRaw), f"Failed to parse: {expr_str}"


# Operator-specific BinOp verification


class TestOperatorMapping:
    """Verify specific BinOp values for operator tokens."""

    @pytest.mark.parametrize(
        "expr_str,expected_op",
        [
            ("$a + $b", BinOp.ADD),
            ("$a - $b", BinOp.SUB),
            ("$a * $b", BinOp.MUL),
            ("$a / $b", BinOp.DIV),
            ("$a % $b", BinOp.MOD),
            ("$a ** $b", BinOp.POW),
            ("$a << $b", BinOp.LSHIFT),
            ("$a >> $b", BinOp.RSHIFT),
            ("$a & $b", BinOp.BIT_AND),
            ("$a | $b", BinOp.BIT_OR),
            ("$a ^ $b", BinOp.BIT_XOR),
            ("$a && $b", BinOp.AND),
            ("$a || $b", BinOp.OR),
            ("$a == $b", BinOp.EQ),
            ("$a != $b", BinOp.NE),
            ("$a < $b", BinOp.LT),
            ("$a <= $b", BinOp.LE),
            ("$a > $b", BinOp.GT),
            ("$a >= $b", BinOp.GE),
            ("$a eq $b", BinOp.STR_EQ),
            ("$a ne $b", BinOp.STR_NE),
            ("$a lt $b", BinOp.STR_LT),
            ("$a le $b", BinOp.STR_LE),
            ("$a gt $b", BinOp.STR_GT),
            ("$a ge $b", BinOp.STR_GE),
            ("$a in $b", BinOp.IN),
            ("$a ni $b", BinOp.NI),
        ],
    )
    def test_binop_mapping(self, expr_str, expected_op):
        node = parse_expr(expr_str)
        assert isinstance(node, ExprBinary)
        assert node.op is expected_op
