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

//! Oracle-pinned trace semantics — the R5 bucket of the WASM native-lowering
//! plan (#1633, #1574, #1575, #1569).
//!
//! Every expectation in this file is a *transcript*, and every transcript was
//! produced by running the same sheet through `tclsh9.0` (9.0.4) — and through
//! `tclsh8.6` (8.6.16) as well wherever the issue says the releases differ.
//! The sheets are plain Tcl and are quoted verbatim in the test bodies, so a
//! reader can paste one into a real `tclsh` and re-derive the expectation
//! without any harness of ours in the way.
//!
//! Transcripts, not counts: a trace bug is almost always the *argument list* or
//! the *order*, and a test that only counts firings passes on both.

use tcl_runtime::interp::{Code, Interp};

/// Run `sheet` on a default (9.0-emulating) interpreter and return its final
/// result. The sheets below all end in a `join $::log \n`, so the return value
/// is the firing transcript.
fn transcript(sheet: &str) -> String {
    transcript_at(sheet, None)
}

/// [`transcript`] against a pinned emulated release.
fn transcript_at(sheet: &str, version: Option<tcl_dialect::TclVersion>) -> String {
    let mut interp = Interp::new();
    if let Some(v) = version {
        interp.set_runtime_version(v);
    }
    let code = interp.eval_str(sheet.as_bytes());
    let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
    assert_eq!(
        code,
        Code::Ok,
        "sheet failed: {result}\n--- sheet ---\n{sheet}"
    );
    result
}

/// The recording callback every sheet installs: one line per firing, holding
/// the *exact* argument list the callback received.
const RECORDER: &str = "set ::log {}\nproc R {n1 n2 op} { lappend ::log [list $n1 $n2 $op] }\n";

// -- #1633's `upvar` row: firing follows the cell, not the spelling ----------

/// ```tcl
/// proc P {} {
///     set loc 0
///     upvar 0 loc alias
///     trace add variable loc write R
///     set alias 5
///     set loc 6
/// }
/// ```
///
/// tclsh 8.6.16 and 9.0.4 both fire twice — `alias` then `loc` — because the
/// alias and its target are one `Var` in C and the trace list hangs off that
/// `Var`. Resolving the trace identity from the *access spelling* instead
/// fires only once (the defect P1 recorded).
#[test]
fn a_write_through_an_upvar_alias_fires_the_targets_trace() {
    let got = transcript(&format!(
        "{RECORDER}\
         proc P {{}} {{\n\
         \x20   set loc 0\n\
         \x20   upvar 0 loc alias\n\
         \x20   trace add variable loc write R\n\
         \x20   set alias 5\n\
         \x20   set loc 6\n\
         }}\n\
         P\n\
         join $::log \\n"
    ));
    assert_eq!(got, "alias {} write\nloc {} write");
}

/// The mirror image: registering *through* the alias must catch a write to the
/// target. Same tclsh transcript at both releases.
#[test]
fn a_trace_registered_through_an_upvar_alias_fires_for_the_target() {
    let got = transcript(&format!(
        "{RECORDER}\
         proc P {{}} {{\n\
         \x20   set loc 0\n\
         \x20   upvar 0 loc alias\n\
         \x20   trace add variable alias write R\n\
         \x20   set loc 6\n\
         \x20   set alias 7\n\
         }}\n\
         P\n\
         join $::log \\n"
    ));
    assert_eq!(got, "loc {} write\nalias {} write");
}

/// `trace info variable` answers for the *cell*, so both spellings report the
/// one trace. C looks the variable up and walks that `Var`'s list; matching the
/// registration spelling textually reports nothing for the alias.
#[test]
fn trace_info_answers_for_the_cell_not_the_spelling() {
    let got = transcript(
        "proc R {n1 n2 op} {}\n\
         set ::log {}\n\
         proc P {} {\n\
         \x20   set loc 0\n\
         \x20   upvar 0 loc alias\n\
         \x20   trace add variable loc write R\n\
         \x20   lappend ::log [trace info variable loc]\n\
         \x20   lappend ::log [trace info variable alias]\n\
         \x20   trace remove variable alias write R\n\
         \x20   lappend ::log [trace info variable loc]\n\
         }\n\
         P\n\
         join $::log \\n",
    );
    assert_eq!(got, "{write R}\n{write R}\n");
}

/// `name1` is the caller's spelling, not the resolved name: C passes `part1`
/// straight through (`TclCallVarTraces`). Every line below is `tclsh9.0` and
/// `tclsh8.6` output for the same sheet.
#[test]
fn the_callback_is_handed_the_access_spelling() {
    let got = transcript(&format!(
        "{RECORDER}\
         namespace eval n {{ variable v 0 }}\n\
         trace add variable ::n::v write R\n\
         set ::n::v 1\n\
         namespace eval n {{ set v 2 }}\n\
         proc P {{}} {{ global ::n::v ; set v 3 }}\n\
         proc Q {{}} {{ upvar #0 ::n::v q ; set q 4 }}\n\
         P\n\
         Q\n\
         join $::log \\n"
    ));
    assert_eq!(got, "::n::v {} write\nv {} write\nv {} write\nq {} write");
}

/// The release-independent half of the same rule, kept at 8.6 so the pinning is
/// explicit: the fallback release resolves and reports identically here.
#[test]
fn the_access_spelling_rule_holds_at_8_6() {
    let got = transcript_at(
        &format!(
            "{RECORDER}\
             set a2 foo\n\
             trace add variable a2 write R\n\
             set ::a2 X\n\
             join $::log \\n"
        ),
        Some(tcl_dialect::TclVersion::V8_6),
    );
    assert_eq!(got, "::a2 {} write");
}
