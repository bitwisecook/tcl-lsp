# Enriched from F5 iRules reference documentation.
"""MQTT::clean_session -- Get or set clean_session flag of MQTT CONNECT message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__clean_session.html"


@register
class MqttCleanSessionCommand(CommandDef):
    name = "MQTT::clean_session"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::clean_session",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set clean_session flag of MQTT CONNECT message.",
                synopsis=("MQTT::clean_session ('0' | '1')?",),
                snippet=(
                    "This command can be used to get or set clean_session flag of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    CONNECT"
                ),
                source=_SOURCE,
                examples=(
                    "# Convert non-clean-session connections to clean-session connections\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "   set type [MQTT::type]\n"
                    "   switch $type {\n"
                    '       "CONNECT" {\n'
                    "           if { [MQTT::clean_session] == 1} {\n"
                    "              MQTT::clean_session 0\n"
                    "           }\n"
                    "       }\n"
                    "   }\n"
                    "}"
                ),
                return_value="When called without an argument, this command returns the clean_session flag of MQTT CONNECT message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::clean_session ('0' | '1')?",
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
