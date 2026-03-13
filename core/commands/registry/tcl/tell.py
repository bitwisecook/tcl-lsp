# Scaffolded from tell.n -- refine and commit
"""tell -- Return current access position for an open channel."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page tell.n"


@register
class TellCommand(CommandDef):
    name = "tell"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tell",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Return current access position for an open channel",
                synopsis=("tell channel",),
                snippet="The tell command has been superceded by the chan tell command which supports the same syntax and options.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tell channel",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
