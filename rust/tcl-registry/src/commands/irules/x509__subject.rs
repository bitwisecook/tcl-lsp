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

//! `X509::subject` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the subject of an X509 certificate.",
            synopsis: &["X509::subject CERTIFICATE (commonName)?"],
            snippet: "Returns the subject of the specified X509 certificate.\nIf commonName RDN is specified, returns the Subject CN in UTF8 format.",
            source: "https://clouddocs.f5.com/api/irules/X509__subject.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n\n  # Check if the client supplied one or more client certs\n  if {[SSL::cert count] > 0}{\n\n    # Check the first client cert subject\n    if { [X509::subject [SSL::cert 0]] equals \"someSubject\" } {\n      log local0. \"X509 Certificate Subject [X509::subject [SSL::cert 0]]\"\n      pool my_pool\n    }\n    # Check the first client cert subject commonName\n    if { [X509::subject [SSL::cert 0] commonName] equals \"someCommonName\" } {",
            return_value: "Returns the subject of an X509 certificate. If commonName RDN is specified, returns the Subject CN in UTF8 format.",
        }),
        forms: &[FormSpec {
            synopsis: "X509::subject CERTIFICATE (commonName)?",
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
