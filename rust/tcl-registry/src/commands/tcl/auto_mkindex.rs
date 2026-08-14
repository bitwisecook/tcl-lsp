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

//! `auto_mkindex` — generate a `tclIndex` file from Tcl source files in a
//! directory.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "auto_mkindex dir ?pattern pattern ...?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `auto_mkindex`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_mkindex",
        // `dialects: Some(DialectSet::ALL_TCL)` is deliberate, not an
        // oversight: F5 iRules bans this proc (it is one of the K36322151
        // filesystem/process bans), and that ban is now carried by this
        // very `dialects` group. `ALL_TCL` spans every core Tcl version
        // but not the `IRULES` bit, so the spec never intersects iRules'
        // bare `IRULES` availability mask and the command is simply
        // unreachable there — no disable list is involved. Widening this
        // to a `TCL84|IRULES`-style union would re-admit exactly the
        // command the ban exists to exclude. Every other dialect profile
        // (Expect, the EDA vendor shells, tmsh, iApps, BPF) carries a
        // core-version bit that `ALL_TCL` does intersect, so
        // `auto_mkindex` stays reachable in every one of them.
        dialects: Some(DialectSet::ALL_TCL),
        // A Tcl-level library proc (`library/auto.tcl`), not a
        // `Tcl_CreateObjCommand`-registered builtin, so it carries no
        // `CmdInfo` row and is absent from the exact C Tcl
        // safe-interpreter hidden-command table documented on
        // `Traits::SAFE_INTERP_HIDDEN` — that trait does not apply here.
        // Stronger than merely hidden, in fact: `auto.tcl` checks
        // `[interp issafe]` at source time and `return`s before even
        // *defining* `auto_mkindex` (real Tcl 8.6 `library/auto.tcl`), so a
        // genuine safe interpreter never has this proc at all — the proc
        // body's own `error "can't generate index within safe interpreter"`
        // guard only matters for an interpreter that becomes safe after
        // `auto_mkindex` was already defined in it.
        //
        // `Traits::TAINT_SINK`: attacker-influenced `dir` (or `pattern`)
        // reaching this command is a genuine hazard, though a narrower one
        // than a plain `source`. `auto_mkindex` `cd`s into `dir`, globs
        // `pattern`, and evaluates each matching file inside a private
        // child interpreter that hides or renames away almost everything
        // (`info`, `rename`, `proc`, `namespace`, `eval`, `puts` — real
        // Tcl 8.6 `library/auto.tcl`, `auto_mkindex_parser::init`, and
        // identical in 8.4/8.5) so that an unrecognised command is
        // silently a no-op; only `proc`, `namespace eval`, and
        // (conditionally) `tbcload::bcproc` do anything there in every
        // version 8.4-9.1. `oo::class`/`class` recognition is Tcl
        // 8.6-onward only: diffing the real `library/auto.tcl` across the
        // upstream `core-8-4-20`, `core-8-5-19`, `core-8-6-14`,
        // `core-9-0-0`, and `main` (9.1) tags shows the
        // `auto_mkindex_parser::command oo::class {…}` /
        // `auto_mkindex_parser::command class {…}` hooks first appear at
        // 8.6 (when TclOO joined Tcl core) and are wholly absent from
        // 8.4/8.5's shipped `auto.tcl`. `namespace eval` bodies are
        // genuinely (recursively) evaluated in that same restricted
        // environment, in every version. The real risk is downstream: each
        // recorded entry is `[list source [file join $dir <file>]]`
        // (`auto_mkindex_parser::indexEntry`), so a tainted `dir` both
        // writes an attacker-influenced `tclIndex` at an attacker-chosen
        // path (path traversal / arbitrary file write — `open tclIndex w`
        // after the `cd`) and seeds entries that `auto_load` will later
        // `source` *without* any sandbox. A `glob` or `tclIndex`-write
        // failure partway through can also leave the process's working
        // directory changed to `dir`, since `cd $oldDir` only runs on the
        // success path and the one `try`/`on error` in the loop.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC | Traits::TAINT_SINK,
        // `auto_mkindex dir ?pattern pattern ...?` — `dir` required,
        // `pattern` variadic (0 or more, defaulting to `*.tcl` when none
        // are given): Tcl's own `library/auto.tcl` defines
        // `proc auto_mkindex {dir args} …`, matching the `{dir args}`
        // fixture already asserted for this proc's real parameter list by
        // `tcl-compiler`'s FP-STY-13 regression test
        // (`analyser/diagnostics/fp/sty.rs`). Unchanged across every
        // documented release, Tcl library.n 8.4 through 9.1.
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Generate a tclIndex file from Tcl source files in a directory.",
            synopsis: &["auto_mkindex dir ?pattern pattern ...?"],
            snippet: "Searches dir for files matching pattern (glob syntax; *.tcl is assumed when no pattern is given), and for each matching file records the name of every top-level proc it contains; from Tcl 8.6 onward (when TclOO joined core), oo::class create and class create are recorded too, but Tcl 8.4 and 8.5 do not recognize either form. The result is written to a file named tclIndex in dir, in the format auto_load reads back later to load commands on demand. Matching files are evaluated inside a private, heavily restricted child interpreter, not merely text-scanned: only proc, namespace eval, and (when tbcload is available) tbcload::bcproc do anything there in every version, plus oo::class/class from 8.6 on; every other command is a silent no-op, but a script with unusual top-level constructs can still misbehave or raise an error partway through. auto_mkindex_old, which only pattern-matches lines starting with \"proc\" without evaluating anything, is the safer choice for a script with global initialization code or a procedure name containing $, *, [ or ]. auto_mkindex changes the process's current working directory to dir for the duration of the call and restores it before returning; an error while globbing or writing tclIndex can leave the working directory changed. Not available inside a safe interpreter (interp issafe) at all — the proc is never even defined there. auto_mkindex is a Tcl-level library procedure (library/auto.tcl), not a C built-in, so redefining it is a supported override rather than shadowing.",
            source: "Tcl library(n)",
            examples: "auto_mkindex $dir\nauto_mkindex $dir *.tcl *.itcl",
            return_value: "An empty string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
