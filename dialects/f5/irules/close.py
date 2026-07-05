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
"""close -- Closes an existing sideband connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/close.html"


@register
class CloseCommand(CommandDef):
    name = "close"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="close",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Closes an existing sideband connection.",
                synopsis=("close CONNECTION",),
                snippet="This command closes an existing sideband connection. It is one of several commands that make up the ability to create sideband connections from iRules.",
                source=_SOURCE,
                examples=(
                    "# Open a sideband connection with a connection timeout of 100 ms and an idle timeout of 30 seconds\n"
                    "#   to a local virtual server name sideband_virtual_server\n"
                    "set conn_id [connect -timeout 100 -idle 30 -status conn_status sideband_virtual_server]\n"
                    "\n"
                    "# Same as above, but use an external host IP:port instead of a virtual server name\n"
                    "set conn_id [connect -timeout 100 -idle 30 -status conn_status 10.0.0.10:80]\n"
                    "\n"
                    "# close the connection\n"
                    "close conn_id"
                ),
                return_value="close <connection> closes an existing connection",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="close CONNECTION",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
