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

//! `incr` — increment a variable.
//
// VERIFIED: Tcl 9.0.3 manpage incr(n) (man3/incr.n).

use crate::forms::CommandForm;
use crate::hooks::{InlineCodegenHookId, LoweringHookId};
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
    synopsis: "incr varName ?increment?",
    dialects: None,
}];

/// `incr varName` — implicit increment of 1.
const INCR_IMPLICIT: CommandForm = CommandForm {
    name: "implicit",
    arity: Arity::exact(1),
    arg_roles: &[(0, ArgRole::VarWrite)],
    lowering_hook: Some(LoweringHookId::Incr),
    ..CommandForm::DEFAULT
};

/// `incr varName increment` — explicit increment.
const INCR_EXPLICIT: CommandForm = CommandForm {
    name: "explicit",
    arity: Arity::exact(2),
    arg_roles: &[(0, ArgRole::VarWrite)],
    lowering_hook: Some(LoweringHookId::Incr),
    ..CommandForm::DEFAULT
};

/// Command spec for `incr`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "incr",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::READS_BEFORE_WRITE
            | Traits::FIRST_ARG_VARNAME,
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        safe_on_uninit: Some(DialectSet::TCL85_PLUS),
        return_type: Some(TclType::Int),
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        hover: Some(HoverSnippet {
            summary: "Increment the value of a variable",
            synopsis: &["incr varName ?increment?"],
            snippet: "Increments the value stored in the variable whose name is varName.",
            source: "Tcl man page incr.n",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Incr),
        inline_codegen_hook: Some(InlineCodegenHookId::Incr),
        command_forms: &[INCR_IMPLICIT, INCR_EXPLICIT],
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Incr),
        ..CommandSpec::DEFAULT
    }
}
