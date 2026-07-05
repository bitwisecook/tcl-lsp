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
"""PCP::request -- Provides access to the data sent in a PCP request."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PCP__request.html"


@register
class PcpRequestCommand(CommandDef):
    name = "PCP::request"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PCP::request",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides access to the data sent in a PCP request.",
                synopsis=("PCP::request (opcode |",),
                snippet=(
                    "This command provides access to the data sent in a PCP (Port Control\n"
                    "Protocol) request. Access to this data is read-only, and the data in\n"
                    "the PCP request cannot be modified via the PCP::request command."
                ),
                source=_SOURCE,
                examples=(
                    "when PCP_REQUEST {\n"
                    '     if {[PCP::request opcode] == "map" && [PCP::request client-addr] == "192.168.1.1" } {\n'
                    '         log "Received PCP map request for port [PCP::request internal-port] from 192.168.1.1"\n'
                    "     }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PCP::request (opcode |",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"PCP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
