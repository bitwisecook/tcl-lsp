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

//! `continue` — skip to the next iteration of a loop.

use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
use tcl_dialect::model::Family;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "continue",
    ..FormSpec::DEFAULT
}];

const COMPLETION_CODES: &[CompletionCode] = &[CompletionCode::Continue];

/// Command spec for `continue`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "continue",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.4", Some("9.2"))]), SpecSurface::core(Family::F5Irules)]),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CONTINUES_LOOP
            | Traits::TRANSFERS_CONTROL
            | Traits::NEEDS_START_CMD,
        arity: Arity::exact(0),
        completion: Some(CompletionDescriptor::exact(COMPLETION_CODES)),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Skip to the next iteration of the innermost enclosing loop.",
            synopsis: &["continue"],
            snippet: "Typically invoked inside the body of a looping command such as for, foreach, or while. It raises a TCL_CONTINUE exception that aborts the remainder of the current iteration and transfers control to the next iteration of the innermost containing loop — the loop's own per-iteration step (for's third clause, foreach's advance to the next list element, while's condition re-check) still runs as normal. Outside a loop the exception does not simply propagate forever: catch traps it and returns an ordinary TCL_CONTINUE result (code 4) with an empty value, and a Tk event binding script that invokes continue is terminated — skipping any further \"+\"-appended scripts for that binding — while Tk goes on to run bindings for other tags. If a continue exception instead escapes the body of a procedure, or reaches the top level of a script, without being absorbed by an enclosing loop, catch, or binding, the interpreter converts it into an error: invoked \"continue\" outside of a loop.",
            source: "Tcl man page continue.n",
            examples: "for {set i 0} {$i < 10} {incr i} {\n    if {$i == 5} {\n        continue\n    }\n    puts $i\n}",
            return_value: "None in normal use — control transfers to the next iteration of the innermost enclosing loop. Trapped with catch, the caught value is an empty string.",
        }),
        inline_codegen_hook: Some(InlineCodegenHookId::Continue),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
