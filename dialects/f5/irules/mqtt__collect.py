# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Enriched from F5 iRules reference documentation.
"""MQTT::collect -- Collect the specified amount of MQTT message payload data"""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__collect.html"


@register
class MqttCollectCommand(CommandDef):
    name = "MQTT::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::collect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Collect the specified amount of MQTT message payload data",
                synopsis=("MQTT::collect (COLLECT)?",),
                snippet=(
                    "Collects the specified amount of MQTT message payload data before triggering a MQTT_CLIENT_DATA or MQTT_SERVER_DATA event.\n"
                    "\n"
                    "When collecting data in a clientside event, the MQTT_CLIENT_DATA event will be triggered.\n"
                    "When collecting data in a serverside event, the MQTT_SERVER_DATA event will be triggered.\n"
                    "\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    PUBLISH\n"
                    "\n"
                    "This command allows you to perform various operations on MQTT PUBLISH message like modify its contents.\n"
                    "NOTE: Please make sure that MQTT PUBLISH message expects to receive a payload by using [MQTT::payload length]."
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
                    synopsis="MQTT::collect (COLLECT)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MQTT"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
