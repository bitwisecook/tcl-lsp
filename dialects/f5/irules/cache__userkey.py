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
"""CACHE::userkey -- Allows users to add user-defined values to the key used by the cache to reference the cached content."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__userkey.html"


@register
class CacheUserkeyCommand(CommandDef):
    name = "CACHE::userkey"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::userkey",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allows users to add user-defined values to the key used by the cache to reference the cached content.",
                synopsis=("CACHE::userkey KEY",),
                snippet=(
                    "By default, cached content is stored with a unique key referring to both\n"
                    "the URI of the resource to be cached and the User-Agent for which it\n"
                    "was formatted. If multiple variations of the same content must be\n"
                    "cached under specific conditions (different client), you can use this\n"
                    "command to create a unique key, thus creating cached content specific\n"
                    "to that condition. This can be used to prevent one user or group's\n"
                    "cached data from being served to different users/groups."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if {[matchclass [IP::client_addr] equals $::InternalIPs]} {\n"
                    '    CACHE::userkey "Internal"\n'
                    "  } else {\n"
                    '    CACHE::userkey "External"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::userkey KEY",
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
