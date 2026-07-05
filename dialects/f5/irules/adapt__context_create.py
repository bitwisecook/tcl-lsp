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
"""ADAPT::context_create -- Creates a new dynamic adaptation context."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__context_create.html"


@register
class AdaptContextCreateCommand(CommandDef):
    name = "ADAPT::context_create"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::context_create",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a new dynamic adaptation context.",
                synopsis=("ADAPT::context_create (ADAPT_SIDE)? NAME",),
                snippet=(
                    "Creates a new dynamic adaptation context in the ADAPT filter on\n"
                    "the current or specified side of the virtual server connection\n"
                    "for which the iRule is being executed. Maybe called mulitple\n"
                    "times to dynamically create chains of adaptation contexts.\n"
                    "\n"
                    "Syntax:\n"
                    "\n"
                    "ADAPT::context_create <name>\n"
                    "\n"
                    "    * Creates a dynamic context on the current side.\n"
                    "      This must be called from the request-adapt side, so has\n"
                    "      the same effect as ADAPT::context_create request <name>."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    # Configure a response context from the current (response) side.\n"
                    "    ADAPT::select $rsp_ctx2 ivs-icap-rsp2\n"
                    "    ADAPT::timeout $rsp_ctx2 2000\n"
                    "}"
                ),
                return_value="Returns the context handle.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::context_create (ADAPT_SIDE)? NAME",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP", "REQUESTADAPT"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
