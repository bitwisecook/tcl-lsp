# Enriched from F5 iRules reference documentation.
"""MQTT::session_present -- Get or set session_present flag of MQTT CONNACK message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__session_present.html"


@register
class MqttSessionPresentCommand(CommandDef):
    name = "MQTT::session_present"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::session_present",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set session_present flag of MQTT CONNACK message.",
                synopsis=("MQTT::session_present ('0' | '1')?",),
                snippet=(
                    "This command can be used to get or set session_present flag of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    CONNACK"
                ),
                source=_SOURCE,
                examples=(
                    "# Prevent session_present from being 1\n"
                    "when MQTT_SERVER_INGRESS {\n"
                    "    set type [MQTT::type]\n"
                    "    switch $type {\n"
                    '        "CONNACK" {\n'
                    "            if { [MQTT::session_present] == 1 } {\n"
                    "                MQTT::session_present 0\n"
                    "            }\n"
                    "        }\n"
                    "    }\n"
                    "}"
                ),
                return_value="When called without an argument, this command returns the session_present flag of MQTT CONNACK message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::session_present ('0' | '1')?",
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
