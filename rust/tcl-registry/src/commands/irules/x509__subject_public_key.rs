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

//! `X509::subject_public_key` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject_public_key",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the subjectXs public key of an X509 certificate.",
            synopsis: &["X509::subject_public_key (type | bits | curve_name)? CERTIFICATE"],
            snippet: "Returns the subject’s public key of the specified X509 certificate.",
            source: "https://clouddocs.f5.com/api/irules/X509__subject_public_key.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n  set client_cert [SSL::cert 0]\n  log local0. \"Cert subject - [X509::subject $client_cert]\"\n  log local0. \"Cert public key - [X509::subject_public_key $client_cert]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "X509::subject_public_key (type | bits | curve_name)? CERTIFICATE",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
