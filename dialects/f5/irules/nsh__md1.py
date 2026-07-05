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
"""NSH::md1 -- Sets/Get the MD1 context for NSH."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/NSH__md1.html"


@register
class NshMd1Command(CommandDef):
    name = "NSH::md1"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NSH::md1",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets/Get the MD1 context for NSH.",
                synopsis=("NSH::md1 DIRECTION UNSIGNED_INT UNSIGNED_INT (METADATA)?",),
                snippet=(
                    "Set: MD1 context for NSH. Offset, length and data string as arguments.\n"
                    "            Get: MD1 context from NSH. Only offset and length as arguments."
                ),
                source=_SOURCE,
                examples=(
                    "ntext for NSH.\n"
                    "            when CLIENT_ACCEPTED {\n"
                    "                set str {1234567890123456}\n"
                    "                NSH::md1 serverside_egress 1 16 [binary format a* $str]\n"
                    "                set myctx1 [NSH::md1 serverside_egress 1 16]\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NSH::md1 DIRECTION UNSIGNED_INT UNSIGNED_INT (METADATA)?",
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
