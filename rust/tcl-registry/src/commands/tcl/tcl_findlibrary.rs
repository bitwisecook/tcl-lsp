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

//! `tcl_findLibrary` — standard search procedure Tcl extensions call
//! during their own initialisation to locate their script library.
//!
//! The SYNOPSIS — `tcl_findLibrary basename version patch initScript
//! enVarName varName`, six mandatory positional words, no `?…?` optional
//! marker anywhere — and the body prose describing the search mechanism
//! are word-for-word identical across every fetched manpage, Tcl
//! library.n 8.4 through 9.1. The one textual delta across the five: Tcl
//! 8.6, 9.0, and 9.1 add the sentence "The patch argument is not used."
//! immediately after the basename+version directory-naming sentence;
//! Tcl 8.4's and 8.5's body text has no such sentence at all.
//!
//! The real `library/auto.tcl` proc body (diffed across the
//! `core-8-4-20`, `core-8-5-19`, `core-8-6-16`, `core-9-0-4`, and
//! `core-9-1-b0` release tags, fetched from `core.tcl-lang.org`) tells a
//! more precise story than that one documented sentence:
//!  - `patch` actually influenced the search in Tcl 8.4 only — it named
//!    two `$basename$patch`-based fallback directories inside a
//!    compatibility code block guarded by `if {1}`. From Tcl 8.5 onward
//!    that same block is guarded by `if {0}` (dead code, never
//!    executed), so `patch` has had zero effect on the search since
//!    8.5 — one release before the manpage started saying so.
//!  - Tcl 9.0 (unchanged in 9.1) added an entirely new, wholly
//!    undocumented search step: a zipfs-based lookup that checks for the
//!    library already mounted under `[zipfs root]`, and failing that,
//!    locates a loaded shared library for `basename` (or computes a
//!    candidate path from `pkgconfig`) and `zipfs mount`s it. Absent
//!    from 8.4, 8.5, and 8.6.
//!  - The internal `source` call that finally loads `initScript` passes
//!    `-encoding utf-8` in Tcl 8.6 only — 8.4, 8.5, 9.0, and 9.1 all
//!    source with the ambient default encoding instead. A genuinely
//!    non-contiguous, single-version-only fact.
//!  - Every version sources at the global stack frame unconditionally,
//!    via `uplevel #0 [list source …]`, regardless of the frame
//!    `tcl_findLibrary` itself was called from.
//!
//! None of this is `dialects:`-gated on the `CommandSpec` below: the
//! documented argument list and arity never change, so it is all
//! current, version-independent *behaviour* prose, captured in the
//! hover snippet and field comments instead of a version gate.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "tcl_findLibrary basename version patch initScript enVarName varName",
    ..FormSpec::DEFAULT
}];

