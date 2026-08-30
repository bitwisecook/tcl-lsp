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

//! `const` — define a constant variable (Tcl 9.0+, TIP 677).
//
// VERIFIED: const(n) exists in the Tcl 9.0 and 9.1 TclCmd manual trees
// (identical DESCRIPTION/EXAMPLES/SEE ALSO/KEYWORDS text in both — TIP 677
// landed in 9.0 and picked up no delta in 9.1) and 404s in the 8.4, 8.5, and
// 8.6 trees — the command genuinely does not exist before 9.0.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "const varName value",
    ..FormSpec::DEFAULT
}];

/// Command spec for `const`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "const",
        // Intentionally universal (`surface: None`) rather than Tcl-9.0-gated:
        // kept dialect-agnostic so it stays valid inside iRules events. See
        // `tcl9_commands_gated_to_tcl90` in registry.rs.
        surface: Some(SpecSurface::TCL90_PLUS),
        traits: Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD | Traits::FIRST_ARG_VARNAME,
        arity: Arity::new(2, 2),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Create and initialize an unmodifiable constant variable.",
            synopsis: &["const varName value"],
            snippet: "Normally used inside a procedure, method, or lambda body to create a constant local to that call, or inside a namespace eval body to create a constant in that namespace. varName is created (if it does not already exist) and initialized to value; no other command — set, append, incr, unset, and so on — may modify or remove it afterwards, and a variable is checked for constant-ness before any trace on it fires. varName may not be a qualified (::-containing) name and may not refer to an array or an array element; if the variable already exists as a plain (non-constant) variable, or as an array, const raises an error. If the variable already exists and is already a constant, the call is a silent no-op, which makes it safe to redeclare the same constant on every pass through a loop. A constant is normally removed only when its containing procedure returns or its containing namespace is deleted. Added in Tcl 9.0 (TIP 677); unchanged in 9.1.",
            source: "Tcl const(n)",
            examples: "proc circleArea {radius} {\n    const PI 3.14159\n    return [expr {$PI * $radius**2}]\n}\n\nnamespace eval ::acme {\n    const VERSION 3\n}\n\n# Safe to redeclare on every iteration of a loop:\nfor {set i 0} {$i < 3} {incr i} {\n    const STEP 1\n}",
            return_value: "The empty string on success. Errors if varName already exists as a non-constant variable or as an array, or if varName is a qualified name or refers to an array element.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
