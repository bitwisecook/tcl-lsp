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
"""MQTT::respond -- Transmit MQTT message to sender"""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MQTT__respond.html"


_av = make_av(_SOURCE)


@register
class MqttRespondCommand(CommandDef):
    name = "MQTT::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MQTT::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Transmit MQTT message to sender",
                synopsis=("MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)",),
                snippet=(
                    "This command can be used to transmit MQTT message back to sender of the incoming message.\n"
                    "If called from MQTT_CLIENT_INGRESS message will be sent to the client.\n"
                    "If called from MQTT_SERVER_INGRESS message will be sent to the server.\n"
                    "Please note that current message will be forwarded to destination. Use MQTT::drop to drop the current message.\n"
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
                    "#Enrich MQTT username with SSL client-certificate common name, reject unauthorized accesses:\n"
                    "when CLIENT_ACCEPTED {\n"
                    '    set cn ""\n'
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)",
                    arg_values={
                        0: (
                            _av(
                                "type",
                                "MQTT::respond type",
                                "MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)",
                            ),
                            _av(
                                "CONNACK",
                                "MQTT::respond CONNACK",
                                "MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)",
                            ),
                            _av(
                                "return_code",
                                "MQTT::respond return_code",
                                "MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)",
                            ),
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
