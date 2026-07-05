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
"""URI::escape -- Percent-encodes a URI string (alias for URI::encode)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__encode.html"


@register
class UriEscapeCommand(CommandDef):
    name = "URI::escape"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::escape",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Percent-encodes a URI string (alias for URI::encode).",
                synopsis=("URI::escape URI_STRING",),
                snippet=(
                    "Percent-encodes *URI_STRING* according to RFC 3986.\n"
                    "This is an alias for ``URI::encode``."
                ),
                source=_SOURCE,
                return_value="Returns a percent-encoded URI string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::escape URI_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            taint_transform=TaintColour.URL_ENCODED | TaintColour.CRLF_FREE,
            taint_double_encode_colour=TaintColour.URL_ENCODED,
        )
