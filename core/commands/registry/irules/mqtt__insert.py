# Enriched from F5 iRules reference documentation.
"""MQTT::insert -- Insert an MQTT message"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__insert.html"


_av = make_av(_SOURCE)


@register
class MqttInsertCommand(CommandDef):
    name = "MQTT::insert"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::insert",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Insert an MQTT message",
                synopsis=("MQTT::insert (('before' | 'after') (",),
                snippet=(
                    "This command can be used to insert an MQTT message before or after current message.\n"
                    "Since MQTT_CLIENT_SHUTDOWN event does not have current message only 'MQTT::insert after' is supported for it.\n"
                    "\n"
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
                    "    set type [MQTT::type]\n"
                    "    switch $type {\n"
                    '       "SUBACK" {\n'
                    "          if { [MQTT::packet_id > 1000] } {\n"
                    "             MQTT::drop\n"
                    "          }\n"
                    "       }\n"
                    "    }\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::insert (('before' | 'after') (",
                    arg_values={
                        0: (
                            _av(
                                "before",
                                "MQTT::insert before",
                                "MQTT::insert (('before' | 'after') (",
                            ),
                            _av(
                                "after",
                                "MQTT::insert after",
                                "MQTT::insert (('before' | 'after') (",
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
