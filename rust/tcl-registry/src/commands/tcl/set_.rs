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

//! `set` — read or write a variable.

// VERIFIED: Tcl 9.0.3 manpage set(n) (man3/set.n).

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set varName ?newValue?",
    dialects: None,
}];

/// Dynamic arg role resolver: getter (1 arg) vs setter (2 args).
fn set_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 2 {
        vec![(0, ArgRole::VarWrite)]
    } else if args.len() == 1 {
        vec![(0, ArgRole::VarRead)]
    } else {
        Vec::new()
    }
}

/// Command spec for `set`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::FIRST_ARG_VARNAME,
        arity: Arity::new(1, 2),
        arg_role_resolver: Some(set_arg_roles),
        assigns_variable_at: Some(0),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Read or write a variable value.",
            synopsis: &["set varName ?newValue?"],
            snippet: "With one argument, returns the value. With two, assigns and returns the new value.",
            source: "Tcl set(1)",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Set),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Set),
        ..CommandSpec::DEFAULT
    }
}
