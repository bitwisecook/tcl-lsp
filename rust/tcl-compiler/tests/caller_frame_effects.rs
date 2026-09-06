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

//! Caller-frame injection — issue #923 differential-audit cluster C1
//! (idx 7, 22, 38, 57, 59, 98) plus the `eval` / `uplevel` FP class #1076
//! left documented as out of scope.
//!
//! Every case below is a whole-file diagnostic run, so it exercises the real
//! chain: the per-proc [frame-effect
//! summary](tcl_compiler::cfg_builder::upvar_info) → the CFG builder's
//! call-site widening and caller-frame barrier → `W210` / `W211` / `W220`.
//!
//! # C Tcl ground truth
//!
//! Every claim about which frame a write lands in is pinned on **tclsh
//! 9.0.4** and **tclsh 8.6.14**, which agree on all of it.  The transcripts
//! are quoted at each test.  Two facts do most of the work:
//!
//! ```text
//! proc worker  {nvar} {upvar 1 $nvar v; set v WORKED}
//! proc wrapper {nvar} {uplevel 1 [list worker $nvar]}   ;# forwards one frame
//! proc wrap2   {nvar} {worker $nvar}                    ;# does NOT forward
//! proc host  {} {wrapper target;  set target}  ;# → WORKED
//! proc host2 {} {wrap2   target2; set target2} ;# → can't read "target2"
//! ```
//!
//! # Soundness direction
//!
//! Every fix here abstains **toward silence**: where the caller-frame effect
//! cannot be named, the analysis says nothing rather than guessing.  Each
//! false-positive test is therefore paired with a true-positive control that
//! must keep firing, so the suppression cannot be over-applied.

use std::collections::HashSet;

use tcl_compiler::analyser::Analyser;

const D: &str = "tcl8.6";

/// Variable names reported by `code` for `src`.
fn vars_for(src: &str, code: &str) -> HashSet<String> {
    Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == code)
        .filter_map(|d| d.message.split('\'').nth(1).map(str::to_owned))
        .collect()
}

fn w210(src: &str) -> HashSet<String> {
    vars_for(src, "W210")
}

fn w211(src: &str) -> HashSet<String> {
    vars_for(src, "W211")
}

// idx 57 / 59 — a proc that upvar-writes a by-name out-parameter.

/// The ticklecharts `setdef` / `estruct` shape: `upvar 1 $name local` plus a
/// write through the alias, called with a literal name.
const OUT_PARAM_HELPER: &str = "\
namespace eval demo {
    proc estruct {name value} {
        upvar 1 $name obj
        set obj [list struct $name $value]
    }
}
";

#[test]
fn idx57_out_param_write_is_seen_through_every_call_spelling() {
    // tclsh 9.0.4 / 8.6.14: `demo::estruct itemLegend1 {a b}` leaves
    // `itemLegend1` set in the caller — reading it is not read-before-set.
    // The bare and fully-absolute spellings already worked; the ordinary
    // relative-qualified one (`demo::estruct`) silently missed, because the
    // summary was keyed only under `estruct` and `::demo::estruct`
    // (issue #923 audit idx 59's isolation ladder p1/p2/p3).
    for call in [
        "estruct itemLegend1 {a b}",
        "demo::estruct itemLegend1 {a b}",
        "::demo::estruct itemLegend1 {a b}",
    ] {
        let src = format!(
            "{OUT_PARAM_HELPER}\nnamespace eval demo {{\n proc user {{}} {{\n {call}\n puts $itemLegend1\n }}\n}}\n"
        );
        assert!(
            !w210(&src).contains("itemLegend1"),
            "`{call}` writes the caller's itemLegend1; got {:?}",
            w210(&src),
        );
    }
}

#[test]
fn idx57_control_an_unrelated_local_still_reports() {
    // TP control — the suppression is per-name, not a blanket silence: a
    // local the helper never touches is still read-before-set (tclsh 9.0.4 /
    // 8.6.14 raise `can't read "neverSet"`).
    let src = format!(
        "{OUT_PARAM_HELPER}\nproc user {{}} {{\n estruct itemLegend1 {{a b}}\n puts $neverSet\n}}\n"
    );
    assert!(w210(&src).contains("neverSet"), "got {:?}", w210(&src));
}

