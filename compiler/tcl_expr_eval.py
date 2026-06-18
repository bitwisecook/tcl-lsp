"""Tcl expression evaluator with Tcl 9.0.2 semantics.

Walks the ExprNode AST produced by expr_parser.parse_expr() and evaluates
constant expressions.  Returns int | float | None — Tcl booleans are
represented as int 0/1.

Semantics follow the Tcl 9.0.2 source (tclExecute.c, tclBasic.c):
- Integer division: floor toward -inf (Python // matches)
- Integer modulo: sign follows divisor (Python % matches)
- Exponentiation: special integer rules for negative exponents
- Comparisons: always int 0/1
- Math functions: Tcl round() ties away from zero (not Python's even)
"""

from __future__ import annotations

import fnmatch
import logging
import math
import re
from collections.abc import Callable
from typing import TypeAlias

from compiler.parsing.expr_parser import parse_expr
from compiler.registry.dialect import active_dialect
from shared.tcl_list import tcl_list_split

from .expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
    ExprVar,
    UnaryOp,
)
from .tcl_constants import TCL_BOOL_FALSE as _BOOL_FALSE
from .tcl_constants import TCL_BOOL_TRUE as _BOOL_TRUE

log = logging.getLogger(__name__)

TclValue: TypeAlias = int | float
"""Result of evaluating a Tcl expression: int or IEEE-754 double."""

_MAX_EXPONENT = (1 << 28) - 1  # reference Tcl INST_EXPON limit: exponent must be < 2^28


# Public API


def eval_tcl_expr(
    node: ExprNode,
    variables: dict[str, int | float | str] | None = None,
) -> TclValue | None:
    """Evaluate a Tcl expression AST with Tcl 9.0.2 semantics.

    Returns None when the expression cannot be evaluated at compile time
    (unbound variables, command substitutions, domain errors, division by
    zero, etc.).
    """
    return _eval(node, variables or {})


def eval_tcl_expr_str(
    expr: str,
    variables: dict[str, int | float | str] | None = None,
    *,
    dialect: str | None = None,
) -> TclValue | None:
    """Parse and evaluate an expression string."""
    node = parse_expr(expr, dialect=dialect or active_dialect())
    if isinstance(node, ExprRaw):
        return None
    return eval_tcl_expr(node, variables)


def format_tcl_value(value: TclValue) -> str:
    """Render a TclValue as a Tcl source literal."""
    if isinstance(value, float):
        if math.isinf(value):
            return "-Inf" if value < 0 else "Inf"
        if math.isnan(value):
            return "NaN"
        if value == 0.0:
            # Preserve the sign bit: IEEE 754 distinguishes -0.0 from +0.0.
            return "-0.0" if math.copysign(1.0, value) < 0 else "0.0"
        # repr() gives "42.0" for whole-number floats and "1e+308" for large
        # values — both correct for Tcl.  The previous int() branch produced
        # a many-digit integer string for values like 1e308.
        return repr(value)
    return str(value)


# Internal evaluator


def _eval(node: ExprNode, env: dict[str, int | float | str]) -> TclValue | None:
    match node:
        case ExprLiteral(text=text):
            return _parse_literal(text)
        case ExprVar(name=name):
            return _resolve_var(name, env)
        case ExprBinary(op=op, left=left, right=right):
            return _eval_binary(op, left, right, env)
        case ExprUnary(op=op, operand=operand):
            return _eval_unary(op, operand, env)
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            cv = _eval(cond, env)
            if cv is None:
                return None
            return _eval(tb if cv else fb, env)
        case ExprCall(function=func, args=args):
            return _eval_call(func, args, env)
        case ExprCommand() | ExprRaw() | ExprString():
            return None
        case _:
            return None


# Literals


