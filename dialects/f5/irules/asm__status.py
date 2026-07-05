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
"""ASM::status -- Returns the current status of the request or response."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__status.html"


@register
class AsmStatusCommand(CommandDef):
    name = "ASM::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the current status of the request or response.",
                synopsis=("ASM::status",),
                snippet=(
                    "Returns the current status of the request or response\n"
                    "Returns one of the following values:\n"
                    "  + Alarm - there are violations and alarm has been raised, but\n"
                    "    request or response is not blocked. This does not apply to\n"
                    "    violations that are in staging mode. This value will also be\n"
                    "    returned if the request had violations but was unblocked using\n"
                    "    a previously called ASM::unblock command.\n"
                    "  + Blocked - violations caused the request/response to be\n"
                    "    blocked. This does not apply to violations that are in staging\n"
                    "    mode.\n"
                    "  + Clear - no violations found"
                ),
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE {\n"
                    '    #log local0.debug "\\[ASM::status\\] = [ASM::status]"\n'
                    '    if { [ASM::status] equals "alarmed" } {\n'
                    "        set x [ASM::violation_data]\n"
                    '        HTTP::header insert X-ASM "violation=[lindex $x 0] supportid=[lindex $x 1]"\n'
                    '        #log local0.debug "DEBUG02: violation=[lindex $x 0] supportid=[lindex $x 1]"\n'
                    "    }\n"
                    "}"
                ),
                return_value="* Alarm * Blocked * Clear",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::status",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
