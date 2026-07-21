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

//! `parray` — print an array.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "parray arrayName ?pattern?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "parray",
        traits: Traits::WHOLE_ARRAY_ARG | Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::VarRead)],
        return_type: Some(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
            // Reads the array VARIABLE.
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Print an array's keys and values",
            synopsis: &["parray arrayName ?pattern?"],
            snippet: "",
            source: "Tcl man page library.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
