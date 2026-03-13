# Scaffolded from fblocked.n -- refine and commit
"""fblocked -- Test whether the last input operation exhausted all available input."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page fblocked.n"


@register
class FblockedCommand(CommandDef):
    name = "fblocked"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="fblocked",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Test whether the last input operation exhausted all available input",
                synopsis=("fblocked channel",),
                snippet="The fblocked command has been superceded by the chan blocked command which supports the same syntax and options.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="fblocked channel",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            return_type=TclType.BOOLEAN,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
