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

//! CFG construction from IR and the shared CFG edge-routing (lane) model.
//!
//! Two areas are covered:
//!   * `build_cfg` over lowered IR
//!   * the `cfg_layout` lane model (`assign_lanes`, `build_cfg_edges`,
//!     `ordered_block_names`)
//!
//! The Rust pipeline is `build_cfg(&lower_to_ir(src, registry), false)`. It
//! returns a [`CfgModule`] whose `.top_level` is the `::top` script CFG and
//! whose `.procedures` map holds each proc's [`Function`]. The lane-routing
//! model is `cfg_layout::{assign_lanes, build_cfg_edges, ordered_block_names}`,
//! the single source the SVG/ASCII renderers share.
//!
//! ## Structural vs. semantic proof split
//!
//! CFG shape — how many basic blocks a snippet produces, which terminator a
//! block ends with, how many dispatch branches a switch lowers to, which lane an
//! edge occupies — is **compiler-internal**: it is a property of the CFG-builder
//! and the lane-colouring algorithm, NOT of any Tcl runtime value. So almost
//! every assertion here is STRUCTURAL and needs no `tclsh` proof.
//!
//! A few assertions DO rest on a Tcl control-flow fact, and those were confirmed
//! against real `tclsh8.6` / `tclsh9.0` via `scripts/dev/tclsh_check.sh` and are
//! cited inline with a `// tclsh:` comment:
//!   * a `for {set i 0} {$i < 3} {incr i} {…}` loop provably runs ≥1 time (it
//!     ends with `i == 3`), which is *why* the faithful/analysis `build_cfg`
//!     rotates it to bottom-tested form — the real condition moves to the latch
//!     (`for_step`) and the header becomes an always-true entry guard.
//!     tclsh: `for {set i 0} {$i<3} {incr i} {}; puts $i` ⇒ `3` (8.6 and 9.0).
//!   * `set y 2` after `return $x` in a proc never executes, so the CFG has no
//!     block carrying that assignment.
//!     tclsh: `proc foo {} { set x 1; return $x; set y 2 }; foo;
//!     puts [info exists ::y]` ⇒ `0` (8.6 and 9.0).
//!
//! ## CFG-shape note: loop rotation
//!
//! The analysis `build_cfg` (`faithful_exceptions` on) ROTATES a provably-once
//! loop to bottom-tested form: the `for_header` keeps a synthetic always-true
//! `1` condition (so it still branches) and the real `$i < 3` condition moves to
//! the latch (`for_step`). The tests therefore assert the rotated shape (header
//! branches; the real condition appears on *some* branch; `for_end` is a loop
//! node). The un-rotated, header-tested shape is the codegen build's
//! (`build_cfg_codegen`), pinned in a separate test so both shapes are covered.

use tcl_compiler::cfg::{CfgModule, Function, Terminator};
use tcl_compiler::cfg_builder::{build_cfg, build_cfg_codegen};
use tcl_compiler::cfg_layout::{EdgeKind, assign_lanes, build_cfg_edges, ordered_block_names};
use tcl_compiler::expr_ast::render_expr;
use tcl_compiler::ir::Statement;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_registry::model::ingress::static_context_for;

// Shared helpers (mirror dataflow.rs / optimiser.rs registry setup)

const TCL: &str = "tcl8.6";

fn registry() -> &'static CommandRegistry {
    static_context_for(TCL).commands()
}

/// `source → CfgModule` via the analysis (faithful) builder.
/// `defer_top_level = false` so the top-level script CFG is built eagerly.
fn cfg(source: &str) -> CfgModule {
    build_cfg(&lower_to_ir(source, registry()), false)
}

/// The top-level (`::top`) script [`Function`].
fn top(module: &CfgModule) -> &Function {
    &module.top_level
}

/// A procedure's [`Function`] by qualified name.
fn proc<'a>(module: &'a CfgModule, qname: &str) -> &'a Function {
    module
        .procedures
        .get(qname)
        .unwrap_or_else(|| panic!("procedure {qname} not found"))
}

/// Every terminator in a function (block order is irrelevant for the
/// any/count predicates the tests use).
fn terminators(func: &Function) -> Vec<&Terminator> {
    func.blocks
        .values()
        .filter_map(|b| b.terminator.as_ref())
        .collect()
}

