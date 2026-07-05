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
"""WS::collect -- This command can be used to collect payload of current Websocket frame."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__collect.html"


_av = make_av(_SOURCE)


@register
class WsCollectCommand(CommandDef):
    name = "WS::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::collect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to collect payload of current Websocket frame.",
                synopsis=("WS::collect ('frame' (LENGTH)? )",),
                snippet=(
                    "WS::collect frame\n"
                    "Collects the entire Websocket frame payload.\n"
                    "\n"
                    "Note that if multiple iRules invoke WS::collect simultaneously,\n"
                    "(perhaps by being called by the same event in multiple iRule scripts)\n"
                    "then the result is undefined.  This is because the amount of payload\n"
                    "collected for the WS_CLIENT_DATA or WS_SERVER_DATA event cannot\n"
                    "satisfy the perhaps differing amounts wanted by the callers. iRules\n"
                    "should arbitrate amoungst themselves to prevent this situation from\n"
                    "occuring, and have only one WS::collect call outstanding at a time."
                ),
                source=_SOURCE,
                examples=("when WS_CLIENT_FRAME {\n    WS::collect frame\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::collect ('frame' (LENGTH)? )",
                    arg_values={
                        0: (_av("frame", "WS::collect frame", "WS::collect ('frame' (LENGTH)? )"),)
                    },
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
