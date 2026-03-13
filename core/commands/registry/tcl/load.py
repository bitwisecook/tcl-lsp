# Scaffolded from load.n -- refine and commit
"""load -- Load machine code and initialize new commands."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page load.n"


@register
class LoadCommand(CommandDef):
    name = "load"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="load",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Load machine code and initialize new commands",
                synopsis=(
                    "load ?-global? ?-lazy? ?--? fileName",
                    "load ?-global? ?-lazy? ?--? fileName prefix",
                    "load ?-global? ?-lazy? ?--? fileName prefix interp",
                ),
                snippet="This command loads binary code from a file into the application's address space and calls an initialization procedure in the library to incorporate it into an interpreter.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="load ?-global? ?-lazy? ?--? fileName",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 3),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
