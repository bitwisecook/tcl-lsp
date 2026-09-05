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

// -- #1633's re-entrancy rows: what a callback changes mid-firing -----------

/// A callback that removes a *later* trace stops it firing in the same pass —
/// C's firing loop follows `active.nextTracePtr`, which `Tcl_UntraceVar2`
/// rewrites — while one it *adds* is not picked up until the next access.
/// `trace info` sees each change immediately, from inside the callback.
///
/// tclsh 8.6.16 and 9.0.4 print exactly the transcript below; the runtime used
/// to fire `B` after `M` had removed it, because the firing loop snapshotted
/// the callbacks up front.
#[test]
fn a_trace_removed_during_firing_does_not_fire_in_that_pass() {
    let got = transcript(
        "set ::log {}\n\
         proc B {n1 n2 op} { lappend ::log B }\n\
         proc C {n1 n2 op} { lappend ::log C }\n\
         proc M {n1 n2 op} {\n\
         \x20   lappend ::log \"M-in: [trace info variable ::x]\"\n\
         \x20   trace remove variable ::x write B\n\
         \x20   trace add variable ::x write C\n\
         \x20   lappend ::log \"M-out: [trace info variable ::x]\"\n\
         }\n\
         set x 0\n\
         trace add variable x write B\n\
         trace add variable x write M\n\
         set x 1\n\
         lappend ::log \"after: [trace info variable ::x]\"\n\
         set x 2\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "M-in: {write M} {write B}\n\
         M-out: {write C} {write M}\n\
         after: {write C} {write M}\n\
         C\n\
         M-in: {write C} {write M}\n\
         M-out: {write C} {write C} {write M}"
    );
}

/// The command and execution firing loops follow the same rule: a trace an
/// earlier callback removed does not fire in that pass. C's
/// `CallCommandTraces` and `TclCheckExecutionTraces` walk the live list through
/// `nextPtr` and `Tcl_UntraceCommand` unlinks a record at once — for `enter`
/// and `delete`, which run newest-first, and for `leave`, whose reverse scan
/// reaches the *oldest* first.
///
/// tclsh 8.6.16 and 9.0.4 print exactly the transcript below; these three loops
/// used to snapshot the callback strings, so `E2`, `D2` and `L1` still fired.
#[test]
fn a_command_or_execution_trace_removed_during_firing_does_not_fire() {
    let got = transcript(
        "set ::log {}\n\
         proc target {} { return T }\n\
         proc E1 args { trace remove execution target enter E2\n\
         \x20   lappend ::log \"E1: [trace info execution target]\" }\n\
         proc E2 args { lappend ::log \"E2 fired\" }\n\
         trace add execution target enter E2\n\
         trace add execution target enter E1\n\
         target\n\
         proc victim {} {}\n\
         proc D1 args { trace remove command victim delete D2\n\
         \x20   lappend ::log \"D1: [trace info command victim]\" }\n\
         proc D2 args { lappend ::log \"D2 fired\" }\n\
         trace add command victim delete D2\n\
         trace add command victim delete D1\n\
         rename victim {}\n\
         proc lt {} { return L }\n\
         proc L1 args { lappend ::log \"L1 fired\" }\n\
         proc L2 args { trace remove execution lt leave L1\n\
         \x20   lappend ::log \"L2: [trace info execution lt]\" }\n\
         trace add execution lt leave L2\n\
         trace add execution lt leave L1\n\
         lt\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "E1: {enter E1}\n\
         D1: {delete D1}\n\
         L2: {leave L2}"
    );
}

