# Enriched from F5 iRules reference documentation.
"""MQTT::type -- Get type of MQTT message"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__type.html"


@register
class MqttTypeCommand(CommandDef):
    name = "MQTT::type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get type of MQTT message",
                synopsis=("MQTT::type",),
                snippet=(
                    "This command can be used to get type of MQTT message.\n"
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
                    "# Typical usage pattern...\n"
                    "#\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "   set type [MQTT::type]\n"
                    "   switch $type {\n"
                    '      "CONNECT" {\n'
                    "         # Do connect processing\n"
                    "      }\n"
                    '      "SUBSCRIBE" {\n'
                    "         # Do subscribe processing\n"
                    "      }\n"
                    '      "PUBLISH" {\n'
                    "         # Do publish processing\n"
                    "      }\n"
                    "   }\n"
                    "}"
                ),
                return_value="A string representation of MQTT message types:",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::type",
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
