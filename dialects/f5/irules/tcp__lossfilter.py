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
"""TCP::lossfilter -- Sets the TCP Loss Ignore Parameters."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__lossfilter.html"


@register
class TcpLossfilterCommand(CommandDef):
    name = "TCP::lossfilter"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::lossfilter",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the TCP Loss Ignore Parameters.",
                synopsis=("TCP::lossfilter TCP_IGNORE_RATE TCP_IGNORE_BURST",),
                snippet=(
                    "Sets the maximum size burst loss (in packets) and maximum number of packets per million lost before triggering congestion response.\n"
                    "  * Burst range is valid from 0 to 32. Higher values decrease the\n"
                    "    chance of performing congestion control.\n"
                    "  * Rate range is valid from 0 to 1,000,000. Rate is X packets lost per\n"
                    "    million before congestion control kicks in."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set client-side loss filter.\n"
                    "    # Ignore up to 150 losses per million packets and burst losses of up to 10 packets.\n"
                    "    clientside {\n"
                    "        TCP::lossfilter 150 10\n"
                    "    }\n"
                    "    # No loss filter on server-side.\n"
                    "    serverside {\n"
                    "        TCP::lossfilter 0 0\n"
                    "    }\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::lossfilter TCP_IGNORE_RATE TCP_IGNORE_BURST",
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
