"""lassign -- Assign list elements to variables."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page lassign.n"


def _lassign_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """D4-F2 closure: ``lassign list ?varName ...?`` -- every arg after
    the list (index >= 1) is a varName.  Dynamic resolver supports any
    number of trailing varName args (the previous hard-coded 18 slots
    falsely flagged W210 on lassigns with more)."""
    return {i: frozenset({ArgRole.VAR_WRITE}) for i in range(1, len(args))}


@register
class LassignCommand(CommandDef):
    name = "lassign"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lassign",
            frameless_runtime=True,
            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Assign list elements to variables",
                synopsis=("lassign list ?varName ...?",),
                snippet="This command treats the value list as a list and assigns successive elements from that list to the variables given by the varName arguments in order.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lassign list ?varName ...?",
                ),
            ),
            arg_role_resolver=_lassign_arg_roles,
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
