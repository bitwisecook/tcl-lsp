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

//! Liveness-based interference colouring over proc locals (ANALYSIS ONLY).
//!
//! Computes, from a CFG/SSA function and its name-level liveness, the classic
//! **interference-graph colouring**: two locals interfere iff both are live at
//! the same program point; a greedy colouring then assigns the fewest "slots"
//! such that no two interfering variables collide.
//!
//! IMPORTANT — this is **not** wired into any emitter, and must not be. It is a
//! self-contained analysis used for register-pressure reporting / a
//! compiler-explorer overlay, where the *result* is informational and never
//! rewrites slots. The reasons it is unsound to actually coalesce LVT slots in
//! the bytecode emitter:
//!
//! * The bytecode path's LVT slot is a *name-table index* — the VM resolves it
//!   to the variable **name** and accesses the frame by name. Coalescing two
//!   variables onto one slot would alias them to the same name.
//! * Tcl compiled locals are name-addressable for the proc's whole lifetime
//!   (`upvar` / `info locals` / `trace` / `uplevel`), so dataflow-"dead" is not
//!   "inaccessible" — liveness-based slot reuse is semantically invalid for Tcl
//!   locals (and tclsh never reuses LVT slots). An earlier opt-in wiring was
//!   reverted after the differential fuzzer caught the aliasing corruption.
//!
//! Parameters are pinned to their incoming order (slots `0..n-1`); being live
//! on entry they mutually interfere and never share.
//!
//! ## Liveness representation
//!
//! Liveness is computed directly **by variable name** ([`live_out_by_name`])
//! because slots are per-name (rather than SSA-version-keyed and then collapsed
//! to names). The interference graph, the instruction-granular backward clique
//! walk, the phi handling, and the greedy colouring build on that.

use std::collections::{HashMap, HashSet};

use tcl_registry::CommandRegistry;

use crate::cfg::{self, BlockId, Terminator};
use crate::ssa::{SsaFunction, SsaStatement};
use crate::var_refs::{VarReferenceScanner, VarScanOptions, vars_in_expr};

/// An undirected interference graph over variable **names**: each name maps to
/// the set of names whose live range overlaps its own.
pub type Interference = HashMap<String, HashSet<String>>;

/// A coalescing: each variable name mapped to its assigned slot index.
pub type SlotMapping = HashMap<String, usize>;

/// Build a fresh [`VarReferenceScanner`] configured exactly like the SSA
/// builder's — variable-read roles included, command substitutions recursed,
/// reads-before-write excluded — so terminator-condition reads recovered here
/// match the names SSA recorded in statement `uses`.
fn make_scanner() -> VarReferenceScanner {
    VarReferenceScanner::new(VarScanOptions {
        include_var_read_roles: true,
        recurse_cmd_substitutions: true,
        include_reads_before_write: false,
        element_qualified: false,
    })
}

