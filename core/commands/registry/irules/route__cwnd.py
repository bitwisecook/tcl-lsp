# Enriched from F5 iRules reference documentation.
"""ROUTE::cwnd -- Returns the cached congestion window (cwnd) value."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__cwnd.html"


@register
class RouteCwndCommand(CommandDef):
    name = "ROUTE::cwnd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::cwnd",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the cached congestion window (cwnd) value.",
                synopsis=("ROUTE::cwnd DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "Returns the cached congestion window (cwnd) value for a given\n"
                    "destination IP and/or gateway.\n"
                    "\n"
                    "The return value only applies to the TMM executing the command. It\n"
                    "does not consider cache entries on other TMMs."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set cwnd [ROUTE::cwnd [IP::remote_addr]]\n"
                    "    if { $cwnd > 0 } {\n"
                    '        log local0. "Destination found in cache. Initializing cwnd to $cwnd"\n'
                    "    } else {\n"
                    '        log local0. "Destination not found in cache."\n'
                    "    }\n"
                    "}"
                ),
                return_value="The cached congestion window in bytes.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::cwnd DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
