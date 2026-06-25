//! Port of the Python CFG suites — CFG construction from IR and the shared
//! CFG edge-routing (lane) model.
//!
//! Ported from two pytest files that drive the *Python* compiler:
//!   * `tests/test_cfg.py`        → `compiler.cfg.build_cfg` over lowered IR
//!   * `tests/test_cfg_layout.py` → `tooling.explorer.cfg_layout` (`assign_lanes`,
//!                                   `build_cfg_edges`)
//!
//! The Rust pipeline is `build_cfg(&lower_to_ir(src, registry), false)`, the
//! analogue of Python's `build_cfg(lower_to_ir(src))`. It returns a
//! [`CfgModule`] whose `.top_level` is the `::top` script CFG and whose
//! `.procedures` map holds each proc's [`Function`] — the same artefacts the
//! Python `build_cfg(mod).top_level` / `.procedures["::foo"]` expose. The
//! lane-routing model is `cfg_layout::{assign_lanes, build_cfg_edges,
//! ordered_block_names}`, the single-source the SVG/ASCII renderers share (the
//! Python `tooling.explorer.cfg_layout`).
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
//! ## CFG-shape divergence from Python: loop rotation
//!
//! Python's `build_cfg` emits header-tested loops, so its `test_for_creates_loop_cfg`
//! finds the `$i < 3` condition on the `for_header` block. The Rust *analysis*
//! `build_cfg` (faithful_exceptions on) ROTATES a provably-once loop to
//! bottom-tested form: the `for_header` keeps a synthetic always-true `1`
//! condition (so it still branches) and the real `$i < 3` condition moves to the
//! latch (`for_step`). The port therefore asserts the rotated shape (header
//! branches; the real condition appears on *some* branch; `for_end` is a loop
//! node). The un-rotated, header-tested shape is the codegen build's
//! (`build_cfg_codegen`), pinned in a separate test so both shapes are covered.

use tcl_compiler::cfg::{CfgModule, Function, Terminator};
use tcl_compiler::cfg_builder::{build_cfg, build_cfg_codegen};
use tcl_compiler::cfg_layout::{assign_lanes, build_cfg_edges, ordered_block_names, EdgeKind};
use tcl_compiler::expr_ast::render_expr;
use tcl_compiler::ir::Statement;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::{registry_for_dialect, CommandRegistry};

// ---------------------------------------------------------------------------
// Shared helpers (mirror dataflow_port.rs / optimiser_port.rs registry setup)
// ---------------------------------------------------------------------------

const TCL: &str = "tcl8.6";

fn registry() -> &'static CommandRegistry {
    registry_for_dialect(TCL)
}

/// `source → CfgModule` via the analysis (faithful) builder — the Rust analogue
/// of Python `build_cfg(lower_to_ir(src))`. `defer_top_level = false` so the
/// top-level script CFG is built eagerly (as Python's is).
fn cfg(source: &str) -> CfgModule {
    build_cfg(&lower_to_ir(source, registry()), false)
}

/// The top-level (`::top`) script [`Function`] — Python `build_cfg(mod).top_level`.
fn top(module: &CfgModule) -> &Function {
    &module.top_level
}

/// A procedure's [`Function`] by qualified name — Python `cfgm.procedures["::foo"]`.
fn proc<'a>(module: &'a CfgModule, qname: &str) -> &'a Function {
    module
        .procedures
        .get(qname)
        .unwrap_or_else(|| panic!("procedure {qname} not found"))
}

/// Every terminator in a function (block order is irrelevant for the
/// any/count predicates the Python tests use).
fn terminators(func: &Function) -> Vec<&Terminator> {
    func.blocks.values().filter_map(|b| b.terminator.as_ref()).collect()
}

/// Does any block hold a `Statement::AssignConst` writing `name`? (Python
/// `any(isinstance(s, IRAssignConst) and s.name == name …)`.)
fn assigns_const(func: &Function, name: &str) -> bool {
    func.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::AssignConst { name: n, .. } if n == name))
    })
}

