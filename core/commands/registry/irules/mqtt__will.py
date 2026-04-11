# Enriched from F5 iRules reference documentation.
"""MQTT::will -- Get or set will-topic, will-message, will-qos, and will-retain fields of MQTT CONNECT message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__will.html"


_av = make_av(_SOURCE)


@register
class MqttWillCommand(CommandDef):
    name = "MQTT::will"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::will",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set will-topic, will-message, will-qos, and will-retain fields of MQTT CONNECT message.",
                synopsis=("MQTT::will (('topic' (TOPIC)?) |",),
                snippet=(
                    "This command can be used to get or set will-topic, will-message, will-qos, and will-retain fields of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    CONNECT"
                ),
                source=_SOURCE,
                examples=(
                    "# Enforce a mandatary default will message, if will is not present in connect\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "    set type [MQTT::type]\n"
                    "    switch $type {\n"
                    '        "CONNECT" {\n'
                    '            if { [MQTT::will topic] == "" } {\n'
                    '                MQTT::will topic "/bigip/default/will/[MQTT::username]/[MQTT::client_id]/[client_addr]"\n'
                    '                MQTT::will message "client disconnected without sending DISCONNECT message"\n'
                    "                MQTT::will qos 0\n"
                    "                MQTT::will retain 0\n"
                    "            }"
                ),
                return_value="When called without an argument, each of the sub-commands return the will-topic, will-message, will-qos, or will-retain field of MQTT CONNECT message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::will (('topic' (TOPIC)?) |",
                    arg_values={
                        0: (_av("topic", "MQTT::will topic", "MQTT::will (('topic' (TOPIC)?) |"),)
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
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
