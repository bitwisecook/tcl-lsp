# Enriched from F5 iRules reference documentation.
"""MQTT::password -- Get or set password field of MQTT CONNECT message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__password.html"


@register
class MqttPasswordCommand(CommandDef):
    name = "MQTT::password"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::password",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set password field of MQTT CONNECT message.",
                synopsis=("MQTT::password (PASSWORD)?",),
                snippet=(
                    "This command can be used to get or set password field of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    CONNECT"
                ),
                source=_SOURCE,
                examples=(
                    "# Reject connections with no password\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "    set type [MQTT::type]\n"
                    "    switch $type {\n"
                    '        "CONNECT" {\n'
                    '            if {[MQTT::username] == "" } {\n'
                    "               MQTT::respond type CONNACK return_code 4\n"
                    "               MQTT::disconnect\n"
                    "            }\n"
                    "        }\n"
                    "    }\n"
                    "}"
                ),
                return_value="When called without an argument, this command returns the password field of MQTT CONNECT message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::password (PASSWORD)?",
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
