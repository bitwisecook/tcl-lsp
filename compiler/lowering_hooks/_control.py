"""Lowering hooks for expr and return."""

# canonicalisation: audited #246

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol

from shared.alias import CommandAliasMap

from ..ir import (
    IRBarrier,
    IRExprEval,
    IRReturn,
)

if TYPE_CHECKING:
    from ..lowering import _Command


class _LowererLike(Protocol):
    """Minimal protocol for the lowerer interface used by hooks."""

    _command_aliases: CommandAliasMap

    def canonicalise_command(self, cmd_name: str) -> str: ...


def lower_expr(lowerer: _LowererLike, cmd: _Command) -> object | None:
    """Lower ``expr`` with a single braced argument to IRExprEval."""
    from compiler.parsing.expr_parser import parse_expr as _parse_expr

    args = cmd.args
    arg_single = cmd.arg_single_token
    # {*} expansion means the argument is not really ``args[0]`` — fall
    # through to the default IRCall lowering so codegen sees the
    # expansion and arity checks operate on the token list.
    if cmd.expand_word is not None and any(cmd.expand_word):
        return None
    if len(args) != 1 or not arg_single or not arg_single[0]:
        return None  # fall through to default
    return IRExprEval(range=cmd.range, expr=_parse_expr(args[0]))


def lower_return(lowerer: _LowererLike, cmd: _Command) -> object | None:
    """Lower ``return`` to IRReturn or IRBarrier for options."""
    from compiler.parsing.command_shapes import extract_single_expr_argument
    from compiler.parsing.expr_parser import parse_expr as _parse_expr
    from shared.alias import expr_alias_names as _expr_alias_names
    from shared.tokens import TokenType

    args = cmd.args
    # {*} expansion makes the argument list unknown at compile time —
    # emit a barrier so downstream passes do not treat any particular
    # arg slot as the return value.
    if cmd.expand_word is not None and any(cmd.expand_word):
        return IRBarrier(
            range=cmd.range,
            reason="return with expansion",
            command=cmd.name,
            canonical_command=lowerer.canonicalise_command(cmd.name),
            args=tuple(args),
            tokens=cmd.cmd_tokens,
        )
    if args and args[0].startswith("-"):
        return IRBarrier(
            range=cmd.range,
            reason="return with options",
            command=cmd.name,
            canonical_command=lowerer.canonicalise_command(cmd.name),
            args=tuple(args),
            tokens=cmd.cmd_tokens,
        )
    value = args[0] if args else None
    expr = None
    braced = False
    if value is not None and cmd.arg_single_token and cmd.arg_single_token[0]:
        tok = cmd.arg_tokens[0]
        if tok.type is TokenType.STR:
            braced = True
        elif tok.type is TokenType.CMD:
            aliases: CommandAliasMap = getattr(lowerer, "_command_aliases", {})
            alias_names = _expr_alias_names(aliases)
            expr_arg = extract_single_expr_argument(tok.text, expr_aliases=alias_names or None)
            if expr_arg is not None:
                expr = _parse_expr(expr_arg)
    return IRReturn(range=cmd.range, value=value, expr=expr, braced=braced)


def register() -> None:
    from compiler.registry import REGISTRY

    REGISTRY.register_lowering("expr", lower_expr)
    REGISTRY.register_lowering("return", lower_return)
