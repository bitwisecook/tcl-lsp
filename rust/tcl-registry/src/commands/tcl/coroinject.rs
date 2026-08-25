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

//! `coroinject` — inject a command into a coroutine.
//!
//! Documented on the shared `coroutine(n)` manpage alongside `coroutine`,
//! `yield`, `yieldto`, and `coroprobe`. Added in Tcl 9.0 — the command is
//! absent from the 8.4/8.5/8.6 `coroutine(n)` pages (8.4/8.5 predate
//! coroutines entirely; 8.6's page documents only `coroutine`/`yield`/
//! `yieldto`) and unchanged, word-for-word, between the 9.0.4 and 9.1b0
//! pages.
use crate::prelude::*;

fn coroinject_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    (args.len() >= 2)
        .then_some((1, ScriptTiming::Deferred))
        .into_iter()
        .collect()
}
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "coroinject coroName command ?arg ...?",
    ..FormSpec::DEFAULT
}];

// `coroinject coroName command ?arg...?` schedules an arbitrary command to run
// the next time the coroutine resumes (its result becomes the resumption
// value).  Deferred arbitrary code with unknown reads/writes — treated like
// `eval` so the optimiser never eliminates or reorders it.
static SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroinject",
        traits: Traits::EVALUATES_CODE | Traits::COROUTINE_PRIMITIVE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(2),
        // `coroinject coroName command ?arg ...?` — `command` (index 1) is a
        // command prefix; when `coroName` next resumes, Tcl appends exactly
        // two more words before invoking it: the name of the command that
        // suspended the coroutine (`yield` or `yieldto`) and its current
        // resumption value (a list, for `yieldto`) — "An additional two
        // arguments are appended to the list of arguments to be run" per
        // the Tcl 9.0/9.1 `coroutine(n)` manpage.
        command_prefixes: &[(1, AppendedArity::Exactly(2))],
        script_timing_resolver: Some(coroinject_script_timing),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Queue a command to run inside a suspended coroutine when it next resumes.",
            synopsis: &["coroinject coroName command ?arg ...?"],
            snippet: "coroName must currently be suspended at a yield or yieldto call — injecting into a running (or nonexistent) coroutine is an error. command, together with any arg words already given, is queued to run the next time coroName resumes; two more arguments are appended automatically at that point: the name of the command that suspended the coroutine (yield or yieldto) and its current resumption value (a list, for yieldto). The injected command's result becomes the result of that yield/yieldto call in place of whatever value the caller supplied. coroinject does not resume the coroutine itself, so several injections can be queued before it next runs; queued injections then fire in reverse (last-queued-first) order, each one's result becoming the resumption value seen by the next. Each injection is one-off — it is discarded once it has executed — but it may itself call yield or yieldto. coroprobe is the immediate counterpart: it runs a command right away inside the suspended coroutine instead of deferring it to the next resume.",
            source: "Tcl coroutine(n)",
            examples: "proc collector {} {\n    set acc {}\n    for {set val [yield]} {$val ne \"\"} {set val [yield]} {\n        lappend acc $val\n    }\n    return $acc\n}\ncoroutine collect collector\ncollect abc\ncoroinject collect apply {{how val} {\n    puts \"resumed via $how with $val\"\n    return [string toupper $val]\n}}\ncollect def",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
