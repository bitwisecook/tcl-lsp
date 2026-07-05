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
"""SSL::sessionid -- Gets the SSL session ID."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__sessionid.html"


_av = make_av(_SOURCE)


@register
class SslSessionidCommand(CommandDef):
    name = "SSL::sessionid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::sessionid",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the SSL session ID.",
                synopsis=("SSL::sessionid (desired)?",),
                snippet="Gets the SSL session ID.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTCERT {\n"
                    "    set cert [SSL::cert 0]\n"
                    "    set sid [SSL::sessionid]\n"
                    '    if { $sid ne "" } {\n'
                    "        # If this SSL session will be cached, then it may be\n"
                    "        # resumed later on a new connection. Cache the cert\n"
                    "        # in the session table in case that happens. Because ID's\n"
                    "        # are not globally unique, the session id needs to be combined\n"
                    "        # with something from client address to avoid mismatch.\n"
                    "        set key [concat [IP::remote_addr]@$sid]"
                ),
                return_value="SSL::sessionid Returns the current connection's SSL session ID if it exists in the session cache. In version 10.x and higher, if the session ID does not exist in the cache, returns a null string. In version 9.x, if the session ID does not exist in the cache, returns a string of 64 zeroes.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::sessionid (desired)?",
                    arg_values={
                        0: (_av("desired", "SSL::sessionid desired", "SSL::sessionid (desired)?"),)
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
