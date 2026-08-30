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

//! `active_nodes` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "active_nodes",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the alias for active members of the specified pool (for BIG-IP version 4.X compatibility).",
            synopsis: &["active_nodes ('-list')? POOL_OBJ"],
            snippet: "Returns the alias for active members of the specified pool (for BIG-IP version 4.X compatibility).",
            source: "https://clouddocs.f5.com/api/irules/active_nodes.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"There are [active_nodes http_pool] active nodes in the pool.\"\n}",
            return_value: "active_nodes <pool name> Returns the number of active members of the specified pool (for BIG-IP version 4.X compatibility).",
        }),
        forms: &[FormSpec {
            synopsis: "active_nodes ('-list')? POOL_OBJ",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[OptionSpec {
                name: "-list",
                value: OptionValue::flag(),
                detail: "Return as list instead of count.",
                surface: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("active_members"),
        deprecated_replacement_drop_in: true,
        ..CommandSpec::DEFAULT
    }
}
