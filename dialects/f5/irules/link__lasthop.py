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
"""LINK::lasthop -- Returns the MAC address of the last hop."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LINK__lasthop.html"


_av = make_av(_SOURCE)


@register
class LinkLasthopCommand(CommandDef):
    name = "LINK::lasthop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LINK::lasthop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the MAC address of the last hop.",
                synopsis=("LINK::lasthop ('id' | 'type' | 'name')?",),
                snippet=(
                    "Returns the MAC address of the last hop.\n"
                    "Note:\n"
                    "  * In 11.4, you can extend LINK::lasthop with sub-commands to retrieve\n"
                    "    the lasthop id, type, name, respectively. Without sub-command,\n"
                    "    LINK::lasthop returns the MAC address as before."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  set lastmac [LINK::lasthop]\n"
                    "  session add uie [IP::client_addr] $lastmac 180\n"
                    "}"
                ),
                return_value="LINK::lasthop [id]",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LINK::lasthop ('id' | 'type' | 'name')?",
                    arg_values={
                        0: (
                            _av(
                                "id", "LINK::lasthop id", "LINK::lasthop ('id' | 'type' | 'name')?"
                            ),
                            _av(
                                "type",
                                "LINK::lasthop type",
                                "LINK::lasthop ('id' | 'type' | 'name')?",
                            ),
                            _av(
                                "name",
                                "LINK::lasthop name",
                                "LINK::lasthop ('id' | 'type' | 'name')?",
                            ),
                        )
                    },
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
