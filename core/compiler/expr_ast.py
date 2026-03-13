"""Structured AST for Tcl [expr] expressions.

Replaces the opaque ``str`` representation used in ``IRAssignExpr.expr``,
``IRIfClause.condition``, ``IRFor.condition``, and ``CFGBranch.condition``.
Parsed once at lowering time via :func:`core.parsing.expr_parser.parse_expr`,
then walked by downstream analyses (SSA, SCCP, type inference, shimmer).

The ``ExprRaw`` node is a fallback for any expression the parser cannot
handle — every consumer must treat it as "give up, use the string".
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import TypeAlias

# Semantic type aliases

ExprOffset: TypeAlias = int
"""Character offset within expression source text."""


# Operator enums


class BinOp(Enum):
    """Binary operators in Tcl expressions."""

    # Arithmetic
    ADD = "+"
    SUB = "-"
    MUL = "*"
    DIV = "/"
    MOD = "%"
    POW = "**"
    # Shift
    LSHIFT = "<<"
    RSHIFT = ">>"
    # Bitwise
    BIT_AND = "&"
    BIT_OR = "|"
    BIT_XOR = "^"
    # Logical
    AND = "&&"
    OR = "||"
    # Numeric comparison
    EQ = "=="
    NE = "!="
    LT = "<"
    LE = "<="
    GT = ">"
    GE = ">="
    # String comparison
    STR_EQ = "eq"
    STR_NE = "ne"
    STR_LT = "lt"
    STR_LE = "le"
    STR_GT = "gt"
    STR_GE = "ge"
    # List membership
    IN = "in"
    NI = "ni"
    # iRules word-based logical operators
    WORD_AND = "and"
    WORD_OR = "or"
    # iRules string comparison operators
    CONTAINS = "contains"
    STARTS_WITH = "starts_with"
    ENDS_WITH = "ends_with"
    STR_EQUALS = "equals"
    MATCHES_GLOB = "matches_glob"
    MATCHES_REGEX = "matches_regex"


class UnaryOp(Enum):
    """Unary operators in Tcl expressions."""

    NEG = "-"
    POS = "+"
    BIT_NOT = "~"
    NOT = "!"
    WORD_NOT = "not"


# AST nodes


@dataclass(frozen=True, slots=True)
class ExprLiteral:
    """Integer, float, or boolean literal."""

    text: str
    start: ExprOffset
    end: ExprOffset


@dataclass(frozen=True, slots=True)
class ExprString:
    """Quoted string literal (``"..."`` or ``{...}``)."""

    text: str
    start: ExprOffset
    end: ExprOffset


@dataclass(frozen=True, slots=True)
class ExprVar:
    """Variable reference (``$var``, ``${var}``, ``$arr(idx)``)."""

    text: str  # full text including ``$``
    name: str  # normalised base name
    start: ExprOffset
    end: ExprOffset


@dataclass(frozen=True, slots=True)
class ExprCommand:
    """Command substitution ``[cmd ...]`` — opaque boundary."""

    text: str  # full text including brackets
    start: ExprOffset
    end: ExprOffset


@dataclass(frozen=True, slots=True)
class ExprBinary:
    """Binary operator application."""

    op: BinOp
    left: ExprNode
    right: ExprNode


@dataclass(frozen=True, slots=True)
class ExprUnary:
    """Unary operator application."""

    op: UnaryOp
    operand: ExprNode


@dataclass(frozen=True, slots=True)
class ExprTernary:
    """Ternary conditional ``cond ? true_val : false_val``."""

    condition: ExprNode
    true_branch: ExprNode
    false_branch: ExprNode


@dataclass(frozen=True, slots=True)
class ExprCall:
    """Math function call: ``sin($x)``, ``int($y)``, ``max($a, $b)``."""

    function: str
    args: tuple[ExprNode, ...]
    start: ExprOffset
    end: ExprOffset


@dataclass(frozen=True, slots=True)
class ExprRaw:
    """Fallback: unparseable expression preserved as raw text.

    Every consumer must handle this as "give up" — returning the same
    result as the old string-based analysis.
    """

    text: str


# Union type for all expression nodes.
ExprNode = (
    ExprLiteral
    | ExprString
    | ExprVar
    | ExprCommand
    | ExprBinary
    | ExprUnary
    | ExprTernary
    | ExprCall
    | ExprRaw
)


# Utility functions


def vars_in_expr_node(node: ExprNode) -> set[str]:
    """Recursively extract variable names from an expression AST.

    This is the structured replacement for the 11 scattered
    ``tokenise_expr()`` → scan-for-variables patterns.
    """
    result: set[str] = set()
    _collect_vars(node, result)
    return result


def _collect_vars(node: ExprNode, out: set[str]) -> None:
    match node:
        case ExprVar(name=name):
            if name:
                out.add(name)
        case ExprBinary(left=left, right=right):
            _collect_vars(left, out)
            _collect_vars(right, out)
        case ExprUnary(operand=operand):
            _collect_vars(operand, out)
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            _collect_vars(cond, out)
            _collect_vars(tb, out)
            _collect_vars(fb, out)
        case ExprCall(args=args):
            for arg in args:
                _collect_vars(arg, out)
        case ExprCommand(text=text):
            # Command substitutions may contain variable references.
            # Delegate to the script-level variable extractor.
            _collect_vars_in_command(text, out)
        case _:
            pass  # ExprLiteral, ExprString, ExprRaw — no variables


def _collect_vars_in_command(cmd_text: str, out: set[str]) -> None:
    """Extract variables from a command substitution body.

    Imports lazily to avoid circular dependency with ssa.py.
    """
    from .ssa import _vars_in_script

    # Strip surrounding brackets if present.
    if len(cmd_text) >= 2 and cmd_text.startswith("[") and cmd_text.endswith("]"):
        cmd_text = cmd_text[1:-1]
    out.update(_vars_in_script(cmd_text))


def _needs_parens_for_unary(operand: ExprNode) -> bool:
    """Return True when a unary operator's operand needs wrapping in parens."""
    return isinstance(operand, (ExprBinary, ExprTernary))


