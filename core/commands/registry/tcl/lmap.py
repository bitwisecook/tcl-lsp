"""lmap -- Iterate over all elements in one or more lists and collect results."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page lmap.n"


@register
class LmapCommand(CommandDef):
    name = "lmap"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lmap",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Iterate over all elements in one or more lists and collect results",
                synopsis=(
                    "lmap varname list body",
                    "lmap varlist1 list1 ?varlist2 list2 ...? body",
                ),
                snippet="The lmap command implements a loop where the loop variable(s) take on values from one or more lists, and the loop returns a list of results collected from each iteration.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lmap varname list body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3),
            ),
            return_type=TclType.LIST,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
