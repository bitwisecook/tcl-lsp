# Enriched from F5 iRules reference documentation.
"""MQTT::qos -- Get or set qos of MQTT PUBLISH message"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__qos.html"


@register
class MqttQosCommand(CommandDef):
    name = "MQTT::qos"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::qos",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set qos of MQTT PUBLISH message",
                synopsis=("MQTT::qos ('0' | '1' | '2')?",),
                snippet=(
                    "This command can be used to get or set qos field of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    PUBLISH"
                ),
                source=_SOURCE,
                examples=("Changing QoS to 1:\n\nwhen CLIENT_ACCEPTED {\n   set self_pktid 1\n}"),
                return_value="When called without an argument, this command returns the qos of MQTT message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::qos ('0' | '1' | '2')?",
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
