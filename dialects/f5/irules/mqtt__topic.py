# Enriched from F5 iRules reference documentation.
"""MQTT::topic -- Manipulate topic(s) of MQTT message."""

# Introduced: BIG-IP v14+ (MQTT protocol) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__topic.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(target=SideEffectTarget.NETWORK_IO, reads=True, connection_side=ConnectionSide.BOTH),
)
_WRITE_EFFECT = (
    SideEffect(
        target=SideEffectTarget.NETWORK_IO,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.BOTH,
    ),
)


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
                synopsis=(
                    "MQTT::topic ?subcommand? ?args?",
                    "MQTT::topic replace <new_topic> ?index?",
                    "MQTT::topic count",
                ),
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
                    synopsis="MQTT::topic ?subcommand? ?args?",
                    arg_values={
                        0: (
                            _av(
                                "replace",
                                "Replace topic name.",
                                "MQTT::topic replace <topic> ?index?",
                            ),
                            _av("count", "Get topic count.", "MQTT::topic count"),
                            _av("list", "List all topics.", "MQTT::topic list"),
                            _av("index", "Get topic at index.", "MQTT::topic index <n>"),
                            _av("qos", "Get/set topic QoS.", "MQTT::topic qos ?index?"),
                            _av("add", "Add a topic.", "MQTT::topic add <topic> ?qos?"),
                            _av("delete", "Delete a topic.", "MQTT::topic delete <index>"),
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
            subcommands={
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(1, 2),
                    detail="Replace topic name.",
                    synopsis="MQTT::topic replace <topic> ?index?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "count": SubCommand(
                    name="count",
                    arity=Arity(0, 0),
                    detail="Get topic count.",
                    synopsis="MQTT::topic count",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "list": SubCommand(
                    name="list",
                    arity=Arity(0, 0),
                    detail="List all topics.",
                    synopsis="MQTT::topic list",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "index": SubCommand(
                    name="index",
                    arity=Arity(1, 1),
                    detail="Get topic at index.",
                    synopsis="MQTT::topic index <n>",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "qos": SubCommand(
                    name="qos",
                    arity=Arity(0, 1),
                    detail="Get/set topic QoS.",
                    synopsis="MQTT::topic qos ?index?",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "add": SubCommand(
                    name="add",
                    arity=Arity(1, 2),
                    detail="Add a topic.",
                    synopsis="MQTT::topic add <topic> ?qos?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(1, 1),
                    detail="Delete a topic.",
                    synopsis="MQTT::topic delete <index>",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
