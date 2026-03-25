# Scaffolded from scan.n -- refine and commit
"""scan -- Parse string using conversion specifiers in the style of sscanf."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page scan.n"


@register
class ScanCommand(CommandDef):
    name = "scan"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="scan",
            hover=HoverSnippet(
                summary="Parse string using conversion specifiers in the style of sscanf",
                synopsis=("scan string format ?varName varName ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="scan string format ?varName varName ...?",
                ),
            ),
            arg_roles={
                2: ArgRole.VAR_NAME,
                3: ArgRole.VAR_NAME,
                4: ArgRole.VAR_NAME,
                5: ArgRole.VAR_NAME,
                6: ArgRole.VAR_NAME,
                7: ArgRole.VAR_NAME,
                8: ArgRole.VAR_NAME,
                9: ArgRole.VAR_NAME,
                10: ArgRole.VAR_NAME,
                11: ArgRole.VAR_NAME,
                12: ArgRole.VAR_NAME,
                13: ArgRole.VAR_NAME,
                14: ArgRole.VAR_NAME,
                15: ArgRole.VAR_NAME,
                16: ArgRole.VAR_NAME,
                17: ArgRole.VAR_NAME,
                18: ArgRole.VAR_NAME,
                19: ArgRole.VAR_NAME,
            },
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
