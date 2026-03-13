"""Direct unit tests for expression type inference (expr_types module).

These tests exercise ``infer_expr_type`` directly with hand-built AST nodes
and pre-defined variable type maps, without running the full compiler
pipeline.  This allows fine-grained coverage of operator typing rules
derived from the Tcl 9.0.2 test suite.
"""

from __future__ import annotations

import pytest

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
)
from core.compiler.expr_types import infer_expr_type
from core.compiler.types import TclType, TypeKind, TypeLattice


def _lit(text: str) -> ExprLiteral:
    return ExprLiteral(text=text, start=0, end=len(text))


def _var(name: str) -> ExprVar:
    return ExprVar(text=f"${name}", name=name, start=0, end=len(name) + 1)


def _str(text: str) -> ExprString:
    return ExprString(text=f'"{text}"', start=0, end=len(text) + 2)


def _cmd(text: str) -> ExprCommand:
    return ExprCommand(text=f"[{text}]", start=0, end=len(text) + 2)


def _binop(op: BinOp, left, right) -> ExprBinary:
    return ExprBinary(op=op, left=left, right=right)


def _unary(op: UnaryOp, operand) -> ExprUnary:
    return ExprUnary(op=op, operand=operand)


def _call(func: str, *args) -> ExprCall:
    return ExprCall(function=func, args=tuple(args), start=0, end=0)


INT_TYPE = TypeLattice.of(TclType.INT)
DOUBLE_TYPE = TypeLattice.of(TclType.DOUBLE)
STRING_TYPE = TypeLattice.of(TclType.STRING)
BOOL_TYPE = TypeLattice.of(TclType.BOOLEAN)
LIST_TYPE = TypeLattice.of(TclType.LIST)


class TestLiteralTypeDetection:
    """Literal text → type inference rules."""

    def test_integer_literal(self):
        result = infer_expr_type(_lit("42"), {})
        assert result.tcl_type is TclType.INT

    def test_negative_integer_text(self):
        # Literal "-7" should parse as INT
        result = infer_expr_type(_lit("-7"), {})
        assert result.tcl_type is TclType.INT

    def test_zero(self):
        result = infer_expr_type(_lit("0"), {})
        assert result.tcl_type is TclType.INT

    def test_hex_literal(self):
        result = infer_expr_type(_lit("0xFF"), {})
        assert result.tcl_type is TclType.INT

    def test_hex_lower(self):
        result = infer_expr_type(_lit("0xff"), {})
        assert result.tcl_type is TclType.INT

    def test_octal_literal(self):
        result = infer_expr_type(_lit("0o77"), {})
        assert result.tcl_type is TclType.INT

    def test_binary_literal(self):
        result = infer_expr_type(_lit("0b1010"), {})
        assert result.tcl_type is TclType.INT

    def test_large_integer(self):
        result = infer_expr_type(_lit("9223372036854775807"), {})
        assert result.tcl_type is TclType.INT

    def test_float_literal(self):
        result = infer_expr_type(_lit("3.14"), {})
        assert result.tcl_type is TclType.DOUBLE

    def test_float_no_leading_zero(self):
        result = infer_expr_type(_lit(".5"), {})
        assert result.tcl_type is TclType.DOUBLE

    def test_scientific_notation(self):
        result = infer_expr_type(_lit("1.5e10"), {})
        assert result.tcl_type is TclType.DOUBLE

    def test_scientific_positive_exponent(self):
        result = infer_expr_type(_lit("2.3e+5"), {})
        assert result.tcl_type is TclType.DOUBLE

    def test_scientific_negative_exponent(self):
        result = infer_expr_type(_lit("1.0e-3"), {})
        assert result.tcl_type is TclType.DOUBLE

    def test_boolean_true(self):
        result = infer_expr_type(_lit("true"), {})
        assert result.tcl_type is TclType.BOOLEAN

    def test_boolean_false(self):
        result = infer_expr_type(_lit("false"), {})
        assert result.tcl_type is TclType.BOOLEAN

    def test_boolean_yes(self):
        result = infer_expr_type(_lit("yes"), {})
        assert result.tcl_type is TclType.BOOLEAN

    def test_boolean_no(self):
        result = infer_expr_type(_lit("no"), {})
        assert result.tcl_type is TclType.BOOLEAN

    def test_boolean_on(self):
        result = infer_expr_type(_lit("on"), {})
        assert result.tcl_type is TclType.BOOLEAN

    def test_boolean_off(self):
        result = infer_expr_type(_lit("off"), {})
        assert result.tcl_type is TclType.BOOLEAN


