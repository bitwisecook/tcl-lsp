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

//! `pkg_mkIndex` — build a `pkgIndex.tcl` package index for automatic
//! loading.
//!
//! This file's registered command name (`CommandSpec.name`, below) is the
//! literal `"pkg_mkindex"` (lower-case `i`), not the real Tcl spelling
//! `pkg_mkIndex` — Tcl command dispatch is case-sensitive, so `pkg_mkindex`
//! does not resolve in real `tclsh` at all. `commands/stdlib/pkg_mkindex.rs`
//! already registers the correctly-cased `"pkg_mkIndex"` with its own
//! metadata, and every other mention of this command elsewhere in the codebase
//! (`package_.rs`'s hover prose, `auto_reset.rs`, `tcl-lsp-core`'s package
//! resolver, the analyser's library-proc signature table in
//! `tcl-compiler/src/analyser/diagnostics/fp/sty.rs`) spells it `pkg_mkIndex`.
//! This audit deliberately leaves the registered name unchanged rather than
//! "fixing" it in isolation: `xtask/src/command_backing.rs`'s
//! WASM-backing-exemption table keys off the exact string `"pkg_mkindex"`
//! today, so renaming it here alone would silently drop that WASM-backing
//! exemption. (The command's iRules exclusion, by contrast, no longer keys off
//! this string at all — it rides on this spec's own surface, see the
//! `dialects` comment in `spec()` below — so a rename would leave the iRules
//! behaviour untouched.) Every other field below documents the real
//! `pkg_mkIndex` command, which is unambiguously the intended referent of this
//! stub; a proper fix needs a coordinated rename across this file and
//! `command_backing.rs` plus a check for the resulting duplicate registration
//! under `"pkg_mkIndex"` alongside `commands/stdlib/pkg_mkindex.rs` — out of
//! scope for a single-file audit.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

// The 8.4 and 8.5 manpages spell the four switches out in the SYNOPSIS
// line itself (`?-direct? ?-lazy? ?-load pkgPat? ?-verbose?`); 8.6, 9.0,
// and 9.1 compress that to a plain `?options...?` placeholder. In every
// one of the five fetched manpages the OPTIONS section documents the same
// four switches plus `--`, so this is a documentation-formatting change
// only, not a functional one — the fully-spelled form below is accurate
// for all five versions.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "pkg_mkIndex ?-direct? ?-lazy? ?-load pkgPat? ?-verbose? dir ?pattern pattern ...?",
    ..FormSpec::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-direct",
        value: OptionValue::flag(),
        detail: "Load the package's files immediately when package require runs. The default when neither -direct nor -lazy is given.",
        ..OptionSpec::DEFAULT
    },
    // The trailing "incompatible with auto_reset, and its use is
    // discouraged" sentence is present in the -lazy OPTIONS text of the
    // 8.5, 8.6, 9.0, and 9.1 manpages (verified verbatim on each) but
    // absent from 8.4's (verified verbatim: the substring "auto_reset"
    // does not appear anywhere on the 8.4 manpage). The flag itself, and
    // its core delay-loading behaviour, are unchanged since 8.4 — only
    // the documented caveat is newer — so this is current-truth prose on
    // a universal option, not a `surface:` gate.
    OptionSpec {
        name: "-lazy",
        value: OptionValue::flag(),
        detail: "Delay loading a package's files until the first use of one of its commands, instead of at package require time. Incompatible with auto_reset, and discouraged for that reason.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-load",
        value: OptionValue::value("pkgPat"),
        detail: "Pre-load already-available packages whose name matches pkgPat (string match rules, case-insensitive) into the child interpreter used to generate the index — needed when indexing a package that depends on another package already loaded in the calling interpreter.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-verbose",
        value: OptionValue::flag(),
        detail: "Log progress during indexing via the tclLog procedure, which prints to stderr by default.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "Marks the end of options, in case dir begins with -.",
        ..OptionSpec::DEFAULT
    },
];

