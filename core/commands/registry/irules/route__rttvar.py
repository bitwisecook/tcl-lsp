# Enriched from F5 iRules reference documentation.
"""ROUTE::rttvar -- Returns the cached round-trip-time variance (rttvar) estimate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__rttvar.html"


@register
class RouteRttvarCommand(CommandDef):
    name = "ROUTE::rttvar"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::rttvar",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the cached round-trip-time variance (rttvar) estimate.",
                synopsis=("ROUTE::rttvar DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "Returns the cached round-trip-time variance (rttvar) for the\n"
                    "destination and/or gateway if the relevant TCP profile enables\n"
                    "cmetrics-cache.\n"
                    "\n"
                    "The return value only applies to the TMM executing the command. It\n"
                    "does not consider cache entries on other TMMs.\n"
                    "\n"
                    "ROUTE::rttvar returns a value of 0 when there are no statistics\n"
                    "available.\n"
                    "\n"
                    "NOTE: The returned value is scaled to units of 100ns; to express it\n"
                    "in the same units as TCP::rttvar multiply it by 16/10000."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    log local0. "Cached rttvar is: [ROUTE::rttvar [IP::remote_addr]]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::rttvar DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
