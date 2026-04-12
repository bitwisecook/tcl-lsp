# Enriched from F5 iRules reference documentation.
"""proc -- Define an iRule proc."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import ArgRole, Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/proc.html"


@register
class ProcCommand(CommandDef):
    name = "proc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="proc",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Define an iRule proc.",
                synopsis=("proc NAME ARGUMENT_N_DEFAULT PROC_SCRIPT",),
                snippet=(
                    "Define an iRule proc which is called by iRule command call.\n"
                    "\n"
                    "The syntax is same as basic TCL proc command."
                ),
                source=_SOURCE,
                examples=('when CLIENT_DATA {\n    call logme "Coming to CLIENT_DATA"\n}'),
                return_value="Returns the value in the return command, if any, in the proc script.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="proc NAME ARGUMENT_N_DEFAULT PROC_SCRIPT",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3, 3),
            ),
            event_requires=EventRequires(),
            arg_roles={0: ArgRole.NAME, 1: ArgRole.PARAM_LIST, 2: ArgRole.BODY},
            defines_procedure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PROC_DEFINITION,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
