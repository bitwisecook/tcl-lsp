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
"""http_uri -- F5 iRules command `http_uri`."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register
from .http__uri import HttpUriCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/http_uri.html"


@register
class DeprecatedHttpUriCommand(CommandDef):
    name = "http_uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http_uri",
            deprecated_replacement=HttpUriCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a URL, but does not include the protocol and the fully qualified domain name (FQDN).",
                synopsis=("http_uri",),
                snippet=(
                    "Returns a URL, but does not include the protocol and the fully\n"
                    "qualified domain name (FQDN). For example, if the URL is\n"
                    "http://www.mysite.com/buy.asp, then the URI is /buy.asp. This command\n"
                    "is a BIG-IP 4.X variable, provided for backward-compatibility. You can\n"
                    "use the equivalent 9.x command HTTP::uri instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="http_uri",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_URI,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