def _parse_literal(text: str) -> TclValue | None:
    low = text.lower()
    if low in _BOOL_TRUE:
        return 1
    if low in _BOOL_FALSE:
        return 0
    try:
        return int(text, 0)  # handles 0x, 0o, 0b prefixes
    except (ValueError, TypeError):
        pass
    # Tcl 9.0 treats leading-zero decimals (e.g. "0005") as integers.
    # Python's int(x, 0) rejects them, so try base-10 explicitly.
    try:
        return int(text, 10)
    except (ValueError, TypeError):
        pass
    try:
        return float(text)
    except (ValueError, TypeError):
        return None


def _is_non_numeric_string(text: str) -> bool:
    """True when *text* cannot be a Tcl number in *any* dialect.

    Used by the ``==``/``!=`` constant folder to decide it is safe to compare
    two operands purely as strings: if either could parse as a number under
    some dialect (decimal, leading-zero octal/decimal, ``0x``/``0o``/``0b``,
    float, ``Inf``/``NaN``) the numeric-vs-string result would be
    dialect-dependent, so the caller declines to fold.  Boolean words
    (``true``/``yes``/…) are *not* numbers for ``==``/``!=`` and so count as
    non-numeric strings.  Deliberately stricter than :func:`_parse_literal`,
    which coerces boolean words and leading-zero decimals to integers.
    """
    t = text.strip()
    if not t:
        return True
    # ``int(t, 0)`` accepts decimal without leading zeros plus ``0x``/``0o``/
    # ``0b``; ``int(t, 10)`` additionally accepts leading-zero decimals (octal
    # in 8.x, decimal in 9.0 — dialect-dependent, hence "looks numeric").
    for base in (0, 10):
        try:
            int(t, base)
            return False
        except (ValueError, TypeError):
            pass
    try:
        float(t)
        return False
    except (ValueError, TypeError):
        return True


def _resolve_var(
    name: str,
    env: dict[str, int | float | str],
) -> TclValue | None:
    val = env.get(name)
    if val is None:
        return None
    if isinstance(val, (int, float)):
        return val
    if isinstance(val, str):
        return _parse_literal(val)
    return None


# String extraction (for iRules string operators)


def _strip_string_delimiters(text: str) -> str:
    """Strip surrounding quotes or braces from a string literal."""
    if len(text) >= 2:
        if (text[0] == '"' and text[-1] == '"') or (text[0] == "{" and text[-1] == "}"):
            return text[1:-1]
    return text


def _eval_as_string(
    node: ExprNode,
    env: dict[str, int | float | str],
) -> str | None:
    """Extract a string value from a node for iRules string operators."""
    match node:
        case ExprString(text=text):
            return _strip_string_delimiters(text)
        case ExprLiteral(text=text):
            return text
        case ExprVar(name=name):
            val = env.get(name)
            if val is None:
                return None
            return str(val)
        case _:
            # Try numeric evaluation and convert to string
            v = _eval(node, env)
            if v is not None:
                return format_tcl_value(v)
            return None


def _split_tcl_list(text: str) -> list[str]:
    """Split a simple Tcl list string into elements (lenient).

    Thin wrapper over the canonical :func:`shared.tcl_list.tcl_list_split`.
    """
    return tcl_list_split(text)


# Binary operators

_IRULES_STRING_OPS = frozenset(
    {
        BinOp.CONTAINS,
        BinOp.STARTS_WITH,
        BinOp.ENDS_WITH,
        BinOp.STR_EQUALS,
        BinOp.MATCHES_GLOB,
        BinOp.MATCHES_REGEX,
        BinOp.IN,
        BinOp.NI,
    }
)

# ``eq`` / ``ne`` / ``lt`` / ``gt`` / ``le`` / ``ge`` always compare their
# operands *as strings* (unlike ``==``/``!=``), so a constant fold must extract
# string operands — evaluating ``"x"`` numerically yields nothing.
_STRING_COMPARE_OPS = frozenset(
    {
        BinOp.STR_EQ,
        BinOp.STR_NE,
        BinOp.STR_LT,
        BinOp.STR_LE,
        BinOp.STR_GT,
        BinOp.STR_GE,
    }
)


