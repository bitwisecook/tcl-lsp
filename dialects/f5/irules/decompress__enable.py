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
"""DECOMPRESS::enable -- Enable DECOMPRESS feature on current flow."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DECOMPRESS__enable.html"


_av = make_av(_SOURCE)


@register
class DecompressEnableCommand(CommandDef):
    name = "DECOMPRESS::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DECOMPRESS::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable DECOMPRESS feature on current flow.",
                synopsis=("DECOMPRESS::enable (request | response)?",),
                snippet="Enable DECOMPRESS feature on current flow.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DECOMPRESS::enable (request | response)?",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "DECOMPRESS::enable request",
                                "DECOMPRESS::enable (request | response)?",
                            ),
                            _av(
                                "response",
                                "DECOMPRESS::enable response",
                                "DECOMPRESS::enable (request | response)?",
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
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
