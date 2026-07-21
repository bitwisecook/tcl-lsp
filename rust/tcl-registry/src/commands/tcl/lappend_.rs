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

//! `lappend` — append list elements onto a variable.

use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lappend varName ?value value value ...?",
}];

/// Command spec for `lappend`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lappend",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::READS_BEFORE_WRITE
            | Traits::FIRST_ARG_VARNAME,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        return_type: Some(TclType::List),
        var_elements_effect: Some(VarElementsEffect::AppendsListElements { values_from: 1 }),
        inferred_storage_type: Some(StorageType::List),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Append list elements onto a variable",
            synopsis: &["lappend varName ?value value value ...?"],
            snippet: "This command treats the variable given by varName as a list and appends each of the value arguments to that list as a separate element, with spaces between elements.",
            source: "Tcl man page lappend.n",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::AppendOrLappend),
        codegen_hook: Some(CodegenHookId::Lappend),
        forms: FORMS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Lappend),
        ..CommandSpec::DEFAULT
    }
}
