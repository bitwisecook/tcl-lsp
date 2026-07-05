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
"""snatpool -- Assigns the specified SNAT pool or SNAT pool member to the current connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/snatpool.html"


_av = make_av(_SOURCE)


@register
class SnatpoolCommand(CommandDef):
    name = "snatpool"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="snatpool",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Assigns the specified SNAT pool or SNAT pool member to the current connection.",
                synopsis=("snatpool SNAT_POOL_OBJ (member IP_ADDR)?",),
                snippet=(
                    "Causes the pool of addresses identified by <snatpool_name> to be used\n"
                    "as translation addresses to create a SNAT."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [TCP::local_port] == 531 } {\n"
                    "     snatpool chat_snatpool\n"
                    "}\n"
                    "  elseif { [TCP::local_port] == 25 } {\n"
                    "     snatpool smtp_snatpool member 10.20.30.40\n"
                    " }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="snatpool SNAT_POOL_OBJ (member IP_ADDR)?",
                    arg_values={
                        0: (
                            _av(
                                "member",
                                "snatpool member",
                                "snatpool SNAT_POOL_OBJ (member IP_ADDR)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SNAT_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
