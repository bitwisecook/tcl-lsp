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
"""REWRITE::enable -- Changes the REWRITE plugin from passthrough to full patching mode."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/REWRITE__enable.html"


@register
class RewriteEnableCommand(CommandDef):
    name = "REWRITE::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="REWRITE::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Changes the REWRITE plugin from passthrough to full patching mode.",
                synopsis=("REWRITE::enable",),
                snippet=(
                    "Changes the REWRITE plugin from passthrough to full patching mode. A\n"
                    "place where this might be helpful would be a POST request where REWRITE\n"
                    "would modify the post body unnecessarily, so we disable it. However, we\n"
                    "want REWRITE to modify the response, so we would enable it later in the\n"
                    "HTTP_RESPONSE. Use of this command can be extremely tricky to get\n"
                    "exactly right; its use is not recommended in the majority of cases"
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="REWRITE::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ACCESS", "FASTHTTP", "REWRITE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
