"""Differential test harness for the Rust and Python expression parsers.

The Rust expression parser (C1) is a Pratt parser that converts
``ExprToken`` lists into ``ExprNode`` ASTs — the same pipeline as the
Python ``core.parsing.expr_parser.parse_expr``.  This harness:

1. hand-curates a corpus of expression inputs covering every construct;
2. feeds each input through both the Python and Rust parsers;
3. asserts the two produce identical output on three axes:
   - rendered text (round-trip via ``render_expr``)
   - structural tag (top-level node kind + operator)
   - extracted variables (variable names in the expression)
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.expr_ast import render_expr, vars_in_expr_node
from core.parsing.expr_parser import parse_expr as python_parse_expr

tcl_lsp_rust = pytest.importorskip(
    "tcl_lsp_rust",
    reason="Rust extension not built; run `make rust-build`",
)

rust_parse_expr_render = tcl_lsp_rust.parse_expr_render
rust_parse_expr_vars = tcl_lsp_rust.parse_expr_vars
rust_parse_expr_tag = tcl_lsp_rust.parse_expr_tag


def _python_tag(source: str, dialect: str | None = None) -> str:
    """Produce a structural tag from the Python parser result."""
    from core.compiler.expr_ast import (
        ExprBinary,
        ExprCall,
        ExprCommand,
        ExprLiteral,
        ExprRaw,
        ExprString,
        ExprTernary,
        ExprUnary,
        ExprVar,
    )

    node = python_parse_expr(source, dialect=dialect)
    match node:
        case ExprLiteral():
            return "Literal"
        case ExprString():
            return "String"
        case ExprVar(name=name):
            return f"Var:{name}"
        case ExprCommand():
            return "Command"
        case ExprBinary(op=op):
            return f"Binary:{op.value}"
        case ExprUnary(op=op):
            return f"Unary:{op.value}"
        case ExprTernary():
            return "Ternary"
        case ExprCall(function=func):
            return f"Call:{func}"
        case ExprRaw():
            return "Raw"
        case _:
            return f"Unknown:{type(node).__name__}"


def _check_parity(source: str, dialect: str | None = None) -> None:
    """Assert Python and Rust parsers agree on a given expression."""
    # Rendered text parity
    py_node = python_parse_expr(source, dialect=dialect)
    py_rendered = render_expr(py_node)
    rust_rendered = rust_parse_expr_render(source, dialect)
    assert py_rendered == rust_rendered, (
        f"render mismatch for {source!r}:\n  Python: {py_rendered!r}\n  Rust:   {rust_rendered!r}"
    )

    # Structural tag parity
    py_tag = _python_tag(source, dialect)
    rust_tag = rust_parse_expr_tag(source, dialect)
    assert py_tag == rust_tag, (
        f"tag mismatch for {source!r}:\n  Python: {py_tag!r}\n  Rust:   {rust_tag!r}"
    )

    # Variable extraction parity.
    #
    # The Python ``vars_in_expr_node`` crosses into command-substitution
    # bodies via ``ssa._vars_in_script`` — a script-level analysis the
    # Rust AST-level ``ExprNode::vars()`` intentionally does not perform
    # (it stops at the ``[cmd]`` boundary). Skip the vars check when the
    # expression contains command substitutions; full parity comes when
    # the SSA module moves to Rust.
    if "[" not in source:
        py_vars = vars_in_expr_node(py_node)
        rust_vars = rust_parse_expr_vars(source, dialect)
        assert py_vars == rust_vars, (
            f"vars mismatch for {source!r}:\n  Python: {py_vars!r}\n  Rust:   {rust_vars!r}"
        )


# Curated corpus — organised by expression construct.

LITERALS = [
    "42",
    "0",
    "3.14",
    "1.5e10",
    "0xFF",
    "0o77",
    "0b1010",
    "true",
    "false",
    "yes",
    "no",
    "on",
    "off",
]


class TestLiterals:
    @pytest.mark.parametrize("source", LITERALS)
    def test_literal(self, source: str) -> None:
        _check_parity(source)


STRINGS = [
    '"hello"',
    '"a b c"',
    '""',
    '"\\"quoted\\""',
    "{1 + 2}",
]


class TestStrings:
    @pytest.mark.parametrize("source", STRINGS)
    def test_string(self, source: str) -> None:
        _check_parity(source)


VARIABLES = [
    "$x",
    "$foo",
    "${bar}",
    "$arr(idx)",
    "$ns::var",
    "$::global",
]


class TestVariables:
    @pytest.mark.parametrize("source", VARIABLES)
    def test_variable(self, source: str) -> None:
        _check_parity(source)


COMMANDS = [
    "[cmd]",
    "[llength $list]",
    "[expr {1 + 2}]",
    "[string length $s]",
]


class TestCommands:
    @pytest.mark.parametrize("source", COMMANDS)
    def test_command(self, source: str) -> None:
        _check_parity(source)


BINARY_OPS = [
    ("$a + $b", "+"),
    ("$a - $b", "-"),
    ("$a * $b", "*"),
    ("$a / $b", "/"),
    ("$a % $b", "%"),
    ("$a ** $b", "**"),
    ("$a << $b", "<<"),
    ("$a >> $b", ">>"),
    ("$a & $b", "&"),
    ("$a | $b", "|"),
    ("$a ^ $b", "^"),
    ("$a && $b", "&&"),
    ("$a || $b", "||"),
    ("$a == $b", "=="),
    ("$a != $b", "!="),
    ("$a < $b", "<"),
    ("$a <= $b", "<="),
    ("$a > $b", ">"),
    ("$a >= $b", ">="),
    ("$a eq $b", "eq"),
    ("$a ne $b", "ne"),
    ("$a lt $b", "lt"),
    ("$a le $b", "le"),
    ("$a gt $b", "gt"),
    ("$a ge $b", "ge"),
    ("$x in $list", "in"),
    ("$x ni $list", "ni"),
]


class TestBinaryOps:
    @pytest.mark.parametrize("source,op", BINARY_OPS, ids=[op for _, op in BINARY_OPS])
    def test_binary(self, source: str, op: str) -> None:
        _check_parity(source)


UNARY_OPS = [
    "-$x",
    "+$x",
    "~$x",
    "!$x",
]


class TestUnaryOps:
    @pytest.mark.parametrize("source", UNARY_OPS)
    def test_unary(self, source: str) -> None:
        _check_parity(source)


PRECEDENCE = [
    "$a + $b * $c",
    "$a * $b + $c",
    "($a + $b) * $c",
    "$a ** $b ** $c",
    "-$a + $b",
    "$a + $b + $c",
    "$a > 0 && $b < 10",
    "$a || $b && $c",
    "$a == $b || $c != $d",
]


class TestPrecedence:
    @pytest.mark.parametrize("source", PRECEDENCE)
    def test_precedence(self, source: str) -> None:
        _check_parity(source)


TERNARY = [
    "$x ? 1 : 0",
    "$a ? $b : $c",
    "$a ? 1 : $b ? 2 : 3",
]


class TestTernary:
    @pytest.mark.parametrize("source", TERNARY)
    def test_ternary(self, source: str) -> None:
        _check_parity(source)


FUNCTIONS = [
    "rand()",
    "sin($x)",
    "max($a, $b)",
    "int(sin($x))",
    "abs($a - $b)",
    "atan2($y, $x)",
    "pow($base, $exp)",
]


class TestFunctions:
    @pytest.mark.parametrize("source", FUNCTIONS)
    def test_function(self, source: str) -> None:
        _check_parity(source)


COMPLEX = [
    "[llength $list] + 1",
    "$a > 0 && $b < 10 || $c == 0",
    "sin($x) * cos($y) + 1.0",
    "($a + $b) * ($c - $d)",
    "$x ? sin($y) : cos($z)",
]


class TestComplex:
    @pytest.mark.parametrize("source", COMPLEX)
    def test_complex(self, source: str) -> None:
        _check_parity(source)


FALLBACK = [
    "",
    "   ",
]


class TestFallback:
    @pytest.mark.parametrize("source", FALLBACK)
    def test_fallback(self, source: str) -> None:
        _check_parity(source)
