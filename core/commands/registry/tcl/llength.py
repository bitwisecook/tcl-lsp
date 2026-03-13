# Scaffolded from llength.n -- refine and commit
"""llength -- Count the number of elements in a list."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page llength.n"


@register
class LlengthCommand(CommandDef):
    name = "llength"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="llength",
            hover=HoverSnippet(
                summary="Count the number of elements in a list",
                synopsis=("llength list",),
                snippet="Treats list as a list and returns a decimal string giving the number of elements in it.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="llength list",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.INT,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
