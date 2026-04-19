"""unknown -- Handle attempts to use non-existent commands."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page unknown.n"


@register
class UnknownCommand(CommandDef):
    name = "unknown"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="unknown",
            dialects=DIALECTS_EXCEPT_IRULES,
            creates_dynamic_barrier=True,
            hover=HoverSnippet(
                summary="Handle attempts to use non-existent commands",
                synopsis=("unknown cmdName ?arg arg ...?",),
                snippet="This command is invoked by the Tcl interpreter whenever a "
                "script tries to invoke a command that does not exist.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="unknown cmdName ?arg arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
