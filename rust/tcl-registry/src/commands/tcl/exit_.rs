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

//! `exit` — end the application.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "exit ?returnCode?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `exit`.
///
/// Synopsis, arity, and semantics are identical across Tcl 8.4, 8.5, 8.6,
/// 9.0, and 9.1 — `exit ?returnCode?` has taken exactly one form since 8.4,
/// with no options ever added, removed, or changed in any of the five
/// manpages (`tcl-lang.org/man/tcl{8.4,8.5,8.6,9.0,9.1}/TclCmd/exit.html`,
/// the 8.6 tree served under the `.htm` extension). `returnCode` defaults
/// to 0 in every version; the 8.6, 9.0, and 9.1 manpages add "abort" to
/// the KEYWORDS section (alongside the "exit"/"process" keywords already
/// present in 8.4–8.5) but the command's synopsis, description, and
/// behaviour are unchanged.
///
/// `exit` is one of the K36322151 commands F5's TMM interpreter strips from
/// iRules, so its surface below is `ALL_TCL`, which does not carry an iRules
/// row and therefore is simply absent under iRules — this exclusion is correct
/// and deliberate (same pattern as `open`, also banned from the TMM sandbox).
/// Expect registers its own unrelated `exit` (`-onexit`/`-noexit`, closing
/// spawned processes rather than the interpreter) as a fully separate spec
/// under `commands/expect/exit.rs`, loaded only for the `EXPECT` dialect, so
/// no dialect-specific form is needed here either.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exit",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::BYTE_COMPILED
            | Traits::TERMINATES_BLOCK
            | Traits::TERMINATES_PROCESS
            | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::new(0, 1),
        // The process-terminal trait says that a *successful* invocation
        // cannot fall through.  This descriptor owns the value grammar too:
        // a static non-integer returnCode is TCL_ERROR, while a substituted
        // one is the typed ProcessExit-or-Error union for flow consumers.
        completion: Some(CompletionDescriptor::process_exit_status()),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "End the application",
            synopsis: &["exit ?returnCode?"],
            snippet: "Terminates the process immediately, returning returnCode to the system as the exit status; nothing after the call runs, in this command or anywhere else in the process. returnCode must be an integer and defaults to 0 (success) when omitted. Non-zero codes are conventionally interpreted as error cases by the calling process, so exit is commonly used at the end of a top-level error handler to signal a fatal condition with a distinct status.",
            source: "Tcl exit(n)",
            examples: "if {[catch {main} msg]} {\n    puts stderr \"unexpected script error: $msg\"\n    exit 2\n}\nexit 0",
            return_value: "Never returns: the process terminates immediately and no value comes back to the calling script.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
