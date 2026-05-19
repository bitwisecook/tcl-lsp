"""bind -- Arrange for event bindings on windows or tags."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tk man page bind.n"


@register
class BindCommand(CommandDef):
    name = "bind"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="bind",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Arrange for X event bindings on windows or tags.",
                synopsis=(
                    "bind tag",
                    "bind tag sequence",
                    "bind tag sequence script",
                    "bind tag sequence +script",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="bind tag ?sequence? ?+??command?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 3),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
