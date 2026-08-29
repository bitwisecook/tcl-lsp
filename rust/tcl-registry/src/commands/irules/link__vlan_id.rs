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

//! `LINK::vlan_id` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::vlan_id",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the VLAN tag of the packet.",
            synopsis: &["LINK::vlan_id"],
            snippet: "Returns the VLAN tag of the packet. This command is equivalent to the\nBIG-IP 4.X variable vlan_id.",
            source: "https://clouddocs.f5.com/api/irules/LINK__vlan_id.html",
            examples: "# log requests\nwhen CLIENT_ACCEPTED {\n    set info \"client { [IP::client_addr]:[TCP::client_port] -> [IP::local_addr]:[TCP::local_port] }\"\n    append info \" ethernet \"\n    append info \" { [string range [LINK::lasthop] 0 16] -> [string range [LINK::nexthop] 0 16] \"\n    append info \"tag [LINK::vlan_id] qos [LINK::qos] }\"\n    log local0. $info\n}",
            return_value: "LINK::vlan_id",
        }),
        forms: &[FormSpec {
            synopsis: "LINK::vlan_id",
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
