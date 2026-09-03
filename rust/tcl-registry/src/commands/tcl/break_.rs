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

//! `break` — abort looping command.

use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "break",
    ..FormSpec::DEFAULT
}];

const COMPLETION_CODES: &[CompletionCode] = &[CompletionCode::Break];

/// Command spec for `break`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "break",
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::BREAKS_LOOP
            | Traits::TRANSFERS_CONTROL
            | Traits::NEEDS_START_CMD,
        arity: Arity::exact(0),
        completion: Some(CompletionDescriptor::exact(COMPLETION_CODES)),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Abort execution of the innermost enclosing loop.",
            synopsis: &["break"],
            snippet: "Typically invoked inside the body of a looping command such as for, foreach, or while. It raises a TCL_BREAK exception that aborts the current script out to the innermost containing loop, which then stops iterating and returns normally — execution resumes with the statement after the loop. The exception is also handled by catch, Tk event bindings, and the outermost script of a procedure body. Used anywhere else — outside a loop, catch, Tk binding, or a procedure's top level — it is an error: invoked \"break\" outside of a loop.",
            source: "Tcl man page break.n",
            examples: "for {set i 0} {$i < 10} {incr i} {\n    if {$i == 5} {\n        break\n    }\n    puts $i\n}",
            return_value: "None in normal use — control transfers to just past the innermost enclosing loop. Trapped with catch, the caught value is an empty string.",
        }),
        inline_codegen_hook: Some(InlineCodegenHookId::Break),
        native_lowering: Some(NativeLowering::Completion(CompletionCode::Break)),
        // The command's whole effect is its completion code, which the exact
        // completion descriptor above already carries: it changes no
        // interpreter state a dispatch proof could depend on, so its world
        // footprint is closed and empty.
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(StateTransitionDescriptor::EMPTY),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
