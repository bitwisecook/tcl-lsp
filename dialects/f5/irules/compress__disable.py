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
"""COMPRESS::disable -- Disables compression for the current HTTP response."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/COMPRESS__disable.html"


_av = make_av(_SOURCE)


@register
class CompressDisableCommand(CommandDef):
    name = "COMPRESS::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="COMPRESS::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables compression for the current HTTP response.",
                synopsis=("COMPRESS::disable (request | response)?",),
                snippet=(
                    "Disables compression for the current HTTP response. Note that when using this command, you must set the HTTP profile setting Compression to Selective.\n"
                    "\n"
                    "COMPRESS::disable\n"
                    "    Disables compression for the current HTTP response. Note that when using this command, you must set the HTTP profile setting Compression to Selective."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if { [TCP::mss] >= 1280 } {\n"
                    "    COMPRESS::disable\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="COMPRESS::disable (request | response)?",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "COMPRESS::disable request",
                                "COMPRESS::disable (request | response)?",
                            ),
                            _av(
                                "response",
                                "COMPRESS::disable response",
                                "COMPRESS::disable (request | response)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
