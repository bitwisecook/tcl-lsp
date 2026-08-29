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

use crate::hooks::{InlineCodegenHookId, LoweringHookId};
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
use tcl_dialect::model::Family;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "expr arg ?arg arg ...?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `expr`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expr",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.4", Some("9.2"))]), SpecSurface::core(Family::F5Irules)]),
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
            synopsis: &["expr arg ?arg arg ...?"],
            snippet: "**Always brace expressions**: `expr {$a + $b}`.\n\nWithout braces, `expr $x + 1` undergoes double substitution: the Tcl parser expands `$x` first, then `expr` evaluates the result. If `$x` contains `[dangerous_command]`, it executes. Bracing also enables bytecode compilation for better performance — and does not fully neutralise the risk, since a bracketed command inside a braced expression (`expr {[cmd]}`) is still executed as part of evaluating the operand.\n\nOperands may be numbers, booleans (any form `string is boolean` accepts), `$variables`, double- or brace-quoted strings, bracketed commands, or `func(...)` calls into the `tcl::mathfunc` namespace (Tcl 8.5+; in 8.4 math functions exist only as expr syntax, with no namespace-command form).\n\nThe expression grammar has grown over releases. Tcl 8.5 added the exponentiation operator `**`, the list-membership operators `in`/`ni`, `Inf`/`NaN` floating-point literals, explicit `0b`/`0o` integer prefixes, and switched integer arithmetic to arbitrary precision (LibTomMath) — Tcl 8.4 computed integers with fixed-width C `long`/`wide` types. Tcl 9.0 added the always-UNICODE-string relational operators `lt`/`gt`/`le`/`ge`, an explicit `0d` decimal prefix, `_` digit separators (`1_000_000`), and removed the ambiguous \"leading zero means octal\" reading (`013` is decimal 13, not octal 11, as of 9.0).\n\nIn F5 iRules, `expr` runs on an embedded Tcl 8.4.6 core, so none of the 8.5+/9.0+ additions above are available: no `**`, `in`/`ni`, `lt`/`gt`/`le`/`ge`, `Inf`/`NaN`, or `0b`/`0o`/`0d` prefixes, and integers stay fixed-width.",
            source: "Tcl expr(n)",
            examples: "expr {8.2 + 6}\nexpr {($x*$x - $y*$y) / exp($x*$x + $y*$y)}\nexpr {$a eq $b ? \"equal\" : \"different\"}\nset randNum [expr {int(100 * rand())}]",
            return_value: "The value of the evaluated expression — typically numeric, but a bare operand (e.g. `expr {$x}`) or a ternary branch can pass through an arbitrary string or list value unchanged.",
        }),
        lowering_hook: Some(LoweringHookId::Expr),
        inline_codegen_hook: Some(InlineCodegenHookId::Expr),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
