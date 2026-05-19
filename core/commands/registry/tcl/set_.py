"""set -- Read or write a variable value."""

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl set(1)"


def _set_arg_role_resolver(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Resolve variable role based on the shape of ``set``.

    ``set varName``       -- read: returns the value (VAR_READ)
    ``set varName value`` -- write: assigns and returns (VAR_WRITE)
    """
    if len(args) >= 2:
        return {0: frozenset({ArgRole.VAR_WRITE})}
    return {0: frozenset({ArgRole.VAR_READ})}


@register
class SetCommand(CommandDef):
    name = "set"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="set",
            hover=HoverSnippet(
                summary="Read or write a variable value.",
                synopsis=("set varName ?newValue?",),
                snippet="With one argument, returns the value. With two, assigns and returns the new value.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="set varName ?newValue?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            assigns_variable_at=0,
            arg_role_resolver=_set_arg_role_resolver,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
