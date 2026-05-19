"""lower -- Lower a window's position in the stacking order."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tk man page lower.n"


@register
class LowerCommand(CommandDef):
    name = "lower"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lower",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Lower a window's position in the stacking order.",
                synopsis=("lower window ?belowThis?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lower window ?belowThis?",
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
