# Scaffolded from vwait.n -- refine and commit
"""vwait -- Process events until a variable is written."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page vwait.n"


@register
class VwaitCommand(CommandDef):
    name = "vwait"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="vwait",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Process events until a variable is written",
                synopsis=(
                    "vwait varName",
                    "vwait ?options? ?varName ...?",
                ),
                snippet="This command enters the Tcl event loop to process events, blocking the application if no events are ready.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="vwait varName",
                    options=(
                        OptionSpec(
                            name="--", detail="Marks the end of options.", takes_value=False
                        ),
                        OptionSpec(
                            name="-all",
                            detail="All conditions for the wait operation must be met to complete the wait operation.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-extended",
                            detail="An extended result in list form is returned, see below for explanation.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-nofileevents",
                            detail="File events are not handled in the wait operation.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-noidleevents",
                            detail="Idle handlers are not invoked during the wait operation.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-notimerevents",
                            detail="Timer handlers are not serviced during the wait operation.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-nowindowevents",
                            detail="Events of the windowing system are not handled during the wait operation.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-readable",
                            detail="Channel must name a Tcl channel open for reading.",
                            takes_value=True,
                        ),
                        OptionSpec(
                            name="-timeout",
                            detail="The wait operation is constrained to milliseconds.",
                            takes_value=True,
                        ),
                        OptionSpec(
                            name="-variable",
                            detail="VarName must be the name of a global variable.",
                            takes_value=True,
                        ),
                        OptionSpec(
                            name="-writable",
                            detail="Channel must name a Tcl channel open for writing.",
                            takes_value=True,
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            arg_roles={0: ArgRole.VAR_READ},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
