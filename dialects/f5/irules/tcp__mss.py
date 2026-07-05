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
"""TCP::mss -- Returns the Maximum Segment Size (MSS) for a TCP connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__mss.html"


@register
class TcpMssCommand(CommandDef):
    name = "TCP::mss"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::mss",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the Maximum Segment Size (MSS) for a TCP connection.",
                synopsis=("TCP::mss",),
                snippet="Returns the initial connection negotiated MSS. It does not deduct bytes for any common TCP options present in data packets are not deducted. In other words, it is the minimum of the MSS options in the SYN and SYN-ACK packets, or the MSS default of 536 bytes if one packet is missing the option.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if { [TCP::mss] >= 1280 } {\n"
                    "    COMPRESS::disable\n"
                    "  }\n"
                    "}"
                ),
                return_value="MSS in bytes.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::mss",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp",
                also_in=frozenset({"SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
