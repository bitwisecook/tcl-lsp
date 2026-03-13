"""lrepeat -- Build a list by repeating elements."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page lrepeat.n"


@register
class LrepeatCommand(CommandDef):
    name = "lrepeat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lrepeat",
            hover=HoverSnippet(
                summary="Build a list by repeating elements",
                synopsis=("lrepeat count ?element ...?",),
                snippet="The lrepeat command creates a list of size count * number of elements by repeating count times the sequence of elements element ....",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lrepeat count ?element ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            pure=True,
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.INT)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
