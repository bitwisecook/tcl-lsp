# Scaffolded from variable.n -- refine and commit
"""variable -- create and initialize a namespace variable."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page variable.n"


@register
class VariableCommand(CommandDef):
    name = "variable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="variable",
            hover=HoverSnippet(
                summary="create and initialize a namespace variable",
                synopsis=(
                    "variable name",
                    "variable ?name value...?",
                ),
                snippet="This command is normally used within a namespace eval command to create one or more variables within a namespace.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="variable name",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            arg_roles={0: ArgRole.VAR_NAME},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
