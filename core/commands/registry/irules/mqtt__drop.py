# Enriched from F5 iRules reference documentation.
"""MQTT::drop -- Drop the current MQTT message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__drop.html"


@register
class MqttDropCommand(CommandDef):
    name = "MQTT::drop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::drop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Drop the current MQTT message.",
                synopsis=("MQTT::drop",),
                snippet=(
                    "This command can be used to drop the current MQTT message. The MQTT message will not be forwarded to its destination.\n"
                    "This command is valid for all MQTT message types."
                ),
                source=_SOURCE,
                examples=(
                    "#Enrich MQTT username with SSL client-certificate common name, reject unauthorized accesses:\n"
                    "when CLIENT_ACCEPTED {\n"
                    '    set cn ""\n'
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::drop",
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
