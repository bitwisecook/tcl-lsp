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
"""HTML::comment -- Query and update HTML comment."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTML__comment.html"


_av = make_av(_SOURCE)


@register
class HtmlCommentCommand(CommandDef):
    name = "HTML::comment"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTML::comment",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Query and update HTML comment.",
                synopsis=("HTML::comment ((append STRING) | (prepend STRING) | remove)?",),
                snippet=(
                    "Queries, removes HTML comment or appends/prepends it by a string.\n"
                    "\n"
                    "HTML::comment\n"
                    "Return the entire HTML comment, including the opening and the closing delimiter.\n"
                    "\n"
                    "HTML::comment append <string>\n"
                    "Insert a string after the closing delimiter of the HTML comment; when multiple appends are issued, the inserted strings are ordered according to the sequence of the append commands as they are issued for the given comment."
                ),
                source=_SOURCE,
                examples=('when HTML_COMMENT_MATCHED {\n    HTML::comment append "some_string"\n}'),
                return_value="HTML::comment returns the entire HTML comment; others do not return anything.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTML::comment ((append STRING) | (prepend STRING) | remove)?",
                    arg_values={
                        0: (
                            _av(
                                "append",
                                "HTML::comment append",
                                "HTML::comment ((append STRING) | (prepend STRING) | remove)?",
                            ),
                            _av(
                                "prepend",
                                "HTML::comment prepend",
                                "HTML::comment ((append STRING) | (prepend STRING) | remove)?",
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
