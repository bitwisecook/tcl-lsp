// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `AUTH::ssl_cc_ldap_username` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::ssl_cc_ldap_username",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a user name that the system retrieved from the LDAP database.",
            synopsis: &["AUTH::ssl_cc_ldap_username AUTH_ID"],
            snippet: "Returns the user name that the system retrieved from the LDAP database\nfrom the last successful client certificate-based LDAP query for the\nspecified authorization session <authid>. The system returns an empty\nstring if the last successful query did not perform a successful client\ncertificate-based LDAP query, or if no query has yet been performed.\nThis command has been deprecated in favor of AUTH::response_data.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__ssl_cc_ldap_username.html",
            examples: "when RULE_INIT {\n    set cc_ldap_username \"defaultuser\"\n    set tmm_auth_subscription \"*\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "AUTH::ssl_cc_ldap_username AUTH_ID",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
