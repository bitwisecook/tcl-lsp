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

//! `active_members` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "active_members",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the number or list of active members in the specified pool.",
            synopsis: &["active_members ('-list')? POOL_OBJ"],
            snippet: "Returns the number or list of active members in the specified pool.",
            source: "https://clouddocs.f5.com/api/irules/active_members.html",
            examples: "when HTTP_REQUEST {\n    if { [active_members http_pool] >= 2 } {\n        pool http_pool\n    }\n}",
            return_value: "active_members <pool_name> Returns the number of active members in the specified pool.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            // `active_members` is valid in ordinary HTTP request handling;
            // DNS is not a prerequisite. Keep the documented LB lifecycle
            // events below, but do not turn an optional profile into a
            // registry-wide legality requirement (IRULE1001 consumes this
            // descriptor generically).
            profiles: &[],
            also_in: &["LB_FAILED", "LB_SELECTED"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "active_members ('-list')? POOL_OBJ",
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
        ..CommandSpec::DEFAULT
    }
}
