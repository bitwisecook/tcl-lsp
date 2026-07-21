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

//! `TCP::earlyrxmit` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::earlyrxmit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Toggles TCP early retransmit.",
            synopsis: &["TCP::earlyrxmit (BOOL_VALUE)?"],
            snippet: "Early retransmit allows TCP to assume a packet is lost after fewer than the standard number of duplicate ACKs, if there is no way to send new data and generate more duplicate ACKs (specified in RFC 5827).",
            source: "https://clouddocs.f5.com/api/irules/TCP__earlyrxmit.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set client-side early retransmit to enabled.\n    clientside {\n        log local0. \"Client: earlyrxmit [TCP::earlyrxmit], enabling\"\n        TCP::earlyrxmit enable\n    }\n    # Set server-side early retransmit to disabled.\n    serverside {\n        log local0. \"Server: earlyrxmit [TCP::earlyrxmit], disabling\"\n        TCP::earlyrxmit disable\n    }\n}",
            return_value: "TCP::earlyrxmit returns whether TCP early retransmit is enabled.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::earlyrxmit (BOOL_VALUE)?",
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
