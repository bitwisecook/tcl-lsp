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

//! `lasthop` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "lasthop",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the lasthop of an IP connection.",
            synopsis: &["lasthop (VLAN_OBJ)? (IP_ADDR | MAC_ADDR)"],
            snippet: "Sets the lasthop of a IP connection. The lasthop is the MAC destination\nfor packets going back to the client. This is usually the router\n(gateway) that forwards the client's packets to the BIG-IP (if \"auto\nlasthop\" is set), or is determined by the IP routing table. This\ncommand lets you specify the lasthop to use for a particular\nconnection.",
            source: "https://clouddocs.f5.com/api/irules/lasthop.html",
            examples: "when CLIENT_ACCEPTED {\n  lasthop external 01:23:45:ab:cd:ef\n}",
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
            synopsis: "lasthop (VLAN_OBJ)? (IP_ADDR | MAC_ADDR)",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