/// A **delete** walk is handed the dying token's own trace list
/// (`CallCommandTraces`, `tclBasic.c` 9.0.4:3972-3993), so a callback that
/// re-creates the command under the same name takes the *name-keyed* entry over
/// while the rest of that list still fires. Only an explicit `trace remove`
/// cancels a pending callback, and once a replacement holds the name a
/// `trace remove` reaches the replacement's empty list and cancels nothing.
///
/// tclsh 8.6.16 and 9.0.4 print exactly the transcript below. Deciding liveness
/// by name-keyed table membership skipped `OLD` and `B1`, because the
/// callback's `proc` emptied the entry the walk was consulting.
#[test]
fn a_delete_callback_recreating_the_command_does_not_cancel_the_walk() {
    let got = transcript(
        "set ::log {}\n\
         proc foo {} { return FOO }\n\
         proc OLD {o n op} { lappend ::log [list OLD $o $n $op] }\n\
         proc NEW {o n op} { lappend ::log [list NEW $o $n $op]\n\
         \x20   proc foo {} { return FOO2 } }\n\
         trace add command foo delete OLD\n\
         trace add command foo delete NEW\n\
         rename foo {}\n\
         lappend ::log \"call:[foo]\" \"traces:[trace info command foo]\"\n\
         proc A1 {o n op} { lappend ::log A1 }\n\
         proc A2 {o n op} { lappend ::log A2\n\
         \x20   trace remove command a delete A1 }\n\
         proc a {} {}\n\
         trace add command a delete A1\n\
         trace add command a delete A2\n\
         rename a {}\n\
         proc B1 {o n op} { lappend ::log B1 }\n\
         proc B2 {o n op} { lappend ::log B2\n\
         \x20   proc b {} {}\n\
         \x20   trace remove command b delete B1 }\n\
         proc b {} {}\n\
         trace add command b delete B1\n\
         trace add command b delete B2\n\
         rename b {}\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "NEW ::foo {} delete\n\
         OLD ::foo {} delete\n\
         call:FOO2\n\
         traces:\n\
         A2\n\
         B2\n\
         B1"
    );
}

/// The counterpart to the walk above, and the reason the two loops ask
/// different questions. `TclCheckExecutionTraces` follows the list of whatever
/// command the name holds *now*, so a callback that redefines the traced
/// command stops the rest of that walk — `E1` and `L1` never run. The delete
/// walk keeps firing the dying token's list instead. Measured on tclsh 8.6.16
/// and 9.0.4.
#[test]
fn an_execution_callback_redefining_the_command_stops_that_walk() {
    let got = transcript(
        "set ::log {}\n\
         proc t {} { return T }\n\
         proc E1 args { lappend ::log E1 }\n\
         proc E2 args { lappend ::log E2\n\
         \x20   proc t {} { return T2 } }\n\
         trace add execution t enter E1\n\
         trace add execution t enter E2\n\
         lappend ::log \"call:[t]\" \"traces:[trace info execution t]\"\n\
         proc u {} { return U }\n\
         proc L1 args { lappend ::log L1 }\n\
         proc L2 args { lappend ::log L2\n\
         \x20   proc u {} { return U2 } }\n\
         trace add execution u leave L2\n\
         trace add execution u leave L1\n\
         lappend ::log \"call:[u]\"\n\
         join $::log \\n",
    );
    assert_eq!(got, "E2\ncall:T2\ntraces:\nL2\ncall:U");
}

/// A command-delete trace whose callback re-creates the command leaves the
/// *new* command standing: C deletes the token it captured before the callback
/// (`CMD_DYING`, its hash entry taken over by the new command), not whatever
/// the name holds afterwards. The old command's traces still go.
///
/// tclsh 8.6.16 and 9.0.4 both print the transcript below; the runtime used to
/// delete the callback's fresh `foo`, leaving `unknown command "foo"`.
#[test]
fn a_delete_trace_that_recreates_the_command_leaves_it_alive() {
    let got = transcript(
        "set ::log {}\n\
         proc foo {} { return FOO }\n\
         proc D {old new op} {\n\
         \x20   lappend ::log [list D $old $new $op]\n\
         \x20   proc foo {} { return FOO2 }\n\
         }\n\
         trace add command foo delete D\n\
         rename foo {}\n\
         lappend ::log \"exists: [llength [info commands foo]]\"\n\
         lappend ::log \"call: [foo]\"\n\
         lappend ::log \"traces: [trace info command foo]\"\n\
         join $::log \\n",
    );
    assert_eq!(got, "D ::foo {} delete\nexists: 1\ncall: FOO2\ntraces: ");
}

