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
"""TCP::close -- Closes the TCP connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__close.html"


@register
class TcpCloseCommand(CommandDef):
    name = "TCP::close"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::close",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Closes the TCP connection.",
                synopsis=("TCP::close",),
                snippet="Sends the FIN byte to gracefully close the connection.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    set my_loc "http://www.i-want-a-bigip-for-christmas.com"\n'
                    '    TCP::respond "HTTP/1.1 302 Found\\r\\nLocation: $my_loc\\r\\nConnection: close\\r\\nContent-Length: 0\\r\\n\\r\\n"\n'
                    "    TCP::close\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::close",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp",
                also_in=frozenset({"SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"}),
            ),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
