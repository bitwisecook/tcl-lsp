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

"""regex::quote -- Escape regex metacharacters in a string."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour

from ._base import register


@register
class RegexQuoteCommand(CommandDef):
    name = "regex::quote"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="regex::quote",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Escape regex metacharacters in a string.",
                synopsis=("regex::quote STRING",),
                snippet=(
                    "Returns *STRING* with all regular-expression\n"
                    "metacharacters (``[ ] { } ( ) * + ? . \\\\ ^ $ |``)\n"
                    "backslash-escaped so it can be used as a literal\n"
                    "pattern in ``regexp`` or ``regsub``."
                ),
                examples=(
                    "set safe_pattern [regex::quote $user_input]\n"
                    "if {[regexp $safe_pattern $haystack]} { ... }"
                ),
                return_value="Returns a regex-escaped string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="regex::quote STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            taint_transform=TaintColour.REGEX_LITERAL,
            taint_double_encode_colour=TaintColour.REGEX_LITERAL,
        )
