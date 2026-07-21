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

//! `traffic_group` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "traffic_group",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the current traffic group.",
            synopsis: &["traffic_group"],
            snippet: "This command returns the current traffic group. Useful for\ntroubleshooting the partitioning of data in the session table.",
            source: "https://clouddocs.f5.com/api/irules/traffic_group.html",
            examples: "when RULE_INIT {\n  log local0. \"traffic_group name: [traffic_group]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "traffic_group",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::BigipConfig,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