/// Does any block *reachable from entry* hold a `Statement::AssignConst` writing
/// `name`? The faithful analogue of Python's "is this dead store on a live
/// path?" — Rust retains dead-after-`return` statements in an explicitly
/// `unreachable_*` block rather than dropping them, so the live-path question is
/// asked over the reachable set (see `return_terminates_block`).
fn assigns_const_reachable(func: &Function, name: &str) -> bool {
    let reachable = func.reachable_blocks();
    func.blocks.iter().any(|(id, b)| {
        reachable.contains(id)
            && b.statements
                .iter()
                .any(|s| matches!(s, Statement::AssignConst { name: n, .. } if n == name))
    })
}

// ===========================================================================
// test_cfg.py — CFG construction
// ===========================================================================

#[test]
fn linear_script_cfg() {
    // Python TestCFG::test_linear_script_cfg: `set a 1\nset b 2` — the entry
    // block holds both AssignConst statements and ends with a Goto (to the
    // synthetic trailing exit block). STRUCTURAL.
    let module = cfg("set a 1\nset b 2");
    let func = top(&module);
    let entry = &func.blocks[&func.entry];
    assert_eq!(entry.statements.len(), 2);
    assert!(matches!(entry.statements[0], Statement::AssignConst { .. }));
    assert!(matches!(entry.terminator, Some(Terminator::Goto { .. })));
}

#[test]
fn if_creates_branching_cfg() {
    // Python TestCFG::test_if_creates_branching_cfg: an if/else creates a block
    // ending in a Branch, and the post-if `set z 1` survives somewhere in the
    // CFG. STRUCTURAL (which blocks branch is a property of the lowering).
    let module = cfg("if {$x > 0} {set y 1} else {set y 0}\nset z 1");
    let func = top(&module);
    assert!(terminators(func).iter().any(|t| matches!(t, Terminator::Branch { .. })));
    assert!(assigns_const(func, "z"), "post-if `set z 1` must remain in the CFG");
}

#[test]
fn switch_creates_dispatch_branches() {
    // Python TestCFG::test_switch_creates_dispatch_branches: a non-fallthrough
    // EXACT switch is expanded (not opaque) into a dispatch chain — one Branch
    // per arm — so ≥2 blocks end in a Branch. STRUCTURAL (the exact/expanded vs.
    // glob/regexp/fallthrough/opaque split is a CFG-builder decision).
    let module = cfg("switch $x {a {set y 1} b {set y 2}}");
    let func = top(&module);
    let branch_count = terminators(func)
        .iter()
        .filter(|t| matches!(t, Terminator::Branch { .. }))
        .count();
    assert!(branch_count >= 2, "expected ≥2 dispatch branches, got {branch_count}");
}

#[test]
fn switch_fallthrough_stays_opaque() {
    // Python TestCFG::test_switch_fallthrough_stays_as_irswitch: an exact switch
    // with a fall-through arm (`b - default`) cannot be expressed as structured
    // control flow, so it is kept OPAQUE — a single `Statement::Switch` in the
    // block (the Rust analogue of Python's IRSwitch staying put). STRUCTURAL.
    let module = cfg("switch $x {a {set y 1} b - default {set y 0}}");
    let func = top(&module);
    let switch_count = func
        .blocks
        .values()
        .flat_map(|b| b.statements.iter())
        .filter(|s| matches!(s, Statement::Switch { .. }))
        .count();
    assert_eq!(switch_count, 1, "fall-through exact switch stays one opaque Statement::Switch");
}

#[test]
fn for_creates_loop_cfg() {
    // Python TestCFG::test_for_creates_loop_cfg: a `for` lowers to a loop with a
    // `for_header` block that branches on the loop condition, true/false targets
    // both present, and the false (exit) target recorded as a loop node.
    //
    // CFG-SHAPE DIVERGENCE (analysis rotation): the faithful/analysis `build_cfg`
    // rotates this provably-once loop to bottom-tested form. The `for_header`
    // still ends in a Branch, but on a synthetic always-true `1` guard; the REAL
    // `$i < 3` condition moves to the latch (`for_step`). So the port asserts:
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
    assert!(func.blocks.contains_key(&cond_branch.0), "true target is a real block");
    assert!(func.blocks.contains_key(&cond_branch.1), "false target is a real block");

    // (d) The loop's exit block (`for_end`) is recorded as a loop node — Python
    // `term.false_target in cfg.loop_nodes`. `loop_nodes` is keyed by the loop's
    // EXIT block id, so assert there is one and it names a `for_end` block.
    assert!(!func.loop_nodes.is_empty(), "the `for` must register a loop node");
    assert!(
        func.loop_nodes
            .keys()
            .any(|id| func.block_name(*id).starts_with("for_end")),
        "the loop node is keyed by the `for_end` exit block"
    );
}

