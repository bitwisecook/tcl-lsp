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
"""UDP::max_buf_pkts -- This command can be used to set/get the maximum buffer packets value of a UDP connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__max_buf_pkts.html"


@register
class UdpMaxBufPktsCommand(CommandDef):
    name = "UDP::max_buf_pkts"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::max_buf_pkts",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to set/get the maximum buffer packets value of a UDP connection.",
                synopsis=("UDP::max_buf_pkts (UDP_MAX_BUF_PKTS)?",),
                snippet=(
                    "UDP::max_buf_pkts returns the maximum buffer packets value of a UDP connection.\n"
                    "UDP::max_buf_pkts UDP_MAX_BUF_PKTS sets the maximum buffer packets value to specified value."
                ),
                source=_SOURCE,
                examples=(
                    "# Get/set the max buffer packets of the UDP flow.\n"
                    "when CLIENT_ACCEPTED {\n"
                    '    log local0. "UDP get max buffer packets: [UDP::max_buf_pkts]"\n'
                    "    # Set the max buffer packets to 5,000\n"
                    '    log local0. "UDP set max buffer packets: [UPD::max_buf_pkts 5000]"\n'
                    '    log local0. "UDP get max buffer packets: [UDP::max_buf_pkts]"\n'
                    "}"
                ),
                return_value="UDP::max_buf_pkts returns the maximum buffer packets value of a UDP connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::max_buf_pkts (UDP_MAX_BUF_PKTS)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