/// Command spec for `pkg_mkIndex` (registered under the legacy
/// `"pkg_mkindex"` name — see the module doc comment).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg_mkindex",
        // `Some(SpecSurface::ALL_TCL)` is deliberate, mirroring
        // `auto_mkindex`'s precedent: F5 iRules excludes this proc (K36322151
        // — `"pkg_mkindex"` sits among the filesystem bans alongside
        // `auto_mkindex`/`auto_mkindex_old`), and that exclusion now falls
        // straight out of this `surface:` group — `ALL_TCL` carries no iRules
        // row, so it never intersects the bare `IRULES` mask. There is no
        // disable list. No other dialect directory (Expect, the EDA vendor
        // shells, tmsh, iApps, Tk, itcl, BPF) registers a
        // `pkg_mkIndex`/`pkg_mkindex` spec of its own or otherwise alters it,
        // and each carries a Tcl release that `ALL_TCL` intersects, so it
        // stays reachable in every one of them.
        surface: Some(SpecSurface::ALL_TCL),
        // A Tcl-level library proc (`library/package.tcl`, `proc
        // pkg_mkIndex {args} {...}`), not a `Tcl_CreateObjCommand`-registered
        // builtin — carries no `CmdInfo` row, so `Traits::SAFE_INTERP_HIDDEN`
        // does not apply (that trait models the exact C-level hidden-command
        // table). Unlike `auto_mkindex`, this proc has no `[interp issafe]`
        // early-return guard of its own — confirmed identical (absent) in
        // the real `library/package.tcl` from both the `core-8-4-20` and
        // `core-9-0-0` tags — so it stays *defined* inside a safe
        // interpreter; calling it there fails once its body reaches a
        // hidden command (`glob`, `file`, `load`) a safe interpreter
        // cannot invoke directly.
        //
        // `Traits::TAINT_SINK`: attacker-influenced `dir` (or `pattern`)
        // reaching this command is a genuine code-execution hazard, and a
        // more direct one than `auto_mkindex`'s. Real `pkg_mkIndex`
        // (`core-8-4-20` and `core-9-0-0`, identical in this respect)
        // builds its scanning interpreter with a plain, unrestricted
        // `interp create` (no command hiding at all, unlike
        // `auto_mkindex`'s heavily-restricted child interpreter), then for
        // each matched file either `load`s it (when its extension matches
        // `[info sharedlibextension]`) or `source`s it otherwise — both
        // real code execution, and `load` additionally maps
        // attacker-influenced native code into the process. The resulting
        // `pkgIndex.tcl` is also written with a tainted `dir` (path
        // traversal / arbitrary file write via `open $dir/pkgIndex.tcl
        // w`), and later becomes a `source`d file itself the first time
        // some interpreter's `package require` triggers the auto-loader.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC | Traits::TAINT_SINK,
        // `pkg_mkIndex ?options...? dir ?pattern ...?` — `dir` is the only
        // required word; every option is a boolean flag or a single-value
        // switch recognised ahead of it, and `pattern` is 0 or more
        // trailing words (defaulting to `*.tcl` and `*[info
        // sharedlibextension]` when none are given — confirmed directly in
        // the real proc body, both `core-8-4-20` and `core-9-0-0`). Unlike
        // `auto_mkindex` (`proc auto_mkindex {dir args}`), the real proc
        // signature here is the bare `proc pkg_mkIndex {args} {...}` — it
        // hand-scans `args` itself to separate leading option words from
        // `dir`, raising `"unknown flag $flag: should be\n$usage"` for
        // anything else starting with `-` — but the effective minimum
        // stays 1 (`dir` alone, no flags, no patterns), unchanged across
        // every documented release, Tcl pkgMkIndex.n 8.4 through 9.1.
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: true,
                ..SideEffect::DEFAULT
            },
            SideEffect {
                target: SideEffectTarget::InterpState,
                reads: true,
                writes: true,
                ..SideEffect::DEFAULT
            },
        ],
        // Two behavioural facts below are not documented in any of the
        // five manpages and were only found by diffing the real
        // `library/package.tcl` proc body across the `core-8-4-20`,
        // `core-8-5-19`, `core-8-6-16`, `core-9-0-0`, and `core-9-1-b0`
        // tags (the last confirmed byte-identical to the `main` branch at
        // audit time):
        //  - The "no files matched" error: 8.4-8.6 glob for the pattern(s)
        //    and, if nothing matches, simply run an empty `foreach` and
        //    write a header-only pkgIndex.tcl — no error. 9.0 added `if
        //    {[llength $fileList] == 0} {return -code error "no files
        //    matched glob pattern \"$patternList\""}` immediately after the
        //    glob; 9.1's proc (heavily refactored internally, via a new
        //    `DiscoverPackageContents` helper) keeps this check verbatim.
        //    Confirmed absent (grep-negative) in 8.4/8.5/8.6 and present in
        //    both 9.0 and 9.1 source.
        //  - The transient `cd`: 8.4's proc does `set oldDir [pwd]; cd
        //    $dir` before its glob and `cd $oldDir`/`cd $dir` pairs around
        //    each child-interpreter iteration (restored on every exit
        //    path). 8.5 replaced this entirely with `glob -directory $dir
        //    -tails …`, which never changes the process's working
        //    directory; 8.6/9.0/9.1 keep 8.5's `-directory` approach. Same
        //    category of fact `auto_mkindex.rs`'s hover documents for that
        //    command (there asserted as version-unqualified — not
        //    re-verified here, out of scope for this file), but for
        //    `pkg_mkIndex` specifically it is 8.4-only.
        // Everything else behavioural (options, arity, defaults, the
        // `-lazy`/auto_reset caveat, `-load`'s case-insensitive match) is
        // identical across all five fetched manpages and all five source
        // tags; the 8.6+ `?options...?` SYNOPSIS compression is confirmed
        // purely typesetting — the proc's own `$usage` wrong-#-args string
        // still spells out every switch, unchanged, in all five source
        // tags — so `FORMS` above is correctly left as one dialect-less
        // spelled-out form rather than split like `package_.rs`'s (whose
        // *own* wrong-#-args wording did change at 8.6).
        hover: Some(HoverSnippet {
            summary: "Build a pkgIndex.tcl package index for automatic loading via package require.",
            synopsis: &[
                "pkg_mkIndex ?-direct? ?-lazy? ?-load pkgPat? ?-verbose? dir ?pattern pattern ...?",
            ],
            snippet: "Scans dir for files matching pattern (glob syntax; *.tcl and *[info sharedlibextension] are assumed when no pattern is given) and, for each match, loads it into a fresh child interpreter to observe the packages it provides — a package provide call for a script file, a Tcl_PkgProvide call for a binary loaded via load. On Tcl 9.0+, a pattern (or the defaults) that matches no file in dir is an error (\"no files matched glob pattern ...\"); on Tcl 8.4-8.6 the same call instead succeeds and writes a pkgIndex.tcl containing only its header comment, with no package entries. The results are written as a pkgIndex.tcl file in dir containing package ifneeded commands, which a later package require consults through the package unknown auto-loading handler. Unlike auto_mkindex, several versions of the same package can be indexed together, with each interpreter's package require picking the version it asks for. -direct (the default) loads a package's files immediately when package require runs; -lazy defers actual loading until the first use of one of its commands, and is documented as incompatible with auto_reset and discouraged for that reason. -load pkgPat pre-loads already-available packages whose name matches pkgPat (case-insensitive string match) into the indexing interpreter — needed when a binary package depends on another, e.g. indexing a Tk-dependent package requires -load Tk in an interpreter that already has Tk loaded. -verbose logs progress via tclLog (stderr by default); -- ends option parsing, for a dir that begins with -. On Tcl 8.4, pkg_mkIndex changes the process's current working directory to dir for the duration of the call and restores it before returning; Tcl 8.5 onward reads dir via glob -directory without ever changing the working directory. Because it scans a real directory and can source or load whatever it finds there, this is a dev-time/build-time tool, not one meant to run against untrusted input.",
            source: "Tcl pkgMkIndex(n)",
            examples: "pkg_mkIndex .\npkg_mkIndex -verbose $dir *.tcl\npkg_mkIndex -load Tk $dir libwidget[info sharedlibextension]",
            return_value: "An empty string.",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
