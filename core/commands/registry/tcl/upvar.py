# Scaffolded from upvar.n -- refine and commit
"""upvar -- Create link to variable in a different stack frame."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page upvar.n"


@register
class UpvarCommand(CommandDef):
    name = "upvar"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="upvar",
            creates_dynamic_barrier=True,
            hover=HoverSnippet(
                summary="Create link to variable in a different stack frame",
                synopsis=("upvar ?level? otherVar myVar ?otherVar myVar ...?",),
                snippet="This command arranges for one or more local variables in the current procedure to refer to variables in an enclosing procedure call or to global variables.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="upvar ?level? otherVar myVar ?otherVar myVar ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            xc_translatable=False,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
