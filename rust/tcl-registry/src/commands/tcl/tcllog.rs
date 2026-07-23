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

//! `tclLog` — default logging hook for Tcl's library code.
//!
//! Genuinely undocumented, not merely unlinked: `tclLog` is absent from
//! every version's `library.n` page (fetched and read directly, Tcl 8.4
//! through 9.1 — that page documents `auto_execok`/`auto_import`/
//! `auto_load`/`auto_mkindex`/`auto_mkindex_old`/`auto_qualify`/
//! `auto_reset`/`tcl_findLibrary`/`parray`/the `tcl_*Word*` helpers, never
//! `tclLog`), and it has no manual page of its own on any version either
//! (`TclCmd/tclLog.html`/`.htm` 404s). The facts below instead come from
//! `library/init.tcl` and `library/package.tcl` themselves, fetched at the
//! `core-8-4-20`/`core-8-5-19`/`core-8-6-16`/`core-9-0-4`/`core-9-1-b0`
//! release tags. The guarded definition — `if {[namespace which -command
//! tclLog] eq ""} { proc tclLog {string} { catch {puts stderr $string} } }`
//! — is byte-for-byte identical in every one of the five fetched versions,
//! including the guard itself (so an application that predefines its own
//! `tclLog` before `init.tcl` runs is never overridden by this default).
//! The only textual change anywhere nearby across all five tags is a
//! comment typo fix ("overwitten" -> "overwritten") between the 8.5.19
//! and 8.6.16 tags — no behavioural difference. `library/package.tcl`'s
//! calls into `tclLog` carry the same messages and the same `-verbose`
//! gating in every one of the five tags too (only the surrounding
//! control-flow syntax modernised, `catch`/`if` in 8.4 to `try`/`on` from
//! 8.6 on — unrelated to `tclLog` itself): `tclPkgUnknown` (the default
//! `package unknown` handler, registered by `init.tcl`'s `package unknown
//! tclPkgUnknown`) unconditionally logs any unexpected error while
//! sourcing a candidate `pkgIndex.tcl` file during `package require`, and
//! `pkg_mkIndex` (`tclPkgMkIndex`)'s `-verbose` option routes its indexing
//! progress output through it — matching `pkg_mkindex.rs`'s own hover
//! text. `init.tcl`'s `unknown` handler also calls `tclLog` to record the
//! reconstructed command text when it resolves a C-shell-style history
//! substitution (`!!`, `!N`, `^old^new^`) before re-evaluating it.
//!
//! `dialects: None` (universal) is a deliberate reading of the current
//! data, not an oversight: `tclLog` is conspicuously absent from F5
//! iRules' subtractive `IRULES_DISABLED_COMMANDS` list
//! (`tcl-dialect/src/profile.rs`) even though every *other* `init.tcl`
//! library proc already audited (`auto_execok`, `auto_import`,
//! `auto_load`, `auto_mkindex`, `auto_mkindex_old`, `auto_qualify`,
//! `auto_reset`, `bgerror`, `tcl_findLibrary`) is named there —
//! `parray.rs` documents the identical kind of gap for its own, unrelated
//! `library/parray.tcl` proc. No other dialect profile (Expect, the five
//! EDA vendor shells, tmsh, iApps, Tk, itcl, BPF) lists `tclLog` in its
//! `disabled_commands` either, and no dialect-specific override anywhere
//! under `commands/irules`, `commands/expect`, `commands/eda_*`,
//! `commands/iapps`, `commands/tk`, or `commands/itcl` mentions the name
//! (checked by grep). Whether real iRules genuinely exposes a working
//! `tclLog` is not independently confirmed — only that the current
//! ground-truth ban list does not exclude it — so this follows the same
//! data-over-assumption call `parray.rs` made, rather than pre-empting
//! `IRULES_DISABLED_COMMANDS` with a `dialects:` restriction here: per
//! `tcl_findlibrary.rs`'s identical note, a `Some(DialectSet::ALL_TCL)`
//! gate has no `IRULES` bit, so it would silently exclude `tclLog` from
//! iRules regardless of what the disable list says, moving the ban onto
//! the wrong axis. (`Some(DialectSet::ALL_TCL)` was this file's own
//! previous, unaudited value — the same stub placeholder
//! `disabled_in_irules.rs`, `tclpkgsetup.rs`, and `tclpkgunknown.rs`
//! still carry.)

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tclLog string",
    dialects: None,
}];

// The default body's only effect is the stderr write inside `catch {puts
// stderr $string}` (see the module doc comment) — `LogIo` is the
// dedicated target for exactly this shape; its own doc comment on
// `SideEffectTarget` names `puts stderr` as the textbook example.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::LogIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// Command spec for `tclLog`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tclLog",
        dialects: None,
        // A redefinable Tcl library proc — see `Traits::OVERRIDABLE_LIBRARY_PROC`.
        // The default body directly wraps `puts stderr $string` (module
        // doc comment above) — the same output-injection hazard `puts`
        // itself carries on its `string` argument — so `TAINT_SINK` +
        // `taint_output_sink: Some("T101")` below mirror `puts_.rs`
        // exactly.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC | Traits::TAINT_SINK,
        arity: Arity::exact(1),
        return_type: Some(TclType::Int),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Default logging hook for Tcl's library code; writes to stderr unless redefined.",
            synopsis: &["tclLog string"],
            snippet: "tclLog is a small, user-redefinable proc defined in Tcl's init.tcl, guarded so that an application which predefines its own tclLog before init.tcl runs keeps that definition instead of the default one. The default implementation is catch {puts stderr $string}: it writes string to the stderr channel, silently discarding any error the write itself raises (e.g. no stderr channel available) rather than propagating it. Tcl's own library code calls tclLog rather than puts directly wherever it wants to report something without aborting: the unknown command handler logs the reconstructed command text when it resolves a history substitution (!!, !N, ^old^new^) before re-evaluating it; the default package unknown handler (tclPkgUnknown) logs any unexpected error while sourcing a candidate pkgIndex.tcl file during package require; and pkg_mkIndex's -verbose option routes its indexing progress output through it. Redefining tclLog — to write to a file, a real logging package, or nowhere at all — changes every one of these call sites at once, which is the point of the indirection.",
            source: "Tcl library/init.tcl (undocumented; no manual page)",
            examples: "tclLog \"starting up\"\n\n# Redirect Tcl's own internal logging (unknown's history-substitution\n# log, tclPkgUnknown's index-file errors, pkg_mkIndex -verbose, ...)\n# from stderr to a file instead\nset ::logChan [open app.log a]\nproc tclLog {string} {\n    puts $::logChan $string\n    flush $::logChan\n}",
            return_value: "0 if the write to stderr succeeded (the ordinary case); 1 if the write itself raised an error (e.g. no stderr channel available), since tclLog's own catch converts that failure into a plain return instead of propagating it.",
        }),
        taint_output_sink: Some("T101"),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
