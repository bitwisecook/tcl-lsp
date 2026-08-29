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

//! `TCP::local_port` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::local_port",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the local port of a TCP connection.",
            synopsis: &["TCP::local_port (clientside | serverside)?"],
            snippet: "Returns the local port/service number of the specified side, or the current context (client or server) if there is no argument.\nThis command is equivalent to the BIG-IP 4.X variable local_port. When used\nin a clientside context, this command returns the client-side TCP\ndestination port. When used in a serverside context, this command\nreturns the server-side TCP source port.",
            source: "https://clouddocs.f5.com/api/irules/TCP__local_port.html",
            examples: "when SERVER_CONNECTED {\n  # This logs information about the TCP connections on *both* sides of the fullproxy\n  set client_remote \"[IP::client_addr]:[TCP::client_port]\"\n  set client_local  \"[IP::local_addr clientside]:[TCP::local_port clientside]\"\n  set server_local  \"[IP::local_addr]:[TCP::local_port]\"\n  set server_remote \"[IP::server_addr]:[TCP::server_port]\"\n  log local0. \"Got connection: Client($client_remote)<->($client_local)LTM($server_local)<->($server_remote)Server\"\n}",
            return_value: "The local port.",
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
            synopsis: "TCP::local_port (clientside | serverside)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
