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

//! `TclOO` object.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "oo::copy sourceObject ?targetObject? ?targetNamespace?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::copy",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "create copies of objects and classes",
            synopsis: &[
                "oo::copy sourceObject ?targetObject? ?targetNamespace?",
                "oo::copy sourceObject ?targetObject?",
            ],
            snippet: "The oo::copy command creates a copy of an object or class.",
            source: "Tcl man page copy.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