def _eval_binary(
    op: BinOp,
    left: ExprNode,
    right: ExprNode,
    env: dict[str, int | float | str],
) -> TclValue | None:
    # Short-circuit operators: don't evaluate right if not needed
    if op in (BinOp.AND, BinOp.WORD_AND):
        lv = _eval(left, env)
        if lv is None:
            return None
        if not lv:
            return 0
        rv = _eval(right, env)
        if rv is None:
            return None
        return 1 if rv else 0

    if op in (BinOp.OR, BinOp.WORD_OR):
        lv = _eval(left, env)
        if lv is None:
            return None
        if lv:
            return 1
        rv = _eval(right, env)
        if rv is None:
            return None
        return 1 if rv else 0

    # iRules string operators: extract string operands
    if op in _IRULES_STRING_OPS:
        ls = _eval_as_string(left, env)
        if ls is None:
            return None
        rs = _eval_as_string(right, env)
        if rs is None:
            return None
        return _apply_irules_string_op(op, ls, rs)

    # ``eq``/``ne``/``lt``/``gt``/``le``/``ge`` — string comparison: extract
    # string operands (so ``"x" ne "y"`` and ``5 eq "5"`` fold) and compare.
    if op in _STRING_COMPARE_OPS:
        ls = _eval_as_string(left, env)
        if ls is None:
            return None
        rs = _eval_as_string(right, env)
        if rs is None:
            return None
        return _apply_string_compare(op, ls, rs)

    # ``==`` / ``!=`` follow Tcl's polymorphic comparison: numeric when *both*
    # operands are numbers, string otherwise.  The numeric case is covered by
    # the generic ``_eval`` path (numeric literals / numeric-valued vars); here
    # we add the *string* case the old code missed (``"foo" == "foo"`` never
    # folded).  Fold to a string comparison only when both operands are
    # definitively non-numeric, so the result is dialect-independent.  If
    # either operand merely *looks* numeric — a boolean word, or a leading-zero
    # decimal that is octal in 8.x but decimal in 9.0 — the numeric-vs-string
    # outcome depends on the dialect's equality-conversion rules, so we decline
    # to fold rather than risk a wrong answer (a false I230 unreachable-branch).
    if op in (BinOp.EQ, BinOp.NE):
        lv = _eval(left, env)
        rv = _eval(right, env)
        if lv is not None and rv is not None:
            return _apply_binary(op, lv, rv)
        ls = _eval_as_string(left, env)
        if ls is None:
            return None
        rs = _eval_as_string(right, env)
        if rs is None:
            return None
        if _is_non_numeric_string(ls) and _is_non_numeric_string(rs):
            str_op = BinOp.STR_EQ if op is BinOp.EQ else BinOp.STR_NE
            return _apply_string_compare(str_op, ls, rs)
        return None

    # All other operators evaluate both sides
    lv = _eval(left, env)
    if lv is None:
        return None
    rv = _eval(right, env)
    if rv is None:
        return None

    try:
        result = _apply_binary(op, lv, rv)
        # Tcl 9.0 raises ARITH DOMAIN for NaN-producing operations; let the
        # VM runtime handle them rather than silently constant-folding to NaN.
        if isinstance(result, float) and math.isnan(result):
            return None
        return result
    except Exception:
        log.debug("expr_eval: binary operation failed", exc_info=True)
        return None


