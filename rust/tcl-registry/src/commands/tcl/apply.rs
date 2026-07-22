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

//! `apply` — apply an anonymous procedure.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::ProcDefinition,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "apply func ?arg1 arg2 ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "apply",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::DYNAMIC_EVAL_BODY,
        // Added in Tcl 8.5 (TIP 194).
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        // The lambda literal is a `{argList body ?ns?}` list, not a plain
        // script — `ArgRole::LambdaLiteral` (not `Body`) says so, so generic
        // Body-role walkers (SSA's caller-scope scan, the semantic-token
        // highlighter's default body recursion) never try to re-segment the
        // list itself as script source. The real body (element 1) is walked
        // by the dedicated `LoweringHookId::Apply` / `AnalyserHookId::Apply`
        // hooks below, which already split the list correctly.
        arg_roles: &[(0, ArgRole::LambdaLiteral)],
        lowering_hook: Some(crate::hooks::LoweringHookId::Apply),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Apply an anonymous function",
            synopsis: &["apply func ?arg1 arg2 ...?", "apply func ?arg ...?"],
            snippet: "The command apply applies the function func to the arguments arg1 arg2 ...",
            source: "Tcl man page apply.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Apply),
        ..CommandSpec::DEFAULT
    }
}
