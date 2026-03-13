# Scaffolded from flush.n -- refine and commit
"""flush -- Flush buffered output for a channel."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page flush.n"


@register
class FlushCommand(CommandDef):
    name = "flush"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="flush",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Flush buffered output for a channel",
                synopsis=("flush channel",),
                snippet="The flush command has been superceded by the chan flush command which supports the same syntax and options.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="flush channel",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
