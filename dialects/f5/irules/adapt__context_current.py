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
"""ADAPT::context_current -- Gets the current context."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__context_current.html"


@register
class AdaptContextCurrentCommand(CommandDef):
    name = "ADAPT::context_current"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::context_current",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the current context.",
                synopsis=("ADAPT::context_current",),
                snippet=(
                    "Obtains a handle for the current context. The current context\n"
                    "is usually that in which the event occurred from which this\n"
                    "command was issued.\n"
                    "\n"
                    "Syntax:\n"
                    "\n"
                    "ADAPT::context_current"
                ),
                source=_SOURCE,
                examples=(
                    "when ADAPT_REQUEST_RESULT {\n"
                    "    set ctx [ADAPT::context_current]\n"
                    "    if {$ctx == $req_ctx2 && $need_another_ctx} {\n"
                    "        set req_ctx3 [ADAPT::context_create my_req_ctx3]\n"
                    "        ADAPT::select $req_ctx3 ivs-icap-req3\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns the handle of the current context.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::context_current",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"HTTP", "REQUESTADAPT", "RESPONSEADAPT"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
