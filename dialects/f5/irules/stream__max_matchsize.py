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
"""STREAM::max_matchsize -- Sets a maximum number of bytes that the system can buffer during partial matches."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STREAM__max_matchsize.html"


@register
class StreamMaxMatchsizeCommand(CommandDef):
    name = "STREAM::max_matchsize"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STREAM::max_matchsize",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets a maximum number of bytes that the system can buffer during partial matches.",
                synopsis=("STREAM::max_matchsize SIZE",),
                snippet=(
                    "Sets the maximum size, in bytes, that the system can buffer during\n"
                    "partial matches. The default value is 4096.\n"
                    "The STREAM profile will buffer data for partial matches; if more than\n"
                    "max_matchsize would be buffered, the connection will be torn down. This\n"
                    "way a regex like foobarbaz+ won't keep matching until the box runs\n"
                    "out of memory. The default is 4K, and STREAM::max_matchsize can be\n"
                    "use to set it to something else."
                ),
                source=_SOURCE,
                examples=("when HTTP_RESPONSE {\n    STREAM::max_matchsize 2048\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STREAM::max_matchsize SIZE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
