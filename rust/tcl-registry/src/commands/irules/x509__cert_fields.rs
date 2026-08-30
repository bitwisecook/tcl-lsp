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

//! `X509::cert_fields` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::cert_fields",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a list of X509 certificate fields to be added to HTTP headers for ModSSL behavior.",
            synopsis: &["X509::cert_fields CERTIFICATE ERROR_CODE ((hash"],
            snippet: "When given a valid certificate, returns a TCL list of field names and\nvalues which can be added to the HTTP headers in order to emulate\nModSSL behavior. The output can be passed to 'HTTP::header insert\n$list' as a list for insertion in the HTTP request or response.",
            source: "https://clouddocs.f5.com/api/irules/X509__cert_fields.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n    if { [SSL::cert count] > 0 } {\n        session add ssl [SSL::sessionid] [X509::cert_fields [SSL::cert 0] [SSL::verify_result] whole] $timeout\n    }\n}",
            return_value: "Returns a list of X509 certificate fields to be added to HTTP headers.",
        }),
        forms: &[FormSpec {
            synopsis: "X509::cert_fields CERTIFICATE ERROR_CODE ((hash",
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
