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

//! `DATAGRAM::udp` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::udp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns UDP payload information.",
            synopsis: &[
                "DATAGRAM::udp payload (LENGTH)?",
                "DATAGRAM::udp payload_length",
            ],
            snippet: "This iRules command returns UDP payload information.\nNote: throws an error if L4 protocol of the current connection is not\nUDP\n\nDATAGRAM::udp payload [<size>]\n\n     * Returns the content of the current UDP payload. If <size> is specified and more than <size>\n       bytes are available, only the first <size> bytes of collected data are returned.\n\nDATAGRAM::udp payload_length\n\n     * Returns the length, in bytes, of the current UDP payload.",
            source: "https://clouddocs.f5.com/api/irules/DATAGRAM__udp.html",
            examples: "when FLOW_INIT {\n  if { [IP::protocol] == 17 } {\n     log local0. \"UDP Flow: [IP::client_addr] [UDP::client_port] --> [IP::local_addr] [UDP::local_port]\"\n     log local0. \"UDP Payload Length = [DATAGRAM::udp payload_length] Payload: [DATAGRAM::udp payload 100]\"\n   }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FLOW"],
            also_in: &["CLIENT_DATA"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "DATAGRAM::udp payload (LENGTH)?",
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