class TestVariableLookup:
    """Variable reference type lookup."""

    def test_known_int_var(self):
        result = infer_expr_type(_var("x"), {"x": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_known_double_var(self):
        result = infer_expr_type(_var("x"), {"x": DOUBLE_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_known_string_var(self):
        result = infer_expr_type(_var("x"), {"x": STRING_TYPE})
        assert result.tcl_type is TclType.STRING

    def test_unknown_var(self):
        result = infer_expr_type(_var("x"), {})
        assert result.kind is TypeKind.UNKNOWN


class TestAtomTypes:
    """Atom node types."""

    def test_string_node(self):
        result = infer_expr_type(_str("hello"), {})
        assert result.tcl_type is TclType.STRING

    def test_command_node(self):
        result = infer_expr_type(_cmd("some_cmd"), {})
        assert result.kind is TypeKind.OVERDEFINED

    def test_raw_node(self):
        result = infer_expr_type(ExprRaw(text="whatever"), {})
        assert result.tcl_type is TclType.NUMERIC


class TestArithmeticResult:
    """Arithmetic operator type inference."""

    def test_int_add_int(self):
        node = _binop(BinOp.ADD, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_int_add_double(self):
        node = _binop(BinOp.ADD, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": DOUBLE_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_double_add_int(self):
        node = _binop(BinOp.ADD, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": DOUBLE_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_double_sub_double(self):
        node = _binop(BinOp.SUB, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": DOUBLE_TYPE, "b": DOUBLE_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_int_mul_int(self):
        node = _binop(BinOp.MUL, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_int_div_int(self):
        """Integer division: INT / INT → INT (Tcl truncating division)."""
        node = _binop(BinOp.DIV, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_double_div_int(self):
        node = _binop(BinOp.DIV, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": DOUBLE_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_int_mod_int(self):
        node = _binop(BinOp.MOD, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_int_pow_int(self):
        node = _binop(BinOp.POW, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_int_pow_double(self):
        node = _binop(BinOp.POW, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": DOUBLE_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_bool_add_int(self):
        """BOOLEAN + INT → INT (booleans are integers in Tcl)."""
        node = _binop(BinOp.ADD, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": BOOL_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_int_add_bool(self):
        node = _binop(BinOp.ADD, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": BOOL_TYPE})
        assert result.tcl_type is TclType.INT

    def test_bool_add_bool(self):
        node = _binop(BinOp.ADD, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": BOOL_TYPE, "b": BOOL_TYPE})
        assert result.tcl_type is TclType.INT

    def test_unknown_operand_gives_numeric(self):
        """If either operand is unknown, fallback to NUMERIC."""
        node = _binop(BinOp.ADD, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE})  # b unknown
        assert result.tcl_type is TclType.NUMERIC


class TestBitwiseResult:
    """Bitwise operators always → INT."""

    def test_bit_and(self):
        node = _binop(BinOp.BIT_AND, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_bit_or(self):
        node = _binop(BinOp.BIT_OR, _var("a"), _var("b"))
        result = infer_expr_type(node, {})
        assert result.tcl_type is TclType.INT

    def test_bit_xor(self):
        node = _binop(BinOp.BIT_XOR, _var("a"), _var("b"))
        result = infer_expr_type(node, {})
        assert result.tcl_type is TclType.INT

    def test_lshift(self):
        node = _binop(BinOp.LSHIFT, _var("a"), _var("b"))
        result = infer_expr_type(node, {})
        assert result.tcl_type is TclType.INT

    def test_rshift(self):
        node = _binop(BinOp.RSHIFT, _var("a"), _var("b"))
        result = infer_expr_type(node, {})
        assert result.tcl_type is TclType.INT


class TestComparisonResult:
    """All comparison operators → BOOLEAN."""

    @pytest.mark.parametrize(
        "op",
        [
            BinOp.EQ,
            BinOp.NE,
            BinOp.LT,
            BinOp.LE,
            BinOp.GT,
            BinOp.GE,
            BinOp.STR_EQ,
            BinOp.STR_NE,
            BinOp.STR_LT,
            BinOp.STR_LE,
            BinOp.STR_GT,
            BinOp.STR_GE,
            BinOp.IN,
            BinOp.NI,
        ],
    )
    def test_comparison_is_boolean(self, op):
        node = _binop(op, _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.BOOLEAN


class TestLogicalResult:
    """Logical operators → BOOLEAN."""

    def test_and(self):
        node = _binop(BinOp.AND, _var("a"), _var("b"))
        result = infer_expr_type(node, {})
        assert result.tcl_type is TclType.BOOLEAN

    def test_or(self):
        node = _binop(BinOp.OR, _var("a"), _var("b"))
        result = infer_expr_type(node, {})
        assert result.tcl_type is TclType.BOOLEAN


class TestUnaryResult:
    """Unary operator type inference."""

    def test_not_is_boolean(self):
        node = _unary(UnaryOp.NOT, _var("x"))
        result = infer_expr_type(node, {"x": INT_TYPE})
        assert result.tcl_type is TclType.BOOLEAN

    def test_bit_not_is_int(self):
        node = _unary(UnaryOp.BIT_NOT, _var("x"))
        result = infer_expr_type(node, {"x": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_neg_preserves_int(self):
        node = _unary(UnaryOp.NEG, _var("x"))
        result = infer_expr_type(node, {"x": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_neg_preserves_double(self):
        node = _unary(UnaryOp.NEG, _var("x"))
        result = infer_expr_type(node, {"x": DOUBLE_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_pos_preserves_int(self):
        node = _unary(UnaryOp.POS, _var("x"))
        result = infer_expr_type(node, {"x": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_pos_preserves_double(self):
        node = _unary(UnaryOp.POS, _var("x"))
        result = infer_expr_type(node, {"x": DOUBLE_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_double_not_is_boolean(self):
        """``!!$x`` → BOOLEAN."""
        node = _unary(UnaryOp.NOT, _unary(UnaryOp.NOT, _var("x")))
        result = infer_expr_type(node, {"x": INT_TYPE})
        assert result.tcl_type is TclType.BOOLEAN


class TestTernaryResult:
    """Ternary type join rules."""

    def test_same_type_both_branches(self):
        node = ExprTernary(condition=_var("c"), true_branch=_lit("1"), false_branch=_lit("2"))
        result = infer_expr_type(node, {"c": BOOL_TYPE})
        assert result.tcl_type is TclType.INT

    def test_int_double_join(self):
        node = ExprTernary(condition=_var("c"), true_branch=_lit("1"), false_branch=_lit("2.0"))
        result = infer_expr_type(node, {"c": BOOL_TYPE})
        assert result.tcl_type is TclType.NUMERIC

    def test_int_string_join_is_shimmered(self):
        node = ExprTernary(
            condition=_var("c"),
            true_branch=_lit("1"),
            false_branch=_str("hello"),
        )
        result = infer_expr_type(node, {"c": BOOL_TYPE})
        # INT and STRING are incompatible → SHIMMERED
        assert result.kind is TypeKind.SHIMMERED

    def test_bool_bool_join(self):
        node = ExprTernary(
            condition=_var("c"),
            true_branch=_lit("true"),
            false_branch=_lit("false"),
        )
        result = infer_expr_type(node, {"c": BOOL_TYPE})
        assert result.tcl_type is TclType.BOOLEAN


class TestFunctionReturnTypes:
    """Math function return types."""

    @pytest.mark.parametrize(
        "func,expected",
        [
            ("int", TclType.INT),
            ("round", TclType.INT),
            ("ceil", TclType.INT),
            ("floor", TclType.INT),
            ("isqrt", TclType.INT),
            ("wide", TclType.INT),
            ("entier", TclType.INT),
        ],
    )
    def test_int_returning_functions(self, func, expected):
        node = _call(func, _var("x"))
        result = infer_expr_type(node, {"x": DOUBLE_TYPE})
        assert result.tcl_type is expected

    @pytest.mark.parametrize(
        "func",
        [
            "sin",
            "cos",
            "tan",
            "asin",
            "acos",
            "atan",
            "sinh",
            "cosh",
            "tanh",
            "sqrt",
            "exp",
            "log",
            "log10",
            "pow",
            "double",
            "rand",
            "srand",
        ],
    )
    def test_double_returning_functions(self, func):
        if func == "rand":
            node = _call(func)
        elif func in ("atan2", "pow", "hypot", "fmod"):
            node = _call(func, _var("x"), _var("y"))
        else:
            node = _call(func, _var("x"))
        result = infer_expr_type(node, {"x": INT_TYPE, "y": INT_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    @pytest.mark.parametrize("func", ["bool", "isnan", "isinf"])
    def test_boolean_returning_functions(self, func):
        node = _call(func, _var("x"))
        result = infer_expr_type(node, {"x": INT_TYPE})
        assert result.tcl_type is TclType.BOOLEAN

    def test_abs_preserves_int(self):
        node = _call("abs", _var("x"))
        result = infer_expr_type(node, {"x": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_abs_preserves_double(self):
        node = _call("abs", _var("x"))
        result = infer_expr_type(node, {"x": DOUBLE_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_max_int_int(self):
        node = _call("max", _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_max_int_double(self):
        node = _call("max", _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": DOUBLE_TYPE})
        assert result.tcl_type is TclType.NUMERIC

    def test_min_double_double(self):
        node = _call("min", _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": DOUBLE_TYPE, "b": DOUBLE_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_max_three_args(self):
        node = _call("max", _var("a"), _var("b"), _var("c"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE, "c": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_max_three_mixed(self):
        node = _call("max", _var("a"), _var("b"), _var("c"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": DOUBLE_TYPE, "c": INT_TYPE})
        assert result.tcl_type is TclType.NUMERIC

    def test_unknown_function_returns_numeric(self):
        node = _call("unknown_func", _var("x"))
        result = infer_expr_type(node, {"x": INT_TYPE})
        assert result.tcl_type is TclType.NUMERIC

    def test_atan2_is_double(self):
        node = _call("atan2", _var("y"), _var("x"))
        result = infer_expr_type(node, {"x": INT_TYPE, "y": INT_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_hypot_is_double(self):
        node = _call("hypot", _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_fmod_is_double(self):
        node = _call("fmod", _var("a"), _var("b"))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.DOUBLE


class TestComplexExpressions:
    """Complex nested expression type inference."""

    def test_comparison_of_arithmetic(self):
        """``($a + $b) == ($c * $d)`` → BOOLEAN."""
        node = _binop(
            BinOp.EQ,
            _binop(BinOp.ADD, _var("a"), _var("b")),
            _binop(BinOp.MUL, _var("c"), _var("d")),
        )
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE, "c": INT_TYPE, "d": INT_TYPE})
        assert result.tcl_type is TclType.BOOLEAN

    def test_arithmetic_of_functions(self):
        """``sin($x) + cos($x)`` → DOUBLE."""
        node = _binop(
            BinOp.ADD,
            _call("sin", _var("x")),
            _call("cos", _var("x")),
        )
        result = infer_expr_type(node, {"x": DOUBLE_TYPE})
        assert result.tcl_type is TclType.DOUBLE

    def test_logical_of_comparisons(self):
        """``($a > 0) && ($b < 10)`` → BOOLEAN."""
        node = _binop(
            BinOp.AND,
            _binop(BinOp.GT, _var("a"), _lit("0")),
            _binop(BinOp.LT, _var("b"), _lit("10")),
        )
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.BOOLEAN

    def test_bitwise_shift_chain(self):
        """``($a << 2) | ($b >> 1)`` → INT."""
        node = _binop(
            BinOp.BIT_OR,
            _binop(BinOp.LSHIFT, _var("a"), _lit("2")),
            _binop(BinOp.RSHIFT, _var("b"), _lit("1")),
        )
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.INT

    def test_ternary_with_complex_branches(self):
        """``$c ? ($a + $b) : ($d * 2.0)`` → NUMERIC (INT branch + DOUBLE branch)."""
        node = ExprTernary(
            condition=_var("c"),
            true_branch=_binop(BinOp.ADD, _var("a"), _var("b")),
            false_branch=_binop(BinOp.MUL, _var("d"), _lit("2.0")),
        )
        result = infer_expr_type(
            node,
            {"c": BOOL_TYPE, "a": INT_TYPE, "b": INT_TYPE, "d": INT_TYPE},
        )
        assert result.tcl_type is TclType.NUMERIC

    def test_nested_function_in_arithmetic(self):
        """``int(sin($x)) + 1`` → INT."""
        node = _binop(
            BinOp.ADD,
            _call("int", _call("sin", _var("x"))),
            _lit("1"),
        )
        result = infer_expr_type(node, {"x": DOUBLE_TYPE})
        assert result.tcl_type is TclType.INT

    def test_abs_of_subtraction(self):
        """``abs($a - $b)`` with INT operands → INT."""
        node = _call("abs", _binop(BinOp.SUB, _var("a"), _var("b")))
        result = infer_expr_type(node, {"a": INT_TYPE, "b": INT_TYPE})
        assert result.tcl_type is TclType.INT


class TestIRulesOperatorTypes:
    """Type inference for iRules-specific operators."""

    @pytest.mark.parametrize(
        "op",
        [
            BinOp.CONTAINS,
            BinOp.STARTS_WITH,
            BinOp.ENDS_WITH,
            BinOp.STR_EQUALS,
            BinOp.MATCHES_GLOB,
            BinOp.MATCHES_REGEX,
        ],
    )
    def test_string_ops_return_boolean(self, op):
        node = _binop(op, _var("a"), _str("pattern"))
        result = infer_expr_type(node, {"a": STRING_TYPE})
        assert result.tcl_type is TclType.BOOLEAN

    def test_word_and_returns_boolean(self):
        node = _binop(BinOp.WORD_AND, _var("a"), _var("b"))
        result = infer_expr_type(node, {})
        assert result.tcl_type is TclType.BOOLEAN

    def test_word_or_returns_boolean(self):
        node = _binop(BinOp.WORD_OR, _var("a"), _var("b"))
        result = infer_expr_type(node, {})
        assert result.tcl_type is TclType.BOOLEAN

    def test_word_not_returns_boolean(self):
        node = _unary(UnaryOp.WORD_NOT, _var("x"))
        result = infer_expr_type(node, {"x": INT_TYPE})
        assert result.tcl_type is TclType.BOOLEAN
