# Scaffolded from error.n -- refine and commit
"""error -- Generate an error."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page error.n"


@register
class ErrorCommand(CommandDef):
    name = "error"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="error",
            needs_start_cmd=True,
            hover=HoverSnippet(
                summary="Generate an error",
                synopsis=("error message ?info? ?code?",),
                snippet="Returns a TCL_ERROR code, which causes command interpretation to be unwound.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="error message ?info? ?code?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 3),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
