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
"""RTSP::version -- Returns the version in the current RTSP request/response."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__version.html"


@register
class RtspVersionCommand(CommandDef):
    name = "RTSP::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::version",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the version in the current RTSP request/response.",
                synopsis=("RTSP::version",),
                snippet=(
                    "Returns the version (for example, RTSP/1.0) in the current RTSP\n"
                    "request/response. You can use this command to determine if RTSP is\n"
                    "being tunneled over HTTP on the RTSP port (the version would be an HTTP\n"
                    "version). The command is valid in the RTSP_REQUEST and RTSP_RESPONSE\n"
                    "events."
                ),
                source=_SOURCE,
                examples=("when RTSP_REQUEST {\n        puts [RTSP::version]\n    }"),
                return_value="Returns the version in the current RTSP request/response.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::version",
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
