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
"""HTTP::hsts -- Controls HTTP Strict Transport Security."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__hsts.html"


@register
class HttpHstsCommand(CommandDef):
    name = "HTTP::hsts"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::hsts",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Controls HTTP Strict Transport Security.",
                synopsis=(
                    "HTTP::hsts",
                    "HTTP::hsts mode <enable | disable>",
                    "HTTP::hsts maximum-age <seconds>",
                    "HTTP::hsts include-subdomains <enable | disable>",
                    "HTTP::hsts preload <enable | disable>",
                ),
                snippet="Controls HSTS options on a per-flow basis, overriding the configured values in the HTTP profile.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    if { [HTTP::uri] contains "secure"} {\n'
                    "        HTTP::hsts mode enable\n"
                    "        HTTP::hsts maximum-age 8600\n"
                    "        HTTP::hsts include-subdomains disable\n"
                    "        HTTP::hsts preload enable\n"
                    "    }\n"
                    "}"
                ),
                return_value="With no arguments, returns the currently configured HSTS header value for this connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP::hsts",
                    arity=Arity(0, 0),
                    pure=True,
                ),
            ),
            subcommands={
                "mode": SubCommand(
                    name="mode",
                    arity=Arity(1, 1),
                    detail="Enable or disable HSTS on a per-flow basis.",
                    synopsis="HTTP::hsts mode <enable | disable>",
                    mutator=True,
                ),
                "maximum-age": SubCommand(
                    name="maximum-age",
                    arity=Arity(1, 1),
                    detail="Set HSTS maximum-age on a per-flow basis.",
                    synopsis="HTTP::hsts maximum-age <seconds>",
                    mutator=True,
                ),
                "include-subdomains": SubCommand(
                    name="include-subdomains",
                    arity=Arity(1, 1),
                    detail="Enable or disable HSTS include-subdomains on a per-flow basis.",
                    synopsis="HTTP::hsts include-subdomains <enable | disable>",
                    mutator=True,
                ),
                "preload": SubCommand(
                    name="preload",
                    arity=Arity(1, 1),
                    detail="Enable or disable HSTS preload on a per-flow basis (v13+).",
                    synopsis="HTTP::hsts preload <enable | disable>",
                    mutator=True,
                ),
            },
            validation=ValidationSpec(
                arity=Arity(0, 2),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
