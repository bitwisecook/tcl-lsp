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
"""LB::reselect -- Advance to the next available node in a pool."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__reselect.html"


_av = make_av(_SOURCE)


@register
class LbReselectCommand(CommandDef):
    name = "LB::reselect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::reselect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Advance to the next available node in a pool.",
                synopsis=(
                    "LB::reselect (clone pool POOL_OBJ (member IP_ADDR)?)?",
                    "LB::reselect pool POOL_OBJ (member ((IP_ADDR PORT) |",
                    "LB::reselect snat (automap |",
                    "LB::reselect snatpool SNAT_POOL_OBJ (member IP_ADDR)?",
                ),
                snippet=(
                    "This command is used to advance to the next available node in a pool, either using the load balancing settings of that pool, or by specifying a member explicitly. Note that the reselect may not happen immediately; it may wait until the current iRule event is completely finished executing.\n"
                    "\n"
                    "There is no reselect retry limit built into the command: You MUST implement a limiting mechanism in your iRule using logic similar to that in the examples below."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set def_pool [LB::server pool]\n"
                    "    set lb_fails 0\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::reselect (clone pool POOL_OBJ (member IP_ADDR)?)?",
                    arg_values={
                        0: (
                            _av(
                                "member",
                                "LB::reselect member",
                                "LB::reselect (clone pool POOL_OBJ (member IP_ADDR)?)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"LB_FAILED", "LB_QUEUED", "LB_SELECTED", "PERSIST_DOWN"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
