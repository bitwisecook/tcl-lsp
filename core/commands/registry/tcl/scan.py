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
                2: frozenset({ArgRole.VAR_WRITE}),
                3: frozenset({ArgRole.VAR_WRITE}),
                4: frozenset({ArgRole.VAR_WRITE}),
                5: frozenset({ArgRole.VAR_WRITE}),
                6: frozenset({ArgRole.VAR_WRITE}),
                7: frozenset({ArgRole.VAR_WRITE}),
                8: frozenset({ArgRole.VAR_WRITE}),
                9: frozenset({ArgRole.VAR_WRITE}),
                10: frozenset({ArgRole.VAR_WRITE}),
                11: frozenset({ArgRole.VAR_WRITE}),
                12: frozenset({ArgRole.VAR_WRITE}),
                13: frozenset({ArgRole.VAR_WRITE}),
                14: frozenset({ArgRole.VAR_WRITE}),
                15: frozenset({ArgRole.VAR_WRITE}),
                16: frozenset({ArgRole.VAR_WRITE}),
                17: frozenset({ArgRole.VAR_WRITE}),
                18: frozenset({ArgRole.VAR_WRITE}),
                19: frozenset({ArgRole.VAR_WRITE}),
            },
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
