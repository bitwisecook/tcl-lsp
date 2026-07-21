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

//! `TCP::lossfilter` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::lossfilter",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the TCP Loss Ignore Parameters.",
            synopsis: &["TCP::lossfilter TCP_IGNORE_RATE TCP_IGNORE_BURST"],
            snippet: "Sets the maximum size burst loss (in packets) and maximum number of packets per million lost before triggering congestion response.\n  * Burst range is valid from 0 to 32. Higher values decrease the\n    chance of performing congestion control.\n  * Rate range is valid from 0 to 1,000,000. Rate is X packets lost per\n    million before congestion control kicks in.",
            source: "https://clouddocs.f5.com/api/irules/TCP__lossfilter.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set client-side loss filter.\n    # Ignore up to 150 losses per million packets and burst losses of up to 10 packets.\n    clientside {\n        TCP::lossfilter 150 10\n    }\n    # No loss filter on server-side.\n    serverside {\n        TCP::lossfilter 0 0\n    }\n}",
            return_value: "None.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::lossfilter TCP_IGNORE_RATE TCP_IGNORE_BURST",
            dialects: None,
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
