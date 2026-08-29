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

//! `append` — append to variable.

use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "append varName ?value value value ...?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `append`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "append",
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::READS_BEFORE_WRITE
            | Traits::STRING_LIST_CONFUSION
            | Traits::FIRST_ARG_VARNAME,
        arity: Arity::at_least(1),
        // S110: string-concatenates onto the target variable, coercing a
        // binary target (or a binary appended operand) to a character string.
        // Combined with `assigns_variable_at`/`READS_BEFORE_WRITE` below,
        // this is what makes the S110 pass damage the *variable* rather than
        // a returned value. Conservative: when the target **and** every
        // appended value are pure byte arrays, `Tcl_AppendObjToObj` keeps the
        // byte-array rep in both 8.6 and 9.0 (tclsh 8.6.14-verified:
        // bytearray+bytearray append stays a bytearray; 9.0.1 tclStringObj.c
        // has the same `TclIsPureByteArray` append path) — but any
        // string-rep'd operand (the typical `append v " text"`) coerces, and
        // the pass cannot prove runtime purity, so it warns on may-coerce.
        byte_array_effect: ByteArrayEffect::Coerces,
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        // iRules embeds Tcl 8.4.6 and retains append's documented
        // auto-creation behaviour. Its bare availability mask therefore
        // needs an explicit membership bit alongside the ordinary Tcl cores.
        safe_on_uninit: Some(SpecSurface::ALL_TCL_AND_IRULES),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::String),
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
            summary: "Append values to a variable, creating it if it does not already exist.",
            synopsis: &["append varName ?value value value ...?"],
            snippet: "Append all of the value arguments to the current value of variable varName. If varName does not exist, it is created with the concatenation of the value arguments (the empty string if none are given). This is an efficient way to build up a long string incrementally: `append a $b` is much cheaper than `set a $a$b` once $a is already long. From Tcl 9.0, appending to a nonexistent element of an array that has a default value set (see `array default`) stores the concatenation of that default value and the value arguments, rather than just the value arguments.",
            source: "Tcl man page append.n",
            examples: "set msg \"Tcl\"\nappend msg \" is \" \"fun\"\n\nset out {}\nforeach n {1 2 3 4 5} {\n    append out $n \",\"\n}\n# out is now \"1,2,3,4,5,\"",
            return_value: "The new value stored in varName after the append.",
        }),
        lowering_hook: Some(LoweringHookId::AppendOrLappend),
        codegen_hook: Some(CodegenHookId::Append),
        forms: FORMS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Append),
        ..CommandSpec::DEFAULT
    }
}
