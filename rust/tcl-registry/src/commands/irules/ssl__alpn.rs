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

//! `SSL::alpn` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::alpn",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Handle the ALPN TLS extension.",
            synopsis: &["SSL::alpn set (ARG)+", "SSL::alpn"],
            snippet: "Sets or retrieves the Application Layer Protocol Negotiation (ALPN) string.\n\nSSL::alpn\n  Retrieve the selected ALPN string\n\nSSL::alpn set str1[ str2...]\n  Set the advertised ALPN string",
            source: "https://clouddocs.f5.com/api/irules/SSL__alpn.html",
            examples: "when CLIENTSSL_CLIENTHELLO {\n    SSL::alpn set \"spdy/1\" \"spdy/2\" \"http/2\"\n}",
            return_value: "SSL::alpn Returns the negotiated ALPN string SSL::alpn set ... There is no return value.",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::alpn set (ARG)+",
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
