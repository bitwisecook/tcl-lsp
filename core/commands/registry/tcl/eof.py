# Scaffolded from eof.n -- refine and commit
"""eof -- Check for end of file condition on channel."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page eof.n"


@register
class EofCommand(CommandDef):
    name = "eof"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="eof",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Check for end of file condition on channel",
                synopsis=("eof channel",),
                snippet="The eof command has been superceded by the chan eof command which supports the same syntax and options.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="eof channel",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            return_type=TclType.BOOLEAN,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
