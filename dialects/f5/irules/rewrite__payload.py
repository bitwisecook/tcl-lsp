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
"""REWRITE::payload -- Queries for or manipulates REWRITE payload."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/REWRITE__payload.html"


@register
class RewritePayloadCommand(CommandDef):
    name = "REWRITE::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="REWRITE::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Queries for or manipulates REWRITE payload.",
                synopsis=(
                    "REWRITE::payload (LENGTH | (OFFSET LENGTH))?",
                    "REWRITE::payload length",
                    "REWRITE::payload replace OFFSET LENGTH PAYLOAD",
                ),
                snippet=(
                    "Queries for or manipulates REWRITE payload (content) information. With\n"
                    "this command, you can retrieve content, query for content size, or\n"
                    "replace a certain amount of content."
                ),
                source=_SOURCE,
                examples=(
                    "when REWRITE_RESPONSE_DONE {\n"
                    "    # The rewrite_response_done event isn't absolutely necessary because browser will just ignore any html tags that it doesn't recongnize.\n"
                    "    # However, it will be cleaner if we remove it nevertheless\n"
                    "\n"
                    "    set data [REWRITE::payload]\n"
                    "    # Find the tags we inserted\n"
                    "    set start [string first {<apm_do_not_touch>} $data]\n"
                    "    set end [string last {</apm_do_not_touch>} $data]\n"
                    "    # Determines the amount of characters to remove"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="REWRITE::payload (LENGTH | (OFFSET LENGTH))?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"REWRITE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
