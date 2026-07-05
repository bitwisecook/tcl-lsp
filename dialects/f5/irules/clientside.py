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
"""clientside -- Causes the specified iRule commands to be evaluated under the client-side context."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/clientside.html"


def _clientside_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """The optional nesting script is a body evaluated in the client-side context.

    With no argument the command is a context query returning 1/0, so there is
    no body.  When the script is supplied it is the sole argument (index 0) and
    runs synchronously in the caller's scope (``BodyKind.INLINE``).
    """
    if args:
        return {0: frozenset({ArgRole.BODY})}
    return {}


@register
class ClientsideCommand(CommandDef):
    name = "clientside"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="clientside",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Causes the specified iRule commands to be evaluated under the client-side context.",
                synopsis=("clientside (NESTING_SCRIPT)?",),
                snippet="Causes the specified iRule commands to be evaluated under the client-side context. This command has no effect if the iRule is already being evaluated under the client-side context. If there is no argument, the command returns 1 if the current event is in the clientside context or 0 if not.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "   # Check if the client IP address is 10.1.1.80\n"
                    "   # [clientside {IP::remote_addr}] is equivalent to [IP::client_addr]\n"
                    "   if { [IP::addr [clientside {IP::remote_addr}] equals 10.1.1.80] } {\n"
                    "      # Do something like drop the packets in this example\n"
                    "      discard\n"
                    "   }\n"
                    "}"
                ),
                return_value="clientside Returns 1 if the current event is in the clientside context or 0 if not.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="clientside (NESTING_SCRIPT)?",
                ),
            ),
            validation=ValidationSpec(
                # ``clientside (NESTING_SCRIPT)?`` — the bare query form (0
                # args) or a single optional nesting-script body.
                arity=Arity(0, 1),
            ),
            arg_role_resolver=_clientside_arg_roles,
            is_side_switch=True,
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
