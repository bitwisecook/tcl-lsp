# Scaffolded from global.n -- refine and commit
"""global -- Access global variables."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page global.n"


@register
class GlobalCommand(CommandDef):
    name = "global"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="global",
            hover=HoverSnippet(
                summary="Access global variables",
                synopsis=("global ?varname ...?",),
                snippet="This command has no effect unless executed in the context of a proc body.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="global ?varname ...?",
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
