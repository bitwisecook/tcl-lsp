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
"""lasthop -- Sets the lasthop of an IP connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/lasthop.html"


@register
class LasthopCommand(CommandDef):
    name = "lasthop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lasthop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the lasthop of an IP connection.",
                synopsis=("lasthop (VLAN_OBJ)? (IP_ADDR | MAC_ADDR)",),
                snippet=(
                    "Sets the lasthop of a IP connection. The lasthop is the MAC destination\n"
                    "for packets going back to the client. This is usually the router\n"
                    "(gateway) that forwards the client's packets to the BIG-IP (if \"auto\n"
                    'lasthop" is set), or is determined by the IP routing table. This\n'
                    "command lets you specify the lasthop to use for a particular\n"
                    "connection."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  lasthop external 01:23:45:ab:cd:ef\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lasthop (VLAN_OBJ)? (IP_ADDR | MAC_ADDR)",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True, also_in=frozenset({"PERSIST_DOWN"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
