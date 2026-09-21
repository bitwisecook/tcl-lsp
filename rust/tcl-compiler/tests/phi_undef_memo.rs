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

//! Scaling and result pins for the W210 phi-from-undef trace
//! (`src/analyser/diagnostics/helpers.rs::phi_can_undef`).
//!
//! The trace answers "can this SSA version be undefined on some executable
//! path?" over the phi graph. It used to do that with a DFS per query whose
//! `seen` set was a *path* (grey) set — a version is removed again when its
//! frame closes, and no answer was kept — so it enumerated every simple path
//! through the graph: N sibling conditional writes to one variable followed by
//! a read cost ~2^N visits, and a well-written script was the slow case,
//! because only a genuinely undefined path short-circuits the search
//! (issue #2021). Real Quartus sources reach N ≈ 105. It is now one
//! reverse-reachability pass that answers every version at once.
//!
//! [`nested_if_stages_analyse_in_linear_time`] is the regression pin: 60
//! stages is ~2^60 visits for the old walk and must instead finish in well
//! under the wall-clock budget asserted here. The rest of the file pins that
//! no answer moved — the classic W210 phi shapes (undef on one path, defined
//! on all paths, a loop-header cycle, an `unset` kill, an `info exists` guard)
//! keep the verdicts they had before.
//!
//! ## C-Tcl ground truth vs analyser policy
//!
//! The premises ("does `if {$c} { set x 1 }` leave `x` unset when `$c` is
//! false?", "does `unset x` remove the binding?") are Tcl-observable and cited
//! inline with a `// tclsh:` note. *Which* code is emitted for a premise is
//! analyser policy and is asserted against the analyser surface directly.

use std::fmt::Write as _;

use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::model::ingress::static_context_for;

const D: &str = "tcl8.6";

/// Every diagnostic code the user-facing `tcl diag` path surfaces for `src`:
/// the analyser pass plus `run_all_checks`, with optimisation codes excluded
/// (the harness `tests/fp_depth.rs` uses).
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

/// True if `code` appears anywhere in the `tcl diag` surface for `src`.
fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

/// The issue #2021 shape: `n` sibling two-deep conditional writes to one
/// variable, after a dominating `set x 0` and before a read of it. Each stage
/// contributes a merge phi whose operands are the previous stage's phi and the
/// inner arm's phi, so the phi graph has 2^n simple paths from the read to the
/// initial definition while the answer ("`x` is always defined") needs each
/// version visited once. The conditions are opaque `lindex` results so nothing
/// folds away, and they are set first so they are not themselves W210 reads.
///
/// This is `repro/issue-2021/phi/gen.py nested_if <n>` as self-contained Rust.
fn nested_if_stages(n: usize) -> String {
    let mut src = String::new();
    for i in 0..n {
        let _ = writeln!(src, "set c{i} [lindex $argv {i}]");
        let _ = writeln!(src, "set d{i} [lindex $argv {i}]");
    }
    src.push_str("set x 0\n");
    for i in 0..n {
        let _ = writeln!(src, "if {{$c{i}}} {{\n    if {{$d{i}}} {{ set x {i} }}\n}}");
    }
    src.push_str("puts $x\n");
    src
}

/// 60 stages is ~2^60 phi-trace visits without the memo, and a handful with
/// it. The budget is deliberately generous (the memoised analysis is
/// milliseconds); anything that reintroduces path enumeration cannot finish
/// this side of the heat death of the universe, so the test is decisive
/// without being flaky on a loaded machine.
#[test]
fn nested_if_stages_analyse_in_linear_time() {
    let src = nested_if_stages(60);
    let started = std::time::Instant::now();
    let found = codes(&src, D);
    let elapsed = started.elapsed();
    // tclsh: `set x 0` dominates every conditional write, so `$x` always
    // reads a value — no read-before-set.
    assert!(
        !found.iter().any(|c| c == "W210"),
        "`x` is defined on every path; no W210 expected, got {found:?}"
    );
    assert!(
        elapsed.as_secs() < 10,
        "phi-from-undef trace must not enumerate paths: 60 stages took {elapsed:?}"
    );
}

