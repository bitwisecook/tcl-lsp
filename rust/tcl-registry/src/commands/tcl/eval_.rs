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

//! `eval` — evaluate a Tcl script dynamically.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "eval arg ?arg ...?",
    dialects: None,
}];

/// Command spec for `eval`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "eval",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CREATES_BARRIER
            | Traits::EVALUATES_CODE
            | Traits::TAINT_SINK
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Body)],
        lowering_hook: Some(crate::hooks::LoweringHookId::Eval),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Evaluate a Tcl script.",
            synopsis: &["eval arg ?arg ...?"],
            snippet: "Concatenates its arguments and executes the result as a Tcl script.\n\n**Security**: If any argument contains user-controlled data, this enables arbitrary code injection. Prefer `{*}$cmdList` (Tcl 8.5+) to expand pre-built command lists safely, or use direct invocation.",
            source: "Tcl man page eval.n",
            examples: "",
            return_value: "",
        }),
        // A `LIST_CANONICAL` value preserves element
        // boundaries and suppresses T100.
        taint_sink_safe_colour: Some(TaintColour::LIST_CANONICAL),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        xc_translatable: Some(false),
        ..CommandSpec::DEFAULT
    }
}
