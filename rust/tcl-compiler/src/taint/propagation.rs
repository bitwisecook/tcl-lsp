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

//! Intra-procedural taint propagation over one SSA function: entry seeds, phi joins and the per-statement transfer.

use super::definition_taint::{colour_from_rendered, evaluate_taint_def_at};
use super::registry_colours::reg_colour;
use super::source_context::{TaintPropagationInputs, TaintSourceContext};
use super::{TaintCtx, TaintLattice};
use crate::cfg::{BlockId, Function as CfgFunction};
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::Statement;
use crate::rendered_properties::RenderedValueProps;
use crate::sccp::{SccpResult, cfg_order};
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

/// True when any procedure reachable from the current function (the
/// caller named `ssa.name`) has `writes_global = true`.
///
/// For a known proc, uses its summary's transitive `calls` list. For
/// `::top` or an unknown caller, enumerates direct callees from the
/// function's IR via `Statement::Call` commands that resolve to an
/// internal proc, then unions their transitive closures.
fn reachable_writes_global(
    _ssa: &SsaFunction,
    cfg: &CfgFunction,
    ia: &InterproceduralAnalysis,
    source: TaintSourceContext<'_>,
    registry: &CommandRegistry,
) -> bool {
    if cfg.has_opaque_native_accesses() {
        return true;
    }
    if let Some(summary) = source.body_id().and_then(|id| ia.procedures.get(&id)) {
        return summary.writes_global
            || summary
                .calls
                .iter()
                .any(|id| ia.procedures.get(id).is_some_and(|row| row.writes_global));
    }
    for statement in cfg.blocks.values().flat_map(|block| &block.statements) {
        let Statement::Call {
            command, tokens, ..
        } = statement
        else {
            continue;
        };
        let Some(ids) = tokens
            .as_ref()
            .and_then(|tokens| source.procedure_ids(tokens, command, registry))
        else {
            if ia.procedures.values().any(|row| row.writes_global) {
                return true;
            }
            continue;
        };
        if ids.iter().any(|id| {
            ia.procedures.get(id).is_some_and(|row| {
                row.writes_global
                    || row.calls.iter().any(|callee| {
                        ia.procedures
                            .get(callee)
                            .is_some_and(|row| row.writes_global)
                    })
            })
        }) {
            return true;
        }
    }
    false
}

/// Collect version-zero reads whose SSA cells have an actual namespace owner.
/// The display name is not a storage-domain discriminator.
pub(super) fn collect_global_reads(ssa: &SsaFunction) -> FxHashSet<Symbol> {
    let mut out: FxHashSet<Symbol> = FxHashSet::default();
    let mut consider = |symbol: Symbol| {
        if ssa.cell_key(symbol).is_some_and(|key| {
            matches!(
                key.root(),
                crate::var_resolve::VariableCellKey::Namespace { .. }
            )
        }) {
            out.insert(symbol);
        }
    };
    for block in ssa.blocks.values() {
        for &symbol in block.entry_versions.keys() {
            consider(symbol);
        }
        for phi in &block.phis {
            if phi.incoming.values().any(|&version| version == 0) {
                consider(phi.name);
            }
        }
        for statement in &block.statements {
            for (&symbol, &version) in &statement.uses {
                if version == 0 {
                    consider(symbol);
                }
            }
        }
    }
    out
}

/// One function's CFG/SSA analyses plus the CFG-derived indices
/// [`propagate_taints`] walks, built once and reused across every
/// propagation over that function.
///
/// Both indices are pure functions of the CFG, so rebuilding them per
/// propagation is wasted work — and it was not a small amount of it:
/// [`crate::taint_interproc::infer_proc_summary`] runs one propagation for the
/// baseline plus one per (parameter, basis) scenario, and the summary
/// worklist re-infers a procedure whenever a callee's summary moves, so an
/// O(V+E) predecessor map and an O(V+E) reverse-postorder were being rebuilt
/// dozens of times per function without this cache.
pub(crate) struct TaintGraph<'a> {
    /// The function's CFG.
    pub cfg: &'a CfgFunction,
    /// Its SSA form.
    pub ssa: &'a SsaFunction,
    /// The SCCP result gating block/edge executability.
    pub sccp: &'a SccpResult,
    /// Predecessor map for the phi joins, `rustc-hash`-keyed.
    preds: FxHashMap<BlockId, FxHashSet<BlockId>>,
    /// Reverse-postorder block visit order for the fixpoint sweep.
    order: Vec<BlockId>,
}

