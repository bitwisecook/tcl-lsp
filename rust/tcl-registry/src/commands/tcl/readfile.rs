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

//! `readFile` — read the contents of a text or binary file (Tcl 9.0+, TIP 670).
//
// VERIFIED: `readFile` is documented in library.n's FILE ACCESS HELPERS
// section in the Tcl 9.0 and 9.1 TclCmd manual trees, with byte-identical
// synopsis and description text in both (`readFile filename
// ?text|binary?`; TIP 670 landed in 9.0 and picked up no delta in 9.1) —
// library.n in the 8.4, 8.5, and 8.6 trees documents no such command
// (confirmed absent from each version's own NAME/command list, not
// merely unlisted here), and none of the five fetched TclCmd trees
// serves a dedicated readFile.n/readFile.html page either — unlike
// `const`/`lpop`, which each have their own "Tcl Built-In Commands"
// manual page (a genuine C-command signal readFile lacks; a direct
// fetch of readFile.html in every tree returns a soft-404 "URL Not
// Found" body). The command genuinely does not exist before 9.0, and is
// a plain, autoloaded, redefinable Tcl library procedure rather than a
// bytecode-compiled builtin: TIP 670's shipped reference implementation
// is `proc readFile {filename {mode text}} { set mode [tcl::prefix
// match -message mode -error $ERR {binary text} $mode]; set f [open
// $filename [dict get {text r binary rb} $mode]]; try { return [read
// $f] } finally { close $f } }` — `mode` is resolved through `tcl::prefix
// match`, so any unambiguous abbreviation of `text`/`binary` is
// accepted, not only the two full words spelled out in the manpage
// synopsis.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "readFile filename ?text|binary?",
    ..FormSpec::DEFAULT
}];

/// `mode` (position 1) is `text` (the default) or `binary`, or any
/// unambiguous prefix of either — TIP 670's reference implementation
/// resolves it with `tcl::prefix match -message "mode" ... {binary
/// text} $mode`, so a single letter such as `b` or `t` already resolves
/// unambiguously and is accepted. A flat per-word enum can't express "or
/// an abbreviation of one of these", so this is completion/hover data
/// only — `readFile` has no `closed_value_args` entry for this
/// position, since a legitimate abbreviation would otherwise be
/// misreported as an invalid value.
const MODE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "text",
        detail: "Decode using the system encoding and the platform's line-ending translation (the default when mode is omitted). The result includes any trailing newline.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "binary",
        detail: "Return the file's exact, uninterpreted bytes, with no encoding or translation applied.",
        ..ArgValue::DEFAULT
    },
];

/// Command spec for `readFile`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "readFile",
        traits: Traits::TAINT_SOURCE
            // TIP 670's reference implementation opens the file with a
            // plain `open $filename [dict get {text r binary rb}
            // $mode]]` on the raw filename argument — the same
            // unconditional pipe-detection a direct `open` call gets, so
            // a filename beginning with `|` is run as a command
            // pipeline instead of being read as a file (see open_.rs's
            // own pipe-form documentation). Tainted data reaching this
            // argument is therefore a code-execution sink, not merely a
            // file-read one — the same reasoning foreachLine's own
            // `open $filename "r"` already carries (see foreachline.rs).
            | Traits::TAINT_SINK
            // A standard-library Tcl procedure — documented in
            // library.n alongside `parray`/`auto_load`/`foreachLine`,
            // autoloaded, and available for a script to redefine — so
            // redefining it must not fire the W113 "overrides a
            // built-in" warning.
            | Traits::OVERRIDABLE_LIBRARY_PROC,
        surface: Some(SpecSurface::TCL90_PLUS),
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                ..SideEffect::DEFAULT
            },
            // Pipe-form: see the `TAINT_SINK` trait comment above — a
            // `|command` filename spawns a child process and reads its
            // stdout, exactly as `open`'s own pipeline form does
            // (open_.rs).
            SideEffect {
                target: SideEffectTarget::Process,
                reads: true,
                ..SideEffect::DEFAULT
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Read the whole contents of a file and return them.",
            synopsis: &["readFile filename ?text|binary?"],
            snippet: "Reads filename and returns its contents. The optional second argument, mode — text or binary, or any unambiguous prefix of either — selects how: text (the default) decodes the file using the system encoding and the platform's line-ending translation, and the result includes any trailing newline; binary returns the exact, uninterpreted bytes with no translation applied. An unrecognised or ambiguous mode raises an error. The file is always closed before readFile returns, even if the read itself fails. Because filename is passed straight to open, a name beginning with | runs as a command pipeline instead of being opened as a plain file, exactly as with open itself. readFile is a standard-library Tcl procedure, like parray or foreachLine — it is autoloaded and a script may redefine it.",
            source: "Tcl library(n)",
            examples: "set config [readFile settings.txt]\nset logo [readFile logo.png binary]\nputs \"[string length $config] characters read\"",
            return_value: "The file's contents: a decoded string, including any trailing newline, in text mode; the exact, uninterpreted bytes in binary mode.",
        }),
        forms: FORMS,
        arg_values: &[(1, MODE_VALUES)],
        ..CommandSpec::DEFAULT
    }
}
