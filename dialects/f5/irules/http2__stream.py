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
"""HTTP2::stream -- Gets or sets the stream attributes including id and priority."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP2__stream.html"


@register
class Http2StreamCommand(CommandDef):
    name = "HTTP2::stream"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP2::stream",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the stream attributes including id and priority.",
                synopsis=("HTTP2::stream (id | (priority (PRIORITY)?))?",),
                snippet=(
                    "This command can be used to determine the stream attributes including id and priority. This command can also be used to set the priority for a current active stream.\n"
                    "\n"
                    "HTTP2::stream\n"
                    "    Returns the stream id. Returns 0 if HTTP/2 is not active.\n"
                    "\n"
                    "HTTP2::stream id\n"
                    "    Returns the stream id. Returns 0 if HTTP/2 is not active.\n"
                    "\n"
                    "HTTP2::stream priority\n"
                    "    Returns the priority of the current stream.\n"
                    "\n"
                    "HTTP2::stream priority <priority>\n"
                    "    Sets the priority of the current active stream. The return is 0 is the priority was set. Return is an error if the priority is out of bounds (exceeding 8 bits)."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    if {[HTTP2::version] != 0} {\n"
                    "        set new_pri [URI::query [HTTP::uri] pri]\n"
                    '        if { $new_pri != "" } {\n'
                    "             HTTP2::stream priority $new_pri\n"
                    "        }\n"
                    '        HTTP::header insert "X-HTTP2-Values stream/pritority " "[HTTP2::stream]/[HTTP2::stream priority]"\n'
                    "    }\n"
                    "}"
                ),
            ),
            pure=True,
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP2::stream",
                    arity=Arity(0, 0),
                    pure=True,
                ),
            ),
            subcommands={
                "id": SubCommand(
                    name="id",
                    arity=Arity(0, 0),
                    detail="Returns the stream id. Returns 0 if HTTP/2 is not active.",
                    synopsis="HTTP2::stream id",
                ),
                "priority": SubCommand(
                    name="priority",
                    arity=Arity(0, 1),
                    detail="Get or set the priority of the current stream.",
                    synopsis="HTTP2::stream priority ?<priority>?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP2::stream priority",
                            arity=Arity(0, 0),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP2::stream priority <priority>",
                            arity=Arity(1, 1),
                            mutator=True,
                        ),
                    ),
                ),
            },
            validation=ValidationSpec(
                arity=Arity(0, 2),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP2_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
