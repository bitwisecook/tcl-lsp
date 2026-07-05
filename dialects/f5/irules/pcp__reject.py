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
"""PCP::reject -- Provides the ability to cause a PCP reqeust to fail based on processing in the iRule."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PCP__reject.html"


@register
class PcpRejectCommand(CommandDef):
    name = "PCP::reject"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PCP::reject",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides the ability to cause a PCP reqeust to fail based on processing in the iRule.",
                synopsis=("PCP::reject PCP_RESULT_CODE",),
                snippet=(
                    "This command provides the ability to cause a PCP (Port Control\n"
                    "Protocol) reqeust to fail based on processing in the iRule. If the\n"
                    "reject command is issued, the PCP request is rejected with the\n"
                    "specified result code and no other action is taken by the PCP proxy."
                ),
                source=_SOURCE,
                examples=(
                    "when PCP_REQUEST {\n"
                    '     if {[PCP::request opcode] == "map" &&\n'
                    "             [PCP::request internal-port] == 22 } {\n"
                    '         log "Rejecting PCP request to map SSH"\n'
                    "         PCP::reject 1\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PCP::reject PCP_RESULT_CODE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"PCP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
