# Scaffolded from timerate.n -- refine and commit
"""timerate -- Measure the rate of execution of a script."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page timerate.n"


@register
class TimeRateCommand(CommandDef):
    name = "timerate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="timerate",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Measure the rate of execution of a script",
                synopsis=(
                    "timerate ?-direct? ?-calibrate? ?-overhead double? command ?time ?max-count??",
                ),
                snippet=(
                    "Repeatedly evaluates command for the given time (in "
                    "milliseconds, default 1000) or until max-count "
                    "iterations, and returns a summary of the measured rate."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis=(
                        "timerate ?-direct? ?-calibrate? ?-overhead double? "
                        "command ?time ?max-count??"
                    ),
                ),
            ),
            validation=ValidationSpec(
                # At least the command; the option/positional mix makes
                # the upper bound effectively unbounded.
                arity=Arity(1),
            ),
            # The body's position varies with the leading options, so we
            # don't pin a fixed BODY arg index; mark the command as one
            # that evaluates code with the usual unknown side effects.
            arg_roles={0: frozenset({ArgRole.BODY})},
            arg_types={1: ArgTypeHint(expected=TclType.INT, shimmers=True)},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
