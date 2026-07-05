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
"""SIP::record-route -- This command allows you get get information in the SIP record-route header."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__record_route.html"


_av = make_av(_SOURCE)


@register
class SipRecordRouteCommand(CommandDef):
    name = "SIP::record-route"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::record-route",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you get get information in the SIP record-route header.",
                synopsis=("SIP::record-route (INDEX | 'top')",),
                snippet=(
                    "This command allows you get get information in the SIP record-route header.\n"
                    "\n"
                    "Synax\n"
                    "\n"
                    "SIP::record-route <index>\n"
                    "\n"
                    '     * Get SIP header "route" at index\n'
                    "\n"
                    ' <index> is a numeric zero-based index or the keyword "top". The "top" keyword" will acess the first element as opposed to the first line of the record-route headers.'
                ),
                source=_SOURCE,
                examples=("when SIP_REQUEST {\n  log local0. [SIP::recordroute top]\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::record-route (INDEX | 'top')",
                    arg_values={
                        0: (
                            _av(
                                "top", "SIP::record-route top", "SIP::record-route (INDEX | 'top')"
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
