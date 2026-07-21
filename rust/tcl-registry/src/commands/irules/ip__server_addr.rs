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

//! `IP::server_addr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::server_addr",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the server's IP address.",
            synopsis: &["IP::server_addr"],
            snippet: "Returns the server's (node's) IP address once a serverside connection has been established. This command is equivalent to the command serverside { IP::remote_addr } and to the BIG-IP 4.X variable server_addr. The command returns 0 if the serverside connection has not been made.\n\nIn BIG-IP 10.x with route domains enabled this command returns the server's (node's) address once the serverside connection is established in the x.x.x.x%rd if the server is in any non-default route domains else it returns just the IPv4 address as expected.",
            source: "https://clouddocs.f5.com/api/irules/IP__server_addr.html",
            examples: "when SERVER_CONNECTED {\n   log local0. \"Node IP address: [IP::server_addr]\"\n}",
            return_value: "server's IP address",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: true,
            transport: None,
            profiles: &[],
            also_in: &["IP_GTM"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IP::server_addr",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
