# Enriched from F5 iRules reference documentation.
"""RADIUS::code -- This command returns the RADIUS message code"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RADIUS__code.html"


@register
class RadiusCodeCommand(CommandDef):
    name = "RADIUS::code"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RADIUS::code",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns the RADIUS message code",
                synopsis=("RADIUS::code",),
                snippet="This command returns the RADIUS message code",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    if { [RADIUS::code] == 4 } {\n"
                    "        set rd 0\n"
                    "        # Extract the APN information from the AVP\n"
                    '        set called_station_id [RADIUS::avp 30 "string"]\n'
                    '        if {$called_station_id == "station1"} {\n'
                    "            set rd 1\n"
                    '        } elseif {$called_station_id == "station2"} {\n'
                    "            set rd 2\n"
                    "        }\n"
                    "        # Overwrite the default route domain value with the new value.\n"
                    "        RADIUS::rtdom $rd\n"
                    "    }\n"
                    "}"
                ),
                return_value="returns radius message code.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RADIUS::code",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset(
                    {
                        "CLIENT_ACCEPTED",
                        "CLIENT_CLOSED",
                        "CLIENT_DATA",
                        "SERVER_CLOSED",
                        "SERVER_CONNECTED",
                        "SERVER_DATA",
                    }
                )
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
