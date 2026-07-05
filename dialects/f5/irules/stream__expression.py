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
"""STREAM::expression -- Replaces the expression in a Stream profile with another expression."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STREAM__expression.html"


@register
class StreamExpressionCommand(CommandDef):
    name = "STREAM::expression"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STREAM::expression",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Replaces the expression in a Stream profile with another expression.",
                synopsis=("STREAM::expression EXPRESSION",),
                snippet=(
                    "Replaces the stream expression in the Stream profile with the specified\n"
                    "value. The syntax is identical to the profile syntax.\n"
                    "Note that this change affects this connection only and is sticky\n"
                    "for the duration of the connection."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "   # Disable the stream filter for all requests\n"
                    "   STREAM::disable\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STREAM::expression EXPRESSION",
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