// -- #1574: re-entrancy suppression is per `Var` cell, not per array --------

/// C sets `VAR_TRACE_ACTIVE` on the `Var` an access reached, and an array
/// element is a `Var` of its own. So a whole-array write trace whose callback
/// writes a *different* element fires again — and one that writes the *same*
/// element does not.
///
/// tclsh 8.6.16 and 9.0.4 print the transcript below. Both engines used to
/// suppress per whole array and stop after the first firing in each pair.
// The sheet drives the traces with `if`, which only the tower build
// registers (no `expr`, no condition to evaluate).
#[cfg(have_tommath)]
#[test]
fn re_entrancy_is_suppressed_per_cell_not_per_array() {
    let got = transcript(
        "set ::log {}\n\
         proc W {n1 n2 op} { lappend ::log [list W $n1 $n2 $op]\n\
         \x20   if {$n2 ne \"other\"} { set ::c(other) 1 } }\n\
         trace add variable c write W\n\
         set c(k) 1\n\
         lappend ::log {= elem->elem}\n\
         proc V {n1 n2 op} { lappend ::log [list V $n1 $n2 $op]\n\
         \x20   if {$n2 ne \"j\"} { set ::d(j) 1 } }\n\
         trace add variable d(k) write V\n\
         trace add variable d(j) write V\n\
         set d(k) 1\n\
         lappend ::log {= same elem again}\n\
         proc U {n1 n2 op} { lappend ::log [list U $n1 $n2 $op]; set ::e(k) 9 }\n\
         trace add variable e(k) write U\n\
         set e(k) 1\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "W c k write\nW ::c other write\n\
         = elem->elem\nV d k write\nV ::d j write\n\
         = same elem again\nU e k write"
    );
}

/// The other half of C's rule: the whole-array traces have their own
/// `VAR_TRACE_ACTIVE` gate on `arrayPtr`. While the *array's own* cell is
/// firing (a whole-array `unset`), a callback that writes an element does not
/// re-enter them — one firing, and the array the callback re-created survives.
#[test]
fn the_arrays_own_cell_gates_the_whole_array_traces() {
    let got = transcript(
        "set ::log {}\n\
         proc S {n1 n2 op} { lappend ::log [list S $n1 $n2 $op]; catch {set ::g(z) 1} }\n\
         array set g {q 1}\n\
         trace add variable g write S\n\
         trace add variable g unset S\n\
         unset g\n\
         lappend ::log \"exists: [array exists ::g]\"\n\
         join $::log \\n",
    );
    assert_eq!(got, "S g {} unset\nexists: 1");
}

// -- #1575: the unset-trace firing sites that were missing ------------------

/// A proc's locals are unset when its frame goes, and C's `TclDeleteVars` fires
/// each one's unset traces — newest-first within a variable. runtime/rust fired
/// nothing at all.
///
/// tclsh 8.6.16 and 9.0.4 print the transcript below. *Which* variable comes
/// first is C's local-slot/hash walk and is not a pinned property; that a
/// variable's own callbacks are contiguous and newest-first is.
#[test]
fn proc_frame_teardown_fires_its_locals_unset_traces() {
    let got = transcript(
        "set ::log {}\n\
         proc R1 {n1 n2 op} { lappend ::log [list R1 $n1 $n2 $op] }\n\
         proc R2 {n1 n2 op} { lappend ::log [list R2 $n1 $n2 $op] }\n\
         proc P {} {\n\
         \x20   set a 1\n\
         \x20   set b 2\n\
         \x20   trace add variable a unset R1\n\
         \x20   trace add variable b unset R1\n\
         \x20   trace add variable a unset R2\n\
         }\n\
         P\n\
         join $::log \\n",
    );
    assert_eq!(got, "R2 a {} unset\nR1 a {} unset\nR1 b {} unset");
}