def _apply_binary(op: BinOp, a: TclValue, b: TclValue) -> TclValue | None:
    match op:
        # Arithmetic
        case BinOp.ADD:
            return a + b
        case BinOp.SUB:
            return a - b
        case BinOp.MUL:
            return a * b
        case BinOp.DIV:
            return _tcl_div(a, b)
        case BinOp.MOD:
            return _tcl_mod(a, b)
        case BinOp.POW:
            return _tcl_pow(a, b)

        # Shift (require int operands)
        case BinOp.LSHIFT:
            if not isinstance(a, int) or not isinstance(b, int):
                return None
            if b < 0:
                return None
            if b > 64:
                return None  # guard against huge shifts
            return a << b
        case BinOp.RSHIFT:
            if not isinstance(a, int) or not isinstance(b, int):
                return None
            if b < 0:
                return None
            if b > 64:
                return 0 if a >= 0 else -1
            return a >> b

        # Bitwise (require int operands)
        case BinOp.BIT_AND:
            if not isinstance(a, int) or not isinstance(b, int):
                return None
            return a & b
        case BinOp.BIT_OR:
            if not isinstance(a, int) or not isinstance(b, int):
                return None
            return a | b
        case BinOp.BIT_XOR:
            if not isinstance(a, int) or not isinstance(b, int):
                return None
            return a ^ b

        # Numeric comparison — always int 0/1
        case BinOp.EQ:
            return 1 if a == b else 0
        case BinOp.NE:
            return 1 if a != b else 0
        case BinOp.LT:
            return 1 if a < b else 0
        case BinOp.LE:
            return 1 if a <= b else 0
        case BinOp.GT:
            return 1 if a > b else 0
        case BinOp.GE:
            return 1 if a >= b else 0

        # String-comparison ops (eq/ne/lt/le/gt/ge) are routed to
        # ``_apply_string_compare`` from ``_eval_binary`` before they ever reach
        # here (they need string — not numeric — operands), so they fall through.
        case _:
            return None


def _apply_string_compare(op: BinOp, a: str, b: str) -> int | None:
    """Apply a Tcl string-comparison operator (``eq``/``ne``/``lt``/…) to two
    already-stringified operands, returning ``1``/``0`` or ``None``."""
    match op:
        case BinOp.STR_EQ:
            return 1 if a == b else 0
        case BinOp.STR_NE:
            return 1 if a != b else 0
        case BinOp.STR_LT:
            return 1 if a < b else 0
        case BinOp.STR_LE:
            return 1 if a <= b else 0
        case BinOp.STR_GT:
            return 1 if a > b else 0
        case BinOp.STR_GE:
            return 1 if a >= b else 0
        case _:
            return None
    return None  # unreachable; satisfies static analysis


def _tcl_div(a: TclValue, b: TclValue) -> TclValue | None:
    if b == 0:
        return None
    if isinstance(a, int) and isinstance(b, int):
        return a // b  # Python // is floor division, same as Tcl
    return a / b


def _tcl_mod(a: TclValue, b: TclValue) -> TclValue | None:
    # Tcl 9.0 requires integer operands for %
    if isinstance(a, float) or isinstance(b, float):
        return None
    if b == 0:
        return None
    return a % b  # Python % sign follows divisor, same as Tcl


def _tcl_pow(a: TclValue, b: TclValue) -> TclValue | None:
    # Any float involvement → float result
    if isinstance(a, float) or isinstance(b, float):
        fa, fb = float(a), float(b)
        if fa == 0.0 and fb < 0:
            return None  # error: exponentiation of zero by negative power
        # Tcl 9.0: negative base with non-integer exponent is a domain error
        if fa < 0 and (not math.isfinite(fb) or not fb.is_integer()):
            return None  # domain error: argument not in valid range
        try:
            return math.pow(fa, fb)
        except (ValueError, OverflowError):
            return None

    # Both int
    if not isinstance(a, int) or not isinstance(b, int):
        return None

    if b == 0:
        return 1
    if b == 1:
        return a
    if a == 0:
        if b < 0:
            return None  # error
        return 0
    if a == 1:
        return 1
    if a == -1:
        return -1 if b & 1 else 1

    if b < 0:
        return 0  # Tcl: |base| > 1, negative exponent → integer 0

    if b > _MAX_EXPONENT:
        return None  # guard

    return a**b


