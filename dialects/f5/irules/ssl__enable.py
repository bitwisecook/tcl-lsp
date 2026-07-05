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
"""SSL::enable -- Re-enables SSL processing."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__enable.html"


_av = make_av(_SOURCE)


@register
class SslEnableCommand(CommandDef):
    name = "SSL::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Re-enables SSL processing.",
                synopsis=("SSL::enable (clientside | serverside)?",),
                snippet=(
                    "Re-enables SSL processing.\n"
                    "\n"
                    "Note that the proper use of this command is highly protocol dependent and not recommended in the majority of cases. To selectively enable SSL processing, you should instead use an ssl profile and then use SSL::disable to selectively disable SSL processing.\n"
                    "\n"
                    "If a server-side connection is already established, this command will immediately cause the system to generate a Client Hello to a server, even if an iRule subsequently invokes SSL::disable serverside."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    if { !([IP::addr [IP::client_addr] eq 10.0.0.0/8]) } {\n"
                    "        SSL::enable\n"
                    "        TCP::release\n"
                    "        set sslenable 0\n"
                    "    }\n"
                    "}"
                ),
                return_value="SSL::enable [clientside | serverside] Re-enables SSL processing on one side of the LTM. serverside parameter may optionally be specified to indicate the context in which SSL will be enabled. By default, it will be enabled in the current context (i.e.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::enable (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "SSL::enable clientside",
                                "SSL::enable (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "SSL::enable serverside",
                                "SSL::enable (clientside | serverside)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
