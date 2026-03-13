"""trace -- Monitor variable accesses, command usages and command executions."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page trace.n"


@register
class TraceCommand(CommandDef):
    name = "trace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="trace",
            creates_dynamic_barrier=True,
            hover=HoverSnippet(
                summary="Monitor variable accesses, command usages and command executions",
                synopsis=("trace option ?arg arg ...?",),
                snippet="Arranges for commands to be executed whenever certain operations "
                "are invoked.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="trace option ?arg arg ...?",
                ),
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(2),
                ),
                "info": SubCommand(
                    name="info",
                    arity=Arity(1, 2),
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(2),
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