# iRules string operators

_MATCHES_REGEX_CACHE: dict[str, re.Pattern[str] | None] = {}


def _compile_regex(pattern: str) -> re.Pattern[str] | None:
    """Compile and cache a regex pattern. Returns None on invalid patterns."""
    cached = _MATCHES_REGEX_CACHE.get(pattern)
    if cached is not None:
        return cached
    if pattern in _MATCHES_REGEX_CACHE:
        return None  # previously failed compilation
    try:
        compiled = re.compile(pattern)
    except re.error:
        _MATCHES_REGEX_CACHE[pattern] = None
        return None
    _MATCHES_REGEX_CACHE[pattern] = compiled
    return compiled


def _apply_irules_string_op(op: BinOp, left: str, right: str) -> int | None:
    match op:
        case BinOp.CONTAINS:
            return 1 if right in left else 0
        case BinOp.STARTS_WITH:
            return 1 if left.startswith(right) else 0
        case BinOp.ENDS_WITH:
            return 1 if left.endswith(right) else 0
        case BinOp.STR_EQUALS:
            return 1 if left == right else 0
        case BinOp.MATCHES_GLOB:
            return 1 if fnmatch.fnmatchcase(left, right) else 0
        case BinOp.MATCHES_REGEX:
            pat = _compile_regex(right)
            if pat is None:
                return None
            return 1 if pat.search(left) else 0
        case BinOp.IN:
            elements = _split_tcl_list(right)
            return 1 if left in elements else 0
        case BinOp.NI:
            elements = _split_tcl_list(right)
            return 0 if left in elements else 1
        case _:
            return None


# Unary operators


def _eval_unary(
    op: UnaryOp,
    operand: ExprNode,
    env: dict[str, int | float | str],
) -> TclValue | None:
    val = _eval(operand, env)
    if val is None:
        return None
    try:
        match op:
            case UnaryOp.NEG:
                return -val
            case UnaryOp.POS:
                return +val
            case UnaryOp.NOT | UnaryOp.WORD_NOT:
                return 0 if val else 1
            case UnaryOp.BIT_NOT:
                if not isinstance(val, int):
                    return None
                return ~val
            case _:
                return None
    except Exception:
        log.debug("expr_eval: unary operation failed", exc_info=True)
        return None


# Math functions


def _eval_call(
    func: str,
    args: tuple[ExprNode, ...],
    env: dict[str, int | float | str],
) -> TclValue | None:
    name = func.lower()

    # Skip non-deterministic functions
    if name in ("rand", "srand"):
        return None

    # Evaluate arguments
    vals: list[TclValue] = []
    for arg in args:
        v = _eval(arg, env)
        if v is None:
            return None
        vals.append(v)

    try:
        return _dispatch_func(name, vals)
    except Exception:
        log.debug("expr_eval: function dispatch failed", exc_info=True)
        return None


def _dispatch_func(name: str, vals: list[TclValue]) -> TclValue | None:
    handler = _MATH_FUNCS.get(name)
    if handler is not None:
        return handler(vals)
    return None


# Individual function implementations


def _f_abs(vals: list[TclValue]) -> TclValue | None:
    if len(vals) != 1:
        return None
    return abs(vals[0])  # Python abs preserves int/float type


def _f_int(vals: list[TclValue]) -> TclValue | None:
    if len(vals) != 1:
        return None
    v = vals[0]
    if isinstance(v, int):
        return v
    return int(v)  # truncates toward zero, same as Tcl


def _f_double(vals: list[TclValue]) -> TclValue | None:
    if len(vals) != 1:
        return None
    return float(vals[0])


def _f_bool(vals: list[TclValue]) -> TclValue | None:
    if len(vals) != 1:
        return None
    return 1 if vals[0] else 0


