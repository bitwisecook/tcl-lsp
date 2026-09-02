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

// -- #1633's `incr` row: a read-modify-write command fires `read` then `write`

/// ```tcl
/// set x 1
/// trace add variable x read R ; trace add variable x write R
/// incr x
/// ```
///
/// tclsh 8.6.16 and 9.0.4 both print `x {} read` then `x {} write`: C's
/// `TclPtrIncrObjVar` fetches through `TclPtrGetVarIdx`, which is the read-trace
/// chokepoint. The same sheet on `lappend` already fired both; `append` fires
/// only `write` (its read is inside `TclPtrSetVarIdx`'s append path), and the
/// array-element spelling fires `arr k read` / `arr k write`.
#[test]
fn incr_lappend_and_append_fire_the_reads_c_fires() {
    let got = transcript(&format!(
        "{RECORDER}\
         set x 1\n\
         trace add variable x read R\n\
         trace add variable x write R\n\
         incr x\n\
         lappend ::log --\n\
         set y a\n\
         trace add variable y read R\n\
         trace add variable y write R\n\
         lappend y z\n\
         lappend ::log --\n\
         set z 1\n\
         trace add variable z read R\n\
         trace add variable z write R\n\
         append z q\n\
         lappend ::log --\n\
         array set arr {{k 1}}\n\
         trace add variable arr read R\n\
         trace add variable arr write R\n\
         incr arr(k)\n\
         join $::log \\n"
    ));
    assert_eq!(
        got,
        "x {} read\nx {} write\n--\n\
         y {} read\ny {} write\n--\n\
         z {} write\n--\n\
         arr k read\narr k write"
    );
}

/// A read trace that *errors* does not fail `incr`: C's `TclPtrIncrObjVar`
/// substitutes 0 for the `NULL` fetch, so `incr x` on `x == 1` yields **1**.
/// Pinned at both releases (identical transcripts).
#[test]
fn an_erroring_read_trace_leaves_incr_counting_from_zero() {
    let sheet = "proc RE {n1 n2 op} { error boom }\n\
                 set x 1\n\
                 trace add variable x read RE\n\
                 set c [catch {incr x} m]\n\
                 trace remove variable x read RE\n\
                 set out \"code=$c msg=$m x=$x\"";
    assert_eq!(transcript(sheet), "code=0 msg=1 x=1");
    assert_eq!(
        transcript_at(sheet, Some(tcl_dialect::TclVersion::V8_6)),
        "code=0 msg=1 x=1"
    );
}

// -- #1633's errorInfo row: the trace's own trace survives the access failure

/// ```tcl
/// proc WE {n1 n2 op} { error "wboom" }
/// set x 1
/// trace add variable x write WE
/// catch {set x 2} m
/// ```
///
/// tclsh 8.6.16 and 9.0.4 print, byte for byte:
///
/// ```text
/// wboom
///     while executing
/// "error "wboom" "
///     (procedure "WE" line 1)
///     invoked from within
/// "WE x {} write"
///     (write trace on "x")
///     invoked from within
/// "set x 2"
/// ```
///
/// with `-errorcode TCL WRITE VARNAME`. The runtime used to start a *fresh*
/// error for `can't set "x": wboom`, discarding the callback's whole chain and
/// with it the `(write trace on "x")` frame.
#[test]
fn a_write_trace_error_keeps_its_chain_and_adds_the_trace_frame() {
    let got = transcript(
        "proc WE {n1 n2 op} { error \"wboom\" }\n\
         set x 1\n\
         trace add variable x write WE\n\
         set c [catch {set x 2} m]\n\
         set out \"$c|$m|$::errorCode|$::errorInfo\"",
    );
    assert_eq!(
        got,
        "1|can't set \"x\": wboom|TCL WRITE VARNAME|\
         wboom\n    while executing\n\"error \"wboom\" \"\n\
         \x20   (procedure \"WE\" line 1)\n    invoked from within\n\"WE x {} write\"\n\
         \x20   (write trace on \"x\")\n    invoked from within\n\"set x 2\""
    );
}

