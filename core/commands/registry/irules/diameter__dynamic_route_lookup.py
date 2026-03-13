# Enriched from F5 iRules reference documentation.
"""DIAMETER::dynamic_route_lookup -- Set whether messages should be routed dynamically."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__dynamic_route_lookup.html"


_av = make_av(_SOURCE)


@register
class DiameterDynamicRouteLookupCommand(CommandDef):
    name = "DIAMETER::dynamic_route_lookup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::dynamic_route_lookup",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set whether messages should be routed dynamically.",
                synopsis=("DIAMETER::dynamic_route_lookup ( connection | message ) ( BOOLEAN )?",),
                snippet=(
                    '"message":\n'
                    'If status is set to "enabled", previously created dynamic routes will be consulted during the routing of this message.\n'
                    "\n"
                    '"connection":\n'
                    "The setting will be applied to this and all later messages on this connection.\n"
                    "\n"
                    "The zero-argument form of this command returns whether the setting is enabled on the current message."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    "                if { ([DIAMETER::header appid] equals 666) } {\n"
                    "                    DIAMETER::dynamic_route_lookup message disabled\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::dynamic_route_lookup ( connection | message ) ( BOOLEAN )?",
                    arg_values={
                        0: (
                            _av(
                                "connection",
                                "DIAMETER::dynamic_route_lookup connection",
                                "DIAMETER::dynamic_route_lookup ( connection | message ) ( BOOLEAN )?",
                            ),
                            _av(
                                "message",
                                "DIAMETER::dynamic_route_lookup message",
                                "DIAMETER::dynamic_route_lookup ( connection | message ) ( BOOLEAN )?",
                            ),
                        )
                    },
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
