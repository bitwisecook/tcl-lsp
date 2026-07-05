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

//! `error` — generate an error.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "error message ?info? ?code?",
}];

/// Command spec for `error`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "error",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::TERMINATES_BLOCK
            | Traits::NEEDS_START_CMD,
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Generate an error",
            synopsis: &["error message ?info? ?code?"],
            snippet: "Returns a TCL_ERROR code, which causes command interpretation to be unwound.",
            source: "Tcl man page error.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
