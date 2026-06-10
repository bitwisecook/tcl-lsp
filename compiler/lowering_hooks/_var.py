"""Lowering hooks for variable-related commands: set, incr, append, lappend, unset, global, variable, upvar."""

# canonicalisation: audited #246

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol

from shared.alias import CommandAliasMap
from shared.alias import expr_alias_names as _expr_alias_names
from shared.naming import normalise_var_name as _normalise_var_name
from shared.tcl_subst import backslash_subst as _tcl_backsubst

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


class _LowererLike(Protocol):
    """Minimal protocol for the lowerer interface used by hooks."""

    _command_aliases: CommandAliasMap

    def canonicalise_command(self, cmd_name: str) -> str: ...


def _parse_expr(expr_text: str):  # noqa: ANN202
    """Parse an expr argument lazily (avoids circular import at module level)."""
    from compiler.parsing.expr_parser import parse_expr as _std_parse_expr

    return _std_parse_expr(expr_text)


def _expr_arg_from_expr_command(
    text: str,
    *,
    expr_aliases: frozenset[str] | None = None,
) -> str | None:
    """Extract the expr argument from a [expr {...}] command substitution."""
    from compiler.parsing.command_shapes import extract_single_expr_argument

    return extract_single_expr_argument(text, expr_aliases=expr_aliases)


def lower_set(lowerer: _LowererLike, cmd: _Command) -> object | None:
    """Lower ``set`` to IRAssignConst/IRAssignExpr/IRAssignValue."""
    from shared.tokens import TokenType

    args = cmd.args
    arg_tokens = cmd.arg_tokens
    arg_single = cmd.arg_single_token

    # {*} argument expansion makes the runtime arg count indeterminate,
    # so the ``set x value`` specialised lowering is unsafe — fall
    # through to a generic IRCall so arity checks and the expanded
    # codegen path see the full token list.
    if cmd.expand_word is not None and any(cmd.expand_word):
        return IRCall(
            range=cmd.range,
            command=cmd.name,
            canonical_command=lowerer.canonicalise_command(cmd.name),
            args=tuple(args),
            tokens=cmd.cmd_tokens,
        )
    if not args:
        return IRCall(
            range=cmd.range,
            command=cmd.name,
            canonical_command=lowerer.canonicalise_command(cmd.name),
            args=tuple(args),
            tokens=cmd.cmd_tokens,
        )
    name = args[0]
    if arg_tokens and arg_single[0] and arg_tokens[0].type is TokenType.ESC and "\\" in name:
        name = _tcl_backsubst(name)
    if len(args) < 2 or len(args) > 2:
        return IRCall(
            range=cmd.range,
            command=cmd.name,
            canonical_command=lowerer.canonicalise_command(cmd.name),
            args=tuple(args),
            tokens=cmd.cmd_tokens,
        )
    value = args[1]
    value_needs_backsubst = False
    if len(arg_tokens) > 1 and len(arg_single) > 1 and arg_single[1]:
        tok = arg_tokens[1]
        if tok.type is TokenType.STR:
            return IRAssignConst(range=cmd.range, name=name, value=value)
        const_value = _parse_decimal_int(value) if tok.type is TokenType.ESC else None
        # Only fold to the parsed numeric form when the source
        # spelling already matches the canonical decimal form.
        # ``set arg 0005`` must store ``0005`` verbatim (Tcl
        # preserves the source string repr — the value only
        # shimmers to int 5 when used as an integer); folding
        # eagerly here destroys the leading zeros and breaks
        # ``puts $arg``, ``string length $arg``, and
        # ``expr "0005"`` which all care about the original
        # spelling.
        if const_value is not None and const_value != value.strip():
            const_value = None
        if const_value is not None:
            return IRAssignConst(range=cmd.range, name=name, value=const_value)
        if tok.type is TokenType.CMD:
            aliases = _expr_alias_names(lowerer._command_aliases)
            expr_arg = _expr_arg_from_expr_command(tok.text, expr_aliases=aliases or None)
            if expr_arg is not None:
                return IRAssignExpr(range=cmd.range, name=name, expr=_parse_expr(expr_arg))
        if tok.type is TokenType.ESC and "\\" in value:
            value_needs_backsubst = True
    return IRAssignValue(
        range=cmd.range,
        name=name,
        value=value,
        value_needs_backsubst=value_needs_backsubst,
        tokens=cmd.cmd_tokens,
    )


def lower_incr(lowerer: _LowererLike, cmd: _Command) -> object | None:
    """Lower ``incr`` to IRIncr."""
    args = cmd.args
    # {*} expansion hides the real arg count — keep as a generic call
    # so the specialised IRIncr path isn't given wrong indices.
    if cmd.expand_word is not None and any(cmd.expand_word):
        return IRCall(
            range=cmd.range,
            command=cmd.name,
            canonical_command=lowerer.canonicalise_command(cmd.name),
            args=tuple(args),
            tokens=cmd.cmd_tokens,
        )
    if not args or len(args) > 2:
        return IRCall(
            range=cmd.range,
            command=cmd.name,
            canonical_command=lowerer.canonicalise_command(cmd.name),
            args=tuple(args),
            tokens=cmd.cmd_tokens,
        )
    name = args[0]
    amount = args[1] if len(args) > 1 else None
    return IRIncr(
        range=cmd.range,
        name=name,
        amount=amount,
        safe_on_uninit=_check_safe_on_uninit(cmd.name),
    )


