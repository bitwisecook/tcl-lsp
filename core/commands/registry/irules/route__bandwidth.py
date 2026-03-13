# Enriched from F5 iRules reference documentation.
"""ROUTE::bandwidth -- Returns a bandwidth estimate for a destination derived from entries in the congestion metrics cache."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__bandwidth.html"


@register
class RouteBandwidthCommand(CommandDef):
    name = "ROUTE::bandwidth"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::bandwidth",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a bandwidth estimate for a destination derived from entries in the congestion metrics cache.",
                synopsis=("ROUTE::bandwidth DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "Returns a bandwidth estimate for a destination derived from\n"
                    "entries in the congestion metrics cache.\n"
                    "\n"
                    "As of v12.0, divides the cached congestion window (cwnd) value\n"
                    "by the cached round-trip-time (RTT ) to obtain a bandwidth\n"
                    "estimate in kbps. If there is no entry, it returns 0.\n"
                    "\n"
                    "Note: The return value only applies to the TMM executing the command.\n"
                    "It does not consider cache entries on other TMMs."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    if { [ROUTE::bandwidth [IP::remote_addr]] > 0 } {\n"
                    '        log local0. "cached bandwidth is: [ROUTE::bandwidth [IP::remote_addr]]"\n'
                    "    }\n"
                    "}"
                ),
                return_value="The bandwidth estimate to the destination and/or gateway in kbps.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::bandwidth DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
