# Enriched from F5 iRules reference documentation.
"""MQTT::packet_id -- Get or set packet-id of MQTT message"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__packet_id.html"


@register
class MqttPacketIdCommand(CommandDef):
    name = "MQTT::packet_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::packet_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set packet-id of MQTT message",
                synopsis=("MQTT::packet_id (PACKETID)?",),
                snippet=(
                    "This command can be used to get or set packet-id field of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    PUBLISH (if QoS > 0)\n"
                    "    PUBACK\n"
                    "    PUBREC\n"
                    "    PUBREL\n"
                    "    PUBCOMP\n"
                    "    SUBSCRIBE\n"
                    "    SUBACK\n"
                    "    UNSUBSCRIBE\n"
                    "    UNSUBACK\n"
                    "    PINGREQ\n"
                    "    PINGRESP\n"
                    "    DISCONNECT"
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n   set suback_count 0\n   set rclist [list]\n}"
                ),
                return_value="When called without an argument, this command returns the packet-id of MQTT message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::packet_id (PACKETID)?",
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
