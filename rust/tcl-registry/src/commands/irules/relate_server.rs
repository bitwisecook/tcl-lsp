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

//! `relate_server` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "relate_server",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets up a related established connection.",
            synopsis: &["relate_server CONFIG"],
            snippet: "Sets up a related established connection. This can be used with protocols that parse information out of a control connection and then establish a data connection based on information that was exchanged in the control connection.",
            source: "https://clouddocs.f5.com/api/irules/relate_server.html",
            examples: "when SIP_RESPONSE {\n    # Taken from https://devcentral.f5.com/wiki/irules.Load-Balance-Outbound-SIP-Voice-Traffic-Signaling-AND-Media-with-SNAT.ashx\n    # Pre-establish the UDP connection to allow RTP from Server -> Client (and vice versa)\n    relate_server {\n        proto 17\n        clientflow $source_VLAN $destination_RTP $destination_RTP_port $source_inside $source_RTP_port\n        serverflow $destination_VLAN $source_outside $source_RTP_port $destination_RTP $destination_RTP_port\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "relate_server CONFIG",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Server,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
