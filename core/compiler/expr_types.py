"""Expression-level type inference for Tcl ``[expr]`` AST nodes.

Given an ``ExprNode`` and a map from variable names to their current
``TypeLattice`` values, ``infer_expr_type`` walks the tree and applies
Tcl's operator typing rules to determine the result type.

Tcl typing rules (simplified):

- Arithmetic (``+``, ``-``, ``*``, ``%``, ``**``):
  INT if both sides INT; DOUBLE if either side DOUBLE; else NUMERIC.
- Division (``/``): INT if both INT (Tcl integer division); DOUBLE if
  either is DOUBLE.
- Bitwise / shift (``&``, ``|``, ``^``, ``~``, ``<<``, ``>>``): INT.
- Comparison (``==``, ``!=``, ``<``, ``>``, ``<=``, ``>=``,
  ``eq``, ``ne``, ``lt``, ``le``, ``gt``, ``ge``, ``in``, ``ni``): BOOLEAN.
- Logical (``&&``, ``||``, ``!``): BOOLEAN.
- Unary ``-``, ``+``: same as operand.
- Ternary ``? :``: ``type_join`` of both branches.
- Function calls: per-function table (see ``_FUNC_RETURN_TYPE``).
- Variable reference: look up in ``var_types``.
- Literal: parse text → INT / DOUBLE / BOOLEAN.
- Command substitution: OVERDEFINED.
- ``ExprRaw``: NUMERIC (safe fallback, matches old behaviour).
"""

from __future__ import annotations

from typing import TypeAlias

from .expr_ast import (
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
from .expr_registry import BINOP_KIND, EXPR_FUNC_REGISTRY, UNARYOP_KIND, OpKind, UnaryOpKind
from .types import TclType, TypeKind, TypeLattice, type_join

# Semantic type alias

VarTypes: TypeAlias = dict[str, TypeLattice]
"""Maps variable names to their current type lattice values."""


# Literal type detection


def _literal_type_from_text(text: str) -> TypeLattice:
    """Infer the type of a literal value from its text."""
    stripped = text.strip()
    low = stripped.lower()

    # Boolean
    from .tcl_constants import TCL_BOOL_LITERALS

    if low in TCL_BOOL_LITERALS:
        return TypeLattice.of(TclType.BOOLEAN)

    # Integer (decimal, hex, octal, binary)
    if stripped.startswith(("0x", "0X", "0o", "0O", "0b", "0B")):
        return TypeLattice.of(TclType.INT)
    try:
        int(stripped)
        return TypeLattice.of(TclType.INT)
    except (ValueError, OverflowError):
        pass

    # Double
    try:
        float(stripped)
        return TypeLattice.of(TclType.DOUBLE)
    except (ValueError, OverflowError):
        pass

    # Unrecognised literal — treat as NUMERIC (safe fallback).
    return TypeLattice.of(TclType.NUMERIC)


# Arithmetic result type helper


def _arithmetic_result(left_type: TypeLattice, right_type: TypeLattice) -> TypeLattice:
    """Compute the result type of an arithmetic operation.

    INT op INT → INT
    DOUBLE op anything / anything op DOUBLE → DOUBLE
    Otherwise → NUMERIC.
    """
    lt = left_type.tcl_type
    rt = right_type.tcl_type

    if lt is TclType.INT and rt is TclType.INT:
        return TypeLattice.of(TclType.INT)
    if lt is TclType.INT and rt is TclType.BOOLEAN:
        return TypeLattice.of(TclType.INT)
    if lt is TclType.BOOLEAN and rt is TclType.INT:
        return TypeLattice.of(TclType.INT)
    if lt is TclType.BOOLEAN and rt is TclType.BOOLEAN:
        return TypeLattice.of(TclType.INT)
    if lt is TclType.DOUBLE or rt is TclType.DOUBLE:
        return TypeLattice.of(TclType.DOUBLE)
    return TypeLattice.of(TclType.NUMERIC)


# Main inference function


def infer_expr_type(node: ExprNode, var_types: VarTypes) -> TypeLattice:
    """Infer the result type of a Tcl expression AST node.

    Parameters
    ----------
    node
        The expression AST node to analyse.
    var_types
        Maps normalised variable names to their current ``TypeLattice``.
        This should come from the SSA type propagation pass.

    Returns
    -------
    TypeLattice
        The inferred result type.
    """
    match node:
        # Atoms
        case ExprLiteral(text=text):
            return _literal_type_from_text(text)

        case ExprString():
            return TypeLattice.of(TclType.STRING)

        case ExprVar(name=name):
            return var_types.get(name, TypeLattice.unknown())

        case ExprCommand():
            return TypeLattice.overdefined()

        case ExprRaw():
            # Fallback — matches old behaviour.
            return TypeLattice.of(TclType.NUMERIC)

        # Binary operators
        case ExprBinary(op=op, left=left, right=right):
            kind = BINOP_KIND.get(op)
            if kind is OpKind.COMPARISON or kind is OpKind.LOGICAL:
                return TypeLattice.of(TclType.BOOLEAN)
            if kind is OpKind.BITWISE:
                return TypeLattice.of(TclType.INT)

            left_t = infer_expr_type(left, var_types)
            right_t = infer_expr_type(right, var_types)

            if left_t.kind is not TypeKind.KNOWN or right_t.kind is not TypeKind.KNOWN:
                return TypeLattice.of(TclType.NUMERIC)

            if kind is OpKind.ARITHMETIC or kind is OpKind.DIVISION:
                return _arithmetic_result(left_t, right_t)

            return TypeLattice.of(TclType.NUMERIC)

        # Unary operators
        case ExprUnary(op=op, operand=operand):
            ukind = UNARYOP_KIND.get(op)
            if ukind is UnaryOpKind.LOGICAL:
                return TypeLattice.of(TclType.BOOLEAN)
            if ukind is UnaryOpKind.BITWISE:
                return TypeLattice.of(TclType.INT)
            # IDENTITY: same as operand.
            return infer_expr_type(operand, var_types)

        # Ternary
        case ExprTernary(true_branch=tb, false_branch=fb):
            return type_join(
                infer_expr_type(tb, var_types),
                infer_expr_type(fb, var_types),
            )

        # Function calls
        case ExprCall(function=func, args=args):
            fspec = EXPR_FUNC_REGISTRY.get(func)
            if fspec is not None:
                if fspec.identity and args:
                    return infer_expr_type(args[0], var_types)
                if fspec.variadic_join and args:
                    result = infer_expr_type(args[0], var_types)
                    for arg in args[1:]:
                        result = type_join(result, infer_expr_type(arg, var_types))
                    return result
                return TypeLattice.of(fspec.return_type)

            # Unknown function — conservative.
            return TypeLattice.of(TclType.NUMERIC)

        case _:
            return TypeLattice.of(TclType.NUMERIC)
