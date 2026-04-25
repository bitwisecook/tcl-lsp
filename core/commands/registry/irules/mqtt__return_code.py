# Enriched from F5 iRules reference documentation.
"""MQTT::return_code -- Get or set return-code field of MQTT CONNACK message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__return_code.html"


@register
class MqttReturnCodeCommand(CommandDef):
    name = "MQTT::return_code"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::return_code",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set return-code field of MQTT CONNACK message.",
                synopsis=("MQTT::return_code (RETURN_CODE)?",),
                snippet=(
                    "This command can be used to get or set return-code field of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    CONNACK"
                ),
                source=_SOURCE,
                examples=(
                    "# For security reasons convert all refused reasons to 5\n"
                    "when MQTT_SERVER_INGRESS {\n"
                    "   set type [MQTT::type]\n"
                    "   switch $type {\n"
                    '       "CONNACK" {\n'
                    "          if { [MQTT::return_code] != 0 } {\n"
                    "             MQTT::return_code 5\n"
                    "          }\n"
                    "       }\n"
                    "   }\n"
                    "}"
                ),
                return_value="When called without an argument, this command returns the return-code field of MQTT CONNACK message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::return_code (RETURN_CODE)?",
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
