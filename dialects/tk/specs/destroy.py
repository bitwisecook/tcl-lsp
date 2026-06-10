"""destroy -- Destroy one or more windows."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tk man page destroy.n"


@register
class DestroyCommand(CommandDef):
    name = "destroy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="destroy",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Destroy one or more windows and all their descendants.",
                synopsis=("destroy ?window window ...?",),
                snippet=(
                    "Destroys the specified windows and all of their "
                    'descendants. If the main window (".") is destroyed '
                    "the entire application is terminated."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="destroy ?window window ...?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
