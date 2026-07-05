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
"""HTTP::enable -- Changes the HTTP filter from passthrough to full parsing mode."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__enable.html"


@register
class HttpEnableCommand(CommandDef):
    name = "HTTP::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Changes the HTTP filter from passthrough to full parsing mode.",
                synopsis=("HTTP::enable",),
                snippet=(
                    "Changes the HTTP filter from passthrough to full parsing mode. This\n"
                    "could be useful, for instance, if you need to determine whether or not\n"
                    "HTTP is passing over the connection and enable the HTTP filter\n"
                    "appropriately, or if you have a protocol that is almost but not quite\n"
                    "like HTTP, and you need to re-enable HTTP parsing after temporarily\n"
                    "disabling it.\n"
                    "Use of this command can be extremely tricky to get exactly right; its\n"
                    "use is not recommended in the majority of cases.\n"
                    "Note: This command does not function in certain versions of BIG-IP\n"
                    "(v9.4.0 - v9.4.4)."
                ),
                source=_SOURCE,
                examples=('when HTTP_REQUEST {\nlog local0. "Got request: [HTTP::uri]"\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
