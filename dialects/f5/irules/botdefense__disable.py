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
"""BOTDEFENSE::disable -- Disables processing by Bot Defense on the connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__disable.html"


@register
class BotdefenseDisableCommand(CommandDef):
    name = "BOTDEFENSE::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables processing by Bot Defense on the connection.",
                synopsis=("BOTDEFENSE::disable",),
                snippet=(
                    "Disables processing and blocking of the request by Bot Defense for the duration of the current TCP connection, or until BOTDEFENSE::enable is called.\n"
                    "When called from events that occur before Bot Defense processing such as HTTP_REQUEST then the commands takes effect on the current request. Otherwise, if invoked in the BOTDEFENSE_REQUEST, BOTDEFENSE_ACTION or any other event that occurs after Bot Defense processing then the command will take effect only on the following request on the same connection."
                ),
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Disabling Bot Defense for a netmask of client IP addresses\n"
                    "when CLIENT_ACCEPTED {\n"
                    "    if {[IP::addr [IP::client_addr] equals 10.10.10.0/24]} {\n"
                    "        BOTDEFENSE::disable\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
