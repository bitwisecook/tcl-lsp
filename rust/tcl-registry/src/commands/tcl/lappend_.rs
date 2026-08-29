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
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
use tcl_dialect::model::Family;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "lappend varName ?value value value ...?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `lappend`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lappend",
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::READS_BEFORE_WRITE
            | Traits::FIRST_ARG_VARNAME,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        // iRules embeds Tcl 8.4.6 and retains lappend's documented
        // auto-creation behaviour; its bare profile mask needs explicit
        // membership alongside the ordinary Tcl cores.
        safe_on_uninit: Some(SpecSurface::ALL_TCL_AND_IRULES),
        return_type: Some(TclType::List),
        var_elements_effect: Some(VarElementsEffect::AppendsListElements { values_from: 1 }),
        representation_effect: Some(RepresentationEffect::copy_on_write_container(0, 2)),
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
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Append list elements to a variable, creating it if it does not already exist.",
            synopsis: &["lappend varName ?value value value ...?"],
            snippet: "This command treats the variable given by varName as a list and appends each of the value arguments to that list as a separate element, with spaces between elements. If varName does not exist, it is created as a list containing just the value arguments. lappend is similar to append except that the values are appended as list elements rather than raw text: this makes it a more efficient way to build up a large list than repeated set/concat, since `lappend a $b` is much cheaper than `set a [concat $a [list $b]]` once $a is already long. From Tcl 9.0, if varName names a nonexistent element of an array that has a default value configured (see `array default`), the list stored is that default value with the value arguments appended after it, rather than just the value arguments.",
            source: "Tcl man page lappend.n",
            examples: "set var 1\nlappend var 2\nlappend var 3 4 5\n# var is now \"1 2 3 4 5\"\n\nlappend newlist a b c\n# newlist is created as \"a b c\"",
            return_value: "The new value stored in varName after the values have been appended.",
        }),
        lowering_hook: Some(LoweringHookId::AppendOrLappend),
        codegen_hook: Some(CodegenHookId::Lappend),
        forms: FORMS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Lappend),
        ..CommandSpec::DEFAULT
    }
}
