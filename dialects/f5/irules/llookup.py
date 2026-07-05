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
"""llookup -- Returns a list of values corresponding to the given key in a multimap."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/llookup.html"


@register
class LlookupCommand(CommandDef):
    name = "llookup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="llookup",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a list of values corresponding to the given key in a multimap.",
                synopsis=("llookup MMAP KEY",),
                snippet=(
                    "A *multimap* is a flat Tcl list of `{key value}` pairs — the same "
                    "structure returned by `[ASM::violation details]`.  Because the same "
                    "key can appear more than once, `llookup` returns **a list** of every "
                    "value whose key matches *KEY*.\n"
                    "\n"
                    "Returns an empty string when *KEY* is absent or *MMAP* is not a "
                    "properly structured multimap.\n"
                    "\n"
                    "Equivalent Tcl (what `llookup` replaces):\n"
                    "```tcl\n"
                    "set r {}\n"
                    "foreach pair $mmap {\n"
                    "    if {[lindex $pair 0] eq $key} {\n"
                    "        lappend r [lindex $pair 1]\n"
                    "    }\n"
                    "}\n"
                    "```"
                ),
                return_value=(
                    "A Tcl list of values matching *KEY*.  When used with "
                    "`[ASM::violation details]`, binary values such as "
                    "`http_sub_violation` and `sig_data.kw_data.buffer` are "
                    "base64-encoded."
                ),
                examples=(
                    "# Iterate violations in parallel using llookup\n"
                    "when ASM_REQUEST_DONE {\n"
                    "    set details [ASM::violation details]\n"
                    "    foreach viol_name       [llookup $details viol_name] \\\n"
                    "            sanity_status   [llookup $details http_sanity_checks_status] \\\n"
                    "            sub_viol_status [llookup $details http_sub_violation_status] {\n"
                    '        log local0.info "$viol_name $sanity_status $sub_viol_status"\n'
                    "    }\n"
                    "}"
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="llookup MMAP KEY",
                ),
            ),
            validation=ValidationSpec(arity=Arity(min=2, max=2)),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
