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
"""redirect -- Redirects an HTTP request to the specific location."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register
from .http__redirect import HttpRedirectCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/redirect.html"


@register
class RedirectCommand(CommandDef):
    name = "redirect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="redirect",
            deprecated_replacement=HttpRedirectCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Redirects an HTTP request to the specific location.",
                synopsis=("redirect to HOST_URI",),
                snippet=(
                    "Redirects an HTTP request to a specific location. The location can be\n"
                    "either a host name or a URI. This is a BIG-IP 4.X statement, provided\n"
                    "for backward compatibility. You can use the equivalent 9.X command\n"
                    "HTTP::redirect instead."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    # HTTP::redirect, HTTP::host and HTTP::uri should be used instead\n"
                    '    redirect to "https://[http_host][http_uri]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="redirect to HOST_URI",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.RESPONSE_COMMIT,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
