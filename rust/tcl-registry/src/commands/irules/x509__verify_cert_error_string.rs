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

//! `X509::verify_cert_error_string` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::verify_cert_error_string",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns an X509 certificate error string.",
            synopsis: &["X509::verify_cert_error_string ERROR_CODE"],
            snippet: "Returns the same result as the OpenSSL function\nX509_verify_cert_error_string(). Values for the <X509 verify error\ncode> argument must be the same values as those that the SSL::verify\nresult command returns.",
            source: "https://clouddocs.f5.com/api/irules/X509__verify_cert_error_string.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n  set cert [SSL::cert 0]\n  log local0. \"Cert subject - [X509::subject $cert]\"\n  set error_code [SSL::verify_result]\n  log local0. \"Cert verify result - [X509::verify_cert_error_string $error_code]\"\n}",
            return_value: "Returns an X509 certificate error string.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "X509::verify_cert_error_string ERROR_CODE",
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
