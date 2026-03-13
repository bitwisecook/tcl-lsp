# Scaffolded from catch.n -- refine and commit
"""catch -- Evaluate script and trap exceptional returns."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page catch.n"


@register
class CatchCommand(CommandDef):
    name = "catch"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="catch",
            is_control_flow=True,
            hover=HoverSnippet(
                summary="Evaluate script and trap exceptional returns",
                synopsis=("catch script ?resultVarName? ?optionsVarName?",),
                snippet="The catch command may be used to prevent errors from aborting command interpretation.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="catch script ?resultVarName? ?optionsVarName?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 3),
            ),
            arg_roles={0: ArgRole.BODY, 1: ArgRole.VAR_NAME, 2: ArgRole.VAR_NAME},
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
