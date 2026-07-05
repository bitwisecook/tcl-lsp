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
"""DIAMETER::disconnect -- Sends Disconnect-Peer-Request to client or server based on context."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__disconnect.html"


@register
class DiameterDisconnectCommand(CommandDef):
    name = "DIAMETER::disconnect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::disconnect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends Disconnect-Peer-Request to client or server based on context.",
                synopsis=(
                    "DIAMETER::disconnect ORIGIN_HOST ORIGIN_REALM DIAMETER_DISCONNECT_CAUSE",
                ),
                snippet=(
                    "This iRule command sends Disconnect-Peer-Request to the client (if run\n"
                    "on clientside) or to the server (if run on serverside)."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    "    # 2 = DO_NOT_WANT_TO_TALK_TO_YOU (RFC 6733 sec 5.4.3)\n"
                    '    DIAMETER::disconnect "bigip.core.example.com" "example.com" 2\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::disconnect ORIGIN_HOST ORIGIN_REALM DIAMETER_DISCONNECT_CAUSE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
