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

//! `expr` — evaluate a mathematical expression.
//
// VERIFIED: Tcl 9.0.3 manpage expr(n) (man3/expr.n).

use crate::hooks::{InlineCodegenHookId, LoweringHookId};
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "expr arg ?arg ...?",
}];

/// Command spec for `expr`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expr",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::PURE_EVALUATION
            | Traits::NEEDS_START_CMD
            | Traits::TAINT_SINK
            | Traits::EXPR_CONCATENATES_ARGS,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Expr)],
        return_type: Some(TclType::Numeric),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Numeric),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Evaluate a Tcl expression.",
            synopsis: &["expr arg ?arg ...?"],
            snippet: "**Always brace expressions**: `expr {$a + $b}`.\n\nWithout braces, `expr $x + 1` undergoes double substitution: the Tcl parser expands `$x` first, then `expr` evaluates the result. If `$x` contains `[dangerous_command]`, it executes. Bracing also enables bytecode compilation for better performance.",
            source: "Tcl expr(1)",
            examples: "",
            return_value: "The result of evaluating the expression.",
        }),
        lowering_hook: Some(LoweringHookId::Expr),
        inline_codegen_hook: Some(InlineCodegenHookId::Expr),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
