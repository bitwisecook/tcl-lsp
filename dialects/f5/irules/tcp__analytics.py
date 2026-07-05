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
"""TCP::analytics -- Enable/disable AVR TCP stat reporting, and/or attach a user-defined string to categorize the connection for statistics collection purposes."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__analytics.html"


@register
class TcpAnalyticsCommand(CommandDef):
    name = "TCP::analytics"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::analytics",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable/disable AVR TCP stat reporting, and/or attach a user-defined string to categorize the connection for statistics collection purposes.",
                synopsis=("TCP::analytics (enable | disable | key (KEY)?)",),
                snippet=(
                    'Enables or disables AVR TCP stat reporting ("analytics") for this connection and/or assigns user-defined keys.\n'
                    "\n"
                    "TCP::analytics enable\n"
                    "    Enables analytics on this connection. AVR must be provisioned and the virtual must have a tcp-analytics profile attached. Collection will use the configuration in the profile. If the profile is configured to disable analytics by default, this gives users the ability to collect statistics by exception only.\n"
                    "\n"
                    "TCP::analytics disable\n"
                    "    Disables analytics on this connection."
                ),
                source=_SOURCE,
                examples=(
                    "rt collection for one subnet only.\n"
                    "     when CLIENT_ACCEPTED {\n"
                    "         if [IP::addr [IP::client_addr]/8 equals 10.0.0.0] {\n"
                    "             TCP::analytics enable\n"
                    "         }\n"
                    "     }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::analytics (enable | disable | key (KEY)?)",
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
