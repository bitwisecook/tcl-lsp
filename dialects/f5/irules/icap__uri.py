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
"""ICAP::uri -- Sets or returns the ICAP request URI."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ICAP__uri.html"


@register
class IcapUriCommand(CommandDef):
    name = "ICAP::uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ICAP::uri",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns the ICAP request URI.",
                synopsis=("ICAP::uri (URI_STRING)?",),
                snippet="The ICAP::uri command sets or returns the ICAP request URI.",
                source=_SOURCE,
                examples=(
                    "when ICAP_REQUEST {\n"
                    '                if {[ICAP::uri] contains "movie"} {\n'
                    "                    ICAP::uri http://icap.mydomain.org/video\n"
                    "                }\n"
                    "            }"
                ),
                return_value="Returns the ICAP request URI.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ICAP::uri (URI_STRING)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ICAP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
