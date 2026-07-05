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
"""urlcatquery -- Query the URL for URL categorization."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register
from .category__lookup import CategoryLookupCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/urlcatquery.html"


@register
class UrlcatqueryCommand(CommandDef):
    name = "urlcatquery"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="urlcatquery",
            deprecated_replacement=CategoryLookupCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Query the URL for URL categorization.",
                synopsis=("urlcatquery URL_STRING",),
                snippet=(
                    "This command is similar in functionality to whereis command of geoip.\n"
                    "This will be available from HTTP_REQUEST irule event. It takes the URL\n"
                    "as the input. The input could be a URL string or an IPV4 address. IPV6\n"
                    "addresses are not currently supported. iRule returns the URL categories\n"
                    "returned by the urlcat library."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    set input_url [HTTP::host][HTTP::uri]\n"
                    "    set urlcat [urlcatquery  $input_url]\n"
                    '    log local0. "INPUT-URL: $input_url"\n'
                    '    log local0. "Category - $urlcat"\n'
                    "    CLASSIFY::urlcat add $urlcat\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="urlcatquery URL_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DATA_GROUP,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
