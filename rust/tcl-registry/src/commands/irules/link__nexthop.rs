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

//! `LINK::nexthop` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::nexthop",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the MAC address of the next hop.",
            synopsis: &["LINK::nexthop ('id' | 'type' | 'name')?"],
            snippet: "Returns the MAC address of the next hop. Returns the broadcast address\nff:ff:ff:ff:ff:ff when called before a serverside connection has been\nestablished.\nNote:\n  * In 11.4, you can use LINK::nexthop with sub-commands to retrieve\n    the id, type and name of the next hop, respectively. Without\n    sub-commands, LINK::nexthop returns the MAC address as before.",
            source: "https://clouddocs.f5.com/api/irules/LINK__nexthop.html",
            examples: "# Logging example\nwhen CLIENT_ACCEPTED {\n        log local0. \"\\[LINK::lasthop\\]: [LINK::lasthop], \\[LINK::nexhop\\]: [LINK::nexthop]\"\n}",
            return_value: "LINK::nexthop [id]",
        }),
        forms: &[FormSpec {
            synopsis: "LINK::nexthop ('id' | 'type' | 'name')?",
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