#[test]
fn idx57_control_a_proc_without_an_upvar_does_not_suppress() {
    // TN control — a same-shaped helper that does *not* upvar writes nothing
    // in the caller, so the read stays a genuine finding (tclsh 9.0.4 /
    // 8.6.14: `can't read "itemLegend1"`).
    let src = "\
proc estruct {name value} { set obj [list struct $name $value] }
proc user {} {
    estruct itemLegend1 {a b}
    puts $itemLegend1
}
";
    assert!(w210(src).contains("itemLegend1"), "got {:?}", w210(src));
}

// idx 59 — the same out-parameter helper, defined in ANOTHER FILE.

/// The ticklecharts layout the finding was actually mined from: `setdef`
/// lives in `utils.tcl`, the caller in `options.tcl`, and only a
/// `pkgIndex.tcl` ties them together — so the caller's compilation unit
/// holds no definition of `setdef` at all.  Reproduced here by simply
/// omitting the helper, which is exactly what the caller's unit sees.
///
/// tclsh 9.0.4 / 8.6.16, running the real four-file layout:
/// `demo::build {a b c}` → `{nothing str no} {nothing str no} {nothing str
/// no}` — `options` is populated by the calls and never assigned in
/// `build`'s own text before it is read.
const CROSS_FILE_CALLER: &str = "\
namespace eval demo {}
proc demo::build {items} {
    set opts {}
    foreach item $items {
        setdef options name -type str -default \"nothing\"
        lappend opts [dict get $options name]
        set options {}
    }
    return $opts
}
";

#[test]
fn idx59_a_callee_this_unit_cannot_see_may_write_the_names_it_is_handed() {
    assert!(
        !w210(CROSS_FILE_CALLER).contains("options"),
        "a cross-file helper may create `options` through `upvar`; got {:?}",
        w210(CROSS_FILE_CALLER),
    );
}

#[test]
fn idx59_control_a_name_the_opaque_call_never_spells_still_reports() {
    // TP control — the abstention is per-name, not a blanket silence for the
    // whole frame.  tclsh 9.0.4 / 8.6.16: `can't read "neverPassed"`.
    let src = "\
proc build {} {
    setdef options name -default nothing
    puts $neverPassed
}
";
    assert!(w210(src).contains("neverPassed"), "got {:?}", w210(src));
}

#[test]
fn idx59_control_a_substituted_argument_names_nothing_resolvable() {
    // TP control — only a *literal* word can name a caller-frame variable
    // this analysis can identify.  `setdef $key …` says nothing about
    // `options`, so the read still reports (tclsh: `can't read "options"`).
    let src = "\
proc build {key} {
    setdef $key name -default nothing
    puts $options
}
";
    assert!(w210(src).contains("options"), "got {:?}", w210(src));
}

#[test]
fn idx59_control_a_registry_command_is_not_an_opaque_callee() {
    // TN control — `puts` has a declared frame effect (none), so handing it
    // a bareword proves nothing about a same-named local.  tclsh 9.0.4 /
    // 8.6.16: `can't read "options"`.
    let src = "proc build {} {\n puts options\n puts $options\n}\n";
    assert!(w210(src).contains("options"), "got {:?}", w210(src));
}

// idx 38 — `uplevel <caller> [list set …]`, the tclopt `NewArrays` shape.

#[test]
fn idx38_uplevel_constructed_set_defines_the_callers_variable() {
    // tclsh 9.0.4 / 8.6.14: `NewArrays {p q}` leaves `p` and `q` set in the
    // caller; removing the call makes the read error, so the call is what
    // materialises them.
    let src = "\
proc NewArrays {varNames lengths} {
    foreach varName $varNames length $lengths {
        uplevel 1 [list set $varName [list array $length]]
    }
}
proc Qfrac {n} {
    NewArrays {rdiagArray acnormArray} [list $n $n]
    return [list $rdiagArray $acnormArray]
}
";
    assert!(w210(src).is_empty(), "got {:?}", w210(src));
}

