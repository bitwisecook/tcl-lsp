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
"""AVR::enable -- Enables the AVR plugin for the current connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AVR__enable.html"


@register
class AvrEnableCommand(CommandDef):
    name = "AVR::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AVR::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables the AVR plugin for the current connection.",
                synopsis=("AVR::enable",),
                snippet=(
                    "Enables the AVR plugin for the current connection. AVR will remain\n"
                    "enabled on the current connection until it is closed or\n"
                    "AVR::disable is called.\n"
                    "\n"
                    "Note that enabling AVR alone within the iRule only ensures the\n"
                    "message reaches the AVR plugin, it doesn't ensure that statistics\n"
                    "will be gathered."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AVR::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
