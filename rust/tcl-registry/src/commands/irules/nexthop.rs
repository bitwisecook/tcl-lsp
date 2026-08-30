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

//! `nexthop` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "nexthop",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the nexthop of an IP connection.",
            synopsis: &[
                "nexthop ((IP_ADDR) | ((VLAN_OBJ_NOT_IP_ADDR) (IP_ADDR | MAC_ADDR | transparent)?))",
            ],
            snippet: "Sets the nexthop of an IP connection. The nexthop is the destination\nfor packets going from the BIG-IP to the server. This is usually\ndetermined by the IP routing table. This command lets you specify the\nnexthop to use for a particular connection. When a virtual server is\nassociated with a pool, pool-member selection occurs first; this may\nrequire configuring a route to the selected pool member.\n\nNote: In 11.6, you can use the 'nexthop' command to direct traffic over\n    IPIP tunnels.  In 13.0, you can use the 'nexthop' command to make\n    connections L2 transparent (preserve source and destination MAC address).",
            source: "https://clouddocs.f5.com/api/irules/nexthop.html",
            examples: "when CLIENT_ACCEPTED {\n  nexthop external 01:23:45:ab:cd:ef\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["PERSIST_DOWN"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "nexthop ((IP_ADDR) | ((VLAN_OBJ_NOT_IP_ADDR) (IP_ADDR | MAC_ADDR | transparent)?))",
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
