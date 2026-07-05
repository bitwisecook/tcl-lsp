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
"""STREAM::replace -- Changes a replacement string in the Stream profile."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STREAM__replace.html"


@register
class StreamReplaceCommand(CommandDef):
    name = "STREAM::replace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STREAM::replace",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Changes a replacement string in the Stream profile.",
                synopsis=("STREAM::replace (TARGET_STRING)?",),
                snippet=(
                    "Changes the specified target replacement string in the Stream profile.\n"
                    "This command is not sticky and is applied only once during the current\n"
                    "match. If the target expression is missing, the replacement is skipped."
                ),
                source=_SOURCE,
                examples=(
                    "when STREAM_MATCHED {\n"
                    "    set server [string tolower [STREAM::match]]\n"
                    '    if {$server contains "mail"} {\n'
                    '        STREAM::replace "webmail.yourdomain.com/$mailhost"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STREAM::replace (TARGET_STRING)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
