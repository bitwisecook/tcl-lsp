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
"""LINK::nexthop -- Returns the MAC address of the next hop."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LINK__nexthop.html"


_av = make_av(_SOURCE)


@register
class LinkNexthopCommand(CommandDef):
    name = "LINK::nexthop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LINK::nexthop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the MAC address of the next hop.",
                synopsis=("LINK::nexthop ('id' | 'type' | 'name')?",),
                snippet=(
                    "Returns the MAC address of the next hop. Returns the broadcast address\n"
                    "ff:ff:ff:ff:ff:ff when called before a serverside connection has been\n"
                    "established.\n"
                    "Note:\n"
                    "  * In 11.4, you can use LINK::nexthop with sub-commands to retrieve\n"
                    "    the id, type and name of the next hop, respectively. Without\n"
                    "    sub-commands, LINK::nexthop returns the MAC address as before."
                ),
                source=_SOURCE,
                examples=(
                    "# Logging example\n"
                    "when CLIENT_ACCEPTED {\n"
                    '        log local0. "\\[LINK::lasthop\\]: [LINK::lasthop], \\[LINK::nexhop\\]: [LINK::nexthop]"\n'
                    "}"
                ),
                return_value="LINK::nexthop [id]",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LINK::nexthop ('id' | 'type' | 'name')?",
                    arg_values={
                        0: (
                            _av(
                                "id", "LINK::nexthop id", "LINK::nexthop ('id' | 'type' | 'name')?"
                            ),
                            _av(
                                "type",
                                "LINK::nexthop type",
                                "LINK::nexthop ('id' | 'type' | 'name')?",
                            ),
                            _av(
                                "name",
                                "LINK::nexthop name",
                                "LINK::nexthop ('id' | 'type' | 'name')?",
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
