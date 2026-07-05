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
"""HTTP::query -- Returns or sets the query part of the HTTP request."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__query.html"


@register
class HttpQueryCommand(CommandDef):
    name = "HTTP::query"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::query",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns or sets the query part of the HTTP request.",
                synopsis=("HTTP::query (QUERY_STRING)?",),
                snippet=(
                    "Returns or sets the query part of the HTTP request. The query is defined as the\n"
                    "part of the request past a ? character, if any.\n"
                    "For the following URL:\n"
                    "http://www.example.com:8080/main/index.jsp?user=test&login=check\n"
                    "The query is:\n"
                    "user=test&login=check"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  log local0. "http_path [HTTP::path]"\n'
                    '  log local0. "http_query [HTTP::query]"\n'
                    "  HTTP::query user=test_user&login=test_login\n"
                    "}"
                ),
                return_value="Returns the query part of the HTTP request.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP::query ?-normalized?",
                    arity=Arity(0, 0),
                    options=(
                        OptionSpec(
                            name="-normalized",
                            detail="Return query normalized for consistent comparisons.",
                        ),
                    ),
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.HTTP_URI,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                            scope=StorageScope.EVENT,
                        ),
                    ),
                ),
                FormSpec(
                    kind=FormKind.SETTER,
                    synopsis="HTTP::query <QUERY_STRING>",
                    arity=Arity(1, 1),
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.HTTP_URI,
                            reads=True,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                            scope=StorageScope.EVENT,
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            event_requires=EventRequires(
                transport="tcp",
                profiles=frozenset({"HTTP", "FASTHTTP"}),
                also_in=frozenset({"MR_INGRESS", "SERVER_CONNECTED"}),
            ),
            cse_candidate=True,
            is_unnormalized_http_getter=True,
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
