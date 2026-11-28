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

//! Code sinking O125, load forwarding O127 and the hoists GVN offers under the SCCP executable set.

use super::*;

// Code sinking — O125

#[test]
fn code_sinking_o125_positive() {
    crate::install_surfaces();
    // Basic sink into an `if` whose condition does not read the var. Either O125
    // OR O100 is accepted (propagation may subsume sinking); here O125 fires.
    // tclsh: `set b foo; if {$a} {puts $b}` and the sunk form
    // `if {$a} {set b foo; puts $b}` both print foo iff $a is true.
    let basic = "set a [llength $argv]\nset b foo\nif {$a} {\n    puts $b\n}";
    let codes = opt_codes(basic, TCL);
    assert!(codes.iter().any(|c| c == "O125" || c == "O100"));

    // Sink preserves indentation when a nested redefine blocks O100 folding.
    // The sunk `set b foo` lands at the outer-if body indent (4 spaces).
    let indent = "lassign $argv a c d\nset b foo\nif {$a} {\n    if {$c} {\n        set b bar\n    }\n    if {$d} {\n        puts $b\n    }\n}";
    assert!(opt_fires(indent, TCL, "O125"));
    let io = optimised(indent, TCL);
    let sunk: Vec<&str> = io
        .lines()
        .filter(|l| l.trim_start().starts_with("set b foo"))
        .collect();
    assert!(!sunk.is_empty(), "expected a sunk `set b foo` line: {io}");

    // Coexists with O120 (string-compare rewrite in the condition).
    let with120 = "set kind [lindex $argv 0]\nset b foo\nif {$kind == \"x\"} {\n    puts $b\n}";
    assert!(opt_fires(with120, TCL, "O120"));
    assert!(optimised(with120, TCL).contains("$kind eq \"x\""));
    let c120 = opt_codes(with120, TCL);
    assert!(c120.iter().any(|c| c == "O125" || c == "O100"));
}

#[test]
fn code_sinking_o125_negatives() {
    crate::install_surfaces();
    // O125 must NOT fire when the var is read in the condition, used after the
    // block, or its RHS is a command substitution.
    let neg = [
        "set b $x\nif {$b} {\n    puts hello\n}", // var in condition
        "set b foo\nif {$a} {\n    puts $b\n}\nputs $b", // used after ($-form)
        "set b [clock seconds]\nif {$a} {\n    puts $b\n}", // cmd-sub RHS
        // Used after by *name* rather than by substitution. `set b foo` must
        // stay put: on the false path the sunk form leaves `b` undefined, so
        // the later read errors where the original printed nothing. The
        // registry's VarRead / VarWrite positions answer for a command the
        // lowerer resolved, and the bareword scan covers a name inside a
        // nested command substitution.
        "set b foo\nif {$a} {\n    puts $b\n}\ninfo exists b", // VarRead position
        "set b foo\nif {$a} {\n    puts $b\n}\nappend b tail", // read-before-write
        "set b foo\nif {$a} {\n    puts $b\n}\nincr b",        // read-modify-write
        "set b foo\nif {$a} {\n    puts $b\n}\nputs [set b]",  // nested substitution
        "set b foo\nif {$a} {\n    puts $b\n}\nif {$c} {\n    puts [set b]\n}", // nested, in a body
    ];
    for src in neg {
        assert!(!opt_fires(src, TCL, "O125"), "O125 must not fire: {src}");
    }

    // NOTE on omissions. Sound-but-spurious O125 firings (applied rewrite
    // PREPENDS `set b foo` into the branch while KEEPING the outer assignment,
    // so a tclsh run is unaffected) — omitted:
    //  - var not used in the branch at all (`puts hello`).
    //  - numeric constant `set b 42` (handled by O100/O109, not O125).
    //  - cross-event shared var (excluded from sinking).
    //  - `if {0}` block (O112 drops the block AND all O125 parts).
    // GENUINE BUG (omitted): a sink must NOT happen for
    //   set b $x ; if {[incr x] > 0} { puts $b }
    // because `[incr x]` in the condition mutates b's RHS dependency. O125 fires
    // and the *applied* output
    //   set b $x ; if {[incr x] > 0} { set b $x; puts $b }
    // re-reads $x AFTER the incr — tclsh (x=5) ORIG prints 5, REWRITTEN prints 6.
    // A real miscompile, so it is reported rather than asserted.
}