#[test]
fn idx38_absolute_constructed_set_resolves_to_the_exact_name() {
    // An absolute command and literal target make the constructed caller-frame
    // body exact. A substituted constructor operand remains intentionally
    // opaque because its runtime Tcl value is not available to this summary.
    let src = "\
proc MakeOne {} { uplevel 1 [list ::set wanted made] }
proc host {} {
    MakeOne
    puts $wanted
    puts $notWritten
}
";
    let found = w210(src);
    assert!(!found.contains("wanted"), "got {found:?}");
    // TP control: precision, not silence — an unrelated read still reports.
    assert!(found.contains("notWritten"), "got {found:?}");
}

#[test]
fn idx38_literal_uplevel_body_defines_the_callers_variable() {
    // `uplevel 1 {set litVar hello}` — the brace-literal body the lowering
    // inlines. tclsh 9.0.4 / 8.6.14 set the caller's `litVar`.
    let src = "\
proc lit {} { uplevel 1 {set litVar hello} }
proc host {} { lit; puts $litVar }
";
    assert!(!w210(src).contains("litVar"), "got {:?}", w210(src));
}

// idx 7 — `argparse`, which injects caller-frame locals from its own DSL.

#[test]
fn idx7_argparse_injects_caller_frame_locals() {
    // tclsh 9.0.4 / 8.6.14 (argparse 0.5): `upvarProc p 1 2` prints
    // `a=unset-sentinel b=1 c=2` — `a` is a `-upvar` alias of the caller's
    // `p`, `b`/`c` are ordinary argparse-set locals. None of the three is
    // assigned anywhere in `upvarProc`'s own text.
    let src = "\
package require argparse
proc upvarProc {args} {
    argparse {
        {a -upvar}
        b
        c
    }
    puts \"a=$a b=$b c=$c\"
}
";
    assert!(w210(src).is_empty(), "got {:?}", w210(src));
}