/// Does any block hold a `Statement::AssignConst` writing `name`?
fn assigns_const(func: &Function, name: &str) -> bool {
    func.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::AssignConst { name: n, .. } if n == name))
    })
}

/// Does any block *reachable from entry* hold a `Statement::AssignConst` writing
/// `name`? This answers "is this dead store on a live path?" — dead-after-`return`
/// statements are retained in an explicitly `unreachable_*` block rather than
/// dropped, so the live-path question is asked over the reachable set (see
/// `return_terminates_block`).
fn assigns_const_reachable(func: &Function, name: &str) -> bool {
    let reachable = func.reachable_blocks();
    func.blocks.iter().any(|(id, b)| {
        reachable.contains(id)
            && b.statements
                .iter()
                .any(|s| matches!(s, Statement::AssignConst { name: n, .. } if n == name))
    })
}

// CFG construction

#[test]
fn linear_script_cfg() {
    // `set a 1\nset b 2` — the entry block holds both AssignConst statements and
    // ends with a Goto (to the synthetic trailing exit block). STRUCTURAL.
    let module = cfg("set a 1\nset b 2");
    let func = top(&module);
    let entry = &func.blocks[&func.entry];
    assert_eq!(entry.statements.len(), 2);
    assert!(matches!(entry.statements[0], Statement::AssignConst { .. }));
    assert!(matches!(entry.terminator, Some(Terminator::Goto { .. })));
}

#[test]
fn if_creates_branching_cfg() {
    // An if/else creates a block ending in a Branch, and the post-if `set z 1`
    // survives somewhere in the CFG. STRUCTURAL (which blocks branch is a
    // property of the lowering).
    let module = cfg("if {$x > 0} {set y 1} else {set y 0}\nset z 1");
    let func = top(&module);
    assert!(
        terminators(func)
            .iter()
            .any(|t| matches!(t, Terminator::Branch { .. }))
    );
    assert!(
        assigns_const(func, "z"),
        "post-if `set z 1` must remain in the CFG"
    );
}

#[test]
fn switch_creates_dispatch_branches() {
    // A non-fallthrough EXACT switch is expanded (not opaque) into a dispatch
    // chain — one Branch per arm — so ≥2 blocks end in a Branch. STRUCTURAL (the
    // exact/expanded vs. glob/regexp/fallthrough/opaque split is a CFG-builder
    // decision).
    let module = cfg("switch $x {a {set y 1} b {set y 2}}");
    let func = top(&module);
    let branch_count = terminators(func)
        .iter()
        .filter(|t| matches!(t, Terminator::Branch { .. }))
        .count();
    assert!(
        branch_count >= 2,
        "expected ≥2 dispatch branches, got {branch_count}"
    );
}

#[test]
fn switch_fallthrough_stays_opaque() {
    // An exact switch with a fall-through arm (`b - default`) cannot be expressed
    // as structured control flow, so it is kept OPAQUE — a single
    // `Statement::Switch` in the block. STRUCTURAL.
    let module = cfg("switch $x {a {set y 1} b - default {set y 0}}");
    let func = top(&module);
    let switch_count = func
        .blocks
        .values()
        .flat_map(|b| b.statements.iter())
        .filter(|s| matches!(s, Statement::Switch { .. }))
        .count();
    assert_eq!(
        switch_count, 1,
        "fall-through exact switch stays one opaque Statement::Switch"
    );
}

