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

//! `X509::subject_public_key_type` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject_public_key_type",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the subjectXs public key type of an X509 certificate.",
            synopsis: &["X509::subject_public_key_type CERTIFICATE"],
            snippet: "Returns the subject’s public key type of the specified X509\ncertificate. The returned value can be either RSA, DSA, or unknown.",
            source: "https://clouddocs.f5.com/api/irules/X509__subject_public_key_type.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n  set client_cert [SSL::cert 0]\n  log local0. \"Cert subject - [X509::subject $client_cert]\"\n  log local0. \"Cert public key type - [X509::subject_public_key_type $client_cert]\"\n  if { [X509::subject_public_key_type $client_cert] equals \"unknown\" } {\n    SSL::verify_result 50\n  }\n  set error_code [SSL::verify_result]\n  log local0. \"Cert verify result - [X509::verify_cert_error_string $error_code]\"\n}",
            return_value: "Returns the subject’s public key type of an X509 certificate.",
        }),
        forms: &[FormSpec {
            synopsis: "X509::subject_public_key_type CERTIFICATE",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
