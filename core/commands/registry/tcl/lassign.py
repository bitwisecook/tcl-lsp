"""lassign -- Assign list elements to variables."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page lassign.n"


@register
class LassignCommand(CommandDef):
    name = "lassign"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lassign",
            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Assign list elements to variables",
                synopsis=("lassign list ?varName ...?",),
                snippet="This command treats the value list as a list and assigns successive elements from that list to the variables given by the varName arguments in order.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lassign list ?varName ...?",
                ),
            ),
            arg_roles={
                1: frozenset({ArgRole.VAR_WRITE}),
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
                arity=Arity(1),
            ),
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