#[test]
fn for_creates_loop_cfg() {
    // A `for` lowers to a loop with a `for_header` block that branches on the
    // loop condition, true/false targets both present, and the false (exit)
    // target recorded as a loop node.
    //
    // CFG-SHAPE (analysis rotation): the faithful/analysis `build_cfg` rotates
    // this provably-once loop to bottom-tested form. The `for_header` still ends
    // in a Branch, but on a synthetic always-true `1` guard; the REAL `$i < 3`
    // condition moves to the latch (`for_step`). So the tests assert:
    // (a) a `for_header` block exists and branches, (b) the real `$i < 3`
    // condition appears on *some* Branch in the loop, (c) both of that branch's
    // targets are real blocks, and (d) the loop's exit block is a loop node.
    //
    // tclsh PROVES the rotation precondition (the loop runs ≥1 time):
    // `for {set i 0} {$i<3} {incr i} {}; puts $i` ⇒ `3` (8.6 and 9.0), so the
    // first iteration always executes and bottom-testing is sound.
    let module = cfg("for {set i 0} {$i < 3} {incr i} {set sum [expr {$sum + $i}]}");
    let func = top(&module);

    // (a) A `for_header` block exists and branches.
    let header = func
        .blocks
        .values()
        .find(|b| b.name.starts_with("for_header"))
        .expect("a for_header block");
    assert!(
        matches!(header.terminator, Some(Terminator::Branch { .. })),
        "for_header must end in a Branch, got {:?}",
        header.terminator
    );

    // (b)+(c) The real `$i < 3` condition lives on some loop Branch, and both of
    // that branch's targets resolve to real blocks.
    let cond_branch = terminators(func)
        .into_iter()
        .find_map(|t| match t {
            Terminator::Branch {
                condition,
                true_target,
                false_target,
                ..
            } if render_expr(condition).contains("< 3") => Some((*true_target, *false_target)),
            _ => None,
        })
        .expect("a Branch carrying the real `$i < 3` condition");
    assert!(
        func.blocks.contains_key(&cond_branch.0),
        "true target is a real block"
    );
    assert!(
        func.blocks.contains_key(&cond_branch.1),
        "false target is a real block"
    );

    // (d) The loop's exit block (`for_end`) is recorded as a loop node.
    // `loop_nodes` is keyed by the loop's EXIT block id, so assert there is one
    // and it names a `for_end` block.
    assert!(
        !func.loop_nodes.is_empty(),
        "the `for` must register a loop node"
    );
    assert!(
        func.loop_nodes
            .keys()
            .any(|id| func.block_name(*id).starts_with("for_end")),
        "the loop node is keyed by the `for_end` exit block"
    );
}

/// The `render_expr` of a function's first `for_header` Branch condition.
/// A *rotated* (guaranteed-nonempty) loop carries the synthetic `"1"` guard;
/// a *may-run* loop keeps its real condition (e.g. `"$i < 3"`).
fn for_header_condition(func: &Function) -> String {
    let header = func
        .blocks
        .values()
        .find(|b| b.name.starts_with("for_header"))
        .expect("a for_header block");
    match &header.terminator {
        Some(Terminator::Branch { condition, .. }) => render_expr(condition),
        other => panic!("for_header must branch, got {other:?}"),
    }
}

#[test]
fn for_rotation_requires_a_non_stale_constant_init() {
    // Loop rotation (bottom-testing a provably-once loop) is sound only when the
    // init clause proves the condition true on entry. `for_runs_at_least_once`
    // processes the init in order, so a later non-constant write, an `incr`, or
    // an opaque call that could touch the loop var must invalidate the stale
    // constant — otherwise a possibly-zero-iteration loop would be wrongly
    // rotated (its optimiser static-for summary and its zero-trip edge would be
    // unsound). W210 no longer distinguishes these (a may-run loop whose body
    // defines the var is silent after the loop, matching C Tcl — issue #756), so
    // this pins the rotation decision directly on the CFG shape.

    // Guaranteed: `0 < 3` is true on entry → rotated (header carries `1`).
    assert_eq!(
        for_header_condition(top(&cfg("for {set i 0} {$i < 3} {incr i} {set y $i}"))),
        "1",
        "clean constant init must rotate",
    );
    // A benign init call that cannot write the loop var keeps it guaranteed.
    assert_eq!(
        for_header_condition(top(&cfg(
            "for {set i 0; puts hi} {$i < 3} {incr i} {set y $i}"
        ))),
        "1",
        "benign init call must not invalidate the constant",
    );

    // Stale constant: `set i $n` overwrites `set i 0`, so the condition is
    // unknown → NOT rotated (the real `$i < 3` stays on the header).
    assert_eq!(
        for_header_condition(top(&cfg(
            "for {set i 0; set i $n} {$i < 3} {incr i} {set y $i}"
        ))),
        "$i < 3",
        "a non-constant re-write of the loop var must not be claimed guaranteed",
    );
    // An `incr` in the init likewise invalidates the constant binding.
    assert_eq!(
        for_header_condition(top(&cfg(
            "for {set i 0; incr i 5} {$i < 3} {incr i} {set y $i}"
        ))),
        "$i < 3",
        "incr in for-init must not be claimed guaranteed",
    );
    // An init call that writes the loop var through `upvar` must invalidate it.
    let m = cfg(
        "proc setter {} { upvar 1 i i; set i 5 }\nproc f {} { for {set i 0; setter} {$i < 3} {incr i} { set y $i } }\n",
    );
    assert_eq!(
        for_header_condition(proc(&m, "::f")),
        "$i < 3",
        "an upvar-writing init call must not be claimed guaranteed",
    );
    // A call to a proc whose `upvar` SOURCE is dynamic (`upvar 1 [pick] v`)
    // can write ANY caller variable — the per-name summary is empty, so the
    // call site must widen to an opaque barrier rather than trust it.
    let m = cfg(
        "proc clobber {} { upvar 1 [pick] slot; set slot 99 }\nproc f {} { for {set i 0; clobber} {$i < 3} {incr i} { set y $i } }\n",
    );
    assert_eq!(
        for_header_condition(proc(&m, "::f")),
        "$i < 3",
        "a dynamic-source upvar callee must widen the caller (opaque clobber)",
    );
}

