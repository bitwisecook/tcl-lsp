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

//! `AUTH::cert_issuer_credential` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::cert_issuer_credential",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the peer certificate issuer credential to the value of for a future AUTH::authenticate call.",
            synopsis: &["AUTH::cert_issuer_credential AUTH_ID PEER_CERTIFICATE"],
            snippet: "Sets the peer certificate issuer credential to the value of <peer\ncertificate> for a future AUTH::authenticate call. This command\nreturns an error if attempted for a standby system.\n\nAUTH::cert_issuer_credential authid <peer certificate>\n\n     * Sets the peer certificate issuer credential to <peer certificate>\n       for a future AUTH::authenticate call.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__cert_issuer_credential.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n  set ldap_sid [AUTH::start pam $myprofilename]\n  AUTH::cert_credential $ldap_sid [SSL::cert 0]\n  AUTH::cert_issuer_credential $ldap_sid [SSL::cert issuer 0]\n  AUTH::authenticate $ldap_sid\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "AUTH::cert_issuer_credential AUTH_ID PEER_CERTIFICATE",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
