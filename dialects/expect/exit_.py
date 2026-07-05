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

"""exit -- Exit expect (with onexit/noexit support)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity

from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect exit(1)"


def _exit_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Resolve BODY role only for the -onexit handler argument."""
    roles: dict[int, frozenset[ArgRole]] = {}
    body = frozenset({ArgRole.BODY})
    i = 0
    while i < len(args):
        if args[i] == "-onexit" and i + 1 < len(args):
            roles[i + 1] = body
            i += 2
        else:
            i += 1
    return roles


@register
class ExitCommand(CommandDef):
    name = "exit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exit",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Exit Expect, optionally running an onexit handler.",
                synopsis=(
                    "exit ?-onexit command? ?status?",
                    "exit ?-noexit? ?status?",
                ),
                snippet=(
                    "With ``-onexit``, registers a handler to run at exit. "
                    "With ``-noexit``, prepares for exit but does not actually "
                    "exit (useful for cleaning up in libraries)."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="exit ?-onexit command | -noexit? ?status?",
                    options=(
                        OptionSpec(
                            name="-onexit",
                            takes_value=True,
                            value_hint="command",
                            detail="Register a handler to run at exit.",
                        ),
                        OptionSpec(name="-noexit", detail="Prepare for exit without exiting."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_role_resolver=_exit_arg_roles,
        )
