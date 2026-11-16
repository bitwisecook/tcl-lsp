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

use super::{is_builtin_expr_op, is_builtin_math_function, is_irules_only_expr_op};

/// The W117 stub-shadow check's `is_builtin_*` helpers are derived from
/// `tcl_syntax`; these lists are the hand-typed sets they replace
/// (`BUILTIN_MATH_FUNCTIONS` / `BUILTIN_EXPR_OPS` / `IRULES_EXPR_OPS`),
/// kept here to prove the derived helpers recognise exactly the same
/// names.
const OLD_BUILTIN_MATH_FUNCTIONS: &[&str] = &[
    "abs",
    "acos",
    "asin",
    "atan",
    "atan2",
    "bool",
    "ceil",
    "cos",
    "cosh",
    "double",
    "entier",
    "exp",
    "floor",
    "fmod",
    "hypot",
    "int",
    "isinf",
    "isnan",
    "isqrt",
    "log",
    "log10",
    "max",
    "min",
    "pow",
    "rand",
    "round",
    "sin",
    "sinh",
    "sqrt",
    "srand",
    "tan",
    "tanh",
    "wide",
    "acosh",
    "asinh",
    "atanh",
    "cbrt",
    "copysign",
    "dim",
    "erf",
    "erfc",
    "exp2",
    "expm1",
    "fma",
    "gamma",
    "ldexp",
    "lgamma",
    "log1p",
    "log2",
    "logb",
    "nextafter",
    "remainder",
    "signbit",
    "trunc",
];
const OLD_BUILTIN_EXPR_OPS: &[&str] = &[
    "!", "!=", "%", "&", "&&", "*", "**", "+", "-", "/", "<", "<<", "<=", "==", ">", ">=", ">>",
    "^", "eq", "ge", "gt", "in", "le", "lt", "ne", "ni", "|", "||", "~",
];
const OLD_IRULES_EXPR_OPS: &[&str] = &[
    "and",
    "contains",
    "ends_with",
    "equals",
    "matches_glob",
    "matches_regex",
    "not",
    "or",
    "starts_with",
];

#[test]
fn is_builtin_math_function_matches_the_old_hand_list_exactly() {
    for name in OLD_BUILTIN_MATH_FUNCTIONS {
        assert!(
            is_builtin_math_function(name),
            "{name}: was in the old list"
        );
    }
    // A handful of names that were never math functions.
    for name in ["puts", "expr", "+", "in", "notafunction"] {
        assert!(
            !is_builtin_math_function(name),
            "{name}: not a math function"
        );
    }
}

#[test]
fn is_builtin_expr_op_matches_the_old_hand_list_exactly() {
    for name in OLD_BUILTIN_EXPR_OPS {
        assert!(is_builtin_expr_op(name), "{name}: was in the old list");
    }
    for name in OLD_IRULES_EXPR_OPS {
        assert!(
            !is_builtin_expr_op(name),
            "{name}: is iRules-only, must not count as a plain built-in"
        );
    }
    assert!(!is_builtin_expr_op("notanoperator"));
}

#[test]
fn is_irules_only_expr_op_matches_the_old_hand_list_exactly() {
    for name in OLD_IRULES_EXPR_OPS {
        assert!(is_irules_only_expr_op(name), "{name}: was in the old list");
    }
    for name in OLD_BUILTIN_EXPR_OPS {
        assert!(
            !is_irules_only_expr_op(name),
            "{name}: is a plain built-in, must not count as iRules-only"
        );
    }
    assert!(!is_irules_only_expr_op("notanoperator"));
}