/// Command spec for `tcl_findLibrary`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_findLibrary",
        // `dialects: Some(DialectSet::ALL_TCL)` is deliberate, not an
        // oversight: F5 iRules bans this proc (it is one of the K36322151
        // filesystem/process bans), and under the explicit-per-spec model
        // that ban is carried by this very `dialects` group. `ALL_TCL`
        // spans every core Tcl version but not the `IRULES` bit, so the
        // spec never intersects iRules' bare `IRULES` availability mask and
        // `tcl_findLibrary` is simply unreachable there — no disable list is
        // involved. Widening this to a `TCL84|IRULES`-style union would
        // re-admit exactly the command the ban exists to exclude. The
        // `irules_banned_commands_never_resolve` contract test
        // (`tcl-registry/tests/dialect_profile.rs`) enforces this directly:
        // `tcl_findLibrary` is in its banned set and must not carry the
        // `IRULES` bit nor resolve under f5-irules. Every other dialect
        // profile (Expect, the EDA vendor shells, tmsh, iApps, BPF) carries
        // a core-version bit that `ALL_TCL` does intersect, so it stays
        // reachable in each; and no dialect (Tk and itcl included) registers
        // a competing spec or otherwise bans it.
        dialects: Some(DialectSet::ALL_TCL),
        // A Tcl-level library proc (`library/auto.tcl` — confirmed by
        // name in every one of the five fetched release tags, never
        // `library/init.tcl`), not a `Tcl_CreateObjCommand`-registered
        // builtin, so it carries no `CmdInfo` row and is absent from the
        // exact C Tcl safe-interpreter hidden-command table documented on
        // `Traits::SAFE_INTERP_HIDDEN` — that trait does not apply here.
        // Unlike `auto_mkindex` (never even defined inside a safe
        // interpreter), `tcl_findLibrary` is fully defined and functional
        // there: its own body special-cases `[interp issafe]` to skip the
        // `[file exists]` probe (safe interpreters have no such command)
        // and attempts `source` on every candidate directly instead.
        //
        // `Traits::TAINT_SINK`: the real proc unconditionally appends
        // `initScript` to every candidate directory via `[file join $dir
        // $initScript]` and sources the result (`uplevel #0 [list source
        // $file]`, or Tcl 8.6's `source -encoding utf-8 $file`) with no
        // further check. `file join` lets a later *absolute* path
        // component discard every directory before it, so an
        // attacker-influenced `initScript` can redirect the sourced file
        // to a path of its own choosing regardless of which directory was
        // being searched — the same path-traversal-to-code-execution
        // shape `pkg_mkIndex`'s and `auto_mkindex`'s specs already carry
        // this trait for. `basename` and `version` also feed candidate
        // directory names in every version, `patch` does too but in Tcl
        // 8.4 only (see the `arity` comment below), and `enVarName`
        // selects which environment variable is trusted as a search
        // directory — so taint reaching any of the six arguments is a
        // hazard, not just `initScript` alone.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC | Traits::TAINT_SINK,
        // `tcl_findLibrary basename version patch initScript enVarName
        // varName` — exactly six mandatory positional words. No `?…?`
        // optional marker appears anywhere in the SYNOPSIS of any of the
        // five fetched manpages (Tcl library.n 8.4 through 9.1, all
        // byte-for-byte identical on this line), and the real
        // `library/auto.tcl` proc signature — `proc tcl_findLibrary
        // {basename version patch initScript enVarName varName} …` —
        // takes exactly those six plain (no default value) parameters in
        // every one of the `core-8-4-20`, `core-8-5-19`, `core-8-6-16`,
        // `core-9-0-4`, and `core-9-1-b0` release tags. Calling with five
        // or seven arguments is Tcl's ordinary "wrong # args" error in
        // every version.
        arity: Arity::exact(6),
        // `varName` (index 5) is the name of a global variable this
        // command assigns: the real proc does `upvar #0 $varName
        // the_library` and, once a usable directory is found, `set
        // the_library $i` — so the caller-named global is bound to a
        // directory-path string, not to whatever `tcl_findLibrary` itself
        // returns (an empty string on success). The same destructuring
        // "writes one thing, returns another" shape `gets_.rs` documents
        // for its own `varName` target.
        arg_roles: &[(5, ArgRole::VarWrite)],
        assigns_variable_at: Some(5),
        return_type: Some(TclType::String),
        var_write_typing: VarWriteTyping::Fixed(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                ..SideEffect::DEFAULT
            },
            // Reads the `enVarName`-named entry of the global `env`
            // array, and writes the `varName`-named global — see the
            // `arg_roles`/`assigns_variable_at` comment above.
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: true,
                writes: true,
                ..SideEffect::DEFAULT
            },
            // What the located `initScript` itself does once sourced is
            // unenumerable from `tcl_findLibrary`'s own argument words —
            // the same "unknowable statically" gap `source`'s own spec
            // leaves as a placeholder rather than inventing a write.
            SideEffect {
                ..SideEffect::DEFAULT
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Search standard directories for an extension's script library and source its init script.",
            synopsis: &["tcl_findLibrary basename version patch initScript enVarName varName"],
            snippet: "Standard search procedure Tcl extensions call during their own initialisation to locate their script library. The library directory's final path component is normally basename followed by version (e.g. tk8.0), though it may be named \"library\" inside a build tree. patch is accepted for call-signature compatibility; it actually influenced the search only in Tcl 8.4 (via two legacy basename-plus-patch fallback directories) and has had no effect since Tcl 8.5, though the manpage did not say so explicitly until Tcl 8.6 (\"The patch argument is not used\").\n\nIf varName already names an existing, non-empty global variable — typically set by C code during application initialisation — that directory alone is tried and the location search below is skipped entirely. Otherwise, candidate directories are tried in this order: the directory named by the environment variable enVarName; relative to the Tcl library directory; relative to the executable file in the standard installation bin or bin/arch directory; relative to the executable file in the current build tree; relative to the executable file in a parallel build tree. Since Tcl 9.0, an additional zipfs-based step — not documented in the manpage — also looks for the library already mounted under a zipfs root, or mounts one located via a loaded shared library or a pkgconfig-reported path.\n\nThe first candidate directory whose initScript sources without error wins: varName is set to that directory and tcl_findLibrary returns an empty string immediately. initScript is always sourced at the global (#0) stack frame via uplevel #0, regardless of the frame tcl_findLibrary itself was called from; Tcl 8.6 alone sources it with an explicit -encoding utf-8, while Tcl 8.4, 8.5, 9.0, and 9.1 all use the ambient default encoding instead. If no candidate directory yields a working initScript, tcl_findLibrary raises an error listing every directory it searched and the error produced by each one it actually attempted to source.\n\nInside a safe interpreter, the file-existence probe is skipped (safe interpreters have no file exists command) and every candidate is sourced unconditionally instead; tcl_findLibrary is itself defined and usable there, unlike auto_mkindex. tcl_findLibrary is a standard-library Tcl procedure, autoloaded like the auto_* helpers, and a script may freely redefine it.",
            source: "Tcl library(n)",
            examples: "# A typical extension's own init script calls this once, near the\n# top, to locate and source its companion Tcl library:\nif {[catch {\n    tcl_findLibrary mytcl 2.3 2.3.1 mytcl.tcl MYTCL_LIBRARY ::mytcl::library\n}]} {\n    return -code error \"can't find a usable mytcl.tcl in any of the usual places\"\n}\nputs \"mytcl library found in $::mytcl::library\"",
            return_value: "An empty string on success — the directory that was found is available via varName, not via the return value. Raises an error if initScript cannot be located and sourced successfully in any candidate directory.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
