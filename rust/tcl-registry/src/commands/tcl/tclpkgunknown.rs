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

//! `tclPkgUnknown` — the default `package unknown` handler: scans
//! `auto_path` for `pkgIndex.tcl` files.

use crate::prelude::*;

// Tcl 8.4's real formal parameter list (`library/package.tcl`:
// `proc tclPkgUnknown {name version {exact {}}}`, selected whenever the
// undocumented `TCL_TIP268` compile define is absent — true of every
// standard 8.4 build, including the shipped core-8-4-20 release) requires
// `name` and `version`, with `exact` optional: a call with fewer than 2 or
// more than 3 words is a genuine "wrong # args" error there. Tcl 8.5
// relaxed this to `proc tclPkgUnknown {name args}` (only `name` required,
// everything else variadic), unchanged through 8.6, 9.0, and 9.1. Every
// version's own "Arguments" header comment calls all of `name`/`version`/
// `exact`/`args` "Not used" — confirmed by reading the body: it only ever
// consults the global `auto_path`, never its own formal parameters.
const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "tclPkgUnknown name ?arg arg ...?",
        dialects: Some(DialectSet::TCL85_PLUS),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "tclPkgUnknown name version ?exact?",
        dialects: Some(DialectSet::TCL84),
        ..FormSpec::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::Variable,
        // Always reads the global `auto_path`. Also credited with a
        // write: a sourced `pkgIndex.tcl` extending `auto_path` while the
        // scan runs is an explicitly anticipated, handled case (the
        // proc re-diffs `auto_path` against its last-seen copy after
        // every directory specifically to catch this), not a wild edge
        // case — the same standard `auto_load_index.rs` applies to its
        // own indirect writes through sourced `tclIndex` files.
        reads: true,
        writes: true,
        ..SideEffect::DEFAULT
    },
    SideEffect {
        target: SideEffectTarget::FileIo,
        // `glob`, `file exists`/`dirname`/`join`, and `source` all read
        // the filesystem; the proc itself never writes a file.
        reads: true,
        ..SideEffect::DEFAULT
    },
    SideEffect {
        target: SideEffectTarget::InterpState,
        // Indirect but the entire point of the proc: a sourced
        // `pkgIndex.tcl` almost always calls `package ifneeded`, which
        // is what actually populates the interpreter's package database
        // (`package_.rs` models `package ifneeded`/`forget` against this
        // same `InterpState` target).
        reads: false,
        writes: true,
        ..SideEffect::DEFAULT
    },
];

