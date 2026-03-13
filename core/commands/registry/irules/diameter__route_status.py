# Enriched from F5 iRules reference documentation.
"""DIAMETER::route_status -- Returns the routing status of the current message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__route_status.html"


@register
class DiameterRouteStatusCommand(CommandDef):
    name = "DIAMETER::route_status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::route_status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the routing status of the current message.",
                synopsis=("DIAMETER::route_status",),
                snippet=(
                    "The DIAMETER::route_status command returns the routing status of the current\n"
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
                    '"route found" is based on the DIAMETER RouteTable finding a route. It\n'
                    "is not affected by the proxy’s ability to create a connection, so even\n"
                    "if the server is not listening on the specified address or marked\n"
                    'down, it still returns status as "route found" if the RouteTable is\n'
                    "able to find the route."
                ),
                source=_SOURCE,
                return_value="Returns routing status of the current message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::route_status",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
