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
"""MR::ignore_peer_port -- Gets or sets the ignore_peer_port mode for the current connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__ignore_peer_port.html"


@register
class MrIgnorePeerPortCommand(CommandDef):
    name = "MR::ignore_peer_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::ignore_peer_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the ignore_peer_port mode for the current connection.",
                synopsis=("MR::ignore_peer_port (BOOLEAN)?",),
                snippet="The MR::ignore_peer_port command sets or resets the ignore_peer_port mode of the current connection. If ignore_peer_port mode is enabled, the remote port of the connection will be ignored when determining if the connection is usable for forwarding a message to a peer. For example, if a peer at IP 10.1.2.3 connects using a ephemeral port of 12345 and ignore_peer_port is enabled, a message routed to IP 10.1.2.3 port 2345 can be forwarded using this connection since the port will be ignored.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                MR::ignore_peer_port yes\n"
                    "            }"
                ),
                return_value="Returns the current value of the ignore_peer_port flag. This will be 'true' or 'false'.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::ignore_peer_port (BOOLEAN)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
