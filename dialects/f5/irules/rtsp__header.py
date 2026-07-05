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
"""RTSP::header -- Manages headers in RTSP requests and responses."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__header.html"


_av = make_av(_SOURCE)


@register
class RtspHeaderCommand(CommandDef):
    name = "RTSP::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Manages headers in RTSP requests and responses.",
                synopsis=(
                    "RTSP::header (exists | remove | value) HEADER_NAME",
                    "RTSP::header replace HEADER_NAME HEADER_VALUE",
                    "RTSP::header insert (<(HEADER_NAME HEADER_VALUE)+> |",
                ),
                snippet="Manages headers in RTSP requests and responses.",
                source=_SOURCE,
                examples=(
                    'when RTSP_REQUEST {\n        puts [RTSP::header value "x-header"]\n    }'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::header (exists | remove | value) HEADER_NAME",
                    arg_values={
                        0: (
                            _av(
                                "exists",
                                "RTSP::header exists",
                                "RTSP::header (exists | remove | value) HEADER_NAME",
                            ),
                            _av(
                                "remove",
                                "RTSP::header remove",
                                "RTSP::header (exists | remove | value) HEADER_NAME",
                            ),
                            _av(
                                "value",
                                "RTSP::header value",
                                "RTSP::header (exists | remove | value) HEADER_NAME",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
