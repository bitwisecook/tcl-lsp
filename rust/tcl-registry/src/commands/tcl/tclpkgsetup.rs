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

//! `tclPkgSetup` — utility procedure for a `pkgIndex.tcl` file to declare a
//! package provided and register per-command auto-loading stubs for it.
//!
//! Genuinely undocumented, not merely unlinked: `tclPkgSetup` is absent from
//! every version's `library.n` page (fetched and read directly, Tcl 8.4
//! through 9.1 — that page documents `auto_execok`/`auto_import`/
//! `auto_load`/`auto_mkindex`/`auto_mkindex_old`/`auto_qualify`/
//! `auto_reset`/`tcl_findLibrary`/`parray`/the `tcl_*Word*` helpers, never
//! `tclPkgSetup`), and it has no manual page of its own either: `package.n`,
//! `pkgMkIndex.n`, and `packagens.n` were also fetched for all five versions
//! and none of them mention it (`packagens.n` documents only `pkg::create`).
//! The facts below instead come from `library/package.tcl` itself — confirmed
//! as the backing file both by `xtask/src/command_backing.rs`'s `STDLIB`
//! table and by fetching the real source — at the `core-8-4-20`,
//! `core-8-5-19`, `core-8-6-18`, `core-9-0-4`, and `core-9-1-b0` release
//! tags. The previous stub spec's `source: "Tcl library (init.tcl)"` named
//! the wrong file: `tclPkgSetup` lives in `package.tcl`, not `init.tcl`, in
//! every one of the five fetched tags.
//!
//! `proc tclPkgSetup {dir pkg version files} {...}` is byte-for-byte
//! identical across all five tags but one cosmetic wrinkle: the
//! `core-8-6-18` tag's non-`load` branch sources with an explicit `source
//! -encoding utf-8 …`, where 8.4, 8.5, 9.0, and 9.1 — and the Tcl 8.6.14
//! copy vendored in this repo, `runtime/rust/vendor/tcl_library/package.tcl`
//! — all use the ambient default encoding instead. That is a sub-patch-level
//! change within the 8.6 branch with no clean boundary this registry's
//! `TclVersion` granularity can express (it is not true of 8.6.14, and 9.0
//! reverted it outright), so it is not modelled as a fact of "Tcl 8.6"
//! anywhere below. No other behavioural difference exists across the five
//! tags: `dir`, `pkg`, `version`, and `files` are the same four required
//! parameters in every one, `package provide $pkg $version` runs first in
//! every one, and the `auto_index`-populating loop is otherwise identical.
//! No `[interp issafe]` guard wraps the proc's own definition in any of the
//! five tags either — unlike `auto_mkindex`, which is never even defined
//! inside a safe interpreter.
//!
//! `::tcl::Pkg::Create` (`pkg::create`) is the only place in
//! `library/package.tcl` itself that calls `tclPkgSetup`: it emits a
//! `tclPkgSetup $dir $name $version {...}` call whenever a `-load`/`-source`
//! filespec names specific procs, confirmed directly in the source of all
//! five fetched tags.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "tclPkgSetup dir pkg version files",
    ..FormSpec::DEFAULT
}];