/// The array halves of the same site: a local *array* fires its own whole-array
/// trace and then each element's, and a trace another frame registered through
/// an `upvar` alias belongs to the owning frame and fires when *that* frame
/// goes — newest-first alongside the owner's own.
#[test]
fn proc_frame_teardown_covers_array_elements_and_alias_registrations() {
    let got = transcript(
        "set ::log {}\n\
         proc R1 {n1 n2 op} { lappend ::log [list R1 $n1 $n2 $op] }\n\
         proc R2 {n1 n2 op} { lappend ::log [list R2 $n1 $n2 $op] }\n\
         proc P {} {\n\
         \x20   array set la {m 1 n 2}\n\
         \x20   trace add variable la unset R1\n\
         \x20   trace add variable la(m) unset R2\n\
         \x20   set s 0\n\
         \x20   trace add variable s unset R1\n\
         }\n\
         P\n\
         lappend ::log {= alias registration}\n\
         proc Q {} { set q 1 ; trace add variable q unset R1 ; R }\n\
         proc R {} { upvar 1 q alias ; trace add variable alias unset R2 }\n\
         Q\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "R1 la {} unset\nR2 la m unset\nR1 s {} unset\n\
         = alias registration\nR2 q {} unset\nR1 q {} unset"
    );
}

/// Unsetting a whole array destroys each element cell, and C's `DeleteArray`
/// fires each element's own traces — `arrayPtr` NULL, so the array's traces do
/// *not* run again — after the array's own firing, reporting
/// `name1 = <array>` and `name2 = <element>`. Both engines stopped after the
/// whole-array firing.
#[test]
fn a_whole_array_unset_fires_each_elements_own_traces_too() {
    let got = transcript(
        "set ::log {}\n\
         proc R1 {n1 n2 op} { lappend ::log [list R1 $n1 $n2 $op] }\n\
         proc R2 {n1 n2 op} { lappend ::log [list R2 $n1 $n2 $op] }\n\
         array set arr {j 1 k 2}\n\
         trace add variable arr unset R1\n\
         trace add variable arr(j) unset R1\n\
         trace add variable arr(k) unset R2\n\
         unset arr\n\
         lappend ::log {= elements only}\n\
         array set b2 {p 1 q 2}\n\
         trace add variable b2(p) unset R1\n\
         trace add variable b2(q) unset R2\n\
         unset b2\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "R1 arr {} unset\nR1 arr j unset\nR2 arr k unset\n\
         = elements only\nR1 b2 p unset\nR2 b2 q unset"
    );
}

// -- #1569: `array` traces, which neither engine ever dispatched ------------

/// C's `LocateArray` fires `TclCheckArrayTraces` at the top of every `array`
/// subcommand, so each one invokes the callback exactly once as
/// `<name> {} array` — while an ordinary element read or write fires nothing.
/// The gate is `TclIsVarArray(varPtr) || TclIsVarUndefined(varPtr)`: an
/// undefined variable fires, a scalar never does, and an unknown subcommand
/// errors at the index lookup before `LocateArray` runs.
///
/// tclsh 8.6.16 and 9.0.4 print the transcript below (restricted to the
/// subcommands this build has — 8.6 has no `for`/`default`, and the runtime has
/// no search subcommands yet).
#[test]
fn every_array_subcommand_fires_the_array_trace_once() {
    let got = transcript(
        "set ::log {}\n\
         proc A {n1 n2 op} { lappend ::log [list A $n1 $n2 $op] }\n\
         array set arr {k 1}\n\
         trace add variable arr array A\n\
         foreach sub {{names arr} {size arr} {get arr} {exists arr}} {\n\
         \x20   lappend ::log \"= array $sub\"\n\
         \x20   catch {array {*}$sub}\n\
         }\n\
         lappend ::log {= array set}\n\
         array set arr {j 2}\n\
         lappend ::log {= array unset}\n\
         array unset arr k\n\
         lappend ::log {= plain read/write}\n\
         set arr(j) 5\n\
         set q $arr(j)\n\
         lappend ::log {= undefined var}\n\
         trace add variable novar array A\n\
         array names novar\n\
         array exists novar\n\
         lappend ::log {= scalar}\n\
         set sc 1\n\
         trace add variable sc array A\n\
         catch {array names sc}\n\
         lappend ::log {= unknown subcommand}\n\
         catch {array bogus arr}\n\
         lappend ::log {= via upvar alias}\n\
         upvar #0 arr al\n\
         array names al\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "= array names arr\nA arr {} array\n\
         = array size arr\nA arr {} array\n\
         = array get arr\nA arr {} array\n\
         = array exists arr\nA arr {} array\n\
         = array set\nA arr {} array\n\
         = array unset\nA arr {} array\n\
         = plain read/write\n\
         = undefined var\nA novar {} array\nA novar {} array\n\
         = scalar\n\
         = unknown subcommand\n\
         = via upvar alias\nA al {} array"
    );
}

