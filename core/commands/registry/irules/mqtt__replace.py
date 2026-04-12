# Enriched from F5 iRules reference documentation.
"""MQTT::replace -- Replace MQTT message"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__replace.html"


_av = make_av(_SOURCE)


@register
class MqttReplaceCommand(CommandDef):
    name = "MQTT::replace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::replace",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Replace MQTT message",
                synopsis=("MQTT::replace ( (('type' 'CONNECT') ('client_id' NAME)",),
                snippet=(
                    "This command can be used to replace current MQTT message.\n"
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
                    "when MQTT_SERVER_INGRESS {\n"
                    "   set type [MQTT::type]\n"
                    "   switch $type {\n"
                    '      "SUBACK" {\n'
                    "         if {[MQTT::packet_id] > 1000} {\n"
                    "             MQTT::drop\n"
                    "         }\n"
                    "      }\n"
                    "   }\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::replace ( (('type' 'CONNECT') ('client_id' NAME)",
                    arg_values={
                        0: (
                            _av(
                                "type",
                                "MQTT::replace type",
                                "MQTT::replace ( (('type' 'CONNECT') ('client_id' NAME)",
                            ),
                            _av(
                                "CONNECT",
                                "MQTT::replace CONNECT",
                                "MQTT::replace ( (('type' 'CONNECT') ('client_id' NAME)",
                            ),
                            _av(
                                "client_id",
                                "MQTT::replace client_id",
                                "MQTT::replace ( (('type' 'CONNECT') ('client_id' NAME)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MQTT"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
