# Enriched from F5 iRules reference documentation.
"""ROUTE::age -- Deprecated: Returns the age of the route metrics in seconds."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__age.html"


@register
class RouteAgeCommand(CommandDef):
    name = "ROUTE::age"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::age",
            deprecated_replacement="(removed)",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Returns the age of the route metrics in seconds.",
                synopsis=("ROUTE::age DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "The amount of time that has elapsed since the last update to the\n"
                    "ROUTE::rtt, ROUTE::rttvar and ROUTE::bandwidth\n"
                    "statistics for the matched route metric entry.\n"
                    "ROUTE::age has a value of 0 when there are no statistics\n"
                    "available.\n"
                    "\n"
                    "Note: As of v12.0 ROUTE::age is deprecated, as the expiration time,\n"
                    "rather than the creation time, is now stored. Since deprecation,\n"
                    "ROUTE::age reports the age assuming that initial timeout was the\n"
                    "sys db variable route.metrics.timeout. Results are incorrect if\n"
                    "timeout was changed by the TCP profile or an iRule."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    log local0. "Cached age is: [ROUTE::age [IP::remote_addr]]"\n'
                    "}"
                ),
                return_value="The age of the route metrics in seconds",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::age DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
