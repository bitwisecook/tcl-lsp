# Enriched from F5 iRules reference documentation.
"""FLOW::this -- Returns the TCL handle for the current flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FLOW__this.html"


@register
class FlowThisCommand(CommandDef):
    name = "FLOW::this"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FLOW::this",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the TCL handle for the current flow.",
                synopsis=("FLOW::this",),
                snippet="Returns the TCL handle for the current flow.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set cf [FLOW::this]\n"
                    '    log local0. "Current flow is $cf"\n'
                    "    unset cf\n"
                    "}"
                ),
                return_value="TCL handle for the current flow. On error an exception is thrown with a message indicating the cause of failure.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FLOW::this",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FLOW"}),
                also_in=frozenset(
                    {
                        "CLIENT_ACCEPTED",
                        "CLIENT_DATA",
                        "LB_SELECTED",
                        "SA_PICKED",
                        "SERVER_CONNECTED",
                        "SERVER_DATA",
                    }
                ),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FLOW_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
