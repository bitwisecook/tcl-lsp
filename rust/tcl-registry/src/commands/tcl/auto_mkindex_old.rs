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

//! `auto_mkindex_old` — generate a `tclIndex` file from Tcl source files by
//! naive line-based `proc` scanning, instead of real evaluation.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "auto_mkindex_old dir ?pattern pattern ...?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `auto_mkindex_old`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_mkindex_old",
        // `surface: Some(SpecSurface::ALL_TCL)` is deliberate, not an
        // oversight: F5 iRules bans this proc (it is one of the K36322151
        // filesystem/process bans), and that ban is carried by this very
        // surface. `ALL_TCL` spans every core Tcl release but names no iRules
        // row, so it never admits the iRules point and the command is simply
        // unreachable there — no disable list is involved. Adding an iRules
        // row would re-admit exactly the command the ban exists to exclude.
        // Every other dialect profile (Expect, the EDA vendor shells, tmsh,
        // iApps, BPF) runs a Tcl core that `ALL_TCL` does admit, so
        // `auto_mkindex_old` stays reachable in every one of them.
        surface: Some(SpecSurface::ALL_TCL),
        // A Tcl-level library proc (`library/auto.tcl`), not a
        // `Tcl_CreateObjCommand`-registered builtin, so it carries no
        // `CmdInfo` row and is absent from the exact C Tcl safe-interpreter
        // hidden-command table documented on `Traits::SAFE_INTERP_HIDDEN` —
        // that trait does not apply here. Stronger than merely hidden, in
        // fact: `auto.tcl` checks `[interp issafe]` at source time and
        // `return`s before even *defining* `auto_mkindex` or
        // `auto_mkindex_old` (real Tcl 8.6 `library/auto.tcl`), so a genuine
        // safe interpreter never has either proc at all.
        //
        // `Traits::TAINT_SINK`: the same downstream hazard as `auto_mkindex`
        // (see that spec's comment), with even less protection at the
        // source end. `auto_mkindex_old` never evaluates the scanned files
        // — it only regexp-matches lines of text — so it lacks even
        // `auto_mkindex`'s restricted child-interpreter sandbox, but it
        // still `cd`s into an attacker-influenced `dir`, `glob`s `pattern`
        // there, and unconditionally opens a file named `tclIndex` for
        // writing in that directory: a tainted `dir` is both a
        // path-traversal write primitive (the index file lands wherever
        // `dir` points) and, downstream, seeds `auto_index` entries
        // (`[list source [file join $dir <file>]]`) that `auto_load` will
        // later `source` with no sandbox at all.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC | Traits::TAINT_SINK,
        // `auto_mkindex_old dir ?pattern pattern ...?` — `dir` required,
        // `pattern` variadic (0 or more, defaulting to `*.tcl` when none are
        // given): every real `library/auto.tcl` defines
        // `proc auto_mkindex_old {dir args} …`, the same `{dir args}` shape
        // as `auto_mkindex` — confirmed unchanged in the actual shipped
        // source across Tcl 8.4 (upstream branch core-8-4-branch), 8.5
        // (core-8-5-branch), 8.6 (locally installed
        // /usr/share/tcltk/tcl8.6/auto.tcl), 9.0 (upstream tag
        // core-9-0-0), and 9.1 (upstream pre-release tag core-9-1-b0).
        // This is *not* the same as "documented identically" across those
        // releases, though: `library.n` gives `auto_mkindex_old` its own
        // NAME-list entry and SYNOPSIS line only in the 8.4 and 8.5
        // manuals; from 8.6 onward (8.6, 9.0, 9.1 manuals all checked) the
        // manual drops both and folds the description into `auto_mkindex`'s
        // own entry as a trailing paragraph ("Auto_mkindex_old (which has
        // the same syntax as auto_mkindex) parses …") instead — the proc
        // itself is unaffected, only the manual's presentation changed.
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        // The `error $msg $info $code` bug described in the hover text
        // below is narrower than "the shipped Tcl 8.6 implementation": it
        // affects only the tclIndex-write failure branch, and only from
        // Tcl 8.5 onward. Confirmed by diffing the real `auto_mkindex_old`
        // proc body across upstream `library/auto.tcl`: Tcl 8.4
        // (core-8-4-branch) sets local `code`/`info` from the
        // `errorCode`/`errorInfo` globals immediately before this same
        // `error` call, so it correctly re-raises the original failure
        // there — no bug in 8.4. Tcl 8.5 (core-8-5-branch), 8.6 (locally
        // installed /usr/share/tcltk/tcl8.6/auto.tcl), 9.0 (core-9-0-0),
        // and 9.1 (core-9-1-b0) all switched the enclosing `catch` to the
        // newer `msg opts` form and added a `return -options $opts $msg`
        // line below to use it, but left the old `error $msg $info $code`
        // line in place above it unchanged — dead code that fires first
        // (since `error` throws immediately) and masks the real failure
        // with an unrelated "no such variable" error instead. The
        // corresponding read-failure branch (inside the per-file `foreach`
        // loop, earlier in the same proc) has no such bug in any of these
        // versions: 8.4 correctly uses `errorCode`/`errorInfo` there too,
        // and 8.5 onward correctly uses `return -options $opts $msg`
        // directly with no dead code above it.
        hover: Some(HoverSnippet {
            summary: "Generate a tclIndex file from Tcl source files using a simple line-based proc scan.",
            synopsis: &["auto_mkindex_old dir ?pattern pattern ...?"],
            snippet: "The original tclIndex generator, superseded by auto_mkindex but kept for source files auto_mkindex can't safely handle. Searches dir for files matching pattern (glob syntax; *.tcl is assumed when no pattern is given) and reads each one line by line without evaluating any of it: a line is treated as a procedure definition only when the literal text \"proc\" begins the line with no leading whitespace, and the following word is taken as the procedure name, normalised via auto_qualify into a fully global-qualified name when it already contains a namespace separator (::), or left as a bare name otherwise. Because nothing is ever executed, this is the recommended generator for source files with global initialization side effects, or with procedure names containing $, *, [, or ] that would confuse auto_mkindex's real interpreter-based parser — but the same lack of evaluation means an indented proc (inside a namespace eval or class body), a proc split across multiple lines, or one built by string substitution is silently skipped. The result is written to tclIndex in dir, in the same version-2.0 index format auto_mkindex produces. auto_mkindex_old changes the process's current working directory to dir for the duration of the call and restores it before returning; on a failure opening or writing tclIndex it still restores the directory, but in Tcl 8.5, 8.6, 9.0, and 9.1 that handler then reports an unrelated \"no such variable\" error instead of the real one (it calls error $msg $info $code, referencing undefined info/code locals left over from before the enclosing catch was changed to the msg opts form) — so the underlying I/O failure's message and options are lost rather than propagated on those versions. Tcl 8.4 does not have this bug: its equivalent handler sets info/code from the errorCode/errorInfo globals immediately before using them. Not available inside a safe interpreter at all — interp issafe is checked at source time and the proc is never defined there.",
            source: "Tcl library(n)",
            examples: "auto_mkindex_old $dir\nauto_mkindex_old $dir *.tcl *.itcl",
            return_value: "An empty string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
