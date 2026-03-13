# Scaffolded from fcopy.n -- refine and commit
"""fcopy -- Copy data from one channel to another."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page fcopy.n"


@register
class FcopyCommand(CommandDef):
    name = "fcopy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="fcopy",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Copy data from one channel to another",
                synopsis=("fcopy inputChan outputChan ?-size size? ?-command callback?",),
                snippet="The fcopy command copies data from one I/O channel, inchan, to another I/O channel, outchan.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="fcopy inputChan outputChan ?-size size? ?-command callback?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
