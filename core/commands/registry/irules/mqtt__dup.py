# Enriched from F5 iRules reference documentation.
"""MQTT::dup -- Get or set duplicate flag of MQTT PUBLISH message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__dup.html"


@register
class MqttDupCommand(CommandDef):
    name = "MQTT::dup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::dup",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set duplicate flag of MQTT PUBLISH message.",
                synopsis=("MQTT::dup ('0' | '1')?",),
                snippet=(
                    "This command can be used to get or set duplicate flag of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    PUBLISH"
                ),
                source=_SOURCE,
                examples=(
                    "#Downgrading QoS to 0:\n"
                    "\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "    set type [MQTT::type]\n"
                    "    switch $type {\n"
                    '        "PUBLISH" {\n'
                    "            set in_qos [MQTT::qos]\n"
                    "            if { $in_qos > 0 } {\n"
                    "                set pktid [MQTT::packet_id]\n"
                    "            }\n"
                    "            MQTT::dup 0\n"
                    "            MQTT::qos 0\n"
                    "            if { $in_qos == 1 } {\n"
                    "                MQTT::respond type PUBACK packet_id $pktid\n"
                    "            } elseif { $in_qos == 2 } {\n"
                    "                MQTT::respond type PUBREC packet_id $pktid\n"
                    "            }"
                ),
                return_value="When called without an argument, this command returns the duplicate flag of MQTT message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::dup ('0' | '1')?",
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
