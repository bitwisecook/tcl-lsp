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
"""PEM::session -- This command allows you to create, delete or retreive information of a PEM session using session IP address in the PEM Session DB."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PEM__session.html"


_av = make_av(_SOURCE)


@register
class PemSessionCommand(CommandDef):
    name = "PEM::session"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PEM::session",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to create, delete or retreive information of a PEM session using session IP address in the PEM Session DB.",
                synopsis=(
                    "PEM::session config policy ((get IP_ADDR) |",
                    "PEM::session delete IP_ADDR",
                ),
                snippet=(
                    "This command allows you to create, delete or retreive information of a PEM Session in the PEM Session DB.\n"
                    "Each PEM session carries the following standard attributes: imsi, imeisv, tower-id, rat-type, user-name, state, aaa-reporting-interval, provision.\n"
                    "\n"
                    "Details (Syntax):\n"
                    "PEM::session create <framed ip> [subscriber-id <string> subscriber-type <e164 | imsi | nai | private>] [imsi <sring>] [user-name <string>] [tower-id <string>] [imeisv <string>] [provision <yes | no>] [<custom attr> <custom value>] [policy <string1> ..."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    PEM::session create 10.10.10.10 subscriber-id 12345 subscriber-type e164 policy pem-policy1 pem-policy2\n"
                    "\n"
                    "    set polisy_var [PEM::session config policy get 10.10.10.10]\n"
                    "    set ip_var [PEM::session ip 12345 e164]\n"
                    "    set id_var [PEM::session info 10.10.10.10 subscriber-id]\n"
                    "\n"
                    "    PEM::session delete 10.10.10.10\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PEM::session config policy ((get IP_ADDR) |",
                    arg_values={
                        0: (
                            _av(
                                "get",
                                "PEM::session get",
                                "PEM::session config policy ((get IP_ADDR) |",
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
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
