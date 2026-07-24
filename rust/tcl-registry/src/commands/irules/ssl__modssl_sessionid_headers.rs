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

//! `SSL::modssl_sessionid_headers` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::modssl_sessionid_headers",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a list of fields for HTTP headers.",
            synopsis: &["SSL::modssl_sessionid_headers (initial | current)?"],
            snippet: "Returns a list of fields that the system will add to the HTTP headers, in order to emulate modssl behavior. The return type is a Tcl list; this list will be interpreted as a header-name/header-value pair by HTTP::header, for example.",
            source: "https://clouddocs.f5.com/api/irules/SSL__modssl_sessionid_headers.html",
            examples: "when HTTP_REQUEST {\n    HTTP::header insert [SSL::modssl_sessionid_headers]\n}",
            return_value: "SSL::modssl_sessionid_headers Returns a header name of \"SSLClientSessionId\", and a header value of the session id requested by the client.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::modssl_sessionid_headers (initial | current)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
