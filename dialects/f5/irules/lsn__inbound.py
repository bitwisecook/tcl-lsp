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
"""LSN::inbound -- Disable inbound mapping for translation address and port associated with the current connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, _LSN_EVENT_REQUIRES, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__inbound.html"


@register
class LsnInboundCommand(CommandDef):
    name = "LSN::inbound"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::inbound",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disable inbound mapping for translation address and port associated with the current connection.",
                synopsis=("LSN::inbound disable",),
                snippet="Disable inbound mapping for translation address and port associated with the current connection.",
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    LSN::inbound disable\n}"),
                return_value="LSN::inbound disable - Inbound connections can be permitted for a particular LSN pool to provide end-point independent filtering, described in RFC 4787.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::inbound disable",
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
