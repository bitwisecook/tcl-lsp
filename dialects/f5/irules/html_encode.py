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

"""html_encode -- HTML-encode a string (iRules helper alias)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour

from ._base import _IRULES_ONLY, register


@register
class HtmlEncodeAlias(CommandDef):
    name = "html_encode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="html_encode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="HTML-encode a string (alias for HTML::encode).",
                synopsis=("html_encode STRING",),
                snippet=(
                    "Replaces HTML-special characters with their entity\n"
                    "equivalents.  This is a convenience alias for\n"
                    "``HTML::encode``."
                ),
                return_value="Returns an HTML-escaped string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="html_encode STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            taint_transform=TaintColour.HTML_ESCAPED | TaintColour.CRLF_FREE,
            taint_double_encode_colour=TaintColour.HTML_ESCAPED,
        )
