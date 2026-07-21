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

//! `TCP::abc` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::abc",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Toggles Appropriate Byte Counting.",
            synopsis: &["TCP::abc BOOL_VALUE"],
            snippet: "This command will enable or disable TCP Appropriate Byte Counting. Increases congestion window in accordance with bytes actually acknowledged, rather than allowing small acknowledgements to increase the window by an entire segment.",
            source: "https://clouddocs.f5.com/api/irules/TCP__abc.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # If an HTTP connection, enable ABC on the client side and\n    # disable ABC on the server side.\n    if { [server_port] == 80 } {\n        clientside {\n            TCP::abc enable\n            log local0. \"Client MSS: [TCP::mss]\"\n        }\n        serverside {\n            TCP::abc disable\n            log local0. \"Server MSS: [TCP::mss]\"\n        }\n    }\n}",
            return_value: "None.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::abc BOOL_VALUE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
