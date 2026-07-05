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
"""AUTH::wantcredential_type -- Returns an authorization session authidXs credential type."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__wantcredential_type.html"


@register
class AuthWantcredentialTypeCommand(CommandDef):
    name = "AUTH::wantcredential_type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::wantcredential_type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an authorization session authidXs credential type.",
                synopsis=("AUTH::wantcredential_type AUTH_ID",),
                snippet=(
                    "Returns the authorization session authid’s credential type that the\n"
                    "system last requested (when the system generated an AUTH_WANTCREDENTIAL\n"
                    "event). The value of the <authid> argument is either username,\n"
                    "password, x509, x509_issuer, or unknown, based upon the system’s\n"
                    "assessment of the credential prompt string and style.\n"
                    "\n"
                    "AUTH::wantcredential_type <authid>\n"
                    "\n"
                    "     * Returns the authorization session authid’s credential type that the\n"
                    "       system last requested (when the system generated an\n"
                    "       AUTH_WANTCREDENTIAL event)."
                ),
                source=_SOURCE,
                examples=(
                    "when AUTH_WANTCREDENTIAL {\n"
                    '  HTTP::respond 401 "WWW-Authenticate" "Basic realm=\\"\\""\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::wantcredential_type AUTH_ID",
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
