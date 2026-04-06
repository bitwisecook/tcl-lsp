"""lassign -- Assign list elements to variables."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page lassign.n"


@register
class LassignCommand(CommandDef):
    name = "lassign"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lassign",
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
            arg_roles={
                1: ArgRole.VAR_NAME,
                2: ArgRole.VAR_NAME,
                3: ArgRole.VAR_NAME,
                4: ArgRole.VAR_NAME,
                5: ArgRole.VAR_NAME,
                6: ArgRole.VAR_NAME,
                7: ArgRole.VAR_NAME,
                8: ArgRole.VAR_NAME,
                9: ArgRole.VAR_NAME,
                10: ArgRole.VAR_NAME,
                11: ArgRole.VAR_NAME,
                12: ArgRole.VAR_NAME,
                13: ArgRole.VAR_NAME,
                14: ArgRole.VAR_NAME,
                15: ArgRole.VAR_NAME,
                16: ArgRole.VAR_NAME,
                17: ArgRole.VAR_NAME,
                18: ArgRole.VAR_NAME,
                19: ArgRole.VAR_NAME,
            },
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
