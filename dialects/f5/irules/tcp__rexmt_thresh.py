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
"""TCP::rexmt_thresh -- This command can be used to set/get the retransmission threshold of a TCP connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__rexmt_thresh.html"


@register
class TcpRexmtThreshCommand(CommandDef):
    name = "TCP::rexmt_thresh"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::rexmt_thresh",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to set/get the retransmission threshold of a TCP connection.",
                synopsis=("TCP::rexmt_thresh (TCP_REXMT_THRESH_VALUE)?",),
                snippet=(
                    "TCP::rexmt_thresh returns the retransmission threshold of a TCP connection.\n"
                    "TCP::rexmt_thresh TCP_REXMT_THRESH_VALUE sets the retransmission threshold to specified value."
                ),
                source=_SOURCE,
                examples=(
                    "t/set the retransmission threshold of the TCP flow.\n"
                    "    when CLIENT_ACCEPTED {\n"
                    '        log local0. "TCP set rtx thresh: [TCP::rexmt_thresh 100]"\n'
                    '        log local0. "TCP get rtx thresh: [TCP::rexmt_thresh]"\n'
                    "    }"
                ),
                return_value="TCP::rexmt_thresh returns the retransmission threshold of a TCP connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::rexmt_thresh (TCP_REXMT_THRESH_VALUE)?",
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
