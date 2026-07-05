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
"""HTTP::reject_reason -- Returns the reason HTTP is aborting"""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__reject_reason.html"


_av = make_av(_SOURCE)


@register
class HttpRejectReasonCommand(CommandDef):
    name = "HTTP::reject_reason"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::reject_reason",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the reason HTTP is aborting",
                synopsis=("HTTP::reject_reason ('as_num')?",),
                snippet="This returns the reason HTTP aborted the connection, either as a string, or as a numeric id suitable for an error code.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REJECT {\n"
                    '    log local0. "HTTP Aborted:" [HTTP::reject_reason]\n'
                    '    log local0. "Error code:" [HTTP::reject_reason as_num]\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::reject_reason ('as_num')?",
                    arg_values={
                        0: (
                            _av(
                                "as_num",
                                "HTTP::reject_reason as_num",
                                "HTTP::reject_reason ('as_num')?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
