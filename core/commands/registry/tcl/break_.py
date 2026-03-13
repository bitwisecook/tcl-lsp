# Scaffolded from break.n -- refine and commit
"""break -- Abort looping command."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page break.n"


@register
class BreakCommand(CommandDef):
    name = "break"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="break",
            needs_start_cmd=True,
            hover=HoverSnippet(
                summary="Abort looping command",
                synopsis=("break",),
                snippet="This command is typically invoked inside the body of a looping command such as for or foreach or while.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="break",
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