def _check_safe_on_uninit(command: str, subcommand: str | None = None) -> bool:
    """Query the registry for the ``safe_on_uninit`` trait."""
    from compiler.registry import REGISTRY
    from compiler.registry.dialect import active_dialect

    return REGISTRY.is_safe_on_uninit(command, subcommand, active_dialect())


def lower_append_lappend(lowerer: _LowererLike, cmd: _Command) -> object | None:
    """Lower ``append``/``lappend`` to IRCall with defs."""
    args = cmd.args
    if not args:
        return None  # fall through to default
    name = _normalise_var_name(args[0])
    return IRCall(
        range=cmd.range,
        command=cmd.name,
        canonical_command=lowerer.canonicalise_command(cmd.name),
        args=tuple(args),
        defs=(name,),
        reads_own_defs=True,
        safe_on_uninit=_check_safe_on_uninit(cmd.name),
        tokens=cmd.cmd_tokens,
    )


def lower_unset(lowerer: _LowererLike, cmd: _Command) -> object | None:
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
        canonical_command=lowerer.canonicalise_command(cmd.name),
        args=tuple(args),
        defs=var_names,
        reads_own_defs=not nocomplain,
        tokens=cmd.cmd_tokens,
    )


def lower_global(lowerer: _LowererLike, cmd: _Command) -> object | None:
    """Lower ``global`` to IRCall with defs."""
    args = cmd.args
    if not args:
        return None  # fall through to default
    var_names = tuple(_normalise_var_name(a) for a in args)
    return IRCall(
        range=cmd.range,
        command=cmd.name,
        canonical_command=lowerer.canonicalise_command(cmd.name),
        args=tuple(args),
        defs=var_names,
        tokens=cmd.cmd_tokens,
    )


def lower_variable(lowerer: _LowererLike, cmd: _Command) -> object | None:
    """Lower ``variable`` to IRCall with defs."""
    args = cmd.args
    var_names = tuple(_normalise_var_name(args[i]) for i in range(0, len(args), 2))
    return IRCall(
        range=cmd.range,
        command=cmd.name,
        canonical_command=lowerer.canonicalise_command(cmd.name),
        args=tuple(args),
        defs=var_names,
        tokens=cmd.cmd_tokens,
    )


def lower_upvar(lowerer: _LowererLike, cmd: _Command) -> object | None:
    """Lower ``upvar`` (and ``namespace upvar``) to IRCall with defs.

    Uses the shared :func:`compiler.var_scoping.upvar_local_declaration_indices`
    helper which knows the full grammar for *both* commands -- ``upvar
    ?level? otherVar myVar ...?`` AND ``namespace upvar ns otherVar myVar
    ...?`` (or the lowered ``namespace`` + ``upvar`` subcommand form).
    Centralising the local-alias index logic there means there's a
    single source of truth for the LSP declaration provider
    (server/features/declaration.py), the memory-SSA aliasing
    (compiler/memory_ssa.py), the Place bridge
    (compiler/place_bridge.py), and this lowering -- no per-command
    duplicate parsing.

    When registered on ``namespace``, this hook fires for *every*
    namespace subcommand; only the ``upvar`` subcommand has any locals
    to record.  Return ``None`` for other subcommands so the lowering
    falls through to the generic ``namespace`` path.
    """
    from compiler.var_scoping import upvar_local_declaration_indices

    args = cmd.args
    if len(args) < 2:
        return None
    # Normalise the command name to the bare form so the shared helper
    # (which dispatches on the literal command string) sees ``upvar`` /
    # ``namespace`` even when the source spelled the call qualified
    # (``::upvar`` / ``::namespace``).
    cmd_name = cmd.name.lstrip(":") if cmd.name else cmd.name
    if cmd_name == "namespace" and (not args or args[0] != "upvar"):
        return None
    indices = upvar_local_declaration_indices(cmd_name, list(args))
    my_vars = tuple(_normalise_var_name(args[i]) for i in indices if 0 <= i < len(args))
    return IRCall(
        range=cmd.range,
        command=cmd.name,
        canonical_command=lowerer.canonicalise_command(cmd.name),
        args=tuple(args),
        defs=my_vars,
        tokens=cmd.cmd_tokens,
    )


def register() -> None:
    from compiler.registry import REGISTRY

    REGISTRY.register_lowering("set", lower_set)
    REGISTRY.register_lowering("incr", lower_incr)
    REGISTRY.register_lowering("append", lower_append_lappend)
    REGISTRY.register_lowering("lappend", lower_append_lappend)
    REGISTRY.register_lowering("unset", lower_unset)
    REGISTRY.register_lowering("global", lower_global)
    REGISTRY.register_lowering("variable", lower_variable)
    REGISTRY.register_lowering("upvar", lower_upvar)
    # ``namespace upvar`` shares the alias-pair grammar with bare
    # ``upvar``; route it through the same hook so its local alias is
    # recorded in ``IRCall.defs``.  ``lower_upvar`` returns ``None`` for
    # non-``upvar`` namespace subcommands so other subcommands continue
    # to fall through to the generic ``namespace`` lowering.
    REGISTRY.register_lowering("namespace", lower_upvar)
