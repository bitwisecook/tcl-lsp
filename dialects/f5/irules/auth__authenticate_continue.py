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
"""AUTH::authenticate_continue -- Continues an authentication operation."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__authenticate_continue.html"


@register
class AuthAuthenticateContinueCommand(CommandDef):
    name = "AUTH::authenticate_continue"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::authenticate_continue",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Continues an authentication operation.",
                synopsis=("AUTH::authenticate_continue AUTH_ID RESPONSE",),
                snippet=(
                    "Continues an authentication operation by providing the specified string\n"
                    "as the credential response for the most recent authorization prompt.\n"
                    "This command is only available when the event AUTH_WANTCREDENTIAL is\n"
                    "the most recent event generated, and no AUTH::credential commands have\n"
                    "been issued since the event, for the specified authentication ID.\n"
                    "Unlike the AUTH::credential commands, the string credential provided by\n"
                    "this command does not get cached, even if the desired credential type\n"
                    "had been identified (see the AUTH::wantcredential_type command)."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    set auth_stage 0\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::authenticate_continue AUTH_ID RESPONSE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
