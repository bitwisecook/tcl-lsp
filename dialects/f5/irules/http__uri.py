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
"""HTTP::uri -- Returns or sets the URI part of the HTTP request."""

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
from compiler.registry.taint_hints import SetterConstraint, TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__uri.html"


@register
class HttpUriCommand(CommandDef):
    name = "HTTP::uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::uri",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns or sets the URI part of the HTTP request.",
                synopsis=("HTTP::uri (URI)?",),
                snippet=(
                    "Returns or sets the URI part of the HTTP request. This command replaces\n"
                    "the BIG-IP 4.X variable http_uri.\n"
                    "\n"
                    "For the following URL:\n"
                    "http://www.example.com:8080/main/index.jsp?user=test&login=check\n"
                    "The URI is: /main/index.jsp?user=test&login=check\n"
                    "\n"
                    "Note that in the HTTP_PROXY_REQUEST event, this command returns the complete\n"
                    "proxy URI. This includes the scheme, host and port, and thus the result would be:\n"
                    "http://www.example.com:8080/main/index.jsp?user=test&login=check"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_PROXY_REQUEST {\n"
                    '   log local.0 "This proxy request is:[HTTP::uri]"\n'
                    "}"
                ),
                return_value="Returns the URI part of the HTTP request.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP::uri ?-normalized?",
                    arity=Arity(0, 0),
                    options=(
                        OptionSpec(
                            name="-normalized",
                            detail="Return URI normalized for consistent comparisons.",
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
                    synopsis="HTTP::uri <URI>",
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
                also_in=frozenset({"MR_EGRESS", "MR_FAILED", "MR_INGRESS", "SERVER_CONNECTED"}),
            ),
            cse_candidate=True,
            diagram_action=True,
            is_unnormalized_http_getter=True,
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(
            source={
                # Getter form (0 args): returns a path-prefixed tainted value.
                Arity(0, 0): TaintColour.TAINTED | TaintColour.PATH_PREFIXED,
            },
            setter_constraints=(
                SetterConstraint(
                    arg_index=0,
                    required_prefix="/",
                    code="IRULE3101",
                    message="HTTP::uri value must start with '/'",
                ),
            ),
        )
