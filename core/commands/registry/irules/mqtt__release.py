# Enriched from F5 iRules reference documentation.
"""MQTT::release -- Releases the data collected via MQTT::collect iRule command"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__release.html"


@register
class MqttReleaseCommand(CommandDef):
    name = "MQTT::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Releases the data collected via MQTT::collect iRule command",
                synopsis=("MQTT::release",),
                snippet=(
                    "Releases the payload data collected via MQTT::collect iRule command for further processing.\n"
                    "\n"
                    "This command is valid only when MQTT::collect has been called."
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
                    synopsis="MQTT::release",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
