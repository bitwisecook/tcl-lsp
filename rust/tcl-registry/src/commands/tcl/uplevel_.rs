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

//! `uplevel` — execute a script in a different stack frame.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uplevel ?level? arg ?arg ...?",
}];

/// Command spec for `uplevel`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uplevel",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::EVALUATES_CODE
            | Traits::TAINT_SINK
            | Traits::UNSAFE
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::at_least(1),
        lowering_hook: Some(crate::hooks::LoweringHookId::Uplevel),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Execute a script in a different stack frame",
            synopsis: &["uplevel ?level? arg ?arg ...?"],
            snippet: "All of the arg arguments are concatenated as if they had been passed to concat; the result is then evaluated in the variable context indicated by level.",
            source: "Tcl man page uplevel.n",
            examples: "",
            return_value: "",
        }),
        // A `LIST_CANONICAL` value preserves element
        // boundaries and suppresses T100.
        taint_sink_safe_colour: Some(TaintColour::LIST_CANONICAL),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        xc_translatable: Some(false),
        unsafe_command: true,
        ..CommandSpec::DEFAULT
    }
}
