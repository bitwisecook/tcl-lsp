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

//! `DATAGRAM::dns` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::dns",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns DNS header information.",
            synopsis: &["DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)"],
            snippet: "This iRules command returns DNS header information.\n  Note: L4 protocol of the packet must be either TCP or UDP for this command\n        to work. Also, the L4 port must be equal to the dns port (typically port 53).\n\nDATAGRAM::dns id\n\n     * Returns DNS header 16-bit ‘identification’ field as an integer value.\n\nDATAGRAM::dns qr\n\n     * Returns DNS header ‘query/response’ as a boolean value.\n       0 indicates a ‘query’ and 1 indicates a ‘response’.\n\nDATAGRAM::dns opcode\n\n     * Returns DNS header ‘opcode’ as a string.",
            source: "https://clouddocs.f5.com/api/irules/DATAGRAM__dns.html",
            examples: "when FLOW_INIT {\n  if { [IP::protocol] == 6 } {\n    log local0. \"TCP Payload Length = [DATAGRAM::tcp payload_length] Payload: [DATAGRAM::tcp payload 100]\"\n    log local0. \"DNS Header fields ID: [DATAGRAM::dns id] QR: [DATAGRAM::dns qr] OPCODE: [DATAGRAM::dns opcode] QDCOUNT: [DATAGRAM::dns qdcount]\"\n  }\n  if { [IP::protocol] == 17 } {\n    log local0. \"UDP Payload Length = [DATAGRAM::udp payload_length] Payload: [DATAGRAM::udp payload 100]\"",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FLOW"],
            also_in: &["CLIENT_DATA"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
