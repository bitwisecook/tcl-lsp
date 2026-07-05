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
"""AUTH::last_event_session_id -- Returns the session ID of the last auth event."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__last_event_session_id.html"


@register
class AuthLastEventSessionIdCommand(CommandDef):
    name = "AUTH::last_event_session_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::last_event_session_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the session ID of the last auth event.",
                synopsis=("AUTH::last_event_session_id",),
                snippet=(
                    "This command returns the session ID of the last auth event, which can\n"
                    "then be used to relate to the user behind each session.\n"
                    "\n"
                    "AUTH::last_event_session_id\n"
                    "\n"
                    "     * Returns the session ID of the last auth event"
                ),
                source=_SOURCE,
                examples=(
                    "when AUTH_SUCCESS {\n"
                    "  if {$auth_id eq [AUTH::last_event_session_id]} {\n"
                    '    log local0. "auth success event"\n'
                    "    set authorized 1\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::last_event_session_id",
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
        )
