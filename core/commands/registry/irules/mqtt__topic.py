# Enriched from F5 iRules reference documentation.
"""MQTT::topic -- Manipulate topic(s) of MQTT message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__topic.html"


_av = make_av(_SOURCE)


@register
class MqttTopicCommand(CommandDef):
    name = "MQTT::topic"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::topic",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Manipulate topic(s) of MQTT message.",
                synopsis=("MQTT::topic (('replace' TOPIC) |",),
                snippet=(
                    "This command can be used to manipulate topic(s) of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    PUBLISH\n"
                    "    SUBSCRIBE\n"
                    "    UNSUBSCRIBE"
                ),
                source=_SOURCE,
                examples=(
                    "when MQTT_SERVER_INGRESS {\n"
                    "    set smtype [MQTT::type]\n"
                    '    if {$smtype == "SUBACK"} {\n'
                    "       set mid [MQTT::packet_id]\n"
                    '       set tc [table lookup -subtable "packetid_count_table" "[client_addr]_[client_port]_${mid}"]\n'
                    "       set return_codes [MQTT::return_code_list]\n"
                    "       set return_codes [lreplace $return_codes $tc $tc]\n"
                    "       MQTT::replace type SUBACK packet_id $mid return_code_list $return_codes\n"
                    "    }\n"
                    "}"
                ),
                return_value="When called without an argument, this command returns the topic-name of MQTT PUBLISH message, or the topic-name of the first topic of MQTT SUBSCRIBE and UNSUBSCRIBE messages.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::topic (('replace' TOPIC) |",
                    arg_values={
                        0: (
                            _av(
                                "replace", "MQTT::topic replace", "MQTT::topic (('replace' TOPIC) |"
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
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
