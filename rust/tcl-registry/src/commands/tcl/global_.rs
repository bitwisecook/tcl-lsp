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

//! `global` — access global variables.

use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "global ?varname ...?",
}];

/// Command spec for `global`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "global",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::CREATES_SCOPE_ALIAS
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::FRAME_HASH_BUILTIN,
        // `global ?varName ...?` — zero args is a valid no-op (C
        // `Tcl_GlobalObjCmd` has no `Tcl_WrongNumArgs`; its `for (i=1; …)` loop
        // simply doesn't run), so `global` on its own must not draw E002.
        // Verified against tclsh 9.0.4: `catch {global}` → 0.
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
            summary: "Access global variables",
            synopsis: &["global ?varname ...?"],
            snippet: "This command has no effect unless executed in the context of a proc body.",
            source: "Tcl man page global.n",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Global),
        codegen_hook: Some(CodegenHookId::Global),
        forms: FORMS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Global),
        ..CommandSpec::DEFAULT
    }
}
