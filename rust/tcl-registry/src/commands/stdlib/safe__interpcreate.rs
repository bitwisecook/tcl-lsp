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

//! `safe::interpCreate` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpCreate",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Create a safe child interpreter with restricted capabilities.",
            synopsis: &["safe::interpCreate ?child? ?options...?"],
            snippet: "Creates a safe interpreter.  Options include ``-accessPath``, ``-statics``, ``-noStatics``, ``-nested``, ``-noNested``, ``-deleteHook``.",
            source: "Tcl stdlib Safe Base",
            examples: "",
            return_value: "",
        }),
        required_package: Some("safe"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
