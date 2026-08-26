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

//! Liveness-based slot coalescing (Phase 5 core), the standalone analysis
//! surface in [`tcl_compiler::slot_allocation`].
//!
//! This exercises the slot-allocation **module in isolation**
//! (`build_interference`, `coalesce_slots`, `slot_count` over a CFG/SSA plus
//! name-level liveness) — it is deliberately NOT a codegen-integration test.
//! The colouring is an analysis only and is never wired into the bytecode
//! emitter (coalescing LVT slots is unsound for Tcl's name-addressable locals;
//! see the module docs), so there is no tclsh reference to compare against.
//!
//! ## Structural-assertion note
//!
//! Every fact asserted here is compiler-internal structure with no direct Tcl
//! analogue: the interference graph is an undirected adjacency over *variable
//! names*, and a slot index is the colour a greedy graph-colouring assigns. "Do
//! these two locals' live ranges overlap?" and "how few slots cover them?" are
//! properties of the CFG/SSA dataflow, not of any observable `tclsh` behaviour
//! — so they are asserted directly against the analysis result. This note
//! discharges the structural-assertion requirement for the whole file.

use std::collections::HashMap;

use tcl_compiler::cfg::{BlockId, Function};
use tcl_compiler::cfg_builder::build_cfg_function;
use tcl_compiler::lowering::lower_to_ir;
use tcl_compiler::slot_allocation::{
    Interference, SlotMapping, build_interference, coalesce_slots, live_out_by_name, slot_count,
};
use tcl_compiler::ssa::{SsaFunction, build_ssa};
use tcl_registry::CommandRegistry;
use tcl_registry::model::ingress::static_context_for;

const TCL: &str = "tcl8.6";

fn registry() -> &'static CommandRegistry {
    static_context_for(TCL).commands()
}

/// Lower `src`, build the CFG for the
/// named proc and its SSA, then compute name-level live-out. Returns the pieces
/// the interference / coalescing functions consume.
///
/// `proc` is the **qualified** name (e.g. `"::g"`), matching the keys of
/// `Module.procedures`.
fn setup(
    src: &str,
    proc: &str,
) -> (
    Function,
    SsaFunction,
    HashMap<BlockId, std::collections::HashSet<String>>,
) {
    let reg = registry();
    let module = lower_to_ir(src, reg);
    let procedure = module.procedures.get(proc).unwrap_or_else(|| {
        panic!(
            "proc {proc} not found; keys = {:?}",
            module.procedures.keys().collect::<Vec<_>>()
        )
    });
    // `false` = do not inline loops, matching the analysis CFG (build_cfg);
    // coalescing depends on liveness only.
    let cfg = build_cfg_function(proc, &procedure.body, false);
    let ssa = build_ssa(&cfg, reg);
    let live_out = live_out_by_name(&cfg, &ssa, reg);
    (cfg, ssa, live_out)
}

fn interference(src: &str, proc: &str) -> Interference {
    let (cfg, ssa, live_out) = setup(src, proc);
    build_interference(&cfg, &ssa, &live_out, registry())
}

fn coalesce(src: &str, proc: &str, params: &[&str]) -> SlotMapping {
    let (cfg, ssa, live_out) = setup(src, proc);
    let params: Vec<String> = params.iter().map(|s| (*s).to_owned()).collect();
    coalesce_slots(&cfg, &ssa, &live_out, &params, registry())
}

/// Whether `a` and `b` interfere in `graph` (symmetric adjacency).
fn interferes(graph: &Interference, a: &str, b: &str) -> bool {
    graph.get(a).is_some_and(|s| s.contains(b))
}

// ===========================================================================
// Interference
// ===========================================================================

#[test]
fn overlapping_locals_interfere() {
    // proc g {} { set a 1; set b 2; puts $a; puts $b }
    // a and b are both live between their defs and uses -> interfere.
    let graph = interference("proc g {} { set a 1\n set b 2\n puts $a\n puts $b }", "::g");
    assert!(interferes(&graph, "a", "b"), "b in graph[a]: {graph:?}");
    assert!(interferes(&graph, "b", "a"), "a in graph[b]: {graph:?}");
}

#[test]
fn disjoint_locals_do_not_interfere() {
    // proc f {} { set a 1; puts $a; set b 2; puts $b }
    // a is dead before b is defined -> they do not interfere.
    let graph = interference("proc f {} { set a 1\n puts $a\n set b 2\n puts $b }", "::f");
    assert!(
        !interferes(&graph, "a", "b"),
        "b must NOT be in graph[a]: {graph:?}"
    );
}

// ===========================================================================
// Coalescing
// ===========================================================================

#[test]
fn disjoint_ranges_share_one_slot() {
    // proc f {} { set a 1; puts $a; set b 2; puts $b; set c 3; puts $c }
    // Three sequentially-used locals collapse to a single reused slot.
    let mapping = coalesce(
        "proc f {} { set a 1\n puts $a\n set b 2\n puts $b\n set c 3\n puts $c }",
        "::f",
        &[],
    );
    assert_eq!(
        slot_count(&mapping),
        1,
        "three disjoint locals -> 1 slot: {mapping:?}"
    );
}

#[test]
fn overlapping_ranges_need_distinct_slots() {
    // proc g {} { set a 1; set b 2; puts $a; puts $b }
    // Two overlapping locals need two distinct slots.
    let mapping = coalesce(
        "proc g {} { set a 1\n set b 2\n puts $a\n puts $b }",
        "::g",
        &[],
    );
    assert_ne!(
        mapping["a"], mapping["b"],
        "overlapping locals get distinct slots: {mapping:?}"
    );
    assert_eq!(
        slot_count(&mapping),
        2,
        "two overlapping locals -> 2 slots: {mapping:?}"
    );
}

#[test]
fn params_get_stable_low_slots_and_never_share() {
    // proc h {x y} { return [expr {$x + $y}] }
    // Parameters are live on entry, so they mutually interfere and keep
    // distinct low slots in incoming order: x -> 0, y -> 1.
    let mapping = coalesce(
        "proc h {x y} { return [expr {$x + $y}] }",
        "::h",
        &["x", "y"],
    );
    assert_eq!(mapping["x"], 0, "x gets slot 0: {mapping:?}");
    assert_eq!(mapping["y"], 1, "y gets slot 1: {mapping:?}");
    // Live on entry -> they interfere -> never share.
    assert_ne!(
        mapping["x"], mapping["y"],
        "params never share a slot: {mapping:?}"
    );
}
