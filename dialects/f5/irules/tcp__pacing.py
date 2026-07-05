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
"""TCP::pacing -- Toggles TCP rate pacing."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__pacing.html"


@register
class TcpPacingCommand(CommandDef):
    name = "TCP::pacing"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::pacing",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles TCP rate pacing.",
                synopsis=("TCP::pacing (BOOL_VALUE)?",),
                snippet="Rate pacing limits the data send rate to the physical limitations of the interface to reduce the chance of queue drops.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set client-side rate pacing to enabled.\n"
                    "    clientside {\n"
                    '        log local0. "Client: pacing [TCP::pacing], enabling"\n'
                    "        TCP::pacing enable\n"
                    "    }\n"
                    "    # Set server-side rate pacing to disabled.\n"
                    "    serverside {\n"
                    '        log local0. "Server: pacing [TCP::pacing], disabling"\n'
                    "        TCP::pacing disable\n"
                    "    }\n"
                    "}"
                ),
                return_value="TCP::pacing returns whether TCP rate pacing is enabled.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::pacing (BOOL_VALUE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
