# Enriched from F5 iRules reference documentation.
"""MQTT::enable -- Enable MQTT parsing on a connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__enable.html"


@register
class MqttEnableCommand(CommandDef):
    name = "MQTT::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable MQTT parsing on a connection.",
                synopsis=("MQTT::enable",),
                snippet="This command enables MQTT parsing on a connection.",
                source=_SOURCE,
                examples=("when SERVER_CONNECTED {\n   MQTT::enable\n}"),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
