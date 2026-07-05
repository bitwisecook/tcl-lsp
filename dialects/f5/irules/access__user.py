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
"""ACCESS::user -- Returns user ID information."""

# Introduced: BIG-IP v11+ (Access Policy Manager) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__user.html"


_av = make_av(_SOURCE)


@register
class AccessUserCommand(CommandDef):
    name = "ACCESS::user"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::user",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns user ID information.",
                synopsis=(
                    "ACCESS::user getkey SID_HASH",
                    "ACCESS::user getsid KEY",
                    "ACCESS::user ACCESS_USER_COMMAND (ACCESS_USER_INFO)?",
                ),
                snippet=(
                    "The ACCESS::user commands return user ID information.\n"
                    "\n"
                    "ACCESS::user getsid <key>\n"
                    "\n"
                    "     * Returns the list of created external SIDs which is associated wit\n"
                    "       the specified key\n"
                    "\n"
                    "ACCESS::user getkey <sid_hash>\n"
                    "\n"
                    "     * Returns the original SID for specified hash of SID\n"
                    "     * This command works for clientless mode only\n"
                    "\n"
                    " * Requires APM module"
                ),
                source=_SOURCE,
                examples=(
                    "when ACCESS_SESSION_STARTED {\n"
                    "    # Associate the user_key with the session by assigning the value.\n"
                    "    if { [ info exists user_key ] } {\n"
                    '        ACCESS::session data set "session.user.uuid" $user_key\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::user <subcommand> <arg>",
                    arg_values={
                        0: (
                            _av(
                                "getkey",
                                "Get original SID from hash.",
                                "ACCESS::user getkey <sid_hash>",
                            ),
                            _av(
                                "getsid", "Get external SIDs for key.", "ACCESS::user getsid <key>"
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
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "getkey": SubCommand(
                    name="getkey",
                    arity=Arity(1, 1),
                    detail="Get original SID from hash.",
                    synopsis="ACCESS::user getkey <sid_hash>",
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.APM_STATE,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "getsid": SubCommand(
                    name="getsid",
                    arity=Arity(1, 1),
                    detail="Get external SIDs for key.",
                    synopsis="ACCESS::user getsid <key>",
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.APM_STATE,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
            },
        )
