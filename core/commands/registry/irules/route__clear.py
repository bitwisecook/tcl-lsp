# Enriched from F5 iRules reference documentation.
"""ROUTE::clear -- Removes a Congestion Metrics Cache entry."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__clear.html"


@register
class RouteClearCommand(CommandDef):
    name = "ROUTE::clear"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::clear",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Removes a Congestion Metrics Cache entry.",
                synopsis=("ROUTE::clear DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "Removes the congestion metrics and MTU associated with a\n"
                    "destination IP address and/or gateway.\n"
                    "\n"
                    "Clears the entry on all platform TMMs."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set bandwidth [ROUTE::bandwidth [IP::remote_addr]]\n"
                    "    if { $bandwidth > 0 && $bandwidth < 1000 } {\n"
                    "        # Reject cache entries below 1000 kbps\n"
                    "        ROUTE::clear [IP::remote_addr]\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::clear DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
