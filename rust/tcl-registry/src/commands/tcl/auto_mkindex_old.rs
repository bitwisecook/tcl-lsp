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

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "auto_mkindex_old dir ?pattern pattern ...?",
    dialects: None,
}];

/// Command spec for `auto_mkindex_old`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_mkindex_old",
        // Universal Tcl data (`dialects: None`) is deliberate, not an
        // oversight: F5 iRules bans this proc, but the ban is modelled on
        // the *profile's* subtractive `IRULES_DISABLED_COMMANDS` list
        // (tcl-dialect/src/profile.rs), which explicitly names
        // `"auto_mkindex_old"` alongside the rest of the K36322151
        // filesystem/process bans — never as a `dialects:` restriction here
        // (folding it into a `TCL84|IRULES`-style union on the spec would
        // re-admit exactly the command that list exists to ban; see the
        // comment on `IRULES_DISABLED_COMMANDS`). No other dialect profile
        // (Expect, the EDA vendor shells, tmsh, iApps, BPF) has a
        // `disabled_commands` list at all, so `auto_mkindex_old` stays
        // reachable in every one of them.
        dialects: None,
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
        // given): Tcl's own `library/auto.tcl` defines
        // `proc auto_mkindex_old {dir args} …`, the same `{dir args}` shape
        // as `auto_mkindex`. Unchanged across every documented release, Tcl
        // library.n 8.4 through 9.1.
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Generate a tclIndex file from Tcl source files using a simple line-based proc scan.",
            synopsis: &["auto_mkindex_old dir ?pattern pattern ...?"],
            snippet: "The original tclIndex generator, superseded by auto_mkindex but kept for source files auto_mkindex can't safely handle. Searches dir for files matching pattern (glob syntax; *.tcl is assumed when no pattern is given) and reads each one line by line without evaluating any of it: a line is treated as a procedure definition only when the literal text \"proc\" begins the line with no leading whitespace, and the following word is taken as the procedure name, normalised via auto_qualify into a fully global-qualified name when it already contains a namespace separator (::), or left as a bare name otherwise. Because nothing is ever executed, this is the recommended generator for source files with global initialization side effects, or with procedure names containing $, *, [, or ] that would confuse auto_mkindex's real interpreter-based parser — but the same lack of evaluation means an indented proc (inside a namespace eval or class body), a proc split across multiple lines, or one built by string substitution is silently skipped. The result is written to tclIndex in dir, in the same version-2.0 index format auto_mkindex produces. auto_mkindex_old changes the process's current working directory to dir for the duration of the call and restores it before returning; on a failure opening or writing tclIndex it still restores the directory, but the shipped Tcl 8.6 implementation then reports an unrelated \"no such variable\" error instead of the real one at that point (it references undefined info/code locals), so the underlying I/O failure's message and options are lost rather than propagated. Not available inside a safe interpreter at all — interp issafe is checked at source time and the proc is never defined there.",
            source: "Tcl library(n)",
            examples: "auto_mkindex_old $dir\nauto_mkindex_old $dir *.tcl *.itcl",
            return_value: "An empty string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
