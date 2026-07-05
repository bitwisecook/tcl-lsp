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
"""CACHE::header -- Get/modify the content of an header related to an object stored in the RAM Cache."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__header.html"


_av = make_av(_SOURCE)


@register
class CacheHeaderCommand(CommandDef):
    name = "CACHE::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get/modify the content of an header related to an object stored in the RAM Cache.",
                synopsis=(
                    "CACHE::header ('exists' | 'remove' | 'value') HEADER_NAME",
                    "CACHE::header ('insert' | 'replace') HEADER_NAME HEADER_VALUE",
                ),
                snippet=(
                    "The command is used to gather or modify the content of a header stored\n"
                    "in the cache.\n"
                    "\n"
                    "CACHE::header <name>\n"
                    "\n"
                    "     * Get the content of the requested header\n"
                    "\n"
                    "CACHE::header insert <name> <value>\n"
                    "\n"
                    "     * Add the header with the specified value to the list of headers sent to the\n"
                    "       client when delivering an object from the cache.\n"
                    "\n"
                    "CACHE::header remove <name>\n"
                    "\n"
                    "     * Remove the header with the specified name.\n"
                    "\n"
                    "CACHE::header replace <name> <value>\n"
                    "\n"
                    "     * Replace the header with the specified value.\n"
                    "\n"
                    "CACHE::header value <name>\n"
                    "\n"
                    "     * Return the header value for the specified header name."
                ),
                source=_SOURCE,
                examples=(
                    "when CACHE_UPDATE {\n"
                    "    # cached object's headers manipulation\n"
                    "    # modifications will be seen whenever the object is served from cache\n"
                    "    CACHE::header replace Server Big-IP-Server\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::header ('exists' | 'remove' | 'value') HEADER_NAME",
                    arg_values={
                        0: (
                            _av(
                                "exists",
                                "CACHE::header exists",
                                "CACHE::header ('exists' | 'remove' | 'value') HEADER_NAME",
                            ),
                            _av(
                                "remove",
                                "CACHE::header remove",
                                "CACHE::header ('exists' | 'remove' | 'value') HEADER_NAME",
                            ),
                            _av(
                                "value",
                                "CACHE::header value",
                                "CACHE::header ('exists' | 'remove' | 'value') HEADER_NAME",
                            ),
                            _av(
                                "insert",
                                "CACHE::header insert",
                                "CACHE::header ('insert' | 'replace') HEADER_NAME HEADER_VALUE",
                            ),
                            _av(
                                "replace",
                                "CACHE::header replace",
                                "CACHE::header ('insert' | 'replace') HEADER_NAME HEADER_VALUE",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CACHE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
