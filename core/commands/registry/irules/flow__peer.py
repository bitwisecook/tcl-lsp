# Enriched from F5 iRules reference documentation.
"""FLOW::peer -- Returns the TCL flow handle for the peer flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FLOW__peer.html"


@register
class FlowPeerCommand(CommandDef):
    name = "FLOW::peer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FLOW::peer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the TCL flow handle for the peer flow.",
                synopsis=("FLOW::peer ANY_CHARS",),
                snippet="Returns the TCL flow handle for the peer flow.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    # Get server side flow handle.\n"
                    "    set cf [FLOW::this]\n"
                    "\n"
                    "    # Get client side flow handle.\n"
                    "    set peer [FLOW::peer $cf]\n"
                    '    log local0. "Peer flow is $peer"\n'
                    "    unset cf peer\n"
                    "}"
                ),
                return_value="TCL handle for the peer flow. On error an exception is thrown with a message indicating the cause of failure.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FLOW::peer ANY_CHARS",
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