#[test]
fn return_terminates_block() {
    // In a proc whose body is `set x 1; return $x; set y 2`, some block ends in a
    // Return, and the dead `set y 2` after the return is NOT reachable in the CFG.
    //
    // REPRESENTATION NOTE (NOT a bug). The faithful builder *isolates* the
    // dead-after-return `set y 2` in an explicitly `unreachable_*` block rather
    // than dropping it — the entry block ends in `Return` right after `set x 1`,
    // and a separate, non-entry-reachable `unreachable_2` block holds `set y 2`
    // (retaining the source for diagnostics/LSP; downstream analyses filter on
    // reachability). It still encodes that `set y 2` never executes. The tests
    // assert the live-path form: (a) a block ends in Return, and (b) `set y 2` is
    // in NO reachable block. (Verified empirically: entry_1 `set x 1`→Return;
    // unreachable_2 `set y 2` is not reachable.)
    //
    // tclsh PROVES the dead-code fact the CFG encodes: `proc foo {} { set x 1;
    // return $x; set y 2 }; foo; puts [info exists ::y]` ⇒ `0` (8.6 and 9.0) —
    // `set y 2` never runs.
    let module = cfg("proc foo {} {\n    set x 1\n    return $x\n    set y 2\n}\n");
    let func = proc(&module, "::foo");
    assert!(
        terminators(func)
            .iter()
            .any(|t| matches!(t, Terminator::Return { .. })),
        "the proc must have a Return terminator"
    );
    // `set y 2` is unreachable (the live-path form of "not in the CFG").
    assert!(
        !assigns_const_reachable(func, "y"),
        "dead `set y 2` after return must not be on any reachable path"
    );
    // And it IS retained (in an unreachable block) — pin this representation
    // so a future change that silently drops or revives it is caught.
    assert!(
        assigns_const(func, "y"),
        "Rust retains the dead store in an unreachable block (not dropped)"
    );
    assert!(
        func.blocks
            .values()
            .any(|b| b.name.starts_with("unreachable")),
        "the dead tail lives in an `unreachable_*` block"
    );
}

// Shared CFG edge-routing (lane) model

/// Do two closed integer intervals overlap (touching endpoints count as
/// overlap)?
fn spans_overlap(a: (usize, usize), b: (usize, usize)) -> bool {
    let (lo1, hi1) = if a.0 <= a.1 { (a.0, a.1) } else { (a.1, a.0) };
    let (lo2, hi2) = if b.0 <= b.1 { (b.0, b.1) } else { (b.1, b.0) };
    !(hi1 < lo2 || lo1 > hi2)
}

// The branchy fixture: a proc with a for-loop whose body is an if/else — the
// loop header is a conditional branch, so the routed edges include a true +
// false pair out of `for_header`.
const BRANCHY: &str = "proc f {x} {\n    set total 0\n    for {set i 0} {$i < $x} {incr i} {\n        if {$i % 2 == 0} { incr total $i } else { incr total 1 }\n    }\n    return $total\n}\n";

// -- assign_lanes (the routing contract) --

#[test]
fn disjoint_spans_share_lane_zero() {
    // Sequential non-overlapping spans all nest into the innermost lane.
    // STRUCTURAL (pure algorithm).
    assert_eq!(assign_lanes(&[(0, 1), (2, 3), (4, 5)]), vec![0, 0, 0]);
}