/// An `array` trace whose callback errors fails the subcommand:
/// `LocateArray` passes `leaveErrMsg` 1, so `TclCallVarTraces`'s error tail
/// runs with the `array` verb — `can't trace array "arr": …`, an
/// `(array trace on "arr")` errorInfo frame, and the callback's own
/// `-errorcode` (there is no `Tcl_SetErrorCode` on this path, unlike the read
/// and write ones). Identical at 8.6.16 and 9.0.4.
#[test]
fn an_array_trace_error_fails_the_subcommand_with_cs_verb() {
    let got = transcript(
        "proc AE {n1 n2 op} { error \"aboom\" }\n\
         array set arr {k 1}\n\
         trace add variable arr array AE\n\
         set c [catch {array names arr} m]\n\
         set out \"$c|$m|$::errorCode|$::errorInfo\"",
    );
    assert_eq!(
        got,
        "1|can't trace array \"arr\": aboom|NONE|\
         aboom\n    while executing\n\"error \"aboom\" \"\n\
         \x20   (procedure \"AE\" line 1)\n    invoked from within\n\"AE arr {} array\"\n\
         \x20   (array trace on \"arr\")\n    invoked from within\n\"array names arr\""
    );
}

/// #1633 row 1: `set`/`incr` must return the variable's value *read back
/// after* their own write trace runs, not the value they handed the store —
/// C's `TclPtrSetVarIdx` (tclVar.c 9.0.4:2050-2065) stores, fires the write
/// traces, and only then decides what to return: the cell's current value if
/// a trace rewrote it (still a defined scalar), or the empty string if a
/// trace changed the variable "in some gross way" (here, unset it). `set` and
/// `incr` share this store tail, so both are pinned; identical at 8.6.16 and
/// 9.0.4.
#[test]
fn a_write_trace_that_mutates_or_unsets_changes_what_set_and_incr_return() {
    for version in [None, Some(tcl_dialect::TclVersion::V8_6)] {
        let got = transcript_at(
            "set ::log {}\n\
             proc mangle {n1 n2 op} { set ::x mangled }\n\
             trace add variable x write mangle\n\
             lappend ::log [set x orig]\n\
             proc vanish {n1 n2 op} { unset ::y }\n\
             trace add variable y write vanish\n\
             lappend ::log \"[set y orig]|\"\n\
             proc mangle2 {n1 n2 op} { set ::z mangled }\n\
             trace add variable z write mangle2\n\
             lappend ::log [incr z]\n\
             set w 5\n\
             proc vanish2 {n1 n2 op} { unset ::w }\n\
             trace add variable w write vanish2\n\
             lappend ::log \"[incr w 3]|\"\n\
             join $::log \\n",
            version,
        );
        assert_eq!(got, "mangled\n|\nmangled\n|", "{version:?}");
    }
}

// -- The `rename` trace window: one command under two names ------------------
//
// C's `TclRenameCommand` (`tclBasic.c` 9.0.4) creates the destination hash
// entry, fires the `rename` traces, and only *then* deletes the source one —
// and the traces hang off the shared `Command` rather than off either entry.
// So for the callbacks' duration the vacating name **is** the destination
// command. The runtime used to fire before touching the table, so a callback
// saw the old name but not yet the new one. Every sheet below is identical on
// tclsh 8.6.16 and 9.0.4.

