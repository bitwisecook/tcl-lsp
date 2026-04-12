# Enriched from F5 iRules reference documentation.
"""MQTT::disconnect -- Disconnect the MQTT connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__disconnect.html"


@register
class MqttDisconnectCommand(CommandDef):
    name = "MQTT::disconnect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::disconnect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disconnect the MQTT connection.",
                synopsis=("MQTT::disconnect",),
                snippet="This command disconnects the MQTT connection.",
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
                    synopsis="MQTT::disconnect",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MQTT"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
