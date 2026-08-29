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

//! `writeFile` — write contents to a text or binary file (Tcl 9.0+, TIP 670).
//
// VERIFIED: `writeFile` is documented in library.n's FILE ACCESS HELPERS
// section in the Tcl 9.0 and 9.1 TclCmd manual trees, with byte-identical
// synopsis and description text in both (`writeFile filename
// ?text|binary? contents`; TIP 670 landed in 9.0 and picked up no
// documented delta in 9.1 — the two library.html trees differ only in
// doc-anchor line-number IDs and the version banner, the same
// return.html pattern return_.rs notes) — library.n in the 8.4, 8.5, and
// 8.6 trees documents no such command (confirmed absent from each
// version's own NAME/command list, not merely unlisted here; the 8.6
// tree was fetched via the .htm extension per the documented
// tcl-lang.org redirect gotcha), and none of the five fetched TclCmd
// trees serves a dedicated writeFile.n/writeFile.html page either — a
// direct fetch of writeFile.html/.htm on every one of the five trees
// returns a soft-404 "URL Not Found" body, the same pattern readFile's
// own page lookup hits (see readfile.rs). The command genuinely does not
// exist before 9.0, and is a plain, autoloaded, redefinable Tcl library
// procedure rather than a bytecode-compiled builtin: library/tclIndex
// maps it to library/writefile.tcl via `set auto_index(writeFile) [list
// ::tcl::Pkg::source ...]`, exactly as readFile maps to
// library/readfile.tcl. TIP 670's shipped reference implementation —
// matching the real library/writefile.tcl source on both the
// core-9-0-branch and core-9-1-b0 tags, which differ only by an
// unrelated, behaviourally-inert `switch -integer --` vs plain `switch`
// spelling (both dispatch identically on the always-integer `llength
// $args` subject) — is `proc writeFile {args} {switch [llength $args]
// {2 {lassign $args filename data; set mode text} 3 {lassign $args
// filename mode data; set mode [tcl::prefix match -message mode -error
// $ERR {binary text} $mode]} default {return -code error -errorcode
// {TCL WRONGARGS} "wrong # args: should be \"$COMMAND filename ?mode?
// data\""}}; set f [open $filename [dict get {text w binary wb}
// $mode]]; try {puts -nonewline $f $data} finally {close $f}}` — mode is
// resolved through tcl::prefix match, so any unambiguous abbreviation of
// text/binary is accepted, not only the two full words spelled out in
// the manpage synopsis; a bad mode raises an error with errorCode `TCL
// LOOKUP MODE <value>`; and — unlike readFile, an ordinary `proc …
// {mode text}` whose wrong-arg-count error is Tcl's own generic one — an
// out-of-range argument count raises writeFile's own hand-written
// wrong-#-args error with errorCode `TCL WRONGARGS`, because writeFile
// takes `args` and dispatches on `llength` itself rather than using a
// default-valued formal parameter.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "writeFile filename ?text|binary? contents",
    ..FormSpec::DEFAULT
}];

// Unlike readFile's trailing `?text|binary?` (readfile.rs's MODE_VALUES /
// open_.rs's ACCESS_VALUES precedent for a completion-only enumerable
// value), `mode` here sits *between* the two required arguments rather
// than after them: `writeFile filename ?text|binary? contents`.
// `CommandSpec::arg_values` completion is keyed purely by 0-based
// positional index with no arity awareness (see
// `tcl_lsp_core::completion`'s command-level positional arg-value
// lookup, which resolves index 1 unconditionally off `word_idx - 1`), so
// declaring `arg_values` at index 1 would offer text/binary as
// completions on the *contents* argument in the far more common 2-arg
// call shape (`writeFile $path $data`, where index 1 *is* contents, not
// mode) — actively misleading rather than merely non-exhaustive. The two
// literal mode words are documented in the hover snippet's prose
// instead.

/// Command spec for `writeFile`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "writeFile",
        // TIP 670's reference implementation opens the file with a
        // plain `open $filename [dict get {text w binary wb} $mode]]`
        // on the raw filename argument — the same unconditional
        // pipe-detection a direct `open` call gets (see open_.rs's own
        // pipe-form documentation), so a filename beginning with `|`
        // spawns a command pipeline and writes contents to its stdin
        // instead of writing an ordinary file. Tainted data reaching
        // this argument is therefore a code-execution sink — the same
        // reasoning readFile's own filename argument carries (see
        // readfile.rs), just with the data flowing the other way.
        // writeFile's own return value is always the empty string (see
        // the hover snippet below), so — unlike readFile — it carries
        // no TAINT_SOURCE.
        traits: Traits::TAINT_SINK
            // A standard-library Tcl procedure — documented in
            // library.n alongside `readFile`/`parray`/`foreachLine`,
            // autoloaded via library/tclIndex's `auto_index(writeFile)`
            // entry, and available for a script to redefine — so
            // redefining it must not fire the W113 "overrides a
            // built-in" warning.
            | Traits::OVERRIDABLE_LIBRARY_PROC,
        surface: Some(SpecSurface::TCL90_PLUS),
        arity: Arity::new(2, 3),
        return_type: Some(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                writes: true,
                ..SideEffect::DEFAULT
            },
            // Pipe-form: see the `TAINT_SINK` trait comment above — a
            // `|command` filename spawns a child process and writes
            // contents to its stdin, the mirror image of readFile's own
            // pipe-form side effect (readfile.rs), which reads the
            // child's stdout instead.
            SideEffect {
                target: SideEffectTarget::Process,
                writes: true,
                ..SideEffect::DEFAULT
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Write contents to a file, as text or as raw bytes.",
            synopsis: &["writeFile filename ?text|binary? contents"],
            snippet: "Writes contents to filename, truncating an existing file or creating a new one as needed, and always returns the empty string. mode — text or binary, or any unambiguous prefix of either, given as the middle argument only when three arguments are supplied — selects how: text (the default) writes contents using the system encoding and the platform's line-ending translation; binary writes the exact bytes with no translation applied. Neither mode appends a trailing newline; include one in contents if the file should end with one. An unrecognised or ambiguous mode raises an error (errorCode TCL LOOKUP MODE); calling writeFile with any other number of arguments raises its own \"wrong # args\" error (errorCode TCL WRONGARGS) rather than the generic one an ordinary proc's argument defaults would produce. The file is always closed before writeFile returns, even if the write itself fails. Because filename is passed straight to open, a name beginning with | runs as a command pipeline instead of being opened as a plain file, exactly as with open itself — here the pipeline's stdin receives contents. writeFile is a standard-library Tcl procedure, like readFile or foreachLine — it is autoloaded and a script may redefine it.",
            source: "Tcl library(n)",
            examples: "writeFile settings.txt \"debug = 1\\n\"\nwriteFile logo.png binary $pngBytes\nwriteFile report.txt text [format \"Generated %s\\n\" [clock format [clock seconds]]]",
            return_value: "The empty string, unconditionally — writeFile's only observable result is the write itself.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
