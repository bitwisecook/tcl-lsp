# Scaffolded from continue.n -- refine and commit
"""continue -- Skip to the next iteration of a loop."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page continue.n"


@register
class ContinueCommand(CommandDef):
    name = "continue"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="continue",
            needs_start_cmd=True,
            hover=HoverSnippet(
                summary="Skip to the next iteration of a loop",
                synopsis=("continue",),
                snippet="This command is typically invoked inside the body of a looping command such as for or foreach or while.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="continue",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 0),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
