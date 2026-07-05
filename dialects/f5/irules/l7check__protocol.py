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
"""L7CHECK::protocol -- Set or get L7 protocol value."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/L7CHECK__protocol.html"


@register
class L7checkProtocolCommand(CommandDef):
    name = "L7CHECK::protocol"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="L7CHECK::protocol",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set or get L7 protocol value.",
                synopsis=(
                    "L7CHECK::protocol set VALUE",
                    "L7CHECK::protocol get",
                ),
                snippet="The L7CHECK::protocol commands allow you to set or retrieve L7 protocol value.",
                source=_SOURCE,
                examples=(
                    "when L7CHECK_CLIENT_DATA {\n"
                    '    if { [L7CHECK::protocol get] == "https" } {\n'
                    "        pool clients_https\n"
                    "    } else {\n"
                    "        pool clients_non_https\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="L7CHECK::protocol set VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CONNECTOR", "L7CHECK"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
