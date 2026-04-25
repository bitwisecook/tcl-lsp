"""raise -- Raise a window's position in the stacking order."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page raise.n"


@register
class RaiseCommand(CommandDef):
    name = "raise"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="raise",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Raise a window's position in the stacking order.",
                synopsis=("raise window ?aboveThis?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="raise window ?aboveThis?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