#[test]
fn code_sinking_o125_applies_both_grouped_edits_and_reparses() {
    crate::install_surfaces();
    // The KCS O125 page's Before snippet, in its braced and quoted spellings.
    // Two properties the pair of grouped edits must hold, which either spelling
    // alone would let slip:
    //
    //  1. The deletion and the insertion both survive the manager's
    //     resurrected-reference guard, so the assignment moves rather than
    //     being copied. The insertion's replacement holds the sunk `set msg …`
    //     and the `$msg` consuming it, which reads as a resurrection of the
    //     variable the deletion removes unless group-mates are exempt.
    //  2. The quoted spelling survives the inner-end convention, under which
    //     the assignment's IR statement span stops *on* its closing `"`.
    //     Replaying that span emits `set msg "error`, whose unterminated quote
    //     swallows the rest of the line.
    //
    // tclsh: for either spelling, `set msg V; if {$ok} {return} else {log $msg}`
    // and the sunk `if {$ok} {return} else {set msg V; log $msg}` are
    // observationally identical — `msg` is read in exactly one branch and
    // nowhere after the decision.
    for (before, after) in [
        (
            "proc log {m} {puts $m}\nset ok [llength $argv]\nset msg {error}\nif {$ok} { return } else { log $msg }",
            "proc log {m} {puts $m}\nset ok [llength $argv]\nif {$ok} { return } else { set msg {error}; log $msg }",
        ),
        (
            "proc log {m} {puts $m}\nset ok [llength $argv]\nset msg \"error\"\nif {$ok} { return } else { log $msg }",
            "proc log {m} {puts $m}\nset ok [llength $argv]\nif {$ok} { return } else { set msg \"error\"; log $msg }",
        ),
    ] {
        assert_eq!(
            opt_count(before, TCL, "O125"),
            2,
            "both grouped O125 edits must survive selection: {before}"
        );
        assert_eq!(optimised(before, TCL), after, "sunk output for: {before}");
        assert_eq!(
            optimised(before, TCL).matches("set msg").count(),
            1,
            "the original assignment must be deleted, not duplicated: {before}"
        );
        assert!(
            reparse_errors(&optimised(before, TCL), TCL).is_empty(),
            "the sunk output must re-parse: {}",
            optimised(before, TCL)
        );
    }
}

// Load forwarding — O127 (single-use store-to-load forwarding)

#[test]
fn load_forwarding_o127() {
    crate::install_surfaces();
    // Single-use var with a command substitution is inlined. tclsh: the inlined
    // `[set x [clock seconds]]` both assigns x and yields its value — identical
    // observable result to `set x [clock seconds]; puts $x`.
    let cmdsub = "proc test {} {\n    set x [clock seconds]\n    puts $x\n}";
    assert!(opt_fires(cmdsub, TCL, "O127"));
    assert!(optimised(cmdsub, TCL).contains("puts [set x [clock seconds]]"));

    // Constants are handled by the propagation/DSE pass, NOT O127. tclsh:
    // x=42 ⇒ puts 42. (The single-def literal is forwarded via O102 + DSE O109 —
    // the key invariant is that O127 does NOT claim it.)
    let constv = "proc test {} {\n    set x 42\n    puts $x\n}";
    assert!(!opt_fires(constv, TCL, "O127"));
    assert!(opt_fires(constv, TCL, "O102"));
    assert!(optimised(constv, TCL).contains("puts 42"));

    // Variable used more than once ⇒ not inlined.
    let multi = "proc test {} {\n    set x [clock seconds]\n    puts $x\n    puts $x\n}";
    assert!(!opt_fires(multi, TCL, "O127"));

    // Top-level variables ⇒ not inlined.
    assert!(!opt_fires("set x [clock seconds]\nputs $x", TCL, "O127"));

    // NOTE on omissions (known gaps):
    //  - `set x $arg; puts $x` (var-copy) — left unchanged rather than inlined to
    //    `puts [set x $arg]` (O127); sound, just not forwarded.
    //  - intervening empty `eval {}` barrier — O127 still forwards (the empty eval
    //    is not treated as a barrier).
}

