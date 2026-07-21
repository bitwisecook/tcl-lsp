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

//! `SSL::verify_result` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::verify_result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets the result code for peer certificate verification.",
            synopsis: &["SSL::verify_result (RESULT_CODE)?"],
            snippet: "Gets or sets the result code for peer certificate verification. Result codes use the same values as those of OpenSSL's X509 verify_result (X509_V_ERR_*) definitions.",
            source: "https://clouddocs.f5.com/api/irules/SSL__verify_result.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n    set cert [X509::verify_cert_error_string [SSL::verify_result]]\n}",
            return_value: "SSL::verify_result Gets the result code from peer certificate verification. The returned code uses the same values as those of OpenSSL's X509 verify_result (X509_V_ERR_) definitions.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::verify_result (RESULT_CODE)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
