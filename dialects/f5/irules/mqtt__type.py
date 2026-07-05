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
"""MQTT::type -- Get type of MQTT message"""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__type.html"


@register
class MqttTypeCommand(CommandDef):
    name = "MQTT::type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get type of MQTT message",
                synopsis=("MQTT::type",),
                snippet=(
                    "This command can be used to get type of MQTT message.\n"
                    "This command is valid for all MQTT message types:\n"
                    "\n"
                    "    CONNECT, CONNACK,\n"
                    "    PUBLISH, PUBACK, PUBREC, PUBREL, PUBCOMP,\n"
                    "    SUBSCRIBE, SUBACK,\n"
                    "    UNSUBSCRIBE, UNSUBACK,\n"
                    "    PINGREQ, PINGRESP,\n"
                    "    DISCONNECT"
                ),
                source=_SOURCE,
                examples=(
                    "# Typical usage pattern...\n"
                    "#\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "   set type [MQTT::type]\n"
                    "   switch $type {\n"
                    '      "CONNECT" {\n'
                    "         # Do connect processing\n"
                    "      }\n"
                    '      "SUBSCRIBE" {\n'
                    "         # Do subscribe processing\n"
                    "      }\n"
                    '      "PUBLISH" {\n'
                    "         # Do publish processing\n"
                    "      }\n"
                    "   }\n"
                    "}"
                ),
                return_value="A string representation of MQTT message types:",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::type",
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