/// Names read by a block's terminator (branch condition / return value).
///
/// These reads occur *after* every statement in the block, so they must be in
/// the live set when the backward walk starts — otherwise a value used only in a
/// `return` / `if` (e.g. a parameter passed straight to `return [expr {$x +
/// $y}]`) would never appear live and would wrongly coalesce.
fn terminator_read_names(
    block: &cfg::Block,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> HashSet<String> {
    let mut names: HashSet<String> = HashSet::new();
    match &block.terminator {
        Some(Terminator::Branch { condition, .. }) => {
            names.extend(vars_in_expr(condition));
        }
        Some(Terminator::Return { value, expr, .. }) => {
            if let Some(v) = value {
                names.extend(scanner.scan_word(v, registry));
            }
            if let Some(e) = expr {
                names.extend(vars_in_expr(e));
            }
        }
        _ => {}
    }
    names
}

/// The set of variable **names** an SSA statement reads / writes.
///
/// Resolves the interned [`crate::ssa::Symbol`] keys of the statement's `uses`
/// / `defs` maps back to display names via [`SsaFunction::var_name`]. Slots are
/// per-name, so the version component of each key is irrelevant here.
fn stmt_use_names(stmt: &SsaStatement, ssa: &SsaFunction) -> HashSet<String> {
    stmt.uses
        .keys()
        .map(|sym| ssa.var_name(*sym).to_owned())
        .collect()
}

fn stmt_def_names(stmt: &SsaStatement, ssa: &SsaFunction) -> HashSet<String> {
    stmt.defs
        .keys()
        .map(|sym| ssa.var_name(*sym).to_owned())
        .collect()
}

/// Per-block (use, def) name sets — the seed for the backward liveness
/// dataflow, keyed by **variable name** (not SSA version).
///
/// A name is in `use[b]` if some statement (or the branch terminator) reads it
/// before any in-block statement (re)defines it — an *upward-exposed* use, the
/// only kind a predecessor's value can reach. `def[b]` is every name written in
/// the block (phi targets included).
fn block_use_def(
    cfg: &cfg::Function,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
) -> (
    HashMap<BlockId, HashSet<String>>,
    HashMap<BlockId, HashSet<String>>,
) {
    let mut use_map: HashMap<BlockId, HashSet<String>> = HashMap::new();
    let mut def_map: HashMap<BlockId, HashSet<String>> = HashMap::new();
    for id in cfg.blocks.keys() {
        use_map.insert(*id, HashSet::new());
        def_map.insert(*id, HashSet::new());
    }

    let mut scanner = make_scanner();

    for (id, sblock) in &ssa.blocks {
        let uses = use_map.get_mut(id).expect("block in use map");
        let defs = def_map.get_mut(id).expect("block in def map");
        let mut seen_defs: HashSet<String> = HashSet::new();

        // Phi targets are defined at the block head.
        for phi in &sblock.phis {
            let name = ssa.var_name(phi.name).to_owned();
            defs.insert(name.clone());
            seen_defs.insert(name);
        }

        for stmt in &sblock.statements {
            for name in stmt_use_names(stmt, ssa) {
                if !seen_defs.contains(&name) {
                    uses.insert(name);
                }
            }
            for name in stmt_def_names(stmt, ssa) {
                defs.insert(name.clone());
                seen_defs.insert(name);
            }
        }

        // The branch condition reads after every statement; an upward-exposed
        // condition read (not defined in this block) is a block use.
        if let Some(block) = cfg.blocks.get(id) {
            let term_names = terminator_read_names(block, &mut scanner, registry);
            for name in term_names {
                if !seen_defs.contains(&name) {
                    uses.insert(name);
                }
            }
        }
    }

    (use_map, def_map)
}

/// Compute per-block **live-out** sets keyed by variable **name**, via the
/// standard backward dataflow fixpoint.
///
/// `live_out[b] = ⋃ live_in[s]` over successors `s`; `live_in[b] = use[b] ∪
/// (live_out[b] − def[b])`. Because slots are per-name, this works directly on
/// names rather than SSA `(name, version)` keys — phi renaming across an edge
/// is a no-op once versions are dropped, so the per-name result equals the
/// name-collapse of an SSA-version-keyed `live_out`.
///
/// Iterates a worklist in reverse-reverse-postorder (i.e. exit-first) until the
/// fixpoint settles; on `live_in[b]` change, only `b`'s predecessors are
/// re-enqueued.
#[must_use]
pub fn live_out_by_name(
    cfg: &cfg::Function,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
) -> HashMap<BlockId, HashSet<String>> {
    let (use_map, def_map) = block_use_def(cfg, ssa, registry);

    let mut live_in: HashMap<BlockId, HashSet<String>> = HashMap::new();
    let mut live_out: HashMap<BlockId, HashSet<String>> = HashMap::new();
    for id in cfg.blocks.keys() {
        live_in.insert(*id, HashSet::new());
        live_out.insert(*id, HashSet::new());
    }

    let preds = cfg.predecessors();

    // Exit-first traversal order (reverse of reverse-postorder).
    let mut order = cfg.reverse_postorder();
    order.reverse();

    let mut worklist: Vec<BlockId> = order.clone();
    let mut queued: HashSet<BlockId> = worklist.iter().copied().collect();

    while let Some(bn) = worklist.pop() {
        queued.remove(&bn);

        let succs: Vec<BlockId> = cfg.block_successors(bn);
        let mut out: HashSet<String> = HashSet::new();
        for succ in &succs {
            if let Some(s_in) = live_in.get(succ) {
                out.extend(s_in.iter().cloned());
            }
        }

        let empty = HashSet::new();
        let use_b = use_map.get(&bn).unwrap_or(&empty);
        let def_b = def_map.get(&bn).unwrap_or(&empty);
        let mut new_in: HashSet<String> = use_b.clone();
        for name in &out {
            if !def_b.contains(name) {
                new_in.insert(name.clone());
            }
        }

        live_out.insert(bn, out);
        if live_in.get(&bn) != Some(&new_in) {
            live_in.insert(bn, new_in);
            if let Some(ps) = preds.get(&bn) {
                for p in ps {
                    if queued.insert(*p) {
                        worklist.push(*p);
                    }
                }
            }
        }
    }

    live_out
}

/// Add the clique among the currently-live names to the interference graph:
/// every pair of distinct live names is made to interfere, and each live name
/// is ensured present (so a lone live name still appears as a node).
fn add_clique(graph: &mut Interference, live: &HashSet<String>) {
    for n in live {
        graph.entry(n.clone()).or_default();
    }
    for a in live {
        for b in live {
            if a != b {
                graph.get_mut(a).expect("node present").insert(b.clone());
            }
        }
    }
}

/// Build the undirected interference graph over variable **names**, at
/// instruction granularity.
///
/// For each block we seed the live set from its `live_out` plus the
/// terminator's own reads, add that as an interference clique, then walk
/// the block's statements
/// **backwards** — recording the simultaneously-live names at every point as a
/// clique and updating `live = (live − defs) ∪ uses`. Phi targets defined at
/// the block head are folded in last. Instruction granularity is what lets two
/// straight-line locals with disjoint ranges (`set a …; use a; set b …; use b`)
/// share a slot — block-granular liveness would miss that entirely.
///
/// `live_out` must be the per-block name-level live-out from
/// [`live_out_by_name`] for the same `cfg` / `ssa`.
#[must_use]
pub fn build_interference<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    cfg: &cfg::Function,
    ssa: &SsaFunction,
    live_out: &HashMap<BlockId, HashSet<String, S2>, S1>,
    registry: &CommandRegistry,
) -> Interference {
    let mut graph: Interference = HashMap::new();
    let mut scanner = make_scanner();

    for (id, sblock) in &ssa.blocks {
        // Collect into a fresh default-hashed set so the inner set's hasher
        // (`S2`) does not propagate into the working `live` set / `add_clique`.
        let mut live: HashSet<String> = live_out
            .get(id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        if let Some(block) = cfg.blocks.get(id) {
            live.extend(terminator_read_names(block, &mut scanner, registry));
        }
        add_clique(&mut graph, &live);

        for stmt in sblock.statements.iter().rev() {
            let defs = stmt_def_names(stmt, ssa);
            let uses = stmt_use_names(stmt, ssa);
            for d in &defs {
                live.remove(d);
            }
            live.extend(uses);
            add_clique(&mut graph, &live);
        }

        // Phi targets are defined at the block head; they are live across the
        // incoming edges and must not collide with names live there.
        for phi in &sblock.phis {
            let name = ssa.var_name(phi.name).to_owned();
            graph.entry(name.clone()).or_default();
            live.remove(&name);
            let mut with_phi = live.clone();
            with_phi.insert(name);
            add_clique(&mut graph, &with_phi);
        }
    }

    graph
}

/// Assign each variable name a slot index, reusing slots between names whose
/// live ranges do not interfere (greedy interference-graph colouring).
///
/// `params` are coloured first, in order, so they occupy slots `0..n-1` (an
/// emitter could keep incoming-argument slots stable); since parameters are
/// live on entry they
/// mutually interfere and thus never share a slot. The remaining names are
/// coloured by descending interference degree (most-constrained first — the
/// usual greedy heuristic), each taking the lowest slot not used by an
/// already-coloured interfering neighbour.
///
/// The result maps every name in the interference graph (plus any param) to a
/// slot; [`slot_count`] of it is the coalesced slot count.
#[must_use]
pub fn coalesce_slots<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    cfg: &cfg::Function,
    ssa: &SsaFunction,
    live_out: &HashMap<BlockId, HashSet<String, S2>, S1>,
    params: &[String],
    registry: &CommandRegistry,
) -> SlotMapping {
    // Colour `name` with the lowest slot not taken by a coloured neighbour.
    fn assign(name: &str, graph: &Interference, colour: &mut SlotMapping) {
        if colour.contains_key(name) {
            return;
        }
        let used: HashSet<usize> = graph
            .get(name)
            .into_iter()
            .flatten()
            .filter_map(|nb| colour.get(nb).copied())
            .collect();
        let mut slot = 0usize;
        while used.contains(&slot) {
            slot += 1;
        }
        colour.insert(name.to_owned(), slot);
    }

    let mut graph = build_interference(cfg, ssa, live_out, registry);
    // A parameter never read in the body still needs a slot of its own.
    for p in params {
        graph.entry(p.clone()).or_default();
    }

    let mut colour: SlotMapping = HashMap::new();

    // Parameters first (stable low slots, in incoming order).
    for p in params {
        assign(p, &graph, &mut colour);
    }

    // Then the rest, most-constrained first: descending degree, ties broken by
    // name for determinism (`key=(-deg, name)`).
    let mut rest: Vec<&String> = graph.keys().collect();
    rest.sort_by(|a, b| {
        let da = graph.get(*a).map_or(0, HashSet::len);
        let db = graph.get(*b).map_or(0, HashSet::len);
        db.cmp(&da).then_with(|| a.cmp(b))
    });
    for name in rest {
        assign(name, &graph, &mut colour);
    }

    colour
}

/// Number of distinct slots a coalescing uses (0 for an empty mapping).
///
/// `max(slot) + 1`, or 0 if the mapping is empty.
#[must_use]
pub fn slot_count(mapping: &SlotMapping) -> usize {
    mapping.values().copied().max().map_or(0, |m| m + 1)
}
