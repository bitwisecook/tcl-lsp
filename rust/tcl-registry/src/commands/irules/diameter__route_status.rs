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

//! `DIAMETER::route_status` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::route_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the routing status of the current message.",
            synopsis: &["DIAMETER::route_status"],
            snippet: "The DIAMETER::route_status command returns the routing status of the current\nmessage. Valid status are:\n  * \"unprocessed\"\n  * \"route found\"\n  * \"no route found\"\n  * \"dropped\"\n  * \"queue full\"\n  * \"no connection\"\n  * \"connection closing\"\n  * \"internal error\"\n\n\"route found\" is based on the DIAMETER RouteTable finding a route. It\nis not affected by the proxy’s ability to create a connection, so even\nif the server is not listening on the specified address or marked\ndown, it still returns status as \"route found\" if the RouteTable is\nable to find the route.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__route_status.html",
            examples: "",
            return_value: "Returns routing status of the current message",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DIAMETER::route_status",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
