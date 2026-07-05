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
"""AUTH::status -- Returns authentication status."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__status.html"


@register
class AuthStatusCommand(CommandDef):
    name = "AUTH::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns authentication status.",
                synopsis=("AUTH::status (AUTH_ID)?",),
                snippet=(
                    "Returns authentication status. The returned status is a value of 0, 1,\n"
                    "-1, or 2, corresponding to success, failure, error, or not-authed,\n"
                    "based on the result of the most recent authorization that the system\n"
                    "performed for the specified authorization session .\n"
                    "In the case of a not-authed result, the authentication process desires\n"
                    "a credential not yet provided. Specifics of the requested credential\n"
                    "can be determined using the AUTH::wantcredential_ commands. The\n"
                    "authentication process could be continued using\n"
                    "AUTH::authenticate_continue*."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n  set authStatus [AUTH::status $authSessionId]\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::status (AUTH_ID)?",
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