#[test]
fn touching_endpoints_force_distinct_lanes() {
    // An edge into block 2 and an edge out of block 2 must not share a lane
    // (closed intervals).
    assert_eq!(assign_lanes(&[(0, 2), (2, 4)]), vec![0, 1]);
}

#[test]
fn shortest_span_gets_innermost_lane() {
    // The longest span is processed last → outer lane, so the shorter span gets
    // the inner lane.
    let lanes = assign_lanes(&[(0, 5), (1, 2)]);
    assert!(
        lanes[1] < lanes[0],
        "shortest span (index 1) must take the inner lane: {lanes:?}"
    );
}

#[test]
fn empty_spans_assign_no_lanes() {
    // Empty input → no lanes (the degenerate case the property test's random
    // span count never hits but the contract still requires).
    assert_eq!(assign_lanes(&[]), Vec::<usize>::new());
}

#[test]
fn no_two_same_lane_edges_overlap() {
    // A randomised property check — for 500 random span sets, no two spans
    // sharing a lane overlap. Uses a DETERMINISTIC PRNG seeded by a constant (no
    // time/thread entropy) so the run is reproducible; the contract is invariant
    // to the exact sequence.
    let mut rng = XorShift::new(0x9E37_79B9_7F4A_7C15);
    for _ in 0..500 {
        let n = 1 + (rng.next_u32() % 8) as usize; // 1..=8 spans
        let spans: Vec<(usize, usize)> = (0..n)
            .map(|_| {
                (
                    (rng.next_u32() % 10) as usize,
                    (rng.next_u32() % 10) as usize,
                )
            })
            .collect();
        let lanes = assign_lanes(&spans);
        // No two spans on the same lane may overlap.
        let mut by_lane: Vec<Vec<(usize, usize)>> = Vec::new();
        for (&span, &lane) in spans.iter().zip(&lanes) {
            if lane >= by_lane.len() {
                by_lane.resize(lane + 1, Vec::new());
            }
            for &other in &by_lane[lane] {
                assert!(
                    !spans_overlap(span, other),
                    "same-lane overlap: spans={spans:?} lanes={lanes:?}"
                );
            }
            by_lane[lane].push(span);
        }
    }
}

// -- build_cfg_edges --

#[test]
fn branch_kinds_and_lanes() {
    // The loop header is a conditional branch, so its outgoing edges include one
    // True and one False edge; every edge is routed (a lane is assigned) and no
    // two same-lane edges overlap. (`build_cfg_edges` takes an explicit block
    // order — the canonical creation order from `ordered_block_names`.)
    // STRUCTURAL.
    let module = cfg(BRANCHY);
    let func = proc(&module, "::f");
    let order = ordered_block_names(func);
    let edges = build_cfg_edges(func, &order);

    // The for_header branch contributes a True and a False edge.
    assert!(
        edges
            .iter()
            .any(|e| e.src.starts_with("for_header") && e.kind == EdgeKind::True),
        "expected a True edge out of for_header; edges: {:?}",
        edges.iter().map(|e| (&e.src, e.kind)).collect::<Vec<_>>()
    );
    assert!(
        edges
            .iter()
            .any(|e| e.src.starts_with("for_header") && e.kind == EdgeKind::False),
        "expected a False edge out of for_header"
    );

    // Every successor edge is routed, and no two same-lane edges overlap.
    // (`EdgeKind`/lane usize is always ≥ 0; the meaningful check is no-collision.)
    let mut by_lane: Vec<Vec<(usize, usize)>> = Vec::new();
    for e in &edges {
        if e.lane >= by_lane.len() {
            by_lane.resize(e.lane + 1, Vec::new());
        }
        for &other in &by_lane[e.lane] {
            assert!(
                !spans_overlap((e.src_pos, e.dst_pos), other),
                "routed edges collide on lane {}: {edges:?}",
                e.lane
            );
        }
        by_lane[e.lane].push((e.src_pos, e.dst_pos));
    }
}

