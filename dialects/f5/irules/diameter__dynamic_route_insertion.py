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
"""DIAMETER::dynamic_route_insertion -- Set whether dynamic route insertion is enabled."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__dynamic_route_insertion.html"


@register
class DiameterDynamicRouteInsertionCommand(CommandDef):
    name = "DIAMETER::dynamic_route_insertion"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::dynamic_route_insertion",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set whether dynamic route insertion is enabled.",
                synopsis=("DIAMETER::dynamic_route_insertion ( BOOLEAN )?",),
                snippet=(
                    'If status is set to "enabled", a dynamic route will be created for this connection.\n'
                    "\n"
                    'This value, once set, remains for the life of the connection.  After the connection is closed, this route will be removed once "timeout" seconds have elapsed.  The default timeout is set by the configuration option "dynamic-route-timeout".\n'
                    "\n"
                    "The zero-argument form of this command returns whether the setting is enabled on the current connection."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '                if { ([IP::address] starts_with "192.168.") } {\n'
                    "                    DIAMETER::dynamic_route_insertion disabled\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::dynamic_route_insertion ( BOOLEAN )?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
