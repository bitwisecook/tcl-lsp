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

//! `catch` — evaluate script and trap exceptional returns.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "catch script ?resultVarName? ?optionsVarName?",
        // `optionsVarName` (and the return-options dict it receives) was
        // added to the synopsis in Tcl 8.5's catch(n); 8.4's synopsis is
        // the narrower two-argument form below. `TCL85_PLUS` — not
        // extended to iRules — mirrors `incr_.rs`'s `safe_on_uninit:
        // Some(DialectSet::TCL85_PLUS)`: iRules embeds a genuine Tcl 8.4.6
        // runtime with nothing backported at any BIG-IP version
        // (docs/design/compiler/dialects-events.md, "Dialect base
        // versions", ratified D3), so version-dependent behaviour follows
        // 8.4 there too.
        dialects: Some(DialectSet::TCL85_PLUS),
        ..FormSpec::DEFAULT
    },
    // Tcl 8.4's `catch(n)`: `catch script ?varName?` — a single result
    // variable (there named `varName`, not `resultVarName`) and no
    // `optionsVarName` / return-options dict. iRules' `catch` runs on that
    // same embedded 8.4.6 core (see the comment above), so it follows this
    // shape too.
    FormSpec {
        synopsis: "catch script ?varName?",
        dialects: Some(DialectSet::TCL84.union(DialectSet::IRULES)),
        ..FormSpec::DEFAULT
    },
];

/// Command spec for `catch`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "catch",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD,
        arity: Arity::new(1, 3),
        arg_roles: &[
            (0, ArgRole::Body),
            (1, ArgRole::VarWrite),
            (2, ArgRole::VarWrite),
        ],
        lowering_hook: Some(crate::hooks::LoweringHookId::Catch),
        inline_codegen_hook: Some(crate::hooks::InlineCodegenHookId::Catch),
        return_type: Some(TclType::Int),
        hover: Some(HoverSnippet {
            summary: "Evaluate script and trap exceptional returns",
            synopsis: &["catch script ?resultVarName? ?optionsVarName?"],
            snippet: "Evaluates script in the caller's own variable scope and always completes normally, even when script raises an error — a single unguarded error never aborts the surrounding script. The integer catch returns is script's completion code: 0 (TCL_OK) for normal completion, 1 (TCL_ERROR) for an error, 2 (TCL_RETURN) if script executed return, 3 (TCL_BREAK) for break, 4 (TCL_CONTINUE) for continue, or a larger custom code set via `return -code`. When resultVarName is given, it is set to script's result on success or to the error message on failure. From Tcl 8.5, an optional third word, optionsVarName, receives a dict of the return options: -code and -level are always present, and on an error -errorinfo, -errorcode, and -errorline are added too — joined by -errorstack from Tcl 8.6 onward. Tcl 8.4's catch takes only the single result variable (there named varName) and has no options dict; iRules' embedded Tcl 8.4.6 runtime follows that same two-argument form. Only run-time errors are trapped: a syntax error Tcl finds while compiling script itself (e.g. unmatched braces) is not caught.",
            source: "Tcl man page catch.n",
            examples: "if {[catch {open $someFile w} fid]} {\n    puts stderr \"Could not open $someFile for writing\\n$fid\"\n    exit 1\n}\nif {[catch {expr {1 / 0}} result options]} {\n    puts \"error: $result (code [dict get $options -code])\"\n}",
            return_value: "An integer completion code — 0 (TCL_OK) through 4 (TCL_CONTINUE) for the standard completion codes, or a larger custom code set via `return -code`.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Catch),
        ..CommandSpec::DEFAULT
    }
}
