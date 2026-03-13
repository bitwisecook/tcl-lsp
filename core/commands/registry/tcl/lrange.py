# Scaffolded from lrange.n -- refine and commit
"""lrange -- Return one or more adjacent elements from a list."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page lrange.n"


@register
class LrangeCommand(CommandDef):
    name = "lrange"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lrange",
            hover=HoverSnippet(
                summary="Return one or more adjacent elements from a list",
                synopsis=("lrange list first last",),
                snippet="List must be a valid Tcl list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lrange list first last",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3, 3),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
