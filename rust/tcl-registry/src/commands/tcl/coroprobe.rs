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

//! `coroprobe` — run a command inside a suspended coroutine without
//! resuming it.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "coroprobe coroName command ?arg ...?",
    ..FormSpec::DEFAULT
}];

// `coroprobe coroName command ?arg...?` evaluates an arbitrary command *now* in
// the paused coroutine's context and returns its result — so it runs unknown
// code with unknown reads/writes, matching `eval` / `uplevel`.
static SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// Command spec for `coroprobe`.
///
/// `coroprobe`/`coroinject` were added together with Tcl 9.0 (documented on
/// the shared `coroutine(n)` manual page alongside `coroutine`/`yield`/
/// `yieldto`); the command itself is absent from the 8.4, 8.5, and 8.6
/// manual pages (8.6's `coroutine.n` documents only `coroutine`/`yield`/
/// `yieldto`), so `dialects: TCL90_PLUS` gates the whole command, not just
/// one form or option.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroprobe",
        traits: Traits::EVALUATES_CODE | Traits::COROUTINE_PRIMITIVE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(2),
        // `coroprobe coroName command ?arg ...?` — `command` (index 1) is a
        // command prefix run in the coroutine's context with runtime args
        // appended (variadic ⇒ Unknown: a reference, not arity-checked).
        command_prefixes: &[(1, AppendedArity::Unknown)],
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Run a command inside a suspended coroutine, without resuming it.",
            synopsis: &["coroprobe coroName command ?arg ...?"],
            snippet: "coroName must name a coroutine that is currently suspended (parked at a yield or yieldto); a coroutine that is running, has already finished, or does not exist is an error (\"can only inject a probe command into a coroutine\"). command and its args run immediately, in place, inside the coroutine's own call frame and namespace — so info coroutine, local variables, and any upvar/uplevel levels resolve there, not in the caller's. The coroutine is not resumed: afterwards it is left suspended at the same yield/yieldto, still waiting for the same kind of resumption value as before, though any variables the probe touched keep their new values. Since command is a runtime command reference invoked with unknown arguments, treat it like eval/uplevel for static analysis: unknown reads and writes. coroinject is the companion command for scheduling a command to run when the coroutine is next resumed, rather than immediately.",
            source: "Tcl coroutine(n)",
            examples: "proc worker {} {\n    set state 0\n    while 1 {\n        incr state [yield $state]\n    }\n}\ncoroutine counter worker\ncounter 5\ncounter 10\nputs [coroprobe counter set state]\n# -> 15",
            return_value: "The result of command, evaluated inside the suspended coroutine's own frame and namespace.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
