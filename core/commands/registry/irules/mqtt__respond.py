# Enriched from F5 iRules reference documentation.
"""MQTT::respond -- Transmit MQTT message to sender"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__respond.html"


_av = make_av(_SOURCE)


@register
class MqttRespondCommand(CommandDef):
    name = "MQTT::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Transmit MQTT message to sender",
                synopsis=("MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)",),
                snippet=(
                    "This command can be used to transmit MQTT message back to sender of the incoming message.\n"
                    "If called from MQTT_CLIENT_INGRESS message will be sent to the client.\n"
                    "If called from MQTT_SERVER_INGRESS message will be sent to the server.\n"
                    "Please note that current message will be forwarded to destination. Use MQTT::drop to drop the current message.\n"
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
                    "#Enrich MQTT username with SSL client-certificate common name, reject unauthorized accesses:\n"
                    "when CLIENT_ACCEPTED {\n"
                    '    set cn ""\n'
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)",
                    arg_values={
                        0: (
                            _av(
                                "type",
                                "MQTT::respond type",
                                "MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)",
                            ),
                            _av(
                                "CONNACK",
                                "MQTT::respond CONNACK",
                                "MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)",
                            ),
                            _av(
                                "return_code",
                                "MQTT::respond return_code",
                                "MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)",
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
