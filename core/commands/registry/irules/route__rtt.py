# Enriched from F5 iRules reference documentation.
"""ROUTE::rtt -- Returns the cached round-trip-time estimate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__rtt.html"


@register
class RouteRttCommand(CommandDef):
    name = "ROUTE::rtt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::rtt",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the cached round-trip-time estimate.",
                synopsis=("ROUTE::rtt DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "Returns the cached round-trip-time for the destination and/or\n"
                    "gateway if the relevant TCP profile enables cmetrics-cache.\n"
                    "\n"
                    "The return value only applies to the TMM executing the command. It\n"
                    "does not consider cache entries on other TMMs.\n"
                    "\n"
                    "ROUTE::rtt returns a value of 0 when there are no statistics available.\n"
                    "\n"
                    "NOTE: The returned value is scaled to units of 100ns; to express it\n"
                    "in the same units as TCP::rtt multiply it by 32/10000.\n"
                    "\n"
                    "NOTE: When used with the fastL4 profile, RTT from client/server\n"
                    "needs to be enabled and the client and server need to be using TCP\n"
                    "timestamps."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    log local0. "Cached rtt is: [ROUTE::rtt [IP::remote_addr]]"\n'
                    "}"
                ),
                return_value="RTT in units of 100ns.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::rtt DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
