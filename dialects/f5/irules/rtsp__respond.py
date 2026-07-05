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
"""RTSP::respond -- Sends an RTSP response to the client."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__respond.html"


@register
class RtspRespondCommand(CommandDef):
    name = "RTSP::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends an RTSP response to the client.",
                synopsis=("RTSP::respond STATUS_CODE STATUS_STRING (HEADERS)?",),
                snippet=(
                    "Sends an RTSP response to the client. The return value of the\n"
                    "RTSP::msg_source command must be client. When an iRule responds to an\n"
                    "RTSP request, the RTSP filter performs no further processing on the\n"
                    "request and will not send the RTSP request to the server.\n"
                    "A maximum of one response is allowed per RTSP request."
                ),
                source=_SOURCE,
                examples=(
                    "when RTSP_REQUEST {\n"
                    '        RTSP::respond 401 Unauthorized "x-header\\r\\n\\r\\n  Hey, you need a password"\n'
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::respond STATUS_CODE STATUS_STRING (HEADERS)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
