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
"""HTTP::fallback -- Specifies or overrides a fallback host specified in the HTTP profile."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__fallback.html"


@register
class HttpFallbackCommand(CommandDef):
    name = "HTTP::fallback"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::fallback",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Specifies or overrides a fallback host specified in the HTTP profile.",
                synopsis=("HTTP::fallback <host>",),
                snippet="Specifies or overrides the fallback host specified in the HTTP profile.",
                source=_SOURCE,
                examples=(
                    'when LB_FAILED {\n  HTTP::fallback "http://siteunavailable.mysite.com/"\n}'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.SETTER,
                    synopsis="HTTP::fallback <host>",
                    arity=Arity(1, 1),
                    mutator=True,
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            event_requires=EventRequires(
                transport="tcp",
                profiles=frozenset({"HTTP", "FASTHTTP"}),
                also_in=frozenset({"LB_FAILED", "MR_FAILED"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
