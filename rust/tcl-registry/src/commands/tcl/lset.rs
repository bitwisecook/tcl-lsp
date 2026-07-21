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

//! `lset` — change an element in a list variable.
//
// VERIFIED: Tcl 9.0.3 manpage lset(n) (man3/lset.n).
use crate::forms::CommandForm;
use crate::hooks::CodegenHookId;
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
    synopsis: "lset varName ?index ...? newValue",
    dialects: None,
}];

/// `lset varName newValue` — replace the entire list (no index).
const LSET_REPLACE: CommandForm = CommandForm {
    name: "replace",
    arity: Arity::exact(2),
    arg_roles: &[(0, ArgRole::VarWrite)],
    codegen_hook: Some(CodegenHookId::Lset),
    ..CommandForm::DEFAULT
};

/// `lset varName index newValue` — single-level update.
const LSET_SINGLE_INDEX: CommandForm = CommandForm {
    name: "single_index",
    arity: Arity::exact(3),
    arg_roles: &[(0, ArgRole::VarWrite)],
    codegen_hook: Some(CodegenHookId::Lset),
    ..CommandForm::DEFAULT
};

/// `lset varName index1 ?index2 ...? newValue` — multi-level path.
const LSET_FLAT_PATH: CommandForm = CommandForm {
    name: "flat_path",
    arity: Arity::at_least(4),
    arg_roles: &[(0, ArgRole::VarWrite)],
    codegen_hook: Some(CodegenHookId::Lset),
    ..CommandForm::DEFAULT
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lset",
        // `lset` reads the list's current value before rewriting one element,
        // and — like `set`/`append`/`lappend`/`incr` — its first argument is
        // a variable *name*, so it joins the name-first set the write-command
        // consumers (bounds checks, dead-store cancellation, minifier RMW
        // protection) query via `FIRST_ARG_VARNAME`.
        traits: Traits::FRAME_HASH_BUILTIN
            .union(Traits::READS_BEFORE_WRITE)
            .union(Traits::FIRST_ARG_VARNAME),
        dialects: None,
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Change an element in a list",
            synopsis: &["lset varName ?index ...? newValue"],
            snippet: "The lset command accepts a parameter, varName, which it interprets as the name of a variable containing a Tcl list.",
            source: "Tcl man page lset.n",
            examples: "",
            return_value: "",
        }),
        codegen_hook: Some(CodegenHookId::Lset),
        command_forms: &[LSET_REPLACE, LSET_SINGLE_INDEX, LSET_FLAT_PATH],
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..CommandSpec::DEFAULT
    }
}
