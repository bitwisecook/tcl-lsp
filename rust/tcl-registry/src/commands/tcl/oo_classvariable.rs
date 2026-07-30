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

//! `classvariable` — link a local variable to a class-shared variable.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "classvariable variableName ?...?",
    dialects: None,
}];

/// Command spec for `classvariable`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "classvariable",
        traits: Traits::LANGUAGE_KEYWORD.union(Traits::TCLOO_METHOD_CONTEXT),
        // TIP 478 ("Add Expected Class Level Behaviors to oo::class")
        // introduced `classvariable`; its `Tcl-Version` metadata targeted
        // 8.7, a branch that was never cut as a stable release, and the
        // feature landed when 8.7's work was folded into 9.0. Per-version
        // fetch of classvariable.n: absent (404, both the .html and .htm
        // extensions) under 8.4, 8.5, and 8.6 — and 8.6's own define.n
        // manual has no mention of "classvariable" anywhere in its text —
        // present, word-for-word identical, under 9.0 and 9.1.
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Link local variables to variables shared by every instance of a class.",
            synopsis: &["classvariable variableName ?...?"],
            snippet: "Valid only inside a method, constructor, or destructor body. Each variableName must be an unqualified scalar name — no :: namespace separator and no array-element syntax — and classvariable links it, in the calling scope, to the like-named variable in the namespace of the class that defined the running method, so its value is shared by every instance of that class. This is the class-level counterpart to the variable command, and is equivalent to the typevariable command that the snit package (tcllib) provides for approximately the same purpose. In a method defined directly on an object (e.g. through oo::objdefine), linking a name this way behaves like namespace upvar [namespace current] $var $var for each name listed.",
            source: "Tcl classvariable(n)",
            examples: "oo::class create Counted {\n    initialise {\n        variable count 0\n    }\n    variable number\n    constructor {} {\n        classvariable count\n        set number [incr count]\n    }\n    method report {} {\n        classvariable count\n        puts \"This is instance $number of $count\"\n    }\n}\nset a [Counted new]\nset b [Counted new]\n$a report                 ;# This is instance 1 of 2\nset c [Counted new]\n$b report                 ;# This is instance 2 of 3\n$c report                 ;# This is instance 3 of 3",
            return_value: "The empty string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
