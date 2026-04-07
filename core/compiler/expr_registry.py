"""Centralised metadata for Tcl expression operators and math functions.

Replaces scattered frozensets and dicts in ``expr_types.py`` and
``tcl_expr_eval.py`` with a single registry that can be extended
by dialect stubs.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import TYPE_CHECKING, Callable

from .expr_ast import BinOp, UnaryOp
from .types import TclType

if TYPE_CHECKING:
    TclValue = int | float


# ---------------------------------------------------------------------------
# Operator classification
# ---------------------------------------------------------------------------


class OpKind(Enum):
    """How a binary operator determines its result type."""

    ARITHMETIC = auto()  # result depends on operand types
    DIVISION = auto()  # INT/INT→INT, else DOUBLE
    BITWISE = auto()  # always INT
    COMPARISON = auto()  # always BOOLEAN
    LOGICAL = auto()  # always BOOLEAN


class UnaryOpKind(Enum):
    """How a unary operator determines its result type."""

    IDENTITY = auto()  # preserves operand type (-x, +x)
    BITWISE = auto()  # always INT (~x)
    LOGICAL = auto()  # always BOOLEAN (!x, not x)


BINOP_KIND: dict[BinOp, OpKind] = {
    # Arithmetic
    BinOp.ADD: OpKind.ARITHMETIC,
    BinOp.SUB: OpKind.ARITHMETIC,
    BinOp.MUL: OpKind.ARITHMETIC,
    BinOp.MOD: OpKind.ARITHMETIC,
    BinOp.POW: OpKind.ARITHMETIC,
    # Division
    BinOp.DIV: OpKind.DIVISION,
    # Bitwise / shift
    BinOp.BIT_AND: OpKind.BITWISE,
    BinOp.BIT_OR: OpKind.BITWISE,
    BinOp.BIT_XOR: OpKind.BITWISE,
    BinOp.LSHIFT: OpKind.BITWISE,
    BinOp.RSHIFT: OpKind.BITWISE,
    # Logical
    BinOp.AND: OpKind.LOGICAL,
    BinOp.OR: OpKind.LOGICAL,
    BinOp.WORD_AND: OpKind.LOGICAL,
    BinOp.WORD_OR: OpKind.LOGICAL,
    # Numeric comparison
    BinOp.EQ: OpKind.COMPARISON,
    BinOp.NE: OpKind.COMPARISON,
    BinOp.LT: OpKind.COMPARISON,
    BinOp.LE: OpKind.COMPARISON,
    BinOp.GT: OpKind.COMPARISON,
    BinOp.GE: OpKind.COMPARISON,
    # String comparison
    BinOp.STR_EQ: OpKind.COMPARISON,
    BinOp.STR_NE: OpKind.COMPARISON,
    BinOp.STR_LT: OpKind.COMPARISON,
    BinOp.STR_LE: OpKind.COMPARISON,
    BinOp.STR_GT: OpKind.COMPARISON,
    BinOp.STR_GE: OpKind.COMPARISON,
    # List membership
    BinOp.IN: OpKind.COMPARISON,
    BinOp.NI: OpKind.COMPARISON,
    # iRules string operators
    BinOp.CONTAINS: OpKind.COMPARISON,
    BinOp.STARTS_WITH: OpKind.COMPARISON,
    BinOp.ENDS_WITH: OpKind.COMPARISON,
    BinOp.STR_EQUALS: OpKind.COMPARISON,
    BinOp.MATCHES_GLOB: OpKind.COMPARISON,
    BinOp.MATCHES_REGEX: OpKind.COMPARISON,
}

UNARYOP_KIND: dict[UnaryOp, UnaryOpKind] = {
    UnaryOp.NEG: UnaryOpKind.IDENTITY,
    UnaryOp.POS: UnaryOpKind.IDENTITY,
    UnaryOp.BIT_NOT: UnaryOpKind.BITWISE,
    UnaryOp.NOT: UnaryOpKind.LOGICAL,
    UnaryOp.WORD_NOT: UnaryOpKind.LOGICAL,
}


# ---------------------------------------------------------------------------
# Math function registry
# ---------------------------------------------------------------------------


@dataclass(slots=True)
class ExprFunc:
    """Metadata for a single expr math function."""

    name: str
    return_type: TclType
    min_args: int = 1
    max_args: int = 1  # sys.maxsize for variadic
    identity: bool = False  # abs — preserves operand type
    variadic_join: bool = False  # max, min — type join of operands
    const_fold: Callable[[list[TclValue]], TclValue | None] | None = None


class ExprFuncRegistry:
    """Registry for expr math functions (built-in + user-defined stubs)."""

    def __init__(self) -> None:
        self._funcs: dict[str, ExprFunc] = {}

    def register(self, func: ExprFunc) -> None:
        """Register a math function."""
        self._funcs[func.name] = func

    def get(self, name: str) -> ExprFunc | None:
        """Look up a math function by name."""
        return self._funcs.get(name)

    def names(self) -> frozenset[str]:
        """Return all registered function names."""
        return frozenset(self._funcs)

    def is_known(self, name: str) -> bool:
        """Return True if the function is registered."""
        return name in self._funcs


def _build_default_registry() -> ExprFuncRegistry:
    """Create the registry with all built-in Tcl math functions."""
    reg = ExprFuncRegistry()
    _ANY = sys.maxsize

    # Type conversion
    reg.register(ExprFunc("int", TclType.INT))
    reg.register(ExprFunc("round", TclType.INT))
    reg.register(ExprFunc("ceil", TclType.INT))
    reg.register(ExprFunc("floor", TclType.INT))
    reg.register(ExprFunc("isqrt", TclType.INT))
    reg.register(ExprFunc("wide", TclType.INT))
    reg.register(ExprFunc("entier", TclType.INT))

    # Double-returning
    reg.register(ExprFunc("double", TclType.DOUBLE))
    reg.register(ExprFunc("sin", TclType.DOUBLE))
    reg.register(ExprFunc("cos", TclType.DOUBLE))
    reg.register(ExprFunc("tan", TclType.DOUBLE))
    reg.register(ExprFunc("asin", TclType.DOUBLE))
    reg.register(ExprFunc("acos", TclType.DOUBLE))
    reg.register(ExprFunc("atan", TclType.DOUBLE))
    reg.register(ExprFunc("atan2", TclType.DOUBLE, min_args=2, max_args=2))
    reg.register(ExprFunc("sinh", TclType.DOUBLE))
    reg.register(ExprFunc("cosh", TclType.DOUBLE))
    reg.register(ExprFunc("tanh", TclType.DOUBLE))
    reg.register(ExprFunc("sqrt", TclType.DOUBLE))
    reg.register(ExprFunc("exp", TclType.DOUBLE))
    reg.register(ExprFunc("log", TclType.DOUBLE))
    reg.register(ExprFunc("log10", TclType.DOUBLE))
    reg.register(ExprFunc("pow", TclType.DOUBLE, min_args=2, max_args=2))
    reg.register(ExprFunc("hypot", TclType.DOUBLE, min_args=2, max_args=2))
    reg.register(ExprFunc("fmod", TclType.DOUBLE, min_args=2, max_args=2))
    reg.register(ExprFunc("rand", TclType.DOUBLE, min_args=0, max_args=0))
    reg.register(ExprFunc("srand", TclType.DOUBLE))

    # Boolean-returning
    reg.register(ExprFunc("bool", TclType.BOOLEAN))
    reg.register(ExprFunc("isnan", TclType.BOOLEAN))
    reg.register(ExprFunc("isinf", TclType.BOOLEAN))

    # Identity (preserves operand type)
    reg.register(ExprFunc("abs", TclType.INT, identity=True))

    # Variadic join (type join of all operands)
    reg.register(ExprFunc("max", TclType.NUMERIC, min_args=1, max_args=_ANY, variadic_join=True))
    reg.register(ExprFunc("min", TclType.NUMERIC, min_args=1, max_args=_ANY, variadic_join=True))

    return reg


EXPR_FUNC_REGISTRY = _build_default_registry()
