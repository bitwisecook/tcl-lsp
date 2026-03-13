# Enriched from F5 iRules reference documentation.
"""MQTT::length -- Get length of MQTT message"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__length.html"


@register
class MqttLengthCommand(CommandDef):
    name = "MQTT::length"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::length",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get length of MQTT message",
                synopsis=("MQTT::length",),
                snippet=(
                    "This command can be used to get length of MQTT message.\n"
                    "This command is valid for all MQTT message types:\n"
                    "\n"
                    "    CONNECT, CONNACK,\n"
                    "    PUBLISH, PUBACK, PUBREC, PUBREL, PUBCOMP,\n"
                    "    SUBSCRIBE, SUBACK,\n"
                    "    UNSUBSCRIBE, UNSUBACK,\n"
                    "    PINGREQ, PINGRESP,\n"
                    "    DISCONNECT"
                ),
                source=_SOURCE,
                examples=(
                    "# Drop connections with messages that exceed an administrative max\n"
                    "#\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "  set length [MQTT::length]\n"
                    "  if { $length > 65536 } {\n"
                    "     MQTT::disconnect\n"
                    "  }\n"
                    "}"
                ),
                return_value="This command returns the length of MQTT message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::length",
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
