# Enriched from F5 iRules reference documentation.
"""ROUTE::mtu -- Returns the cached MTU entry."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__mtu.html"


@register
class RouteMtuCommand(CommandDef):
    name = "ROUTE::mtu"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::mtu",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the cached MTU entry.",
                synopsis=("ROUTE::mtu DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "Returns the cached MTU entry for the provided destination and/or gateway.\n"
                    "\n"
                    "Unlike other ROUTE::commands, this value is valid across all TMMs."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set mtu [ROUTE::mtu [IP::remote_addr]]\n"
                    "    if { $mtu > 0 && $mtu < 300 } {\n"
                    "        #Ignore extremely small cached MTUs\n"
                    "        ROUTE::clear [IP::remote_addr]\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::mtu DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
