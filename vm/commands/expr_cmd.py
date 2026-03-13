"""The ``expr`` command — expression evaluation."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_expr(interp: TclInterp, args: list[str]) -> TclResult:
    """expr arg ?arg ...?

    Concatenates all arguments and evaluates the result as a Tcl expression.
    """
    if not args:
        raise TclError('wrong # args: should be "expr arg ?arg ...?"')
    expr_str = " ".join(args)
    result = interp.eval_expr(expr_str)
    return TclResult(value=result)


def register() -> None:
    """Register the expr command."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("expr", _cmd_expr)
