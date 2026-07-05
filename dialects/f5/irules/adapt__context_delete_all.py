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
"""ADAPT::context_delete_all -- Deletes all dynamic contexts."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__context_delete_all.html"


@register
class AdaptContextDeleteAllCommand(CommandDef):
    name = "ADAPT::context_delete_all"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::context_delete_all",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deletes all dynamic contexts.",
                synopsis=("ADAPT::context_delete_all",),
                snippet=(
                    "Deletes all dynamic contexts on both sides of the virtual\n"
                    "server, making the static context the current context. This\n"
                    "is done automatically when the last of a connection flow and\n"
                    "its peer is torn down, so normally need not be called.\n"
                    "\n"
                    "Syntax:\n"
                    "\n"
                    "ADAPT::context_delete_all"
                ),
                source=_SOURCE,
                examples=(
                    "# Conditionally revert to static contexts after request processed\n"
                    "# (contrived example, probably not useful).\n"
                    "when HTTP_PROXY_REQUEST {\n"
                    "    if {$revert_to_profile} {\n"
                    "        ADAPT::context_delete_all\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::context_delete_all",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
