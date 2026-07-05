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
"""ASM::unblock -- Overrides the blocking action for a request that had blocking violation."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__unblock.html"


@register
class AsmUnblockCommand(CommandDef):
    name = "ASM::unblock"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::unblock",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Overrides the blocking action for a request that had blocking violation.",
                synopsis=("ASM::unblock",),
                snippet=(
                    "Overrides the blocking action for a request that had blocking\n"
                    "violations. Consequently, the request will be forwarded to the origin\n"
                    'server and also marked with a special "unblocked" flag which can be\n'
                    "viewed in the request log. If the present request was not supposed to\n"
                    "be blocked then the command has no effect."
                ),
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE {\n"
                    "  set i 0\n"
                    "  foreach {viol} [ASM::violation names]{\n"
                    "  if {$viol eq VIOLATION_ILLEGAL_PARAMETER} {\n"
                    "    set details [lindex [ASM::violation details] $i]\n"
                    '    set param_name [b64decode [llookup $details "param_data.param_name"]]\n'
                    "    #remove the bad parameter from the QS - does not work right in all cases, just for illustration!\n"
                    '    regsub -all "\\?.*($param_name=^\\&*)" [HTTP::uri] "?" $new_uri\n'
                    "    HTTP::uri $new_uri\n"
                    "    ASM::unblock\n"
                    "  }\n"
                    "  set i [expr {$i+1}]\n"
                    "  }\n"
                    "\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::unblock",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
