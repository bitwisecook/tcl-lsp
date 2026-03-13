# Scaffolded from unload.n -- refine and commit
"""unload -- Unload machine code."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page unload.n"


@register
class UnloadCommand(CommandDef):
    name = "unload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="unload",
            hover=HoverSnippet(
                summary="Unload machine code",
                synopsis=(
                    "unload ?switches? fileName",
                    "unload ?switches? fileName prefix",
                    "unload ?switches? fileName prefix interp",
                ),
                snippet="This command tries to unload shared libraries previously loaded with load from the application's address space.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="unload ?switches? fileName",
                    options=(
                        OptionSpec(
                            name="-nocomplain",
                            detail="Suppresses all error messages.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-keeplibrary",
                            detail="This switch will prevent unload from issuing the operating system call that will unload the library from the process.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="--", detail="Marks the end of switches.", takes_value=False
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
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
