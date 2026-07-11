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

//! `variable` — create and initialise a namespace variable.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "variable name",
}];

/// Command spec for `variable`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "variable",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::CREATES_SCOPE_ALIAS
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::FRAME_HASH_BUILTIN,
        // `variable ?name value ...? name ?value?` — zero args is a valid
        // no-op (C `Tcl_VariableObjCmd` has no `Tcl_WrongNumArgs`), so bare
        // `variable` must not draw E002.  Verified against tclsh 9.0.4:
        // `catch {variable}` → 0.
        arity: Arity::any(),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "create and initialize a namespace variable",
            synopsis: &["variable name", "variable ?name value...?"],
            snippet: "This command is normally used within a namespace eval command to create one or more variables within a namespace.",
            source: "Tcl man page variable.n",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Variable),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