/// The read half, on an array element — the frame names the element
/// (`(read trace on "a(k)")`) because C snapshots `part2` from the access
/// spelling. `-errorcode` is `TCL READ VARNAME`. Same at 8.6.16 and 9.0.4.
#[test]
fn a_read_trace_error_names_the_element_in_its_frame() {
    let got = transcript(
        "proc RE {n1 n2 op} { error \"rboom\" }\n\
         array set a {k 1}\n\
         trace add variable a read RE\n\
         set c [catch {set q $a(k)} m]\n\
         set out \"$c|$m|$::errorCode|$::errorInfo\"",
    );
    assert_eq!(
        got,
        "1|can't read \"a(k)\": rboom|TCL READ VARNAME|\
         rboom\n    while executing\n\"error \"rboom\" \"\n\
         \x20   (procedure \"RE\" line 1)\n    invoked from within\n\"RE a k read\"\n\
         \x20   (read trace on \"a(k)\")\n    invoked from within\n\"set q $a(k)\""
    );
}

// -- #1633's two array-element rows, which differ by release ----------------

/// The recording sheet both element rows share: an array with a whole-array
/// trace (`A`) and an element trace (`E`), exercised through the `a(k)`
/// spelling, through an `upvar #0 a(k) e` alias, and on unset.
const ELEMENT_SHEET: &str = "set ::log {}\n\
     proc A {n1 n2 op} { lappend ::log [list ARR $n1 $n2 $op] }\n\
     proc E {n1 n2 op} { lappend ::log [list ELE $n1 $n2 $op] }\n\
     array set a {k 1}\n\
     trace add variable a write A\n\
     trace add variable a(k) write E\n\
     lappend ::log {= set a(k) 2}\n\
     set a(k) 2\n\
     lappend ::log {= upvar+set}\n\
     upvar #0 a(k) e\n\
     set e 5\n\
     lappend ::log {= unset a(k)}\n\
     trace add variable a unset A\n\
     trace add variable a(k) unset E\n\
     unset a(k)\n\
     join $::log \\n";

/// tclsh 9.0.4 recovers the element from the resolved `Var` when the access
/// spelling names none (`tclTrace.c`:2560-2565, `tclVar.c`:2634-2640), so a
/// write through an element alias fires the array's traces *and* the element's
/// with `name2 = k`, and `unset a(k)` reports `name1 = a(k)` — the recovered
/// `part2` stops `TclCallVarTraces` re-splitting the name.
#[test]
fn at_9_0_an_element_alias_fires_the_array_traces_and_unset_keeps_the_spelling() {
    assert_eq!(
        transcript(ELEMENT_SHEET),
        "= set a(k) 2\nARR a k write\nELE a k write\n\
         = upvar+set\nARR e k write\nELE e k write\n\
         = unset a(k)\nARR a(k) k unset\nELE a(k) k unset"
    );
}

/// tclsh 8.6.16 (and 8.4/8.5, which have neither block) leaves `part2` NULL:
/// the alias write fires only the element's own trace, with an empty `name2`,
/// and `unset a(k)` reports the split `name1 = a`.
#[test]
fn at_8_6_an_element_alias_fires_only_the_elements_own_trace() {
    assert_eq!(
        transcript_at(ELEMENT_SHEET, Some(tcl_dialect::TclVersion::V8_6)),
        "= set a(k) 2\nARR a k write\nELE a k write\n\
         = upvar+set\nELE e {} write\n\
         = unset a(k)\nARR a k unset\nELE a k unset"
    );
}

/// Registration follows the same cell: `trace add variable e …` through an
/// element alias installs the trace on the element, so `trace info variable e`
/// and `trace info variable a(k)` both report it and `trace info variable a`
/// (the array itself) reports nothing. Release-independent — both tclsh
/// releases print this — while the `name2` the alias write reports is not.
#[test]
fn a_trace_added_through_an_element_alias_lands_on_the_element() {
    let sheet = "set ::log {}\n\
                 proc E {n1 n2 op} { lappend ::log [list ELE $n1 $n2 $op] }\n\
                 array set a {k 1}\n\
                 upvar #0 a(k) e\n\
                 trace add variable e write E\n\
                 lappend ::log \"info-e: [trace info variable e]\"\n\
                 lappend ::log \"info-a(k): [trace info variable a(k)]\"\n\
                 lappend ::log \"info-a: [trace info variable a]\"\n\
                 set a(k) 3\n\
                 set e 4\n\
                 join $::log \\n";
    assert_eq!(
        transcript(sheet),
        "info-e: {write E}\ninfo-a(k): {write E}\ninfo-a: \n\
         ELE a k write\nELE e k write"
    );
    assert_eq!(
        transcript_at(sheet, Some(tcl_dialect::TclVersion::V8_6)),
        "info-e: {write E}\ninfo-a(k): {write E}\ninfo-a: \n\
         ELE a k write\nELE e {} write"
    );
}
