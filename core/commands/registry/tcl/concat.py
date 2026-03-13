# Scaffolded from concat.n -- refine and commit
"""concat -- Join lists together."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page concat.n"


@register
class ConcatCommand(CommandDef):
    name = "concat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="concat",
            hover=HoverSnippet(
                summary="Join lists together",
                synopsis=("concat ?arg arg ...?",),
                snippet="This command joins each of its arguments together with spaces after trimming leading and trailing white-space from each of them.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="concat ?arg arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            pure=True,
            return_type=TclType.LIST,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
