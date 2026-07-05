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
"""MQTT::disconnect -- Disconnect the MQTT connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

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
