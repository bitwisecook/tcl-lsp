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
"""AUTH::authenticate -- Performs a new authentication operation."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__authenticate.html"


@register
class AuthAuthenticateCommand(CommandDef):
    name = "AUTH::authenticate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::authenticate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Performs a new authentication operation.",
                synopsis=("AUTH::authenticate AUTH_ID",),
                snippet=(
                    "Performs a new authentication operation. This command returns an error\n"
                    "if attempted for a standby system or while an authentication operation\n"
                    "is already in progress for this authentication session.\n"
                    "\n"
                    "AUTH::authenticate <authid>\n"
                    "\n"
                    "     * Performs a new authentication operation. This command returns an\n"
                    "       error if attempted for a standby system or while an authentication\n"
                    "       operation is already in progress for this authentication session."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  AUTH::username_credential $auth_id [HTTP::username]\n"
                    "  AUTH::password_credential $auth_id [HTTP::password]\n"
                    "  AUTH::authenticate $auth_id\n"
                    "  HTTP::collect\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::authenticate AUTH_ID",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
