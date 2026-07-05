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
"""AUTH::ssl_cc_ldap_username -- Returns a user name that the system retrieved from the LDAP database."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__ssl_cc_ldap_username.html"


@register
class AuthSslCcLdapUsernameCommand(CommandDef):
    name = "AUTH::ssl_cc_ldap_username"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::ssl_cc_ldap_username",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a user name that the system retrieved from the LDAP database.",
                synopsis=("AUTH::ssl_cc_ldap_username AUTH_ID",),
                snippet=(
                    "Returns the user name that the system retrieved from the LDAP database\n"
                    "from the last successful client certificate-based LDAP query for the\n"
                    "specified authorization session <authid>. The system returns an empty\n"
                    "string if the last successful query did not perform a successful client\n"
                    "certificate-based LDAP query, or if no query has yet been performed.\n"
                    "This command has been deprecated in favor of AUTH::response_data."
                ),
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    '    set cc_ldap_username "defaultuser"\n'
                    '    set tmm_auth_subscription "*"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::ssl_cc_ldap_username AUTH_ID",
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
