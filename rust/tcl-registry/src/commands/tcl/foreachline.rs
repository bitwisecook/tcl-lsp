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

//! `foreachLine` — iterate over the lines of a text file (Tcl 9.0+, TIP 670).
//
// VERIFIED: `foreachLine` is documented in library.n's FILE ACCESS HELPERS
// section in the Tcl 9.0 and 9.1 TclCmd manual trees, with identical
// synopsis/description text in both (`varName filename body`; TIP 670
// landed in 9.0 and picked up no delta in 9.1) — library.n in the 8.4, 8.5,
// and 8.6 trees documents no such command, no `readFile`/`writeFile`, and
// no `lpop`/`const`; the command genuinely does not exist before 9.0.
// Unlike `const`/`lpop` (which got their own dedicated manual pages,
// consistent with being real C-implemented commands), `foreachLine` stays
// filed under library.n alongside `parray`/`auto_load` — it is still a
// plain, redefinable Tcl-coded library procedure, not a bytecode-compiled
// builtin. The TIP's reference implementation binds `varName` with
// `upvar 1` and reads with `while {[gets $f line] >= 0} { uplevel 1 $body }`
// inside a `try`/`finally` that always closes the channel and re-levels a
// `return` from body, which is why the loop-body semantics (VarWrite,
// break/continue/return passthrough, guaranteed close) mirror plain
// `foreach` even though the underlying value source is a file, not a list.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "foreachLine varName filename body",
    dialects: None,
}];

/// Command spec for `foreachLine`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "foreachLine",
        traits: Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            // The reference implementation opens the file with a plain
            // `open $filename "r"` — a `filename` whose first character is
            // `|` is therefore run as a command pipeline exactly as with a
            // direct `open` call (verified: `open`'s pipe-detection is
            // unconditional on the fileName argument, independent of the
            // access mode word), so tainted data reaching this argument is
            // a code-execution sink like `open`/`exec`, not merely a
            // file-I/O one.
            | Traits::TAINT_SINK
            // `foreachLine varName filename {…body…}` is exactly the
            // `HEAD NAME ARGS BODY` four-token, trailing-braced shape the
            // signature scanner's factory-wrapper heuristic looks for
            // (`maybe_record_factory_candidate`); without this trait every
            // call would be mis-recorded as a potential proc-factory
            // candidate, same as plain `foreach`.
            | Traits::NOT_PROC_FACTORY
            // A standard-library Tcl procedure (documented in library.n
            // alongside `parray`/`auto_load`), autoloaded and available for
            // a script to redefine — redefining it must not fire W113.
            | Traits::OVERRIDABLE_LIBRARY_PROC,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::new(3, 3),
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Body)],
        return_type: Some(TclType::String),
        lowering_hook: Some(LoweringHookId::ForeachLine),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
            // Binds `varName` in the caller's scope on every iteration.
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Iterate over each line of a text file, evaluating a script once per line.",
            synopsis: &["foreachLine varName filename body"],
            snippet: "Reads filename one line at a time using the platform's default text-mode conventions (system encoding and line-ending translation), assigns each line — with its line terminator stripped — to the variable named by varName in the caller's scope, then evaluates body once per line. error, return, break, and continue all behave normally inside body: error and an uncaught return propagate out of the call to foreachLine itself, break stops the iteration early, and continue skips to the next line. The file is always closed before foreachLine returns, even if body errors or exits early. As with open, a filename whose first character is | is treated as a command pipeline rather than a plain file. foreachLine is a standard-library Tcl procedure, like parray or readFile — it is autoloaded and a script may redefine it.",
            source: "Tcl library(n)",
            examples: "foreachLine line report.txt {\n    puts \"> $line\"\n}\n\nset errorCount 0\nforeachLine line server.log {\n    if {[string match \"*ERROR*\" $line]} {\n        incr errorCount\n    }\n}\nputs \"found $errorCount error lines\"",
            return_value: "The empty string, assuming no I/O error and no error raised by body.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
