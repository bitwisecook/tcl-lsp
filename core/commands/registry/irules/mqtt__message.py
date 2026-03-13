# Enriched from F5 iRules reference documentation.
"""MQTT::message -- Returns the full content of the MQTT message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__message.html"


@register
class MqttMessageCommand(CommandDef):
    name = "MQTT::message"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::message",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the full content of the MQTT message.",
                synopsis=("MQTT::message",),
                snippet="The MQTT::message command returns full content of the MQTT message.",
                source=_SOURCE,
                examples=("when MQTT_CLIENT_INGRESS {\n  log local0. [MQTT::message]\n}"),
                return_value="Returns content of the current message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::message",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MQTT", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
