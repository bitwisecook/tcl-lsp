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
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::server_addr",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the server's IP address.",
            synopsis: &["IP::server_addr"],
            snippet: "Returns the server's (node's) IP address once a serverside connection has been established. This command is equivalent to the command serverside { IP::remote_addr } and to the BIG-IP 4.X variable server_addr. The command returns 0 if the serverside connection has not been made.\n\nIn BIG-IP 10.x with route domains enabled this command returns the server's (node's) address once the serverside connection is established in the x.x.x.x%rd if the server is in any non-default route domains else it returns just the IPv4 address as expected.",
            source: "https://clouddocs.f5.com/api/irules/IP__server_addr.html",
            examples: "when SERVER_CONNECTED {\n   log local0. \"Node IP address: [IP::server_addr]\"\n}",
            return_value: "server's IP address",
        }),
        // Not a server-side-only command. Measured on the appliance, the
        // rule compiler accepts `IP::server_addr` in every one of the
        // eight probed events except `RULE_INIT`, client-side events
        // included (`docs/design/bigip-irule-parser-measurements.md` §8 —
        // the same row shape as `LB::server` and `table`). The hover text
        // says why: before the serverside connection exists the command
        // returns `0` rather than failing, so only the absence of traffic
        // flow refuses it.
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["IP_GTM"],
            flow: true,
        }),
        forms: &[FormSpec {
            synopsis: "IP::server_addr",
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
