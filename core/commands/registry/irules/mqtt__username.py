# Enriched from F5 iRules reference documentation.
"""MQTT::username -- Get or set user-name field of MQTT CONNECT message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__username.html"


@register
class MqttUsernameCommand(CommandDef):
    name = "MQTT::username"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::username",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set user-name field of MQTT CONNECT message.",
                synopsis=("MQTT::username (NAME)?",),
                snippet=(
                    "This command can be used to get or set username field of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    CONNECT"
                ),
                source=_SOURCE,
                examples=(
                    "#Enrich MQTT username with SSL client-certificate common name, reject unauthorized accesses:\n"
                    "when CLIENT_ACCEPTED {\n"
                    '    set cn ""\n'
                    "}"
                ),
                return_value="When called without an argument, this command returns the user-name field of MQTT CONNECT message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::username (NAME)?",
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
