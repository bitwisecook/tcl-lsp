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

//! `nodes` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "nodes",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Lists all nodes within a given pool.",
            synopsis: &["nodes (-list)? POOL_OBJ"],
            snippet: "This command behaves like active_nodes but lists all nodes in a pool,\nnot just nodes that are currently active.",
            source: "https://clouddocs.f5.com/api/irules/nodes.html",
            examples: "when HTTP_REQUEST {\n        set in_path [HTTP::path]\n        log local0. \"debug request: path $in_path\"\n        switch -glob $in_path {\n                \"/pool*\" {\n                        set pool [string map {\"/pool\" \"\"} $in_path]\n                        HTTP::respond 200 content \"[active_members $pool]:[nodes $pool]\"\n                }\n        }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "nodes (-list)? POOL_OBJ",
            dialects: None,
        }],
        options: const {
            &[OptionSpec {
                name: "-list",
                value: OptionValue::flag(),
                detail: "Option -list.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
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