// Deletion and replay extents — a removal or replay covers the whole written
// statement, closing delimiter included.

#[test]
fn removed_and_replayed_statements_keep_their_closer() {
    crate::install_surfaces();
    // A statement span stops *on* the closer of a quoted, braced, or bracketed
    // last word. A removal that inherits that boundary strands the closer on a
    // line of its own, and a replay copies an opener without it, so each case
    // pins the exact text and re-checks it through the `tcl diag` surface.
    //
    // tclsh: both pairs are observationally identical. `set a "hello"; puts $a`
    // prints `hello`, as does `puts hello`; `set x V; puts $x` prints V, as
    // does `puts [set x V]`, since `set` yields the value it assigns.
    for (before, after) in [
        // O102 propagation coupled with the O109 removal of the feeding store,
        // whose value word is quoted.
        (
            "proc f {} {\n  set a \"hello\"\n  puts $a\n}",
            "proc f {} {\n  puts hello\n}",
        ),
        // O127 replays the assignment into the use site; the value word is
        // quoted and holds a command substitution.
        (
            "proc f {} {\n  set x \"a [clock seconds] b\"\n  puts $x\n}",
            "proc f {} {\n  puts [set x \"a [clock seconds] b\"]\n}",
        ),
    ] {
        assert_eq!(
            optimised(before, TCL),
            after,
            "rewritten source for: {before}"
        );
        assert!(
            reparse_errors(&optimised(before, TCL), TCL).is_empty(),
            "the rewritten source must re-parse: {}",
            optimised(before, TCL)
        );
    }
}

// Code motion and deletion must share one "is this block executable" fact.
//
// The owner is `SccpResult::executable_blocks` (`sccp.rs:199`), the
// optimistic set SCCP fills in while it folds constant branches. The
// deletion side already reads it: `optimiser/elimination.rs:392`
// (`unreachable_blocks`, the O107 source), and O112's constant-condition
// deletions in `optimiser/structure_elimination.rs` are the structured-IR
// view of the same constant facts. GVN's PRE and LICM must not answer the
// question themselves with a plain terminator-successor walk: that would
// let a block behind a constant-false branch still look executable and
// draw an O106 hoist / O105 partial-redundancy offer into a region O112 is
// simultaneously offering to delete.
//
// `find_loop_invariants` / `find_partial_redundancies` take that set
// instead, and `*_for_function` seed it from `fu.sccp.executable_blocks`.

/// `src` compiled to a unit whose top level carries the SCCP result the
/// deletion passes read.
fn top_level_unit(src: &str) -> tcl_compiler::compilation_unit::CompilationUnit {
    let registry = static_context_for(TCL).commands();
    tcl_compiler::compilation_unit::CompilationUnit::build_for_dialect(src, registry, false, TCL)
}

/// `(licm, pre)` finding counts under the executable set the deletion passes
/// use — the same seeding `find_loop_invariants_for_function` /
/// `find_partial_redundancies_for_function` do internally.
fn gvn_offers_under_sccp(src: &str) -> (usize, usize) {
    let registry = static_context_for(TCL).commands();
    let cu = top_level_unit(src);
    let fu = &cu.top_level;
    let executable = &fu.sccp.executable_blocks;
    (
        tcl_compiler::gvn::find_loop_invariants(
            registry,
            &fu.cfg,
            &fu.ssa,
            executable,
            Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        )
        .len(),
        tcl_compiler::gvn::find_partial_redundancies(
            registry,
            &fu.cfg,
            &fu.ssa,
            executable,
            Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        )
        .len(),
    )
}