/// Command spec for `tclPkgSetup`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tclPkgSetup",
        // `surface: Some(SpecSurface::ALL_TCL)` here is deliberate. Under the
        // explicit-per-spec model `ALL_TCL` spans every core Tcl version but
        // not an iRules row, so the spec never intersects iRules' bare
        // `IRULES` availability point and is unreachable there — there is no
        // subtractive disable list. The modelled K36322151 iRules bans in this
        // family are `package` and `pkg_mkindex` outright; `tclPkgSetup` is
        // not separately named among them (confirmed by grep), and no
        // irules/iapps/tk/expect/eda_*/itcl override directory mentions it
        // either (also confirmed by grep, across all ten
        // `commands/{irules,iapps,tk,expect,eda_*,itcl}` directories). Every
        // dialect other than iRules composes a real Tcl release that `ALL_TCL`
        // does intersect — Expect, the five EDA vendor shells, tmsh, and iApps
        // each embed a genuine (if version-pinned) Tcl core per its own
        // `signature_base`/`runtime_base` — so `tclPkgSetup` stays reachable
        // in every one of them, just as stock Tcl ships it. Whether real
        // iRules still exposes a working `tclPkgSetup` given that `package`
        // itself is banned there is not independently confirmed, so
        // `Some(ALL_TCL)` — which excludes it from iRules while leaving it
        // reachable everywhere else — stands as the conservative value here,
        // the same data-over-assumption call `tcllog.rs` documents for itself.
        surface: Some(SpecSurface::ALL_TCL),
        // A Tcl-level library proc (`library/package.tcl` — see the module
        // doc comment), not a `Tcl_CreateObjCommand`-registered builtin, so
        // it carries no `CmdInfo` row and is absent from the exact C Tcl
        // safe-interpreter hidden-command table `Traits::SAFE_INTERP_HIDDEN`
        // documents; that trait does not apply here. No `[interp issafe]`
        // guard wraps its own definition in any of the five fetched source
        // tags (module doc comment), so it stays defined and callable
        // inside a safe interpreter.
        //
        // `Traits::TAINT_SINK`: `dir` and each `files` entry's `file`
        // element build `[file join $dir $f]` paths that are recorded,
        // unvalidated, as `load`/`source` targets in the global
        // `auto_index` array. `tclPkgSetup` does not itself load or source
        // anything, but Tcl's ordinary `unknown`/`auto_load` resolution
        // will blindly `load` or `source` that exact path with no sandbox
        // the first time the associated command name is invoked — the same
        // deferred path-traversal/code-execution shape `auto_mkindex.rs`,
        // `pkg_mkindex.rs`, and `tcl_findlibrary.rs` already carry this
        // trait for.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC | Traits::TAINT_SINK,
        // `tclPkgSetup dir pkg version files` — exactly four mandatory
        // positional words, no `?…?` optional marker anywhere: `proc
        // tclPkgSetup {dir pkg version files} {...}` takes exactly those
        // four plain (no default value) parameters in every one of the
        // five fetched source tags (module doc comment).
        arity: Arity::exact(4),
        return_type: Some(TclType::String),
        side_effects: &[
            // Writes one `auto_index($cmd)` entry per command name listed
            // in each `files` entry; never reads the array first, so a
            // colliding `$cmd` from an earlier call is silently replaced.
            SideEffect {
                target: SideEffectTarget::Variable,
                writes: true,
                ..SideEffect::DEFAULT
            },
            // `package provide $pkg $version`: reads any version already
            // provided for pkg (to raise an error on a genuine conflict)
            // and writes the newly provided version.
            SideEffect {
                target: SideEffectTarget::InterpState,
                reads: true,
                writes: true,
                ..SideEffect::DEFAULT
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Declare a package provided and register per-command auto-loading stubs for it.",
            synopsis: &["tclPkgSetup dir pkg version files"],
            snippet: "A utility procedure called from a pkgIndex.tcl file's script — the one a package ifneeded entry actually runs once the package is requested. It first runs package provide pkg version, declaring pkg available at that version; this raises an error if a different version of pkg was already provided, though calling it again with the same version is harmless. It then walks files, a list of {file type commands} entries, and for every command name in each entry's commands list records one stub in the global auto_index array: a load stub, equivalent to [list load [file join dir file] pkg], when type is exactly the literal string \"load\", or a source stub, [list source [file join dir file]], for any other type (conventionally \"source\"). tclPkgSetup never itself loads or sources anything: each recorded command is only pulled in later, the first time that command name is actually invoked and falls through to Tcl's ordinary unknown/auto_load resolution. pkg::create builds a call in exactly this shape whenever a -load or -source filespec names specific procs; when a filespec names none, its generated script instead loads or sources that whole file unconditionally and bypasses tclPkgSetup and auto_index entirely.",
            source: "Tcl library/package.tcl (undocumented; no manual page)",
            examples: "set dir [file dirname [info script]]\npackage ifneeded mypkg 1.2 [list tclPkgSetup $dir mypkg 1.2 {{mypkg.tcl source {cmdA cmdB}} {mypkgbin.so load {cmdC}}}]",
            return_value: "The empty string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
