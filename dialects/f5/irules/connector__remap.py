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
"""CONNECTOR::remap -- Set client/server IP/Port from connector."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CONNECTOR__remap.html"


@register
class ConnectorRemapCommand(CommandDef):
    name = "CONNECTOR::remap"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CONNECTOR::remap",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set client/server IP/Port from connector.",
                synopsis=(
                    "CONNECTOR::remap server_addr IP_ADDR",
                    "CONNECTOR::remap client_addr IP_ADDR",
                    "CONNECTOR::remap client_port PORT",
                    "CONNECTOR::remap server_port PORT",
                ),
                snippet=(
                    "CONNECTOR::remap client_addr\n"
                    "    Set the client IP address from connector profile.\n"
                    "CONNECTOR::remap server_addr\n"
                    "    Set the server IP address from connector profile.\n"
                    "CONNECTOR::remap client_port\n"
                    "    Set the client port from connector profile.\n"
                    "CONNECTOR::remap server_port\n"
                    "    Set the server port from connector profile."
                ),
                source=_SOURCE,
                examples=(
                    "when CONNECTOR_OPEN {\n"
                    '                if {([CONNECTOR::profile] eq "/Common/connector_profile_1")} {\n'
                    "                    CONNECTOR::remap client_addr 10.10.10.2\n"
                    '                    log local0. "Remap client IP address from connector to 10.10.10.2"\n'
                    "                    CONNECTOR::remap client_port 333\n"
                    '                    log local0. "Remap client port from connector to 333"\n'
                    "                    CONNECTOR::remap server_addr 20.20.20.2"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CONNECTOR::remap server_addr IP_ADDR",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
