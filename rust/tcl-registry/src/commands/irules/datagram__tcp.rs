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

//! `DATAGRAM::tcp` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::tcp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns TCP header and payload information.",
            synopsis: &[
                "DATAGRAM::tcp (flags | payload_length | window)",
                "DATAGRAM::tcp (option | option_count) (IPV4_OPTION)?",
                "DATAGRAM::tcp payload (LENGTH)?",
            ],
            snippet: "This iRules command returns tcp header information.\nNote: throws an error if L4 protocol of the current connection is not\nTCP\n\nDATAGRAM::tcp flags\n\n     * This command returns TCP header flags as an integer value.\n\nDATAGRAM::tcp option\n\n     * This command returns a Tcl list of TCP options. See\n       DATAGRAM::ip option for details on behavior.\n\nDATAGRAM::tcp option [option-code]\n\n     * This command returns a Tcl list of TCP option values for TCP option\n       with a given option code. See DATAGRAM::ip option for details\n       on behavior.",
            source: "https://clouddocs.f5.com/api/irules/DATAGRAM__tcp.html",
            examples: "when FLOW_INIT {\n  if { [IP::protocol] == 6 } {\n    log local0. \"TCP Flow: [IP::client_addr] [TCP::client_port] --> [IP::local_addr] [TCP::local_port]\"\n    log local0. \"TCP Payload Length = [DATAGRAM::tcp payload_length] Payload: [DATAGRAM::tcp payload 100]\"\n  }\n}",
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
            synopsis: "DATAGRAM::tcp (flags | payload_length | window)",
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
