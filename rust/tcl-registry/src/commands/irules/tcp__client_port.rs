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

//! `TCP::client_port` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::client_port",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the client port of the TCP connection.",
            synopsis: &["TCP::client_port"],
            snippet: "Returns the TCP port/service number of the clientside TCP\nconnection. This command is equivalent to the TCP::remote_port\ncommand in a clientside context, and to the BIG-IP 4.x variable\nclient_port.",
            source: "https://clouddocs.f5.com/api/irules/TCP__client_port.html",
            examples: "when SERVER_CONNECTED {\n   # This logs information about:\n   #  * the clientside part of the client<->LTM connection, and\n   #  * the serverside part of the LTM<->server connection.\n   log local0.info \"Complete connection: [IP::client_addr]:[TCP::client_port]<->LTM<->[IP::server_addr]:[TCP::server_port]\"\n}",
            return_value: "The port advertised by the client. Even on SERVER events, it still returns the client port from the clientside.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "TCP::client_port",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED.union(TaintColour::PORT)),
        ..CommandSpec::DEFAULT
    }
}
