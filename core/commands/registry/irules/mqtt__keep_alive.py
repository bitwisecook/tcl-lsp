# Enriched from F5 iRules reference documentation.
"""MQTT::keep_alive -- Get or set keep_alive field of MQTT CONNECT message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__keep_alive.html"


@register
class MqttKeepAliveCommand(CommandDef):
    name = "MQTT::keep_alive"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::keep_alive",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set keep_alive field of MQTT CONNECT message.",
                synopsis=("MQTT::keep_alive (KEEP_ALIVE)?",),
                snippet=(
                    "This command can be used to get or set keep_alive field of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    CONNECT"
                ),
                source=_SOURCE,
                examples=(
                    "# Increase keep-alive to at least 60 seconds\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "   set type [MQTT::type]\n"
                    "   switch $type {\n"
                    '       "CONNECT"  {\n'
                    "           if { [MQTT::keep_alive] < 60} {\n"
                    "              MQTT::keep_alive 60\n"
                    "           }\n"
                    "       }\n"
                    "   }\n"
                    "}"
                ),
                return_value="When called without an argument, this command returns the keep_alive field of MQTT CONNECT message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::keep_alive (KEEP_ALIVE)?",
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
