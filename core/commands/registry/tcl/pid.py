# Scaffolded from pid.n -- refine and commit
"""pid -- Retrieve process identifiers."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page pid.n"


@register
class PidCommand(CommandDef):
    name = "pid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pid",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Retrieve process identifiers",
                synopsis=("pid ?fileId?",),
                snippet="If the fileId argument is given then it should normally refer to a process pipeline created with the open command.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="pid ?fileId?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
