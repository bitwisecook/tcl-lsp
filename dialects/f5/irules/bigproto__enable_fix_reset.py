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
"""BIGPROTO::enable_fix_reset -- Enable or Disable Reset of FIX Protocol Connections"""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BIGPROTO__enable_fix_reset.html"


@register
class BigprotoEnableFixResetCommand(CommandDef):
    name = "BIGPROTO::enable_fix_reset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BIGPROTO::enable_fix_reset",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable or Disable Reset of FIX Protocol Connections",
                synopsis=("BIGPROTO::enable_fix_reset BOOLEAN",),
                snippet="When set to disabled, TCP RST frame will not be sent when BIG-IP detects there is a hash collision on ePVA offloading of FIX flows. Instead, it will try to re-offload the connection.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    BIGPROTO::enable_fix_reset true\n"
                    "    BIGPROTO::enable_fix_reset false\n"
                    "            }"
                ),
                return_value="none",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BIGPROTO::enable_fix_reset BOOLEAN",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