/// Both names resolve, both are callable, and `trace info command` /
/// `trace info execution` answer the same list through either of them.
#[test]
fn a_rename_callback_sees_the_command_under_both_names() {
    let got = transcript(
        "set ::log {}\n\
         proc victim {} { return V }\n\
         proc E args {}\n\
         proc R {old new op} {\n\
         \x20   lappend ::log [list R $old $new $op]\n\
         \x20   lappend ::log \"live: [llength [info commands ::victim]] \
         [llength [info commands ::victim2]]\"\n\
         \x20   lappend ::log \"call: [victim] [victim2]\"\n\
         \x20   lappend ::log \"cmd: [trace info command victim] | \
         [trace info command victim2]\"\n\
         \x20   lappend ::log \"exec: [trace info execution victim] | \
         [trace info execution victim2]\"\n\
         }\n\
         trace add command victim rename R\n\
         trace add execution victim enter E\n\
         rename victim victim2\n\
         lappend ::log \"after: [llength [info commands ::victim]] \
         [llength [info commands ::victim2]]\"\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "R ::victim ::victim2 rename\n\
         live: 1 1\n\
         call: V V\n\
         cmd: {rename R} | {rename R}\n\
         exec: {enter E} | {enter E}\n\
         after: 0 1"
    );
}

/// There is one trace list, so a callback's `trace remove` through the
/// vacating name edits the list the rest of the pass — and the surviving
/// command — is reading.
#[test]
fn a_rename_callback_edits_one_trace_list_through_either_name() {
    let got = transcript(
        "set ::log {}\n\
         proc victim {} {}\n\
         proc C1 {old new op} {\n\
         \x20   trace remove command victim rename C1\n\
         \x20   lappend ::log \"C1: [trace info command victim]\"\n\
         }\n\
         proc C2 {old new op} { lappend ::log \"C2: [trace info command victim2]\" }\n\
         trace add command victim rename C2\n\
         trace add command victim rename C1\n\
         rename victim victim2\n\
         lappend ::log \"after: [trace info command victim2]\"\n\
         join $::log \\n",
    );
    assert_eq!(got, "C1: {rename C2}\nC2: {rename C2}\nafter: {rename C2}");
}

/// The other direction: a `trace add` through the vacating name lands on the
/// one command, so it is still there under the new name afterwards.
#[test]
fn a_trace_added_through_the_vacating_name_follows_the_command() {
    let got = transcript(
        "set ::log {}\n\
         proc victim {} {}\n\
         proc D {old new op} { lappend ::log [list D $old $new $op] }\n\
         proc R {old new op} { trace add command victim delete D }\n\
         trace add command victim rename R\n\
         rename victim victim2\n\
         lappend ::log \"info: [trace info command victim2]\"\n\
         rename victim2 {}\n\
         join $::log \\n",
    );
    assert_eq!(got, "info: {delete D} {rename R}\nD ::victim2 {} delete");
}

/// A callback that renames *through* the vacating name moves that one command
/// on again — it does not leave a second copy behind — and C's
/// `CMD_TRACE_ACTIVE` keeps the pass's remaining callbacks from re-firing when
/// it does (`C2` runs once, with no nested `C1`/`C2` pass).
#[test]
fn a_rename_callback_renaming_the_vacating_name_moves_the_one_command() {
    let got = transcript(
        "set ::log {}\n\
         proc victim {} { return V }\n\
         proc C1 {old new op} { lappend ::log C1; rename victim victim3 }\n\
         proc C2 {old new op} { lappend ::log C2 }\n\
         trace add command victim rename C2\n\
         trace add command victim rename C1\n\
         rename victim victim2\n\
         lappend ::log \"live: [llength [info commands ::victim]] \
         [llength [info commands ::victim2]] [llength [info commands ::victim3]]\"\n\
         lappend ::log \"info: [trace info command victim3] | [victim3]\"\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "C1\nC2\nlive: 0 0 1\ninfo: {rename C1} {rename C2} | V"
    );
}