#[test]
fn return_terminates_block() {
    // Python TestCFG::test_return_terminates_block: in a proc whose body is
    // `set x 1; return $x; set y 2`, some block ends in a Return, and the
    // dead `set y 2` after the return is NOT reachable in the CFG.
    //
    // REPRESENTATION DIVERGENCE (NOT a bug). Python's builder *drops* the
    // dead-after-return `set y 2` entirely, so its test checks "no block holds
    // `set y 2`". Rust's faithful builder instead *isolates* it in an explicitly
    // `unreachable_*` block — the entry block ends in `Return` right after
    // `set x 1`, and a separate, non-entry-reachable `unreachable_2` block holds
    // `set y 2` (retaining the source for diagnostics/LSP; downstream analyses
    // filter on reachability). Both encode the SAME control-flow fact: `set y 2`
    // never executes. The port asserts the live-path form: (a) a block ends in
    // Return, and (b) `set y 2` is in NO reachable block. (Verified empirically:
    // entry_1 `set x 1`→Return; unreachable_2 `set y 2` is not reachable.)
    //
    // tclsh PROVES the dead-code fact the CFG encodes: `proc foo {} { set x 1;
    // return $x; set y 2 }; foo; puts [info exists ::y]` ⇒ `0` (8.6 and 9.0) —
    // `set y 2` never runs.
    let module = cfg("proc foo {} {\n    set x 1\n    return $x\n    set y 2\n}\n");
    let func = proc(&module, "::foo");
    assert!(
        terminators(func).iter().any(|t| matches!(t, Terminator::Return { .. })),
        "the proc must have a Return terminator"
    );
    // `set y 2` is unreachable (the live-path equivalent of Python's "not in the CFG").
    assert!(
        !assigns_const_reachable(func, "y"),
        "dead `set y 2` after return must not be on any reachable path"
    );
    // And it IS retained (in an unreachable block) — pin the Rust representation
    // so a future change that silently drops or revives it is caught.
    assert!(
        assigns_const(func, "y"),
        "Rust retains the dead store in an unreachable block (not dropped)"
    );
    assert!(
        func.blocks.values().any(|b| b.name.starts_with("unreachable")),
        "the dead tail lives in an `unreachable_*` block"
    );
}

// ===========================================================================
// test_cfg_layout.py — shared CFG edge-routing (lane) model
// ===========================================================================

/// Do two closed integer intervals overlap (touching endpoints count as
/// overlap)? Mirrors the Python `_spans_overlap` helper.
fn spans_overlap(a: (usize, usize), b: (usize, usize)) -> bool {
    let (lo1, hi1) = if a.0 <= a.1 { (a.0, a.1) } else { (a.1, a.0) };
    let (lo2, hi2) = if b.0 <= b.1 { (b.0, b.1) } else { (b.1, b.0) };
    !(hi1 < lo2 || lo1 > hi2)
}

// The branchy fixture from test_cfg_layout.py (`_BRANCHY`): a proc with a
// for-loop whose body is an if/else — the loop header is a conditional branch,
// so the routed edges include a true + false pair out of `for_header`.
const BRANCHY: &str = "proc f {x} {\n    set total 0\n    for {set i 0} {$i < $x} {incr i} {\n        if {$i % 2 == 0} { incr total $i } else { incr total 1 }\n    }\n    return $total\n}\n";

// -- TestAssignLanes (the routing contract) --

#[test]
fn disjoint_spans_share_lane_zero() {
    // Python test_disjoint_spans_share_lane_zero: sequential non-overlapping
    // spans all nest into the innermost lane. STRUCTURAL (pure algorithm).
    assert_eq!(assign_lanes(&[(0, 1), (2, 3), (4, 5)]), vec![0, 0, 0]);
}

#[test]
fn touching_endpoints_force_distinct_lanes() {
    // Python test_touching_endpoints_force_distinct_lanes: an edge into block 2
    // and an edge out of block 2 must not share a lane (closed intervals).
    assert_eq!(assign_lanes(&[(0, 2), (2, 4)]), vec![0, 1]);
}

