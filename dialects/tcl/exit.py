# Scaffolded from exit.n -- refine and commit
"""exit -- End the application."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tcl man page exit.n"


@register
class ExitCommand(CommandDef):
    name = "exit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exit",
            byte_compiled=True,
            terminates_block=True,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="End the application",
                synopsis=("exit ?returnCode?",),
                snippet="Terminate the process, returning returnCode to the system as the exit status.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="exit ?returnCode?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