#[test]
fn idx7_control_a_proc_without_argparse_still_reports() {
    // TP control — `argparse`'s blindness is confined to the frame it is
    // called in.
    let src = "\
package require argparse
proc plain {args} { puts \"a=$a\" }
";
    assert!(w210(src).contains("a"), "got {:?}", w210(src));
}

// The `eval` / `uplevel <dynamic body>` FP class (#1076 "not covered").

#[test]
fn eval_of_an_unreadable_script_blinds_its_own_frame() {
    // tclsh 9.0.4 / 8.6.14: `eval $b` runs in the frame it is written in, so
    // `f {set injected 1}` really does create `injected` there.
    let src = "proc f {b} { eval $b; puts $injected }\n";
    assert!(w210(src).is_empty(), "got {:?}", w210(src));
}

#[test]
fn uplevel_of_an_unreadable_script_blinds_the_callers_frame_not_its_own() {
    // The frame distinction, both directions at once.
    //
    // tclsh 9.0.4 / 8.6.14: `proc runner {body} {set helper 42; uplevel 1
    // $body}` invoked with `{set x $helper}` raises `can't read "helper"` —
    // the script runs one frame *up*, so `runner`'s own `helper` is
    // genuinely unreachable and "set but never used" is a true positive.
    // The caller, though, may well have been written by that script.
    let src = "\
proc runner {body} { set helper 42; uplevel 1 $body }
proc host {s} { runner $s; puts $injected }
";
    assert!(
        w211(src).contains("helper"),
        "the callee's own local is genuinely unread; got {:?}",
        w211(src),
    );
    assert!(
        !w210(src).contains("injected"),
        "the caller's frame is what the script writes; got {:?}",
        w210(src),
    );
}

#[test]
fn uplevel_at_the_current_frame_blinds_the_frame_it_is_written_in() {
    // `uplevel 0 $b` re-enters the current frame — tclsh 9.0.4 / 8.6.14
    // treat it as `eval` for variable purposes.
    let src = "proc f {b} { uplevel 0 $b; puts $injected }\n";
    assert!(w210(src).is_empty(), "got {:?}", w210(src));
}

#[test]
fn control_a_literal_eval_body_is_not_a_barrier() {
    // TN control — a brace-literal script is source text the ordinary
    // walkers read, so it must not blind anything.
    let src = "proc f {} { eval {puts hi}; puts $neverSet }\n";
    assert!(w210(src).contains("neverSet"), "got {:?}", w210(src));
}

// issue #1019 — `uplevel <caller> [list callee …]` forwards one frame.

#[test]
fn idx1019_uplevel_constructed_call_forwards_the_callees_upvar_one_frame() {
    // tclsh 9.0.4 / 8.6.14: `wrapper target` leaves `target` set in
    // *wrapper's* caller, because `worker` runs in that frame and its own
    // `upvar 1` therefore reaches one level further out.
    let src = "\
proc worker {nvar} { upvar 1 $nvar v; set v WORKED }
proc wrapper {nvar} { uplevel 1 [list worker $nvar] }
proc host {} { wrapper target; puts $target }
";
    assert!(!w210(src).contains("target"), "got {:?}", w210(src));
}

#[test]
fn idx1019_control_a_plain_call_wrapper_does_not_forward() {
    // TP control — the same wrapper written as a *plain* call shares values,
    // not frames: `worker`'s `upvar 1` reaches the wrapper, not the
    // wrapper's caller. tclsh 9.0.4 / 8.6.14 raise `can't read "target2"`.
    let src = "\
proc worker {nvar} { upvar 1 $nvar v; set v WORKED }
proc wrapper2 {nvar} { worker $nvar }
proc host2 {} { wrapper2 target2; puts $target2 }
";
    assert!(w210(src).contains("target2"), "got {:?}", w210(src));
}

// Frame targeting — the levels that are *not* the caller.

#[test]
fn upvar_hash_zero_raises_no_caller_frame_barrier() {
    // TP control for the frame table: `upvar #0 counter c` binds the
    // *global* `counter` (tclsh 9.0.4 / 8.6.14, from any call depth), which
    // is `global_write_info`'s business, not a caller-frame effect — so it
    // must not blind the calling function. An unrelated local there is still
    // read-before-set (tclsh raises `can't read "unrelated"`).
    //
    // The complementary half — that `counter` is not recorded as a
    // *caller-frame* binding — is pinned on the summary itself by
    // `cfg_builder::upvar_info::tests::level_hash_zero_is_not_a_caller_frame_binding`.
    let src = "\
proc setsGlobal {} { upvar #0 counter c; incr c }
proc host {} { setsGlobal; puts $unrelated }
";
    assert!(w210(src).contains("unrelated"), "got {:?}", w210(src));
}

#[test]
fn upvar_beyond_the_caller_widens_instead_of_naming() {
    // `upvar 2 far f` writes the caller's caller (tclsh 9.0.4 / 8.6.14:
    // `proc gp {} {upvar 2 far f; set f FAR}` through one intermediate frame
    // sets the grandparent's `far`). Naming `far` as a *direct* caller def
    // would be wrong, so the summary widens — and the direct caller goes
    // quiet rather than reporting a read it cannot disprove.
    let src = "\
proc gp {} { upvar 2 far f; set f FAR }
proc par {} { gp; puts $anything }
";
    assert!(w210(src).is_empty(), "got {:?}", w210(src));
}

#[test]
fn upvar_with_a_computed_level_widens() {
    // `upvar $lvl a b` — three words, so `$lvl` is the level (C Tcl decides
    // on argument-count parity). The frame is unknown, so widen.
    let src = "\
proc reach {lvl} { upvar $lvl a b; set b 1 }
proc host {} { reach 1; puts $a }
";
    assert!(!w210(src).contains("a"), "got {:?}", w210(src));
}