#[test]
fn shortest_span_gets_innermost_lane() {
    // Python test_shortest_span_gets_innermost_lane: the longest span is
    // processed last → outer lane, so the shorter span gets the inner lane.
    let lanes = assign_lanes(&[(0, 5), (1, 2)]);
    assert!(lanes[1] < lanes[0], "shortest span (index 1) must take the inner lane: {lanes:?}");
}

#[test]
fn empty_spans_assign_no_lanes() {
    // Empty input → no lanes (the degenerate case the Python property test's
    // `randint(1, 8)` never hits but the contract still requires).
    assert_eq!(assign_lanes(&[]), Vec::<usize>::new());
}

#[test]
fn no_two_same_lane_edges_overlap() {
    // Python test_no_two_same_lane_edges_overlap: a randomised property check —
    // for 500 random span sets, no two spans sharing a lane overlap. Uses a
    // DETERMINISTIC PRNG seeded by a constant (no time/thread entropy) so the
    // run is reproducible. A small xorshift RNG stands in for Python's
    // `random.Random(7)`; the contract is invariant to the exact sequence.
    let mut rng = XorShift::new(0x9E37_79B9_7F4A_7C15);
    for _ in 0..500 {
        let n = 1 + (rng.next_u32() % 8) as usize; // 1..=8 spans (Python randint(1,8))
        let spans: Vec<(usize, usize)> = (0..n)
            .map(|_| ((rng.next_u32() % 10) as usize, (rng.next_u32() % 10) as usize))
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

// -- TestBuildCfgEdges --

#[test]
fn branch_kinds_and_lanes() {
    // Python TestBuildCfgEdges::test_branch_kinds_and_lanes: the loop header is a
    // conditional branch, so its outgoing edges include one True and one False
    // edge; every edge is routed (a lane is assigned) and no two same-lane edges
    // overlap. (The Rust `build_cfg_edges` takes an explicit block order — the
    // canonical creation order from `ordered_block_names` — where Python's reads
    // it off `snap.cfg` directly.) STRUCTURAL.
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
    // ≥1 False edge. Mirrors the kind contract Python's serialised-edge test
    // (`kind` ∈ {goto,true,false}) pins. STRUCTURAL.
    let lin = cfg("set a 1\nset b 2");
    let lfunc = top(&lin);
    let lorder = ordered_block_names(lfunc);
    let ledges = build_cfg_edges(lfunc, &lorder);
    assert!(!ledges.is_empty(), "a linear script still has a fall-through Goto edge");
    assert!(
        ledges.iter().all(|e| e.kind == EdgeKind::Goto),
        "a branch-free script has only Goto edges: {:?}",
        ledges.iter().map(|e| e.kind).collect::<Vec<_>>()
    );

    let module = cfg(BRANCHY);
    let func = proc(&module, "::f");
    let order = ordered_block_names(func);
    let edges = build_cfg_edges(func, &order);
    assert!(edges.iter().any(|e| e.kind == EdgeKind::True), "branchy fn has a True edge");
    assert!(edges.iter().any(|e| e.kind == EdgeKind::False), "branchy fn has a False edge");
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
    assert_eq!(order.len(), func.blocks.len(), "every block appears once in the order");
    assert_eq!(
        order.iter().cloned().collect::<std::collections::HashSet<_>>(),
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

// ===========================================================================
// Codegen-build shape (un-rotated, header-tested loop)
//
// The analysis `for_creates_loop_cfg` above asserts the ROTATED shape. The
// codegen builder (`build_cfg_codegen`, faithful_exceptions OFF) leaves the
// loop header-tested — exactly Python's shape — so the `$i < 3` condition stays
// on the `for_header` block. Pinning it here keeps both shapes covered and
// documents the rotation as analysis-only.
// ===========================================================================

#[test]
fn codegen_for_loop_is_header_tested() {
    let module =
        build_cfg_codegen(&lower_to_ir("for {set i 0} {$i < 3} {incr i} {set sum 1}", registry()), false);
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

// ---------------------------------------------------------------------------
// Deterministic PRNG for the no-collision property test (no time/Math.random).
// A 64-bit xorshift* — fixed seed in, identical stream out on every run.
// ---------------------------------------------------------------------------

struct XorShift(u64);

impl XorShift {
    fn new(seed: u64) -> Self {
        // Avoid the all-zero fixed point.
        Self(if seed == 0 { 0xDEAD_BEEF_CAFE_F00D } else { seed })
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
