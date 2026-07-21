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

//! `pid` — return process ID(s).
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "pid ?fileId?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pid",
        dialects: None,
        traits: Traits::BYTE_COMPILED | Traits::PURE,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::Int),
        hover: Some(HoverSnippet {
            summary: "Retrieve process identifiers",
            synopsis: &["pid ?fileId?"],
            snippet: "If the fileId argument is given then it should normally refer to a process pipeline created with the open command.",
            source: "Tcl man page pid.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
