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

//! `pool` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "pool",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Select a load-balancing pool for the current flow.",
            synopsis: &["pool pool_name ?member_addr member_port?"],
            snippet: "Can direct traffic to a pool, optionally pinning to a specific member.",
            source: "https://clouddocs.f5.com/api/irules/pool.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "pool pool_name ?member_addr member_port?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Server,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
