# Scaffolded from join.n -- refine and commit
"""join -- Create a string by joining together list elements."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page join.n"


@register
class JoinCommand(CommandDef):
    name = "join"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="join",
            hover=HoverSnippet(
                summary="Create a string by joining together list elements",
                synopsis=("join list ?joinString?",),
                snippet="The list argument must be a valid Tcl list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="join list ?joinString?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.STRING,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
