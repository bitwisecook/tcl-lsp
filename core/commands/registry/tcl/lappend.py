"""lappend -- Append list elements onto a variable."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page lappend.n"


@register
class LappendCommand(CommandDef):
    name = "lappend"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lappend",
            hover=HoverSnippet(
                summary="Append list elements onto a variable",
                synopsis=("lappend varName ?value value value ...?",),
                snippet="This command treats the variable given by varName as a list and appends each of the value arguments to that list as a separate element, with spaces between elements.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lappend varName ?value value value ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            assigns_variable_at=0,
            arg_roles={0: ArgRole.VAR_NAME},
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
