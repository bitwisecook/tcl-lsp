# Enriched from F5 iRules reference documentation.
"""MQTT::collect -- Collect the specified amount of MQTT message payload data"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__collect.html"


@register
class MqttCollectCommand(CommandDef):
    name = "MQTT::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::collect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Collect the specified amount of MQTT message payload data",
                synopsis=("MQTT::collect (COLLECT)?",),
                snippet=(
                    "Collects the specified amount of MQTT message payload data before triggering a MQTT_CLIENT_DATA or MQTT_SERVER_DATA event.\n"
                    "\n"
                    "When collecting data in a clientside event, the MQTT_CLIENT_DATA event will be triggered.\n"
                    "When collecting data in a serverside event, the MQTT_SERVER_DATA event will be triggered.\n"
                    "\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    PUBLISH\n"
                    "\n"
                    "This command allows you to perform various operations on MQTT PUBLISH message like modify its contents.\n"
                    "NOTE: Please make sure that MQTT PUBLISH message expects to receive a payload by using [MQTT::payload length]."
                ),
                source=_SOURCE,
                examples=(
                    "when MQTT_CLIENT_DATA {\n"
                    "   set type [MQTT::type]\n"
                    "   switch $type {\n"
                    '       "PUBLISH" {\n'
                    "          set payload [MQTT::payload]\n"
                    "          MQTT::release\n"
                    "          set found [class match $payload contains blacklisted_keywords_datagroup]\n"
                    '          if { $found != "" } {\n'
                    "              MQTT::disconnect\n"
                    "          }\n"
                    "       }\n"
                    "   }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::collect (COLLECT)?",
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