#[test]
fn edge_kinds_classified_goto_true_false() {
    // Edge-kind classification (the routing model's other half): a Goto-only
    // function yields only Goto edges; the branchy function yields ≥1 True and
    // ≥1 False edge. The kind contract: `kind` ∈ {goto,true,false}. STRUCTURAL.
    let lin = cfg("set a 1\nset b 2");
    let lfunc = top(&lin);
    let lorder = ordered_block_names(lfunc);
    let ledges = build_cfg_edges(lfunc, &lorder);
    assert!(
        !ledges.is_empty(),
        "a linear script still has a fall-through Goto edge"
    );
    assert!(
        ledges.iter().all(|e| e.kind == EdgeKind::Goto),
        "a branch-free script has only Goto edges: {:?}",
        ledges.iter().map(|e| e.kind).collect::<Vec<_>>()
    );

    let module = cfg(BRANCHY);
    let func = proc(&module, "::f");
    let order = ordered_block_names(func);
    let edges = build_cfg_edges(func, &order);
    assert!(
        edges.iter().any(|e| e.kind == EdgeKind::True),
        "branchy fn has a True edge"
    );
    assert!(
        edges.iter().any(|e| e.kind == EdgeKind::False),
        "branchy fn has a False edge"
    );
    // The contract string form (as the JSON/renderers consume it).
    assert_eq!(EdgeKind::Goto.as_str(), "goto");
    assert_eq!(EdgeKind::True.as_str(), "true");
    assert_eq!(EdgeKind::False.as_str(), "false");
}

#[test]
fn block_ordering_follows_creation_order() {
    // `ordered_block_names` recovers the block CREATION order (the order the
    // explorer renders in) by sorting on the trailing `_<n>` counter. For a
    // simple `if`, the entry block (lowest counter) comes first and every named
    // block appears exactly once. STRUCTURAL — pins the display-order contract
    // the edge router's ordinals depend on.
    let module = cfg("if {$x > 0} {set y 1} else {set y 0}\nset z 1");
    let func = top(&module);
    let order = ordered_block_names(func);

    // The set of ordered names equals the set of the function's block names.
    assert_eq!(
        order.len(),
        func.blocks.len(),
        "every block appears once in the order"
    );
    assert_eq!(
        order
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>(),
        func.blocks
            .values()
            .map(|b| b.name.clone())
            .collect::<std::collections::HashSet<_>>(),
        "ordered names match the block set"
    );

    // The entry block sorts first (it carries the lowest creation counter).
    assert_eq!(
        order[0],
        func.block_name(func.entry),
        "entry block (lowest counter) sorts first"
    );

    // The trailing `_<n>` counters are non-decreasing across the order — i.e.
    // the sort is by creation order, not lexicographic on the prefix.
    let counters: Vec<u64> = order
        .iter()
        .map(|name| {
            name.rsplit_once('_')
                .and_then(|(_, n)| n.parse::<u64>().ok())
                .unwrap_or(u64::MAX)
        })
        .collect();
    assert!(
        counters.windows(2).all(|w| w[0] <= w[1]),
        "creation counters must be non-decreasing: {order:?} → {counters:?}"
    );
}

// Codegen-build shape (un-rotated, header-tested loop)
//
// The analysis `for_creates_loop_cfg` above asserts the ROTATED shape. The
// codegen builder (`build_cfg_codegen`, faithful_exceptions OFF) leaves the
// loop header-tested — so the `$i < 3` condition stays on the `for_header`
// block. Pinning it here keeps both shapes covered and documents the rotation
// as analysis-only.

#[test]
fn codegen_for_loop_is_header_tested() {
    let module = build_cfg_codegen(
        &lower_to_ir("for {set i 0} {$i < 3} {incr i} {set sum 1}", registry()),
        false,
    );
    let func = &module.top_level;
    let header = func
        .blocks
        .values()
        .find(|b| b.name.starts_with("for_header"))
        .expect("a for_header block");
    // In the codegen (un-rotated) build the header branches on the REAL
    // condition — not a synthetic `1` guard.
    match &header.terminator {
        Some(Terminator::Branch { condition, .. }) => {
            assert!(
                render_expr(condition).contains("< 3"),
                "codegen for_header must test the real `$i < 3` condition, got `{}`",
                render_expr(condition)
            );
        }
        other => panic!("codegen for_header must end in a Branch, got {other:?}"),
    }
}

// Deterministic PRNG for the no-collision property test (no time/Math.random).
// A 64-bit xorshift* — fixed seed in, identical stream out on every run.

struct XorShift(u64);

impl XorShift {
    fn new(seed: u64) -> Self {
        // Avoid the all-zero fixed point.
        Self(if seed == 0 {
            0xDEAD_BEEF_CAFE_F00D
        } else {
            seed
        })
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        // Take the high bits (better distributed than the low bits).
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }
}
