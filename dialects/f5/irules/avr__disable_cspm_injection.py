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
"""AVR::disable_cspm_injection -- Disables CSPM injection for the current connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AVR__disable_cspm_injection.html"


@register
class AvrDisableCspmInjectionCommand(CommandDef):
    name = "AVR::disable_cspm_injection"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AVR::disable_cspm_injection",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables CSPM injection for the current connection.",
                synopsis=("AVR::disable_cspm_injection",),
                snippet="The CSPM (Client Side Performance Monitoring) feature injects JavaScript into HTTP responses to track the Page Load Time metric. This command disables CSPM JavaScropt injection.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    if { [HTTP::status] == 404 } {\n"
                    "        AVR::disable_cspm_injection\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AVR::disable_cspm_injection",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FASTHTTP"}), also_in=frozenset({"AVR_CSPM_INJECTION"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
