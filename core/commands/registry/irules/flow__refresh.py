# Enriched from F5 iRules reference documentation.
"""FLOW::refresh -- Refresh the flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FLOW__refresh.html"


@register
class FlowRefreshCommand(CommandDef):
    name = "FLOW::refresh"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FLOW::refresh",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Refresh the flow.",
                synopsis=("FLOW::refresh ANY_CHARS",),
                snippet="Updates the last used time on the flow to now.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_DATA {\n"
                    "        # Log and refresh the related flow whenever the client sends data.\n"
                    '        log local0. "Flow idle duration before refresh [FLOW::idle_duration $result]"\n'
                    "        FLOW::refresh $result\n"
                    '        log local0. "Flow idle duration after refresh [FLOW::idle_duration $result]"\n'
                    "        TCP::release\n"
                    "        TCP::collect\n"
                    "\n"
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FLOW::refresh ANY_CHARS",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
