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

//! `SSL::renegotiate` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::renegotiate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Controls renegotiation of an SSL connection.",
            synopsis: &["SSL::renegotiate (enable | disable)?"],
            snippet: "Controls renegotiation of an SSL connection, often used to enforce new encryption settings or certificate requirements.\n\nThis command has different results depending on whether the BIG-IP system evaluates the command under a client-side or a server-side context. The command only succeeds if SSL is enabled on the connection; otherwise, the command returns an error.",
            source: "https://clouddocs.f5.com/api/irules/SSL__renegotiate.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    SSL::renegotiate disable\n}",
            return_value: "SSL::renegotiate Renegotiates a client-side or server-side SSL connection, depending on the context. When the system evaluates the command under a client-side context, the system immediately renegotiates a request for the associated client-side connection, if client-side renegotiation is enabled.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "SSL::renegotiate (enable | disable)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
