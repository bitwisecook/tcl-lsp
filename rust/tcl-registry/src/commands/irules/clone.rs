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

//! `clone` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "clone",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Causes the system to clone traffic to the specified pool, pool member or vlan regardless of monitor status.",
            synopsis: &[
                "clone pool POOL_OBJ (member IP_ADDR (PORT)?)?",
                "clone nexthop VLAN_OBJ",
            ],
            snippet: "Causes the system to clone traffic to the specified pool, pool member or vlan regardless of monitor status. (Pool member status may be determined by the use of the LB::status command. Failure to select a server because none are available may be prevented by using the active_members command to test the number of active members in the target pool before choosing it.) Any responses to cloned traffic from pool members will be ignored.",
            source: "https://clouddocs.f5.com/api/irules/clone.html",
            examples: "when CLIENT_ACCEPTED {\n   clone nexthop tap_vlan\n}",
            return_value: "clone pool <pool_name> Specifies the pool to which you want to send the cloned traffic.",
        }),
        forms: &[FormSpec {
            synopsis: "clone pool POOL_OBJ (member IP_ADDR (PORT)?)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
