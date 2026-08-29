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

//! `while` — loop while a condition is true.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "while test body",
    ..FormSpec::DEFAULT
}];

/// Command spec for `while`.
///
/// Synopsis, grammar, and semantics are identical across Tcl 8.4, 8.5, 8.6,
/// 9.0, and 9.1: the while(n) manpage's SYNOPSIS, DESCRIPTION, EXAMPLE, and
/// SEE ALSO sections are byte-for-byte identical on all five version trees
/// (fetched and diffed directly, not paraphrased) — no wording, grammar,
/// option, or semantics change anywhere in the range, and Tcl 9.1's
/// while.html (a live 9.1b0 beta build) is no exception: its body text
/// matches 9.0's exactly, character for character. The five pages differ
/// only in cosmetic doc-tree navigation headers and a trivial KEYWORDS
/// wording change at the 8.5/8.6 boundary ("boolean value" in 8.4/8.5,
/// "boolean" in 8.6+ — page-metadata only, not documentation of
/// behaviour). `while` has never taken an option, never gained or lost a
/// form, and has no version-gated behaviour to model here.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "while",
        // Present and unrestricted — `while` carries an iRules row explicitly
        // (`ALL_TCL.union(IRULES)`), resolving under the bare `IRULES` mask; a
        // pure control-flow keyword with no filesystem/process/network access,
        // so every dialect that hosts a real Tcl core (irules, iapps, tmsh,
        // the EDA shells, expect, tk) carries it unmodified.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_BOOLEAN_COND
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY,
        arity: Arity::exact(2),
        arg_roles: &[(0, ArgRole::Expr), (1, ArgRole::Body)],
        lowering_hook: Some(crate::hooks::LoweringHookId::While),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Execute script repeatedly as long as a condition is met.",
            synopsis: &["while test body"],
            snippet: "test is evaluated as an expression, the same way expr evaluates its argument, and the result must be a proper boolean value. While it is true, body is executed by passing it to the Tcl interpreter; once body finishes, test is evaluated again, and the cycle repeats until test becomes false. continue inside body terminates the current iteration and jumps straight to the next test check; break inside body causes immediate termination of the whole loop. Either way while itself always returns an empty string. test should almost always be enclosed in braces, e.g. {$x < 10}: an unbraced test is substituted once, before the loop starts, so it never sees changes body makes to the variables involved and this is likely to result in an infinite loop. Enclosing test in braces delays variable substitution until the expression is evaluated before each iteration, so changes body makes are visible on the next check.",
            source: "Tcl while(n)",
            examples: "set x 0\nwhile {$x < 10} {\n    puts \"x is $x\"\n    incr x\n}\n\nset lineCount 0\nwhile {[gets $chan line] >= 0} {\n    puts \"[incr lineCount]: $line\"\n}",
            return_value: "An empty string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
