# Enriched from F5 iRules reference documentation.
"""RADIUS::rtdom -- This command overwrites the default route-domain ID in RADIUS scenario with given value"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RADIUS__rtdom.html"


@register
class RadiusRtdomCommand(CommandDef):
    name = "RADIUS::rtdom"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RADIUS::rtdom",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command overwrites the default route-domain ID in RADIUS scenario with given value",
                synopsis=("RADIUS::rtdom (ROUTE_DOMAIN)?",),
                snippet="This command overwrites the default route-domain ID in RADIUS scenario with given value",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "        if { [RADIUS::code] == 4 } {\n"
                    "            set rd 0\n"
                    "            # Extract the APN information from the AVP\n"
                    '            set called_station_id [RADIUS::avp 30 "string"]\n'
                    '            if {$called_station_id == "station1"} {\n'
                    "                set rd 1\n"
                    '            } elseif {$called_station_id == "station2"} {\n'
                    "                set rd 2\n"
                    "            }\n"
                    "            # Overwrite the default route domain value with the new value.\n"
                    "            RADIUS::rtdom $rd\n"
                    "        }\n"
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RADIUS::rtdom (ROUTE_DOMAIN)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
