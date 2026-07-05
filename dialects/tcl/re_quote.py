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

"""re_quote -- Escape regex metacharacters in a string."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour

from ._base import register


@register
class ReQuoteCommand(CommandDef):
    name = "re_quote"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="re_quote",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Escape regex metacharacters in a string.",
                synopsis=("re_quote STRING",),
                snippet=(
                    "Returns *STRING* with all regular-expression\n"
                    "metacharacters backslash-escaped so it can be\n"
                    "used as a literal pattern in ``regexp`` or\n"
                    "``regsub``.  Alias for ``regex::quote``."
                ),
                return_value="Returns a regex-escaped string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="re_quote STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            taint_transform=TaintColour.REGEX_LITERAL,
            taint_double_encode_colour=TaintColour.REGEX_LITERAL,
        )