// A constant-false `if` wrapping a loop: O112 deletes the whole `if`, so the
// loop body must draw no O106 hoist. Structural (which code fires), not
// Tcl-observable: tclsh 8.6/9.0 runs neither the loop nor any hoisted copy,
// so both the original and the O112 rewrite produce no output.
#[test]
fn licm_offers_no_hoist_inside_a_block_o112_deletes_issue_1385() {
    crate::install_surfaces();
    let dead = "set lst $argv\nif {0} {\n    for {set i 0} {$i < 3} {incr i} {\n        lindex $lst 0\n    }\n}\n";
    assert!(
        opt_fires(dead, TCL, "O112"),
        "the constant-false if must still be offered for deletion"
    );
    assert_eq!(
        gvn_offers_under_sccp(dead).0,
        0,
        "no O106 hoist may be offered inside the region O112 deletes"
    );

    // Control: the same loop outside the dead branch keeps its hoist, so the
    // executable fact is pruning dead code and not disabling LICM.
    let live = "set lst $argv\nfor {set i 0} {$i < 3} {incr i} {\n    lindex $lst 0\n}\n";
    assert!(!opt_fires(live, TCL, "O112"));
    assert_eq!(gvn_offers_under_sccp(live).0, 1);
}

// The same pairing for O105 partial redundancy: the diamond and its merge
// point both sit inside the constant-false `if` O112 deletes.
#[test]
fn pre_offers_no_hoist_inside_a_block_o112_deletes_issue_1385() {
    crate::install_surfaces();
    let dead = "set lst $argv\nif {0} {\n    if {$argc} {\n        lindex $lst 0\n    }\n    lindex $lst 0\n}\n";
    assert!(
        opt_fires(dead, TCL, "O112"),
        "the constant-false if must still be offered for deletion"
    );
    assert_eq!(
        gvn_offers_under_sccp(dead).1,
        0,
        "no O105 partial-redundancy hoist may be offered inside the region O112 deletes"
    );

    // Control: the same diamond outside the dead branch still reports.
    let live = "set lst $argv\nif {$argc} {\n    lindex $lst 0\n}\nlindex $lst 0\n";
    assert!(!opt_fires(live, TCL, "O112"));
    assert_eq!(gvn_offers_under_sccp(live).1, 1);
}

// The production entries carry the same contract without the caller having
// to seed the set: they read `fu.sccp.executable_blocks` themselves.
#[test]
fn production_gvn_entries_read_the_sccp_executable_fact_issue_1385() {
    crate::install_surfaces();
    let registry = static_context_for(TCL).commands();
    let dead = "set lst $argv\nif {0} {\n    for {set i 0} {$i < 3} {incr i} {\n        lindex $lst 0\n    }\n    if {$argc} {\n        llength $lst\n    }\n    llength $lst\n}\n";
    let cu = top_level_unit(dead);
    let fu = &cu.top_level;
    assert!(
        fu.cfg
            .blocks
            .keys()
            .any(|b| !fu.sccp.executable_blocks.contains(b)),
        "the reproducer must actually contain SCCP-dead blocks"
    );
    assert!(
        tcl_compiler::gvn::find_loop_invariants_for_function(
            registry,
            fu,
            Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile())
        )
        .is_empty(),
        "O106 must not fire inside SCCP-dead code"
    );
    assert!(
        tcl_compiler::gvn::find_partial_redundancies_for_function(
            registry,
            fu,
            Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile())
        )
        .is_empty(),
        "O105-PRE must not fire inside SCCP-dead code"
    );
}
