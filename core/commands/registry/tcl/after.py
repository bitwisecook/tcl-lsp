# Scaffolded from after.n -- refine and commit
"""after -- Execute a command after a time delay."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page after.n"


_av = make_av(_SOURCE)


@register
class AfterCommand(CommandDef):
    name = "after"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="after",
            hover=HoverSnippet(
                summary="Execute a command after a time delay",
                synopsis=(
                    "after ms",
                    "after ms ?script script script ...?",
                    "after cancel id",
                    "after cancel script script script ...",
                ),
                snippet="This command is used to delay execution of the program or to execute a command in background sometime in the future.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="after ms",
                    arg_values={
                        0: (
                            _av(
                                "cancel",
                                "Cancels the execution of a delayed command that was previously scheduled.",
                                "after cancel id",
                            ),
                            _av(
                                "idle",
                                "Concatenates the script arguments together with space separators (just as in the concat command), and arranges for the resulting script to be evaluated later as an idle callback.",
                                "after idle script ?script script ...?",
                            ),
                            _av(
                                "info",
                                "This command returns information about existing event handlers.",
                                "after info ?id?",
                            ),
                        )
                    },
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