def _f_round(vals: list[TclValue]) -> TclValue | None:
    """Tcl round: ties away from zero (not Python's ties-to-even)."""
    if len(vals) != 1:
        return None
    v = vals[0]
    if isinstance(v, int):
        return v
    if v >= 0:
        return int(math.floor(v + 0.5))
    return int(math.ceil(v - 0.5))


def _f_ceil(vals: list[TclValue]) -> TclValue | None:
    if len(vals) != 1:
        return None
    return float(math.ceil(vals[0]))  # Tcl ceil returns double


def _f_floor(vals: list[TclValue]) -> TclValue | None:
    if len(vals) != 1:
        return None
    return float(math.floor(vals[0]))  # Tcl floor returns double


def _f_min(vals: list[TclValue]) -> TclValue | None:
    if not vals:
        return None
    return min(vals)


def _f_max(vals: list[TclValue]) -> TclValue | None:
    if not vals:
        return None
    return max(vals)


def _f_isqrt(vals: list[TclValue]) -> TclValue | None:
    if len(vals) != 1:
        return None
    v = vals[0]
    if not isinstance(v, int) or v < 0:
        return None
    return math.isqrt(v)


def _f_isinf(vals: list[TclValue]) -> TclValue | None:
    if len(vals) != 1:
        return None
    return 1 if isinstance(vals[0], float) and math.isinf(vals[0]) else 0


def _f_isnan(vals: list[TclValue]) -> TclValue | None:
    if len(vals) != 1:
        return None
    return 1 if isinstance(vals[0], float) and math.isnan(vals[0]) else 0


def _f_isfinite(vals: list[TclValue]) -> TclValue | None:
    if len(vals) != 1:
        return None
    v = vals[0]
    if isinstance(v, int):
        return 1
    return 1 if math.isfinite(v) else 0


def _f_unary_float(fn):
    """Factory for single-argument float-returning math functions."""

    def wrapper(vals: list[TclValue]) -> TclValue | None:
        if len(vals) != 1:
            return None
        return fn(float(vals[0]))

    return wrapper


def _f_binary_float(fn):
    """Factory for two-argument float-returning math functions."""

    def wrapper(vals: list[TclValue]) -> TclValue | None:
        if len(vals) != 2:
            return None
        return fn(float(vals[0]), float(vals[1]))

    return wrapper


_MathFunc = Callable[[list[TclValue]], TclValue | None]

_MATH_FUNCS: dict[str, _MathFunc] = {
    # Type conversion
    "abs": _f_abs,
    "int": _f_int,
    "entier": _f_int,
    "wide": _f_int,
    "double": _f_double,
    "bool": _f_bool,
    # Rounding
    "round": _f_round,
    "ceil": _f_ceil,
    "floor": _f_floor,
    # Variadic
    "min": _f_min,
    "max": _f_max,
    # Integer
    "isqrt": _f_isqrt,
    # Classification
    "isinf": _f_isinf,
    "isnan": _f_isnan,
    "isfinite": _f_isfinite,
    # Unary float functions
    "sqrt": _f_unary_float(math.sqrt),
    "exp": _f_unary_float(math.exp),
    "log": _f_unary_float(math.log),
    "log10": _f_unary_float(math.log10),
    "sin": _f_unary_float(math.sin),
    "cos": _f_unary_float(math.cos),
    "tan": _f_unary_float(math.tan),
    "asin": _f_unary_float(math.asin),
    "acos": _f_unary_float(math.acos),
    "atan": _f_unary_float(math.atan),
    "sinh": _f_unary_float(math.sinh),
    "cosh": _f_unary_float(math.cosh),
    "tanh": _f_unary_float(math.tanh),
    # Binary float functions
    "atan2": _f_binary_float(math.atan2),
    "hypot": _f_binary_float(math.hypot),
    "fmod": _f_binary_float(math.fmod),
    "pow": _f_binary_float(math.pow),
}
