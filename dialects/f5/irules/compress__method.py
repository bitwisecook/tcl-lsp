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
"""COMPRESS::method -- Specifies the preferred compression algorithm."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/COMPRESS__method.html"


_av = make_av(_SOURCE)


@register
class CompressMethodCommand(CommandDef):
    name = "COMPRESS::method"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="COMPRESS::method",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Specifies the preferred compression algorithm.",
                synopsis=("COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",),
                snippet=(
                    "Specifies the preferred compression algorithm.\n"
                    "\n"
                    "COMPRESS::method prefer [ ’gzip’ | ’deflate’ ]\n"
                    "    Specifies the preferred compression algorithm, either gzip or deflate."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST_SEND {\n COMPRESS::method prefer gzip\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",
                    arg_values={
                        0: (
                            _av(
                                "gzip",
                                "COMPRESS::method gzip",
                                "COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",
                            ),
                            _av(
                                "deflate",
                                "COMPRESS::method deflate",
                                "COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",
                            ),
                            _av(
                                "request",
                                "COMPRESS::method request",
                                "COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",
                            ),
                            _av(
                                "response",
                                "COMPRESS::method response",
                                "COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')",
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
