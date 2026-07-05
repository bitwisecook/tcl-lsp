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
"""TMM::cmp_count -- Provides the active number of TMM instances running."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TMM__cmp_count.html"


@register
class TmmCmpCountCommand(CommandDef):
    name = "TMM::cmp_count"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TMM::cmp_count",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides the active number of TMM instances running.",
                synopsis=("TMM::cmp_count",),
                snippet=(
                    "This command provides the active number of TMM instances running.\n"
                    "To determine the blade the iRule is currently executing on, see the\n"
                    "TMM::cmp_group page. To determine the CPU ID an iRule is currently\n"
                    "executing on within a blade, see the TMM::cmp_unit page."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [TMM::cmp_count] >= 2 } {\n"
                    "    set cmpstatus 1\n"
                    "  } else { set cmpstatus 0 }\n"
                    "}"
                ),
                return_value="Returns the active number of TMM instances running.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TMM::cmp_count",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
