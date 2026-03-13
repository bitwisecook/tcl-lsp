# Scaffolded from library.n -- refine and commit
"""parray -- standard library of Tcl procedures."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page library.n"


@register
class ParrayCommand(CommandDef):
    name = "parray"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="parray",
            hover=HoverSnippet(
                summary="standard library of Tcl procedures",
                synopsis=("parray arrayName ?pattern?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="parray arrayName ?pattern?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            arg_roles={0: ArgRole.VAR_READ},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
