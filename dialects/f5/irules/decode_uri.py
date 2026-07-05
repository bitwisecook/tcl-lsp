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
"""decode_uri -- Decodes the specified string using HTTP URI encoding."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register
from .uri__decode import UriDecodeCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/decode_uri.html"


@register
class DecodeUriCommand(CommandDef):
    name = "decode_uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="decode_uri",
            deprecated_replacement=UriDecodeCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Decodes the specified string using HTTP URI encoding.",
                synopsis=("decode_uri ANY_CHARS",),
                snippet=(
                    "Decodes the specified string using HTTP URI encoding per RFC2616 and\n"
                    "returns the result. This is a BIG-IP 4.x variable, provided for\n"
                    "backward-compatibiliy. You can use the equivalent 9.X commmand\n"
                    "URI::decode instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="decode_uri ANY_CHARS",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            is_unescape_command=True,
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
