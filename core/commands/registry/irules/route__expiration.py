# Enriched from F5 iRules reference documentation.
"""ROUTE::expiration -- Returns the remaining time for a route or congestion metrics cache entry."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__expiration.html"


@register
class RouteExpirationCommand(CommandDef):
    name = "ROUTE::expiration"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::expiration",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the remaining time for a route or congestion metrics cache entry.",
                synopsis=("ROUTE::expiration DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "Returns the remaining time in seconds. The lifetime of an entry may\n"
                    "have been set by the route.metrics.timeout sys db variable, the\n"
                    "cmetrics-cache-timeout TCP profile attribute, or a\n"
                    "TCP::rt_metrics_timeout iRule.\n"
                    "\n"
                    "The return value only applies to the TMM executing the command. It\n"
                    "does not consider cache entries on other TMMs."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_CLOSED {\n"
                    "    # If the entry almost timed out, keep it a little longer next time.\n"
                    "    set time_remaining [ROUTE::expiration [IP::remote_addr]]\n"
                    "    if { $time_remaining > 0 && $time_remaining < 100 } {\n"
                    "         # Default value is 600\n"
                    "         TCP::rt_metrics_timeout 700\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::expiration DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
