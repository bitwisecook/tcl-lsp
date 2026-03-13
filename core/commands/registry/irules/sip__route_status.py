# Enriched from F5 iRules reference documentation.
"""SIP::route_status -- Returns the routing status of the current message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__route_status.html"


@register
class SipRouteStatusCommand(CommandDef):
    name = "SIP::route_status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::route_status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the routing status of the current message.",
                synopsis=("SIP::route_status",),
                snippet=(
                    "The SIP::route_status command returns the routing status of the current\n"
                    "message. Valid status are:\n"
                    '  * "unprocessed"\n'
                    '  * "route found"\n'
                    '  * "no route found"\n'
                    '  * "dropped"\n'
                    '  * "queue full"\n'
                    '  * "no connection"\n'
                    '  * "connection closing"\n'
                    '  * "internal error"\n'
                    "\n"
                    '"route found" is based on the SIP RouteTable finding a route. It is not\n'
                    "effected by the proxy’s ability to create a connection, so even if the\n"
                    "server is not listening on the specified address or marked down, it\n"
                    'might still return status as "route found" if the RouteTable is able to\n'
                    "find the route."
                ),
                source=_SOURCE,
                examples=("when SIP_RESPONSE_SEND {\n  log local0. [SIP::route_status]\n}"),
                return_value="Returns routing status of the current message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::route_status",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
