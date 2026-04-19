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
    OptionSpec,
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
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="unset ?-nocomplain? ?--? ?name name name ...?",
                    options=(
                        OptionSpec(
                            name="-nocomplain", detail="Suppress errors for non-existent variables."
                        ),
                        OptionSpec(name="--", detail="End of options."),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            assigns_variable_at=0,
            destroys_variable=True,
            arg_roles={0: ArgRole.VAR_WRITE},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
