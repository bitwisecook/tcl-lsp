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
"""LSN::address -- Explicitly set the translation address regardless of the configured LSN pool."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, _LSN_EVENT_REQUIRES, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__address.html"


@register
class LsnAddressCommand(CommandDef):
    name = "LSN::address"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::address",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Explicitly set the translation address regardless of the configured LSN pool.",
                synopsis=("LSN::address TRANSLATION_ADDR",),
                snippet=(
                    "Explicitly set the translation address regardless of the configured LSN pool.\n"
                    "\n"
                    "The LSN::address command can be used while processing CLIENT_DATA. This event can occur before and after address translation. If this command is used after translation has occurred an error is thrown.\n"
                    "\n"
                    "Agruments:\n"
                    "    LSN::address - Set the explicit translation IPv4 or IPv6 address for the connection in the current context."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    LSN::address 10.0.0.1\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::address TRANSLATION_ADDR",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=_LSN_EVENT_REQUIRES,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
