# Enriched from F5 iRules reference documentation.
"""MQTT::retain -- Get or set retain flag of MQTT PUBLISH message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__retain.html"


@register
class MqttRetainCommand(CommandDef):
    name = "MQTT::retain"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::retain",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set retain flag of MQTT PUBLISH message.",
                synopsis=("MQTT::retain ('0' | '1')?",),
                snippet=(
                    "This command can be used to get or set retain flag of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    PUBLISH"
                ),
                source=_SOURCE,
                examples=(
                    "# Convert PUBLISH for topics in a retain_datagroup to retain messages\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "   set type [MQTT::type]\n"
                    "   switch $type {\n"
                    '      "PUBLISH" {\n'
                    "          if {[MQTT::retain] eq 0} {\n"
                    "              if { [class exists retain_datagroup] } {\n"
                    "                  if {[class match [MQTT::topic] starts_with retain_datagroup]} {\n"
                    "                     MQTT::retain 1\n"
                    "                  }\n"
                    "              }\n"
                    "          }\n"
                    "      }\n"
                    "   }\n"
                    "}"
                ),
                return_value="When called without an argument, this command returns the retain flag of MQTT PUBLISH message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::retain ('0' | '1')?",
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
