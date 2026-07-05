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
"""priority -- Sets the order of execution for iRule events."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/priority.html"


@register
class PriorityCommand(CommandDef):
    name = "priority"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="priority",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the order of execution for iRule events.",
                synopsis=("priority EVENT_PRIORITY",),
                snippet=(
                    "The priority command is used as an attribute associated with any iRule\n"
                    "event. When the iRules are loaded into the internal iRules engine for a\n"
                    "given virtual server, they are stored in a table with the event name\n"
                    "and a priority (with a default of 500).\n"
                    "Lower numbered priority events are evaluated before higher numbered\n"
                    "priority events: When an event is triggered an event, the irules engine\n"
                    "passes control to each of the code blocks for that given event in the\n"
                    "order of lowest to highest priority."
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENT_ACCEPTED {\n       log "Client [IP::remote_addr] connected"\n    }'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="priority EVENT_PRIORITY",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            irules_top_level_only=True,
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
