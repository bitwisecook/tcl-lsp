# Scaffolded from incr.n -- refine and commit
"""incr -- Increment the value of a variable."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page incr.n"


@register
class IncrCommand(CommandDef):
    name = "incr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="incr",
            hover=HoverSnippet(
                summary="Increment the value of a variable",
                synopsis=("incr varName ?increment?",),
                snippet="Increments the value stored in the variable whose name is varName.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="incr varName ?increment?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            assigns_variable_at=0,
            arg_roles={0: ArgRole.VAR_NAME},
            return_type=TclType.INT,
            arg_types={
                0: ArgTypeHint(expected=TclType.INT, shimmers=True),
                1: ArgTypeHint(expected=TclType.INT),
            },
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
