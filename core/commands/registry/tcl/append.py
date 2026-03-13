# Scaffolded from append.n -- refine and commit
"""append -- Append to variable."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page append.n"


@register
class AppendCommand(CommandDef):
    name = "append"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="append",
            hover=HoverSnippet(
                summary="Append to variable",
                synopsis=("append varName ?value value value ...?",),
                snippet="Append all of the value arguments to the current value of variable varName.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="append varName ?value value value ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            assigns_variable_at=0,
            arg_roles={0: ArgRole.VAR_NAME},
            return_type=TclType.STRING,
            arg_types={0: ArgTypeHint(expected=TclType.STRING, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