def _needs_parens_for_binary_child(parent_op: BinOp, child: ExprNode, *, is_right: bool) -> bool:
    """Return True when a binary child needs parentheses to preserve semantics."""
    if not isinstance(child, ExprBinary):
        return False
    # Operator precedence (Tcl expr): higher number = tighter binding.
    _PRECEDENCE: dict[BinOp, int] = {
        BinOp.OR: 1,
        BinOp.WORD_OR: 1,
        BinOp.AND: 2,
        BinOp.WORD_AND: 2,
        BinOp.BIT_OR: 3,
        BinOp.BIT_XOR: 4,
        BinOp.BIT_AND: 5,
        BinOp.EQ: 6,
        BinOp.NE: 6,
        BinOp.STR_EQ: 6,
        BinOp.STR_NE: 6,
        BinOp.IN: 6,
        BinOp.NI: 6,
        BinOp.LT: 7,
        BinOp.LE: 7,
        BinOp.GT: 7,
        BinOp.GE: 7,
        BinOp.STR_LT: 7,
        BinOp.STR_LE: 7,
        BinOp.STR_GT: 7,
        BinOp.STR_GE: 7,
        BinOp.LSHIFT: 8,
        BinOp.RSHIFT: 8,
        BinOp.ADD: 9,
        BinOp.SUB: 9,
        BinOp.MUL: 10,
        BinOp.DIV: 10,
        BinOp.MOD: 10,
        BinOp.POW: 11,
        # iRules operators — treat as comparison level.
        BinOp.CONTAINS: 6,
        BinOp.STARTS_WITH: 6,
        BinOp.ENDS_WITH: 6,
        BinOp.STR_EQUALS: 6,
        BinOp.MATCHES_GLOB: 6,
        BinOp.MATCHES_REGEX: 6,
    }
    # Operators that are right-associative in Tcl expressions.
    _RIGHT_ASSOC: set[BinOp] = {BinOp.POW}

    parent_prec = _PRECEDENCE.get(parent_op, 0)
    child_prec = _PRECEDENCE.get(child.op, 0)
    if child_prec < parent_prec:
        return True
    if child_prec == parent_prec:
        if parent_op in _RIGHT_ASSOC:
            # Right-associative: parenthesise the left child, not the right.
            return not is_right
        # Left-associative (default): parenthesise the right child.
        return is_right
    return False


def render_expr(node: ExprNode) -> str:
    """Round-trip an ``ExprNode`` back to source text."""
    match node:
        case ExprLiteral(text=text):
            return text
        case ExprString(text=text):
            return text
        case ExprVar(text=text):
            return text
        case ExprCommand(text=text):
            return text
        case ExprBinary(op=op, left=left, right=right):
            left_text = render_expr(left)
            right_text = render_expr(right)
            if _needs_parens_for_binary_child(op, left, is_right=False):
                left_text = f"({left_text})"
            if _needs_parens_for_binary_child(op, right, is_right=True):
                right_text = f"({right_text})"
            return f"{left_text} {op.value} {right_text}"
        case ExprUnary(op=op, operand=operand):
            prefix = op.value
            inner = render_expr(operand)
            if _needs_parens_for_unary(operand):
                inner = f"({inner})"
            if prefix[-1:].isalnum():
                return f"{prefix} {inner}"
            return f"{prefix}{inner}"
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            return f"{render_expr(cond)} ? {render_expr(tb)} : {render_expr(fb)}"
        case ExprCall(function=func, args=args):
            arg_text = ", ".join(render_expr(a) for a in args)
            return f"{func}({arg_text})"
        case ExprRaw(text=text):
            return text
        case _:  # pragma: no cover
            return str(node)


def expr_text(node: ExprNode) -> str:
    """Get the string form of an expression.

    For ``ExprRaw`` returns the original text directly; for structured
    nodes, renders them via ``render_expr``.
    """
    if isinstance(node, ExprRaw):
        return node.text
    return render_expr(node)
