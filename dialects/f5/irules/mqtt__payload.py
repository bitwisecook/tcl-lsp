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
"""MQTT::payload -- Manipulate payload of MQTT PUBLISH message"""

# Introduced: BIG-IP v14+ (MQTT protocol) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    BytePayloadSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__payload.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(target=SideEffectTarget.NETWORK_IO, reads=True, connection_side=ConnectionSide.BOTH),
)
_WRITE_EFFECT = (
    SideEffect(
        target=SideEffectTarget.NETWORK_IO,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.BOTH,
    ),
)


@register
class MqttPayloadCommand(CommandDef):
    name = "MQTT::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::payload",
            dialects=_IRULES_ONLY,
            # `MQTT::payload replace <data> ?offset? ?length?` — data at index 1.
            byte_array_payload=BytePayloadSpec(replace_data_index=1),
            hover=HoverSnippet(
                summary="Manipulate payload of MQTT PUBLISH message",
                synopsis=(
                    "MQTT::payload ?subcommand? ?args?",
                    "MQTT::payload length",
                    "MQTT::payload replace <data> ?offset? ?length?",
                ),
                snippet=(
                    "This command can be used to manipulate payload of MQTT message.\n"
                    "This command is valid only for following MQTT message types:\n"
                    "\n"
                    "    PUBLISH"
                ),
                source=_SOURCE,
                examples=(
                    "#Example: Redirect PUBLISH that has payloads with blocked keywords defined in\n"
                    "#blacklisted_keywords_datagroup in first 200 bytes. Prepend a admin message in\n"
                    "#the payload.\n"
                    "#\n"
                    "when MQTT_CLIENT_INGRESS {\n"
                    "    set type [MQTT::type]\n"
                    "    switch $type {\n"
                    '       "PUBLISH" {\n'
                    "          if { [class exists  blacklisted_keywords_datagroup] } {\n"
                    "             MQTT::collect 200\n"
                    "          }\n"
                    "       }\n"
                    "    }\n"
                    "}"
                ),
                return_value="When called without an argument, this command returns the collected payload of MQTT PUBLISH message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::payload ?subcommand? ?args?",
                    arg_values={
                        0: (
                            _av("length", "Get payload length.", "MQTT::payload length"),
                            _av(
                                "replace",
                                "Replace payload data.",
                                "MQTT::payload replace <data> ?offset? ?length?",
                            ),
                            _av(
                                "prepend",
                                "Prepend data to payload.",
                                "MQTT::payload prepend <data>",
                            ),
                            _av("append", "Append data to payload.", "MQTT::payload append <data>"),
                        )
                    },
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
            subcommands={
                "length": SubCommand(
                    name="length",
                    arity=Arity(0, 0),
                    detail="Get payload length.",
                    synopsis="MQTT::payload length",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(1, 3),
                    detail="Replace payload data.",
                    synopsis="MQTT::payload replace <data> ?offset? ?length?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "prepend": SubCommand(
                    name="prepend",
                    arity=Arity(1, 1),
                    detail="Prepend data to payload.",
                    synopsis="MQTT::payload prepend <data>",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "append": SubCommand(
                    name="append",
                    arity=Arity(1, 1),
                    detail="Append data to payload.",
                    synopsis="MQTT::payload append <data>",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
