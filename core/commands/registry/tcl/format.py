# Scaffolded from format.n -- refine and commit
"""format -- Format a string in the style of sprintf."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page format.n"


@register
class FormatCommand(CommandDef):
    name = "format"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="format",
            hover=HoverSnippet(
                summary="Format a string in the style of sprintf",
                synopsis=("format formatString ?arg arg ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="format formatString ?arg arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
