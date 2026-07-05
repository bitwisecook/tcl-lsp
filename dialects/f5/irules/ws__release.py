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
"""WS::release -- Releases the data collected using WS::collect."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__release.html"


@register
class WsReleaseCommand(CommandDef):
    name = "WS::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Releases the data collected using WS::collect.",
                synopsis=("WS::release",),
                snippet=(
                    "WS::release\n"
                    "    Releases the data collected via WS::collect. Unless a subsequent WS::collect command was issued, there is no need to use the WS::release command inside of the WS_CLIENT_DATA and WS_SERVER_DATA events, since (in these cases) the data is implicitly released."
                ),
                source=_SOURCE,
                examples=(
                    "when WS_CLIENT_FRAME {\n    WS::collect frame 1000\n    set clen 1000\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::release",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
