# Scaffolded from lindex.n -- refine and commit
"""lindex -- Retrieve an element from a list."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page lindex.n"


@register
class LindexCommand(CommandDef):
    name = "lindex"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lindex",
            hover=HoverSnippet(
                summary="Retrieve an element from a list",
                synopsis=("lindex list ?index ...?",),
                snippet="The lindex command accepts a parameter, list, which it treats as a Tcl list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lindex list ?index ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
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