/// And a callback that *deletes* through the vacating name destroys the one
/// command: neither name is left standing.
#[test]
fn a_rename_callback_deleting_the_vacating_name_destroys_the_one_command() {
    let got = transcript(
        "set ::log {}\n\
         proc victim {} {}\n\
         proc R {old new op} { rename victim {} }\n\
         trace add command victim rename R\n\
         rename victim victim2\n\
         lappend ::log \"live: [llength [info commands ::victim]] \
         [llength [info commands ::victim2]]\"\n\
         join $::log \\n",
    );
    assert_eq!(got, "live: 0 0");
}

/// A nested rename must retarget the enclosing window: after the callback
/// moves the command to a third name, the outermost vacating name still
/// reaches it, and a `trace add` through that name lands on the survivor.
#[test]
fn a_nested_rename_keeps_the_vacating_name_on_the_command() {
    let got = transcript(
        "set ::log {}\n\
         proc victim {} {}\n\
         proc D {old new op} { lappend ::log [list D $old $new $op] }\n\
         proc R {old new op} {\n\
         \x20   rename victim2 victim3\n\
         \x20   lappend ::log \"in: [trace info command victim] | \
         [trace info command victim3]\"\n\
         \x20   trace add command victim delete D\n\
         }\n\
         trace add command victim rename R\n\
         rename victim victim2\n\
         lappend ::log \"after: [trace info command victim3]\"\n\
         rename victim3 {}\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "in: {rename R} | {rename R}\n\
         after: {delete D} {rename R}\n\
         D ::victim3 {} delete"
    );
}

/// C re-homes the one `Command` (`cmdPtr->nsPtr = newNsPtr`) before it fires,
/// so a body invoked through the vacating name already reports the
/// *destination's* namespace.
#[test]
fn the_vacating_name_reports_the_destinations_namespace() {
    let got = transcript(
        "set ::log {}\n\
         namespace eval a { proc p {} { return [namespace current] } }\n\
         namespace eval b {}\n\
         proc R {old new op} {\n\
         \x20   lappend ::log [list R $old $new $op]\n\
         \x20   lappend ::log \"old: [a::p]\"\n\
         \x20   lappend ::log \"new: [b::q]\"\n\
         }\n\
         trace add command a::p rename R\n\
         rename a::p ::b::q\n\
         lappend ::log \"after: [b::q]\"\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "R ::a::p ::b::q rename\nold: ::b\nnew: ::b\nafter: ::b"
    );
}

/// The window covers the redirect kinds that carry their own binding identity
/// too: a TclOO object embeds the FQN its registry entry lives under, an
/// ensemble token its name, and an import its source. C re-homes the one
/// `Command` and lets both hash entries reach it, so each is callable under
/// the vacating name *and* the destination for the callbacks' duration.
#[test]
fn the_rename_window_covers_objects_ensembles_and_imports() {
    let got = transcript(
        "set ::log {}\n\
         oo::class create C { method m {} { return M } }\n\
         C create obj\n\
         namespace eval e { proc sub {} { return S }; namespace export sub; \
         namespace ensemble create }\n\
         namespace eval src { proc f {} { return F }; namespace export f }\n\
         namespace eval dst { namespace import ::src::f }\n\
         proc R {old new op} {\n\
         \x20   lappend ::log [list R $old $new $op]\n\
         \x20   lappend ::log \"oo: [obj m] [obj2 m]\"\n\
         }\n\
         proc RE {old new op} { lappend ::log \"ens: [e sub] [e2 sub]\" }\n\
         proc RI {old new op} { lappend ::log \"imp: [dst::f] [src::f] [src::g]\" }\n\
         trace add command obj rename R\n\
         trace add command e rename RE\n\
         trace add command src::f rename RI\n\
         rename obj obj2\n\
         rename e e2\n\
         rename ::src::f ::src::g\n\
         lappend ::log \"after: [obj2 m] [e2 sub] [dst::f]\"\n\
         join $::log \\n",
    );
    assert_eq!(
        got,
        "R ::obj ::obj2 rename\n\
         oo: M M\n\
         ens: S S\n\
         imp: F F F\n\
         after: M S F"
    );
}
