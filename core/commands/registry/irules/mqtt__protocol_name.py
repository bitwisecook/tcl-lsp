# Enriched from F5 iRules reference documentation.
"""MQTT::protocol_name -- Get or set protocol-name of MQTT CONNECT message"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__protocol_name.html"


@register
class MqttProtocolNameCommand(CommandDef):
    name = "MQTT::protocol_name"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::protocol_name",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set protocol-name of MQTT CONNECT message",
                synopsis=("MQTT::protocol_name (PROTOCOL)?",),
                snippet=(
                    "This command can be used to get or set protocol name of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    CONNECT"
                ),
                source=_SOURCE,
                examples=(
                    "# Upgrade protocol from 3.1 to 3.1.1\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "   set type [MQTT::type]\n"
                    "   switch $type {\n"
                    '       "CONNECT" {\n'
                    "          if {[MQTT::protocol_version] == 3 } {\n"
                    "             MQTT::protocol_version 4\n"
                    '             MQTT::protocol_name "MQTT"\n'
                    "          }\n"
                    "       }\n"
                    "   }\n"
                    "}"
                ),
                return_value="When called without an argument, this command returns the protocol name of MQTT CONNECT message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::protocol_name (PROTOCOL)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MQTT"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
