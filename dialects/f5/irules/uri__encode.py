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
"""URI::encode -- Returns an encoded version of a given URI."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__encode.html"


@register
class UriEncodeCommand(CommandDef):
    name = "URI::encode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::encode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an encoded version of a given URI.",
                synopsis=("URI::encode URI_STRING",),
                snippet=(
                    "Returns the encoded version of the given URI.\n"
                    "For details on URI encoding, see RFC3986, section 2.1. Percent-Encoding.\n"
                    "\n"
                    "This command is equivalent to the BIG-IP 4.X variable encode_uri."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  set my_parameter_value "my URL encoded parameter value with metacharacters (&*@#[])"\n'
                    '  log local0. "The encoded version of \\"$my_parameter_value\\" is \\"[URI::encode $my_parameter_value]\\""\n'
                    '  HTTP::redirect "/path?parameter=[URI::encode $my_parameter_value]"\n'
                    "}"
                ),
                return_value="Returns an encoded version of a given URI.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::encode URI_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            taint_transform=TaintColour.URL_ENCODED | TaintColour.CRLF_FREE,
            taint_double_encode_colour=TaintColour.URL_ENCODED,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_URI,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
