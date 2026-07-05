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
"""ADAPT::select -- Sets or returns the internal virtual server (IVS) selection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__select.html"


@register
class AdaptSelectCommand(CommandDef):
    name = "ADAPT::select"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::select",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns the internal virtual server (IVS) selection.",
                synopsis=("ADAPT::select (ADAPT_CTX)? (ADAPT_SIDE)? (NAME)?",),
                snippet=(
                    "The ADAPT::select command returns or selects the name of\n"
                    "the internal virtual server (IVS) associated with the ADAPT\n"
                    "filter on the current or specified side of the virtual server\n"
                    "connection for which the iRule is being executed."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    '     if { [HTTP::header "Content-Type"] contains "image" } {\n'
                    "        ADAPT::select ivs-icap-image\n"
                    "        ADAPT::preview_size 10000\n"
                    "        ADAPT::enable yes\n"
                    "     }\n"
                    '     if { [HTTP::header "Content-Type"] contains "video" } {\n'
                    "        ADAPT::select ivs-icap-video\n"
                    "        ADAPT::preview_size 30000\n"
                    "        ADAPT::enable yes\n"
                    "     }\n"
                    "}"
                ),
                return_value="Returns the current or new internal virtual server (IVS) name.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::select (ADAPT_CTX)? (ADAPT_SIDE)? (NAME)?",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