impl<'a> TaintGraph<'a> {
    /// Build the per-function indices once.
    pub fn new(cfg: &'a CfgFunction, ssa: &'a SsaFunction, sccp: &'a SccpResult) -> Self {
        Self {
            cfg,
            ssa,
            sccp,
            preds: cfg.predecessors_fx(),
            order: cfg_order(cfg),
        }
    }
}

/// Run intra-procedural taint propagation over one SSA function.
///
/// Returns a map from `(variable_name, ssa_version)` to its taint
/// lattice value. Entries absent from the map are implicitly clean.
///
/// Sources are identified by [`is_taint_source`]. Propagation follows
/// SSA phi-join semantics with SCCP edge-level reachability: a phi
/// predecessor only contributes if its incoming CFG edge is executable,
/// preventing taint from propagating through SCCP-proven dead branches.
///
/// Optional inputs:
/// * `rendered_props` — when present, values are coloured using
///   rendered-string properties (`PATH_PREFIXED`, `NON_DASH_PREFIXED`,
///   `CRLF_FREE`).
/// * `interproc` — when present, calls to known user procs apply the
///   proc summary's passthrough-parameter rule, and procs marked
///   `writes_global` taint the version-0 entry of every global in the
///   map (conservative).
/// * `dialect` — when `"f5-irules"` / `"irules"`, iRules-specific
///   namespace-prefixed commands (`HTTP::`, `URI::`, …) are treated
///   as taint sources.
/// * `param_taints` — entry taints seeded by the interprocedural
///   solve: each tainted entry seeds the version-0 slot of the named
///   parameter.
/// * `taint_summaries` — colour-aware return summaries from the
///   interprocedural solve; when present, internal calls apply the
///   full return-summary transfer (`apply_proc_return_summary`)
///   instead of the conservative single-passthrough rule.
/// * `instance_classes` — precomputed function-local receiver facts. The
///   summary solver reuses this across its clean run and every parameter ×
///   colour scenario instead of repeating the same receiver CFG fixpoint.
#[must_use]
pub(crate) fn propagate_taints(
    graph: &TaintGraph<'_>,
    inputs: TaintPropagationInputs<'_>,
) -> HashMap<ValueKey, TaintLattice> {
    let TaintPropagationInputs {
        registry,
        rendered_props,
        interproc,
        source,
        param_taints,
        taint_summaries,
        instance_classes,
    } = inputs;
    let dialect = source.dialect;
    let TaintGraph {
        cfg,
        ssa,
        sccp,
        preds,
        order,
    } = graph;

    // Precompute the set of known procedure names once so per-call
    // resolution in `interproc_call_taint` is O(1) rather than
    // O(procedures) per call site. The solve's summaries take
    // precedence (their key set).
    let known_procs: Option<HashSet<crate::ir::ProcedureId>> = match (taint_summaries, interproc) {
        (Some(s), _) => Some(s.keys().copied().collect()),
        (None, Some(ia)) => Some(ia.procedures.keys().copied().collect()),
        (None, None) => None,
    };
    let ctx = TaintCtx {
        registry,
        ssa,
        interproc,
        known_procs: known_procs.as_ref(),
        dialect,
        source,
        taint_summaries,
        instance_classes: Some(instance_classes),
        source_position: None,
        source_tokens: None,
        word_offset: None,
        source_word: None,
    };

    let mut taints: HashMap<ValueKey, TaintLattice> = HashMap::new();
    seed_entry_taints(
        &mut taints,
        ssa,
        cfg,
        interproc,
        param_taints,
        registry,
        source,
    );
    seed_original_script_unknown_taints(&mut taints, ssa);

    let mut changed = true;
    while changed {
        changed = false;
        for bn in order {
            if !sccp.executable_blocks.contains(bn) {
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };
            changed |= propagate_phi_taints(&mut taints, ssa_block, *bn, preds, sccp);
            changed |=
                propagate_statement_taints(&mut taints, *bn, ssa_block, ctx, ssa, rendered_props);
            for versions in ssa
                .value_clobbers
                .get(bn)
                .into_iter()
                .flat_map(|markers| markers.values())
            {
                for (&symbol, &(prior, fresh)) in versions {
                    if let Some(&old) = taints.get(&(symbol, prior)) {
                        let key = (symbol, fresh);
                        let joined = taints.get(&key).map_or(old, |&current| current.join(old));
                        if taints.get(&key) != Some(&joined) {
                            taints.insert(key, joined);
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    if cfg.has_opaque_native_accesses() {
        widen_opaque_native_taints(&mut taints, ssa);
    }
    taints
}

/// Source names have no complete current-contents coverage. Keep every issued
/// SCRIPT version unknown for raw taint/interprocedural consumers; only the
/// separate owned literal-read advisory may use a positive source constant.
fn seed_original_script_unknown_taints(
    taints: &mut HashMap<ValueKey, TaintLattice>,
    ssa: &SsaFunction,
) {
    let symbols: HashSet<_> = ssa.original_script_source_symbols().collect();
    if symbols.is_empty() {
        return;
    }
    for &symbol in &symbols {
        taints.insert((symbol, 0), TaintLattice::tainted());
    }
    for block in ssa.blocks.values() {
        for (&symbol, &version) in block.entry_versions.iter().chain(&block.exit_versions) {
            if symbols.contains(&symbol) {
                taints.insert((symbol, version), TaintLattice::tainted());
            }
        }
        for phi in &block.phis {
            if symbols.contains(&phi.name) {
                taints.insert((phi.name, phi.version), TaintLattice::tainted());
                for &version in phi.incoming.values() {
                    taints.insert((phi.name, version), TaintLattice::tainted());
                }
            }
        }
        for statement in &block.statements {
            for (&symbol, &version) in statement.uses.iter().chain(&statement.defs) {
                if symbols.contains(&symbol) {
                    taints.insert((symbol, version), TaintLattice::tainted());
                }
            }
        }
    }
    for versions in ssa
        .value_clobbers
        .values()
        .flat_map(|markers| markers.values())
    {
        for (&symbol, &(prior, fresh)) in versions {
            if symbols.contains(&symbol) {
                taints.insert((symbol, prior), TaintLattice::tainted());
                taints.insert((symbol, fresh), TaintLattice::tainted());
            }
        }
    }
}

/// Compatibility taint maps cannot name the stores made by an opaque invocation.
/// Only already represented SSA keys are widened; no variable names are created.
fn widen_opaque_native_taints(taints: &mut HashMap<ValueKey, TaintLattice>, ssa: &SsaFunction) {
    // Named SSA versions cannot describe stores made by this invocation.
    // Retain the opaque residual even when no explicit definition exists.
    for block in ssa.blocks.values() {
        for (&symbol, &version) in block.entry_versions.iter().chain(&block.exit_versions) {
            taints.insert((symbol, version), TaintLattice::tainted());
        }
        for phi in &block.phis {
            taints.insert((phi.name, phi.version), TaintLattice::tainted());
            for &version in phi.incoming.values() {
                taints.insert((phi.name, version), TaintLattice::tainted());
            }
        }
        for statement in &block.statements {
            for (&symbol, &version) in statement.uses.iter().chain(&statement.defs) {
                taints.insert((symbol, version), TaintLattice::tainted());
            }
        }
    }
}

/// Seed the initial taint map: tainted interprocedural parameters, plus
/// version-0 global reads when a reachable callee writes globals.
pub(super) fn is_actual_formal_entry(
    ssa: &SsaFunction,
    symbol: Symbol,
    name: &str,
    source: TaintSourceContext<'_>,
) -> bool {
    let Some(key) = ssa.cell_key(symbol) else {
        return false;
    };
    if !matches!(
        key.root(),
        crate::var_resolve::VariableCellKey::Activation { .. }
    ) {
        return false;
    }
    if source.authored_body_formal(ssa, name) {
        return true;
    }
    if source.allows_nominal_metadata() {
        return true;
    }
    let (Some(module), Some(body)) = (source.module(), source.body_id()) else {
        return false;
    };
    module
        .procedures
        .get(&body)
        .is_some_and(|procedure| procedure.params.iter().any(|formal| formal == name))
}

pub(super) fn seed_entry_taints(
    taints: &mut HashMap<ValueKey, TaintLattice>,
    ssa: &SsaFunction,
    cfg: &CfgFunction,
    interproc: Option<&InterproceduralAnalysis>,
    param_taints: Option<&HashMap<String, TaintLattice>>,
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
) {
    // Seed entry taints for tainted parameters (interprocedural solve).
    // Only tainted params seed a slot; clean params leave the version-0
    // slot absent (implicitly clean).
    if let Some(pt) = param_taints {
        for (name, t) in pt {
            // A tainted param only matters if the body reads it, and any read
            // interns the name; a never-read param's seed slot would never be
            // consulted, so skipping an un-interned name is behaviour-neutral.
            if t.is_tainted()
                && let Some(sym) = ssa.var_symbol(name)
                && is_actual_formal_entry(ssa, sym, name, source)
            {
                taints.insert((sym, 0), *t);
            }
        }
    }

    // Seed: when a callee reachable from the current function writes
    // to global scope, taint version-0 reads of global/namespace
    // variables that this function actually touches. Scoping to
    // reachable callees prevents an unrelated helper proc's global
    // writes from polluting functions that never invoke it; scanning
    // *every* block's entry_versions (plus statement uses / phi
    // incomings) ensures we discover globals even when the entry
    // block has no seeded versions.
    if let Some(ia) = interproc
        && reachable_writes_global(ssa, cfg, ia, source, registry)
    {
        for symbol in collect_global_reads(ssa) {
            taints
                .entry((symbol, 0))
                .or_insert_with(TaintLattice::conservative_global_seed);
        }
    }

    // Registry-declared external inputs colour only their original physical
    // root-global cells. Display aliases and array labels cannot select them.
    let Some((entry, profile)) = source.incoming_native_global_context() else {
        // Missing physical origin contributes no seed. Authored formals and
        // unknown SCRIPT contents retain their independent transfers.
        return;
    };
    let surface = Some(profile.surface_query());
    for (symbol, cell) in ssa.cell_symbols() {
        let Some(global) = cell.incoming_root_global(entry) else {
            continue;
        };
        let Ok(name) = global.name().try_utf8() else {
            continue;
        };
        let Some(spec) = registry.special_var_in_dialect(name, surface) else {
            continue;
        };
        if global.is_element() && spec.kind != tcl_registry::special_vars::SpecialVarKind::Array {
            continue;
        }
        let Some(colour) = spec.read_taint else {
            continue;
        };
        let incoming = TaintLattice::from_colours(reg_colour(colour));
        taints
            .entry((symbol, 0))
            .and_modify(|value| *value = value.join(incoming))
            .or_insert(incoming);
    }
}

/// Join phi taints from edge-executable predecessors into `taints` for one
/// block. Returns `true` if any lattice value changed.
fn propagate_phi_taints(
    taints: &mut HashMap<ValueKey, TaintLattice>,
    ssa_block: &crate::ssa::SsaBlock,
    bn: BlockId,
    preds: &FxHashMap<BlockId, FxHashSet<BlockId>>,
    sccp: &SccpResult,
) -> bool {
    let mut changed = false;
    // Phi nodes: join taint from edge-executable predecessors only.
    // Using executable_edges (not just executable_blocks) ensures
    // taint does not flow through SCCP-proven dead branches.
    for phi in &ssa_block.phis {
        let exec_preds = preds.get(&bn).map_or_default(|ps| {
            ps.iter()
                .filter(|p| sccp.executable_edges.contains(&(**p, bn)))
                .collect::<Vec<_>>()
        });

        if exec_preds.is_empty() {
            continue;
        }

        // Use `Option` to distinguish "no predecessor seen yet"
        // from "clean with no mitigations". The join's intersect
        // semantics for mitigations needs an identity of
        // "all mitigations set" that we don't have a sentinel
        // for; instead, the first incoming value becomes the
        // accumulator directly and subsequent ones join into it.
        let mut phi_taint: Option<TaintLattice> = None;
        for pred in exec_preds {
            let ver = phi.incoming.get(pred).copied().unwrap_or(0);
            // Include version-0 incomings: they represent
            // enclosing-scope reads (possibly pre-seeded with
            // taint when a reachable callee writes globals).
            let incoming = taints
                .get(&(phi.name, ver))
                .copied()
                .unwrap_or(TaintLattice::clean());
            phi_taint = Some(match phi_taint {
                Some(existing) => existing.join(incoming),
                None => incoming,
            });
        }

        let Some(phi_taint) = phi_taint else { continue };
        let key = (phi.name, phi.version);
        let merged = match taints.get(&key) {
            Some(&old) => old.join(phi_taint),
            None => phi_taint,
        };
        if taints.get(&key) != Some(&merged) {
            taints.insert(key, merged);
            changed = true;
        }
    }
    changed
}

/// Apply the per-statement taint transfer for one block's statements,
/// merging results into `taints`. Returns `true` if any value changed.
pub(super) fn propagate_statement_taints(
    taints: &mut HashMap<ValueKey, TaintLattice>,
    block_id: BlockId,
    ssa_block: &crate::ssa::SsaBlock,
    ctx: TaintCtx<'_>,
    ssa: &SsaFunction,
    rendered_props: Option<&HashMap<ValueKey, RenderedValueProps>>,
) -> bool {
    let mut changed = false;
    for (statement_index, ssa_stmt) in ssa_block.statements.iter().enumerate() {
        let stmt = &ssa_stmt.statement;
        for (&var, &ver) in &ssa_stmt.defs {
            let destruction = ssa_stmt.destruction_defs.contains(&var);
            // Deletion supplies no value or sanitizer result. Retain the
            // predecessor's possible taint even for an unconditional tombstone;
            // completed reads and callback stores have their own receipts.
            let mut inferred = if destruction {
                ssa_stmt
                    .uses
                    .get(&var)
                    .and_then(|version| taints.get(&(var, *version)))
                    .copied()
                    .unwrap_or_else(TaintLattice::clean)
            } else {
                evaluate_taint_def_at(
                    stmt,
                    var,
                    &ssa_stmt.uses,
                    &*taints,
                    ctx,
                    ssa,
                    Some((block_id, statement_index)),
                )
            };
            // A name an opaque `switch`'s arms may write keeps the taint it had
            // when no arm runs.
            if crate::ssa::has_arm_may_defs(stmt)
                && ssa_stmt.may_defs.contains(&var)
                && let Some(before) = ssa_stmt
                    .uses
                    .get(&var)
                    .and_then(|&prior| taints.get(&(var, prior)))
            {
                inferred = inferred.join(*before);
            }
            // Enrich the inferred taint with rendered-property
            // colours when available.
            if !destruction
                && let Some(rp) = rendered_props
                && let Some(p) = rp.get(&(var, ver))
            {
                inferred = colour_from_rendered(inferred, *p);
            }
            let key = (var, ver);
            let merged = match taints.get(&key) {
                Some(&old) => old.join(inferred),
                None => inferred,
            };
            if taints.get(&key) != Some(&merged) {
                taints.insert(key, merged);
                changed = true;
            }
        }
    }
    changed
}
