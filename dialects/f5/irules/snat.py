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
"""snat -- Assigns the specified SNAT translation address to the current connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/snat.html"


@register
class SnatCommand(CommandDef):
    name = "snat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="snat",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Assigns the specified SNAT translation address to the current connection.",
                synopsis=("snat (automap | none | IP_TUPLE | (IP_ADDR (PORT)?))",),
                snippet=(
                    "Causes the system to assign the specified source address to the\n"
                    "serverside connection(s). The assignment is valid for the duration of\n"
                    "the clientside connection or until 'snat none' is called. The iRule\n"
                    "SNAT command overrides the SNAT configuration of the virtual server or\n"
                    "a SNAT pool. It does not override the 'Allow SNAT' setting of a pool."
                ),
                source=_SOURCE,
                examples=(
                    "# Apply SNAT autmap if the selected pool member IP address is 1.1.1.1\n"
                    "when LB_SELECTED {\n"
                    "If { [IP::addr [LB::server addr] equals 1.1.1.1] } {\n"
                    "     snat automap\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="snat (automap | none | IP_TUPLE | (IP_ADDR (PORT)?))",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FASTHTTP", "MR"}),
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"}),
            ),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SNAT_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
