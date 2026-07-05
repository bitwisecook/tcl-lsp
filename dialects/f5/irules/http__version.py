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
"""HTTP::version -- Returns or sets the HTTP version of the request or response."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
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

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__version.html"

_av = make_av(_SOURCE)

# The HTTP/1.x versions the bareword setter accepts.  HTTP/2 and HTTP/3 are
# their own command namespaces (``HTTP2::`` etc.), so they are not values here.
_HTTP_VERSIONS = (
    _av("0.9", "HTTP/0.9", "HTTP::version 0.9"),
    _av("1.0", "HTTP/1.0", "HTTP::version 1.0"),
    _av("1.1", "HTTP/1.1", "HTTP::version 1.1"),
)


@register
class HttpVersionCommand(CommandDef):
    name = "HTTP::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::version",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns or sets the HTTP version of the request or response.",
                synopsis=(
                    "HTTP::version ('0.9' | '1.0' | '1.1')?",
                    "HTTP::version '-string' (ANY_CHARS)?",
                ),
                snippet=(
                    "Returns or sets the HTTP version of the request or response. This\n"
                    "command replaces the BIG-IP 4.X variable http_version.\n"
                    "If needed, Connection and Host headers will automatically be added\n"
                    "appropriately.\n"
                    "HTTP::version will return the original version of the request or\n"
                    "response, even if it has been changed.  Note that this will return\n"
                    'the "effective" version used, which may be different than the actual\n'
                    "version string in the request or response.  For example, invalid\n"
                    "version numbers may be parsed as 1.1 in order to increase\n"
                    "inter-operability with common HTTP servers."
                ),
                source=_SOURCE,
                examples=('when HTTP_RESPONSE {\n  HTTP::version "1.1"\n}'),
                return_value="Returns the HTTP version of the request or response",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::version ('0.9' | '1.0' | '1.1')?\nHTTP::version -string ?value?",
                    options=(
                        OptionSpec(
                            name="-string",
                            detail="Get/set version as raw string.",
                            takes_value=True,
                            value_hint="version",
                        ),
                    ),
                    # The bareword setter takes one of the known HTTP/1.x
                    # versions; offer them as completions and treat the set as
                    # exhaustive (W127 flags anything else).  The ``-string``
                    # form (index 1) is intentionally a raw, unconstrained
                    # value, so it is not enumerated or closed here.
                    arg_values={0: _HTTP_VERSIONS},
                    closed_value_args=frozenset({0}),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp",
                profiles=frozenset({"HTTP", "FASTHTTP"}),
                also_in=frozenset({"MR_EGRESS", "MR_INGRESS", "SERVER_CONNECTED"}),
            ),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
