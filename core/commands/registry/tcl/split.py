# Scaffolded from split.n -- refine and commit
"""split -- Split a string into a proper Tcl list."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page split.n"


@register
class SplitCommand(CommandDef):
    name = "split"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="split",
            hover=HoverSnippet(
                summary="Split a string into a proper Tcl list",
                synopsis=("split string ?splitChars?",),
                snippet="Returns a list created by splitting string at each character that is in the splitChars argument.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="split string ?splitChars?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.LIST,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
