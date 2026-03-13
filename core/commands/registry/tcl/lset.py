# Scaffolded from lset.n -- refine and commit
"""lset -- Change an element in a list."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page lset.n"


@register
class LsetCommand(CommandDef):
    name = "lset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lset",
            hover=HoverSnippet(
                summary="Change an element in a list",
                synopsis=("lset varName ?index ...? newValue",),
                snippet="The lset command accepts a parameter, varName, which it interprets as the name of a variable containing a Tcl list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lset varName ?index ...? newValue",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            arg_roles={0: ArgRole.VAR_NAME},
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
