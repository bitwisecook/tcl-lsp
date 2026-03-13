"""Lowering hooks for variable-related commands: set, incr, append, lappend, unset, global, variable, upvar."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ...common.naming import normalise_var_name as _normalise_var_name
from ...parsing.substitution import backslash_subst as _tcl_backsubst
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRCall,
    IRIncr,
)
from ..token_helpers import parse_decimal_int as _parse_decimal_int

if TYPE_CHECKING:
    from ..lowering import _Command


def _parse_expr(expr_text: str):  # noqa: ANN202
    """Parse an expr argument lazily (avoids circular import at module level)."""
    from ...parsing.expr_parser import parse_expr as _std_parse_expr

    return _std_parse_expr(expr_text)


def _expr_arg_from_expr_command(text: str) -> str | None:
    """Extract the expr argument from a [expr {...}] command substitution."""
    from ...parsing.command_shapes import extract_single_expr_argument

    return extract_single_expr_argument(text)


def lower_set(lowerer: object, cmd: _Command) -> object | None:
    """Lower ``set`` to IRAssignConst/IRAssignExpr/IRAssignValue."""
    from ...parsing.tokens import TokenType

    args = cmd.args
    arg_tokens = cmd.arg_tokens
    arg_single = cmd.arg_single_token

    if not args:
        return IRCall(range=cmd.range, command=cmd.name, args=tuple(args), tokens=cmd.cmd_tokens)
    name = args[0]
    if arg_tokens and arg_single[0] and arg_tokens[0].type is TokenType.ESC and "\\" in name:
        name = _tcl_backsubst(name)
    if len(args) < 2 or len(args) > 2:
        return IRCall(range=cmd.range, command=cmd.name, args=tuple(args), tokens=cmd.cmd_tokens)
    value = args[1]
    value_needs_backsubst = False
    if len(arg_tokens) > 1 and len(arg_single) > 1 and arg_single[1]:
        tok = arg_tokens[1]
        if tok.type is TokenType.STR:
            return IRAssignConst(range=cmd.range, name=name, value=value)
        const_value = _parse_decimal_int(value) if tok.type is TokenType.ESC else None
        if const_value is not None:
            return IRAssignConst(range=cmd.range, name=name, value=const_value)
        if tok.type is TokenType.CMD:
            expr_arg = _expr_arg_from_expr_command(tok.text)
            if expr_arg is not None:
                return IRAssignExpr(range=cmd.range, name=name, expr=_parse_expr(expr_arg))
        if tok.type is TokenType.ESC and "\\" in value:
            value_needs_backsubst = True
    return IRAssignValue(
        range=cmd.range,
        name=name,
        value=value,
        value_needs_backsubst=value_needs_backsubst,
    )


def lower_incr(lowerer: object, cmd: _Command) -> object | None:
    """Lower ``incr`` to IRIncr."""
    args = cmd.args
    if not args or len(args) > 2:
        return IRCall(range=cmd.range, command=cmd.name, args=tuple(args), tokens=cmd.cmd_tokens)
    name = args[0]
    amount = args[1] if len(args) > 1 else None
    return IRIncr(range=cmd.range, name=name, amount=amount)


def lower_append_lappend(lowerer: object, cmd: _Command) -> object | None:
    """Lower ``append``/``lappend`` to IRCall with defs."""
    args = cmd.args
    if not args:
        return None  # fall through to default
    name = _normalise_var_name(args[0])
    return IRCall(
        range=cmd.range,
        command=cmd.name,
        args=tuple(args),
        defs=(name,),
        reads_own_defs=True,
        tokens=cmd.cmd_tokens,
    )


def lower_unset(lowerer: object, cmd: _Command) -> object | None:
    """Lower ``unset`` to IRCall with defs."""
    args = cmd.args
    i = 0
    nocomplain = False
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "-nocomplain":
            nocomplain = True
        if args[i] == "--":
            i += 1
            break
        i += 1
    var_names = tuple(_normalise_var_name(a) for a in args[i:])
    return IRCall(
        range=cmd.range,
        command=cmd.name,
        args=tuple(args),
        defs=var_names,
        reads_own_defs=not nocomplain,
        tokens=cmd.cmd_tokens,
    )


def lower_global(lowerer: object, cmd: _Command) -> object | None:
    """Lower ``global`` to IRCall with defs."""
    args = cmd.args
    if not args:
        return None  # fall through to default
    var_names = tuple(_normalise_var_name(a) for a in args)
    return IRCall(
        range=cmd.range,
        command=cmd.name,
        args=tuple(args),
        defs=var_names,
        tokens=cmd.cmd_tokens,
    )


def lower_variable(lowerer: object, cmd: _Command) -> object | None:
    """Lower ``variable`` to IRCall with defs."""
    args = cmd.args
    var_names = tuple(_normalise_var_name(args[i]) for i in range(0, len(args), 2))
    return IRCall(
        range=cmd.range,
        command=cmd.name,
        args=tuple(args),
        defs=var_names,
        tokens=cmd.cmd_tokens,
    )


def lower_upvar(lowerer: object, cmd: _Command) -> object | None:
    """Lower ``upvar`` to IRCall with defs."""
    args = cmd.args
    if len(args) < 2:
        return None  # fall through to default
    start = 1 if args[0].lstrip("-").isdigit() or args[0].startswith("#") else 0
    my_vars = tuple(_normalise_var_name(args[i]) for i in range(start + 1, len(args), 2))
    return IRCall(
        range=cmd.range,
        command=cmd.name,
        args=tuple(args),
        defs=my_vars,
        tokens=cmd.cmd_tokens,
    )


def register() -> None:
    from core.commands.registry import REGISTRY

    REGISTRY.register_lowering("set", lower_set)
    REGISTRY.register_lowering("incr", lower_incr)
    REGISTRY.register_lowering("append", lower_append_lappend)
    REGISTRY.register_lowering("lappend", lower_append_lappend)
    REGISTRY.register_lowering("unset", lower_unset)
    REGISTRY.register_lowering("global", lower_global)
    REGISTRY.register_lowering("variable", lower_variable)
    REGISTRY.register_lowering("upvar", lower_upvar)
