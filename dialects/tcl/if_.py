"""if -- Conditional execution with optional elseif/else branches."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl if(1)"


_EXPR = frozenset({ArgRole.EXPR})
_BODY = frozenset({ArgRole.BODY})


def _if_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Resolve BODY and EXPR roles for if/elseif/else chains."""
    roles: dict[int, frozenset[ArgRole]] = {}
    i = 0
    if i < len(args):
        roles[i] = _EXPR
        i += 1
    if i < len(args) and args[i] == "then":
        i += 1
    if i < len(args):
        roles[i] = _BODY
        i += 1
    while i < len(args):
        kw = args[i]
        if kw == "elseif":
            i += 1
            if i < len(args):
                roles[i] = _EXPR
                i += 1
            if i < len(args) and args[i] == "then":
                i += 1
            if i < len(args):
                roles[i] = _BODY
                i += 1
            continue
        if kw == "else":
            if i + 1 < len(args):
                roles[i + 1] = _BODY
            break
        # Implicit else: a trailing word after the last body (no keyword).
        if i == len(args) - 1:
            roles[i] = _BODY
            break
        i += 1
    return roles


@register
class IfCommand(CommandDef):
    name = "if"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="if",
            byte_compiled=True,
            not_proc_factory=True,
            is_control_flow=True,
            is_language_keyword=True,
            has_boolean_condition=True,
            never_inline_body=True,
            hover=HoverSnippet(
                summary="Conditional execution with optional elseif/else branches.",
                synopsis=("if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?",),
                snippet="Expressions are evaluated left-to-right until a true branch is selected.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            arg_role_resolver=_if_arg_roles,
            return_type=TclType.STRING,
            arg_types={0: ArgTypeHint(expected=TclType.BOOLEAN, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
