# Enriched from F5 iRules reference documentation.
"""MQTT::return_code_list -- Get return-code-list of MQTT SUBACK message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__return_code_list.html"


@register
class MqttReturnCodeListCommand(CommandDef):
    name = "MQTT::return_code_list"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::return_code_list",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get return-code-list of MQTT SUBACK message.",
                synopsis=("MQTT::return_code_list",),
                snippet=(
                    "This command can be used to get return-code-list of MQTT message.\n"
                    "Note that this command does not support a 'set' operation.\n"
                    "In order to change the retun-code-list please use 'MQTT::replace type SUBACK'.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    SUBACK"
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n   set suback_count 0\n   set rclist [list]\n}"
                ),
                return_value="When called without an argument, this command returns the return-code-list of MQTT SUBACK message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::return_code_list",
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
