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
"""HTML::tag -- Query and update the HTML tag."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTML__tag.html"


_av = make_av(_SOURCE)


@register
class HtmlTagCommand(CommandDef):
    name = "HTML::tag"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTML::tag",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Query and update the HTML tag.",
                synopsis=(
                    "HTML::tag ((append STRING) | name | (prepend STRING) | remove)",
                    "HTML::tag append <data>",
                    "HTML::tag name",
                    "HTML::tag prepend <data>",
                ),
                snippet=(
                    "Queries, removes and changes attribute/value pairs of this HTML tag.\n"
                    "        \n"
                    "HTML::tag append <data>\n"
                    "Insert a string after the closing delimiter of the HTML tag; when multiple appends are issued, the inserted strings are ordered according to the sequence of the append commands as they are issued for the given tag.\n"
                    "\n"
                    "HTML::tag name\n"
                    'Return HTML tag name, where name is the HTML element if the tag is a start tag, and if the tag is an end tag, tag name returns "/" + the HTML element.'
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    set uri [HTTP::uri]\n"
                    '    HTTP::header replace "Host" "finance.yahoo.com"\n'
                    "}"
                ),
                return_value='"HTML::tag name" returns tag name. "HTML::tag attribute value <name>" returns the value of the attribute under this HTML tag. "HTML::tag attribute count" returns the number of attributes in this HTML tag.',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTML::tag ((append STRING) | name | (prepend STRING) | remove)",
                    arg_values={
                        0: (
                            _av(
                                "append",
                                "HTML::tag append",
                                "HTML::tag ((append STRING) | name | (prepend STRING) | remove)",
                            ),
                            _av(
                                "prepend",
                                "HTML::tag prepend",
                                "HTML::tag ((append STRING) | name | (prepend STRING) | remove)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTML"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
