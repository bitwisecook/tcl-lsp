# Scaffolded from lreplace.n -- refine and commit
"""lreplace -- Replace elements in a list with new elements."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page lreplace.n"


@register
class LreplaceCommand(CommandDef):
    name = "lreplace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lreplace",
            hover=HoverSnippet(
                summary="Replace elements in a list with new elements",
                synopsis=("lreplace list first last ?element element ...?",),
                snippet="lreplace returns a new list formed by replacing zero or more elements of list with the element arguments.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lreplace list first last ?element element ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3),
            ),
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
