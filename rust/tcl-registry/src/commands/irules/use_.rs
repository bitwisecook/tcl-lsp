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

//! `use` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "use",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "A BIG-IP 4.X statement.",
            synopsis: &[
                "use clone pool POOL_OBJ (member IP_ADDR)?",
                "use nexthop ((IP_ADDR) | ((VLAN_OBJ) (IP_ADDR | MAC_ADDR | transparent)?))",
                "use node (IP_TUPLE | (IP_ADDR (PORT)?))",
                "use pool POOL_OBJ (member (IP_TUPLE | (IP_ADDR (PORT)?)))?",
            ],
            snippet: "This is a BIG-IP 4.X statement, provided for backward-compatibility. The use statement must be paired with certain BIG-IP 9.X commands such as node, pool, rateclass, snat, and snatpool.\n\nThe use command is not required on BIG-IP 9.X systems.",
            source: "https://clouddocs.f5.com/api/irules/use.html",
            examples: "when HTTP_REQUEST {\n    if { [HTTP::uri] contains \"aol\" } {\n        use pool aol_pool\n    } else {\n        use pool all_pool\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "use clone pool POOL_OBJ (member IP_ADDR)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("virtual"),
        ..CommandSpec::DEFAULT
    }
}
