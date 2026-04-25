# Enriched from F5 iRules reference documentation.
"""MQTT::client_id -- Get or set client identifier of MQTT CONNECT message"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__client_id.html"


@register
class MqttClientIdCommand(CommandDef):
    name = "MQTT::client_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::client_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set client identifier of MQTT CONNECT message",
                synopsis=("MQTT::client_id (CLIENTID)?",),
                snippet=(
                    "This command can be used to get or set client identifier of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    CONNECT"
                ),
                source=_SOURCE,
                examples=(
                    "# Block connections from clientid in the blacklist_clientid_datagroup\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "   set type [MQTT::type]\n"
                    "   switch $type {\n"
                    '       "CONNECT" {\n'
                    "           set cid [MQTT::client_id]\n"
                    "           if { [class exists blacklist_clientid_datagroup] } {\n"
                    '               if {[class match  $cid equals blacklist_clientid_datagroup] != ""} {\n'
                    "                   MQTT::drop\n"
                    "                   MQTT::respond type CONNACK return_code 2\n"
                    "                   MQTT::disconnect\n"
                    "               }\n"
                    "           }"
                ),
                return_value="When called without an argument, this command returns the client identifier of MQTT message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::client_id (CLIENTID)?",
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
