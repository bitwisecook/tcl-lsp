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

//! `TCP::dsack` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::dsack",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Toggles TCP duplicate selective acknowledgments (D-SACK).",
            synopsis: &["TCP::dsack BOOL_VALUE"],
            snippet: "Enables or disables TCP duplicate selective acknowledgements.\nWhen enabled, accepts D-SACKs from remote hosts, which explicitly acknowledge duplicate packets and allow more accurate reaction to out-of-order and late packets.  See RFC3708 for details.",
            source: "https://clouddocs.f5.com/api/irules/TCP__dsack.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set client-side D-SACKs to enabled.\n    clientside {\n        TCP::dsack enable\n    }\n    # Set server-side D-SACKs to disabled.\n    serverside {\n        TCP::dsack disable\n    }\n}",
            return_value: "None.",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::dsack BOOL_VALUE",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
