"""Lowering hooks for expr and return."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..ir import (
    IRBarrier,
    IRExprEval,
    IRReturn,
)

if TYPE_CHECKING:
    from ..lowering import _Command


def lower_expr(lowerer: object, cmd: _Command) -> object | None:
    """Lower ``expr`` with a single braced argument to IRExprEval."""
    from ...parsing.expr_parser import parse_expr as _parse_expr

    args = cmd.args
    arg_single = cmd.arg_single_token
    if len(args) != 1 or not arg_single or not arg_single[0]:
        return None  # fall through to default
    return IRExprEval(range=cmd.range, expr=_parse_expr(args[0]))


def lower_return(lowerer: object, cmd: _Command) -> object | None:
    """Lower ``return`` to IRReturn or IRBarrier for options."""
    args = cmd.args
    if args and args[0].startswith("-"):
        return IRBarrier(
            range=cmd.range,
            reason="return with options",
            command=cmd.name,
            args=tuple(args),
            tokens=cmd.cmd_tokens,
        )
    value = args[0] if args else None
    return IRReturn(range=cmd.range, value=value)


def register() -> None:
    from core.commands.registry import REGISTRY

    REGISTRY.register_lowering("expr", lower_expr)
    REGISTRY.register_lowering("return", lower_return)
