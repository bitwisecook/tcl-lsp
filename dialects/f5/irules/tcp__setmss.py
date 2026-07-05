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
"""TCP::setmss -- Sets the TCP max segment size."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__setmss.html"


@register
class TcpSetmssCommand(CommandDef):
    name = "TCP::setmss"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::setmss",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the TCP max segment size.",
                synopsis=("TCP::setmss TCP_MAX_SEGMENT_SIZE",),
                snippet=(
                    "This iRule command sets the TCP max segment size in bytes.\n"
                    "The MSS does not consider the length of any common TCP options.\n"
                    "Users should set MSS to the desired path IP packet size, minus the\n"
                    "IP header length (typically 20 bytes), minus the minimum TCP header\n"
                    "length of 20 bytes.\n"
                    "\n"
                    "TCP will automatically apply the length of common options when\n"
                    "partitioning data for delivery."
                ),
                source=_SOURCE,
                examples=(
                    "# Match clientside MSS to serverside MSS\n"
                    "when SERVER_CONNECTED {\n"
                    "    set cli_mss [clientside { TCP::mss }]\n"
                    "    set svr_mss [TCP::mss]\n"
                    "    if { $cli_mss > $svr_mss } {\n"
                    "        clientside { TCP::setmss $svr_mss }\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::setmss TCP_MAX_SEGMENT_SIZE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
