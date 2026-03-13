# Scaffolded from linsert.n -- refine and commit
"""linsert -- Insert elements into a list."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page linsert.n"


@register
class LinsertCommand(CommandDef):
    name = "linsert"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="linsert",
            hover=HoverSnippet(
                summary="Insert elements into a list",
                synopsis=("linsert list index ?element element ...?",),
                snippet="This command produces a new list from list by inserting all of the element arguments just before the index'th element of list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="linsert list index ?element element ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