/// The same shape scaled: each added stage doubles the unmemoised walk, so a
/// larger N cannot be materially slower once answers are reused. Compares 20
/// stages against 40 rather than pinning an absolute time.
#[test]
fn nested_if_stages_do_not_grow_exponentially() {
    let small = nested_if_stages(20);
    let large = nested_if_stages(40);
    let t = std::time::Instant::now();
    let _ = codes(&small, D);
    let small_time = t.elapsed();
    let t = std::time::Instant::now();
    let _ = codes(&large, D);
    let large_time = t.elapsed();
    // 2^20 → 2^40 is a millionfold; anything under a 100× factor (plus a floor
    // so a sub-millisecond baseline can't make the ratio meaningless) proves
    // the walk is no longer path-enumerating.
    let budget = small_time.max(std::time::Duration::from_millis(50)) * 100;
    assert!(
        large_time < budget,
        "doubling the stage count must not explode: 20 stages {small_time:?}, \
         40 stages {large_time:?}"
    );
}

/// The same stages inside a `foreach` body: the loop-carried phi makes the
/// graph cyclic, which the old walk answered with a path cut — and a cut made
/// every answer above it path-dependent, so this shape stayed exponential even
/// with per-answer memoisation. Reachability needs no cut, so the loop case is
/// linear too.
#[test]
fn nested_if_stages_in_a_loop_analyse_in_linear_time() {
    let mut src = String::new();
    src.push_str("set items [lindex $argv 0]\n");
    for i in 0..60 {
        let _ = writeln!(src, "set c{i} [lindex $argv {i}]");
        let _ = writeln!(src, "set d{i} [lindex $argv {i}]");
    }
    src.push_str("set x 0\nforeach it $items {\n");
    for i in 0..60 {
        let _ = writeln!(
            src,
            "    if {{$c{i}}} {{\n        if {{$d{i}}} {{ set x {i} }}\n    }}"
        );
    }
    src.push_str("}\nputs $x\n");
    let started = std::time::Instant::now();
    let found = codes(&src, D);
    let elapsed = started.elapsed();
    // tclsh: `set x 0` runs before the loop, so `$x` always reads a value.
    assert!(
        !found.iter().any(|c| c == "W210"),
        "`x` is defined before the loop; no W210 expected, got {found:?}"
    );
    assert!(
        elapsed.as_secs() < 10,
        "conditional writes inside a loop must not enumerate paths: took {elapsed:?}"
    );
}

/// A `switch` with empty arms multiplies the operand count per stage (each
/// empty arm is another predecessor edge carrying the previous version), the
/// 4^N variant of the same bug. Small N, so it pins the shape rather than the
/// clock.
#[test]
fn switch_empty_arm_stages_analyse_in_linear_time() {
    let mut src = String::new();
    for i in 0..30 {
        let _ = writeln!(src, "set v{i} [lindex $argv {i}]");
    }
    src.push_str("set x 0\n");
    for i in 0..30 {
        let _ = writeln!(
            src,
            "switch -- $v{i} {{\n    a {{ set x {i} }}\n    b0 {{ }}\n    b1 {{ }}\n    b2 {{ }}\n}}"
        );
    }
    src.push_str("puts $x\n");
    let started = std::time::Instant::now();
    let found = codes(&src, D);
    assert!(
        !found.iter().any(|c| c == "W210"),
        "`x` is defined on every path; no W210 expected, got {found:?}"
    );
    assert!(
        started.elapsed().as_secs() < 10,
        "empty-arm switch stages must not enumerate paths"
    );
}

// --- result pins: the classic W210 phi-from-undef shapes ---

/// One-armed conditional write, no dominating definition: the merge phi has an
/// undef (version-0) incoming, so the read is read-before-set.
/// tclsh (`$c` false): `can't read "x": no such variable`.
#[test]
fn conditional_write_then_read_fires_w210() {
    let src = "proc f {c} { if {$c} { set x 1 }\n puts $x }\n";
    assert!(
        fires(src, D, "W210"),
        "a one-armed conditional write can leave `x` unset; emitted {:?}",
        codes(src, D)
    );
}

/// Both arms write, so every path into the merge defines the variable and the
/// phi has no undef incoming. tclsh: always prints. The memo must not turn
/// this into a false positive.
#[test]
fn both_arms_write_then_read_silent() {
    let src = "proc f {c} { if {$c} { set x 1 } else { set x 2 }\n puts $x }\n";
    assert!(
        !fires(src, D, "W210"),
        "`x` is written on both arms; no W210; emitted {:?}",
        codes(src, D)
    );
}

