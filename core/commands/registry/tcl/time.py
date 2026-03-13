# Scaffolded from time.n -- refine and commit
"""time -- Time the execution of a script."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page time.n"


@register
class TimeCommand(CommandDef):
    name = "time"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="time",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Time the execution of a script",
                synopsis=("time script ?count?",),
                snippet="This command will call the Tcl interpreter count times to evaluate script (or once if count is not specified).",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="time script ?count?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            arg_roles={0: ArgRole.BODY},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
