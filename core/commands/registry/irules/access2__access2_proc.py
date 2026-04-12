# Generated from F5 iRules reference documentation -- do not edit manually.
"""ACCESS2::access2_proc -- This command is used to get the TCL procedure registered for currently executing per-request policy expression."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS2__access2_proc.html"


@register
class Access2Access2ProcCommand(CommandDef):
    name = "ACCESS2::access2_proc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS2::access2_proc",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used to get the TCL procedure registered for currently executing per-request policy expression.",
                synopsis=("ACCESS2::access2_proc",),
                snippet="This command will return the TCL procedure registered for currently executing per-request policy expression.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS2::access2_proc",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