/// A dominating definition ahead of any number of conditional writes — the
/// small case of [`nested_if_stages_analyse_in_linear_time`], asserted on the
/// diagnostics rather than the clock.
#[test]
fn dominating_definition_before_conditional_writes_silent() {
    let src = nested_if_stages(3);
    assert!(
        !fires(&src, D, "W210"),
        "`set x 0` dominates the merges; no W210; emitted {:?}",
        codes(&src, D)
    );
}

/// Loop-header cycle: the accumulator's header phi takes its own back-edge
/// version as an incoming. The old walk cut that back-edge and answered "not
/// undef" on it; reachability gives the same verdict, because going round the
/// loop reaches no origin the entry edge did not already offer. tclsh:
/// `set acc {}` runs before the loop, so the append and the read always see a
/// value.
#[test]
fn loop_header_phi_cycle_silent() {
    let src = "proc f {n} { set acc {}\n for {set i 0} {$i < $n} {incr i} { lappend acc $i }\n \
                puts $acc }\n";
    assert!(
        !fires(src, D, "W210"),
        "a loop-carried accumulator defined before the loop is not RBS; emitted {:?}",
        codes(src, D)
    );
}

/// A body-only definition read after the loop: the header phi's entry operand
/// is the undef origin, but the analyser assumes a may-run loop runs (matching
/// the `loop_entry_only_undef` carve-out, which consults the same trace inside
/// its fixpoint). Pins that sharing one index across that fixpoint changes
/// nothing.
#[test]
fn loop_body_definition_read_after_loop_silent() {
    let src = "proc f {items} { foreach it $items { set last $it }\n puts $last }\n";
    assert!(
        !fires(src, D, "W210"),
        "a loop body that defines on every iteration is assumed to run; emitted {:?}",
        codes(src, D)
    );
}

/// A conditional `unset` kills the variable on one incoming, so the merge phi
/// can be undef even though every version is > 0. tclsh (`$c` true):
/// `can't read "x"`.
#[test]
fn conditional_unset_then_read_fires_w210() {
    let src = "proc f {c} { set x 1\n if {$c} { unset x }\n puts $x }\n";
    assert!(
        fires(src, D, "W210"),
        "a killed incoming makes the merge undef; emitted {:?}",
        codes(src, D)
    );
}

/// Straight-line `unset` then read — the non-phi control for the kill.
/// tclsh: `can't read "x": no such variable`.
#[test]
fn unset_then_read_fires_w210() {
    let src = "proc f {} { set x 1\n unset x\n puts $x }\n";
    assert!(
        fires(src, D, "W210"),
        "reading an unset-killed variable is RBS; emitted {:?}",
        codes(src, D)
    );
}

/// An `info exists` guard dominating the read proves the variable is defined
/// there, whatever the phi says. The guard drops the guarded operand edge, so
/// the origin behind it must stop propagating at the guard.
#[test]
fn info_exists_guard_silent() {
    let src = "proc f {c} { if {$c} { set x 1 }\n if {[info exists x]} { puts $x } }\n";
    assert!(
        !fires(src, D, "W210"),
        "a dominating `info exists` guard suppresses the read; emitted {:?}",
        codes(src, D)
    );
}

/// The `return`-value emitter reaches the trace by its own path
/// (`emit_return_phi_undef_w210`), which now shares one index across every
/// return block. Its verdicts are unchanged: a conditionally-written variable
/// returned unguarded is read-before-set.
#[test]
fn conditional_write_then_return_fires_w210() {
    let src = "proc f {c} { if {$c} { set x 1 }\n return $x }\n";
    assert!(
        fires(src, D, "W210"),
        "returning a conditionally-written variable is RBS; emitted {:?}",
        codes(src, D)
    );
}

/// …and the same shape with both arms writing stays silent.
#[test]
fn both_arms_write_then_return_silent() {
    let src = "proc f {c} { if {$c} { set x 1 } else { set x 2 }\n return $x }\n";
    assert!(
        !fires(src, D, "W210"),
        "`x` is written on both arms; returning it is not RBS; emitted {:?}",
        codes(src, D)
    );
}