/// Command spec for `tclPkgUnknown`.
///
/// No manual page documents this proc in any of Tcl 8.4, 8.5, 8.6, 9.0, or
/// 9.1 — both `library.n` and `package.n` were fetched for all five
/// versions and neither mentions it by name. Its only documentation is its
/// own header comment in `library/package.tcl`, which is where every field
/// below is sourced from directly. See the per-field comments for the
/// specific 8.4/8.5/8.6/9.0/9.1 deltas found there.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tclPkgUnknown",
        // Not a `Tcl_CreateObjCommand` builtin — a Tcl-level library proc
        // (`library/package.tcl` in every one of 8.4 through 9.1, never
        // `init.tcl`) with no `CmdInfo` row of its own.
        // `Some(DialectSet::ALL_TCL)` carries the same value as its sibling
        // autoloading procs (`auto_mkindex`/`auto_load_index`): under the
        // explicit-per-spec model `ALL_TCL` spans every core Tcl version but
        // not the `IRULES` bit, so the spec never intersects iRules' bare
        // `IRULES` availability mask and is unreachable there — there is no
        // subtractive disable list. The modelled K36322151 iRules bans in
        // this family are `package` and `pkg_mkindex` outright;
        // `tclPkgUnknown` itself is not separately named among them. Every
        // other dialect composes a real Tcl-version bit that `ALL_TCL` does
        // intersect (Expect, Tk, the five EDA vendor shells, tmsh, iApps),
        // so `tclPkgUnknown` stays reachable in each. Whether real iRules
        // still exposes a working `tclPkgUnknown` given that `package`
        // itself is banned there is not independently confirmed, so
        // `Some(ALL_TCL)` — which excludes it from iRules while leaving it
        // reachable everywhere else — stands as the conservative value here.
        dialects: Some(DialectSet::ALL_TCL),
        // A redefinable Tcl library proc — see
        // `Traits::OVERRIDABLE_LIBRARY_PROC`. Not `SAFE_INTERP_HIDDEN`:
        // unlike `auto_mkindex` (which refuses to even be defined inside
        // a safe interpreter), `tclPkgUnknown` explicitly special-cases
        // `[interp issafe]` in its own body, and every version installs
        // it as the safe interpreter's `package unknown` handler too.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        // The tight floor across every version without over-flagging a
        // legal call: Tcl 8.4's `{name version {exact {}}}` needs at
        // least 2 words (and no more than 3), but Tcl 8.5 through 9.1's
        // `{name args}` needs only 1, with no upper bound. `at_least(1)`
        // is exact for 8.5-9.1 and permissive for 8.4 (a 1-word 8.4 call
        // is really a "wrong # args" error there, which this floor
        // doesn't flag) rather than a `Some(ALL_TCL)`-wide floor of 2
        // that would wrongly flag every legal 1-word call under 8.5-9.1.
        // Mirrors the cross-version floor reasoning `package_.rs` uses
        // for `package vsatisfies`'s own arity.
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Scan auto_path for pkgIndex.tcl files as Tcl's default package unknown handler.",
            synopsis: &[
                "tclPkgUnknown name ?arg arg ...?",
                "tclPkgUnknown name version ?exact?",
            ],
            snippet: "The library/package.tcl implementation of Tcl's default package unknown handler, invoked as a last resort when package require can't satisfy a package from the ifneeded database. On Tcl 8.4 it is installed directly (package unknown tclPkgUnknown, or tcl::MacOSXPkgUnknown tclPkgUnknown on a non-safe Darwin interpreter); from Tcl 8.5 onward it is always installed nested inside ::tcl::tm::UnknownHandler, which first searches the Tcl Modules path (TIP 189) for a matching .tm file before falling through to this proc's own scan. The scan itself is unchanged across every version: walk auto_path from last directory to first, and for each one, source any pkgIndex.tcl found either directly inside it or one level down in an immediate subdirectory, skipping any directory already scanned. The requested package name (and, on Tcl 8.4, a required version and optional exact flag; on 8.5 onward, any further requirement words) is accepted purely for interface compatibility — every version's own header comment marks it \"Not used\", and the scan runs identically no matter what was actually requested. A pkgIndex.tcl that extends auto_path while the scan is running is honoured: newly-added directories are folded into the remaining scan. A pkgIndex.tcl unreadable due to a POSIX permission error is silently skipped in every version; any other error while sourcing one is logged via tclLog rather than raised — except that from Tcl 9.0 an error tagged with the {TCL PACKAGE VERSIONCONFLICT} error code (a pkgIndex.tcl re-registering a package version that conflicts with one already provided) is silently skipped instead, and Tcl 9.0 additionally matches the literal text \"version conflict for package\" in the error message as a fallback for the same case, one that Tcl 9.1 drops in favour of the error code alone. Tcl 8.6 sources every pkgIndex.tcl as UTF-8 explicitly, unlike 8.4/8.5's platform-default encoding; Tcl 9.0 and 9.1 route sourcing through an internal ::tcl::Pkg::source helper instead, which also keeps the file out of package files' bookkeeping. Runs safely inside a Safe Base interpreter, unlike auto_mkindex, which refuses to even be defined there. Always returns an empty string. Has no dedicated Tcl manual page in any version — library.n and package.n both omit it.",
            source: "Tcl library (library/package.tcl)",
            examples: "lappend auto_path /opt/mylib/tcl\npackage require MyLib\n\n# package require triggers this automatically on a miss; it can also\n# be called directly to force an immediate rescan of auto_path:\ntclPkgUnknown MyLib 1.0",
            return_value: "Always an empty string; the proc's effect is entirely the package ifneeded registrations made by whichever pkgIndex.tcl files it sources along the way.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
