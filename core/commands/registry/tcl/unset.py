# Scaffolded from unset.n -- refine and commit
"""unset -- Delete variables."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionTerminatorSpec,
    ValidationSpec,
)
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page unset.n"


@register
class UnsetCommand(CommandDef):
    name = "unset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="unset",
            hover=HoverSnippet(
                summary="Delete variables",
                synopsis=("unset ?-nocomplain? ?--? ?name name name ...?",),
                snippet="This command removes one or more variables.",
                source=_SOURCE,
            ),
            option_terminator_profiles=(
                OptionTerminatorSpec(
                    scan_start=0,
                    options_with_values=frozenset(),
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="unset ?-nocomplain? ?--? ?name name name ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            arg_roles={0: ArgRole.VAR_NAME},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
