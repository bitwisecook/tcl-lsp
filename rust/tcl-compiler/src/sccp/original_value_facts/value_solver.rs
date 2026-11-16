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

//! The purpose-aware value solver: seeds, the fixpoint over the blocks, φ joins and the finished facts.

use super::super::{TraceInputs, cfg_order, compute_predecessors, join, set_value};
use super::block_transfer::{
    collect_constant_branches, sccp_process_statements, sccp_process_terminator,
};
use super::{
    BranchFold, MathDependencies, SettledValueFactState, SolvedValueFacts, SolverPurpose,
    StatementInputs, StoreInputs, TerminatorInputs, ValueFactInputs,
};
use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ssa::{SsaFunction, SsaSourceView, ValueKey};
use crate::tcl_expr_eval::FoldPolicy;
use rustc_hash::FxHashSet;
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

fn seed_value_facts(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    policy: FoldPolicy,
    registry: &CommandRegistry,
) -> (HashMap<ValueKey, LatticeValue>, tcl_dialect::LexerGrammar) {
    let mut values = HashMap::new();
    seed_parameter_constants(cfg, ssa, param_constants, registry, &mut values);
    let grammar = policy.preparation_context().map_or_else(
        || {
            registry
                .profile()
                .map_or_else(tcl_dialect::LexerGrammar::default, |profile| {
                    profile.grammar
                })
        },
        |context| context.lexer_grammar,
    );
    seed_live_in_roots(cfg, ssa, &mut values, grammar);
    (values, grammar)
}

pub(super) fn solve_value_facts(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    inputs: &ValueFactInputs<'_>,
    purpose: SolverPurpose,
) -> SolvedValueFacts {
    let ValueFactInputs {
        param_constants,
        policy,
        extra_escaping,
        trace,
        ..
    } = *inputs;
    let preds = compute_predecessors(cfg);
    let (values, grammar) = seed_value_facts(cfg, ssa, param_constants, policy, trace.registry);

    // Retained point contexts describe actual read and write observers and
    // contents origins. A later alias, opaque call or unreachable branch must
    // not retroactively erase an earlier definition. Carrierless compatibility
    // SSA still uses the conservative whole-function exposure summary.
    let mut escaping = compatibility_escaping(cfg, ssa, trace, extra_escaping);
    // A name a variable trace or a stored callback writes changes whenever
    // they run (`after idle {set ::go 0}; update`), which no point context
    // states; the execution solver treats them the same way.
    if ssa.point_contexts.is_some() {
        trace.extend_module_escaping(&mut escaping);
    }
    // A block the solve proves never runs accesses nothing, so the opaque
    // native accesses of such a block (an arm the walk proves dead, whose
    // calls it binds nothing for) leave the running code's values alone.
    // Solve first without them and again with them only when a block that
    // runs has one, as the execution solver does with its hazards (#2375).
    let mut opaque_native_accesses = false;
    loop {
        let solved = solve_value_facts_once(
            (cfg, ssa),
            (inputs, purpose),
            (&preds, values.clone(), grammar),
            &escaping,
            opaque_native_accesses,
        );
        if opaque_native_accesses
            || !ssa
                .has_opaque_native_accesses_in(&|id| solved.result.executable_blocks.contains(&id))
        {
            return solved;
        }
        opaque_native_accesses = true;
    }
}

fn solve_value_facts_once(
    (cfg, ssa): (&CfgFunction, &SsaFunction),
    (inputs, purpose): (&ValueFactInputs<'_>, SolverPurpose),
    (preds, mut values, grammar): (
        &HashMap<BlockId, HashSet<BlockId>>,
        HashMap<ValueKey, LatticeValue>,
        tcl_dialect::LexerGrammar,
    ),
    escaping: &HashSet<String>,
    opaque_native_accesses: bool,
) -> SolvedValueFacts {
    let ValueFactInputs {
        param_constants,
        policy,
        trace,
        folds,
        ..
    } = *inputs;
    let required_math_invocations = MathDependencies::for_parameters(purpose, param_constants);
    let reference_config =
        crate::value_transfer::LatticeDriver::new(trace, folds, policy, escaping)
            .with_cfg_metadata(&cfg.metadata_context)
            .source_reference_lexer_config();
    let store_inputs = StoreInputs {
        reference_config,
        math_dependencies: Some(&required_math_invocations),
        cfg: Some(cfg),
        escaping,
        policy,
        has_dynamic_variable_trace: trace.writes_any_variable() && ssa.point_contexts.is_none(),
        opaque_native_accesses,
        folds,
        registry: trace.registry,
    };

    let mut executable_blocks: HashSet<BlockId> = HashSet::new();
    let mut executable_edges: HashSet<(BlockId, BlockId)> = HashSet::new();
    if cfg.blocks.contains_key(&cfg.entry) {
        executable_blocks.insert(cfg.entry);
    }
    let order = cfg_order(cfg);

    // Optimistic fixpoint over the RPO sweep, followed by a finalising pass
    // that forces both arms for any executable branch still stuck on an UNKNOWN
    // condition (defensive: a value defined only in unreachable code could
    // otherwise leave a successor spuriously unreachable). `finalizing` is
    // monotone, so the outer loop runs at most twice.
    let mut finalizing = false;
    loop {
        let mut changed = true;
        while changed {
            changed = false;
            for bn in &order {
                if !executable_blocks.contains(bn) {
                    continue;
                }
                let Some(ssa_block) = ssa.blocks.get(bn) else {
                    continue;
                };

                let incoming_exec = incoming_executable_predecessors(*bn, preds, &executable_edges);

                // Phi nodes (not at entry, only when some predecessor is
                // executable).
                if bn != &cfg.entry {
                    changed |= sccp_process_phis(&mut values, ssa_block, &incoming_exec);
                }

                // Statements.
                changed |= sccp_process_statements(
                    &mut values,
                    ssa_block,
                    ssa,
                    StatementInputs {
                        store: &store_inputs,
                        clobbers: ssa.value_clobbers.get(bn),
                    },
                );

                // Terminator.
                let inputs = TerminatorInputs {
                    math_dependencies: Some(&required_math_invocations),
                    cfg,
                    ssa,
                    values: &values,
                    policy,
                    grammar,
                    registry: trace.registry,
                    reference_config,
                };
                if sccp_process_terminator(
                    *bn,
                    &inputs,
                    &mut executable_blocks,
                    &mut executable_edges,
                    finalizing,
                ) {
                    changed = true;
                }
            }
        }
        if finalizing {
            break;
        }
        finalizing = true;
    }

    finish_value_facts(
        (cfg, ssa),
        (inputs, purpose, grammar),
        (&order, escaping),
        required_math_invocations,
        SettledValueFactState {
            values,
            executable_blocks,
            executable_edges,
        },
    )
}

fn finish_value_facts(
    (cfg, ssa): (&CfgFunction, &SsaFunction),
    (inputs, purpose, grammar): (
        &ValueFactInputs<'_>,
        SolverPurpose,
        tcl_dialect::LexerGrammar,
    ),
    (order, escaping): (&[BlockId], &HashSet<String>),
    required_math_invocations: MathDependencies,
    state: SettledValueFactState,
) -> SolvedValueFacts {
    let ValueFactInputs {
        policy,
        trace,
        folds,
        ..
    } = *inputs;
    let SettledValueFactState {
        values,
        executable_blocks,
        executable_edges,
    } = state;
    let constant_branches = collect_constant_branches(
        cfg,
        ssa,
        &values,
        &executable_blocks,
        order,
        BranchFold {
            math_dependencies: Some(&required_math_invocations),
            policy,
            grammar,
            registry: trace.registry,
            reference_config: crate::value_transfer::LatticeDriver::new(
                trace, folds, policy, escaping,
            )
            .with_cfg_metadata(&cfg.metadata_context)
            .source_reference_lexer_config(),
        },
    );

    // Advisory plans read this purpose's settled values and original source
    // metadata. They never borrow execution-SCCP's option values or results.
    let template_plans = if purpose == SolverPurpose::SemanticAnalysis {
        let driver = crate::value_transfer::LatticeDriver::new(trace, folds, policy, escaping)
            .with_cfg_metadata(&cfg.metadata_context);
        let mut plans = Vec::new();
        for &id in order {
            if executable_blocks.contains(&id)
                && let Some(block) = ssa.blocks.get(&id)
            {
                plans.extend(driver.template_plans_in(ssa, (id, block), &values));
            }
        }
        plans.sort_by_key(|record| (record.span.start(), record.span.end()));
        plans
    } else {
        Vec::new()
    };
    let mut solved = required_math_invocations.finish(
        values,
        executable_blocks,
        executable_edges,
        constant_branches,
    );
    solved.result.template_plans = template_plans;
    solved
}

fn incoming_executable_predecessors(
    block: BlockId,
    predecessors: &HashMap<BlockId, HashSet<BlockId>>,
    executable_edges: &HashSet<(BlockId, BlockId)>,
) -> Vec<BlockId> {
    predecessors
        .get(&block)
        .into_iter()
        .flatten()
        .copied()
        .filter(|predecessor| executable_edges.contains(&(*predecessor, block)))
        .collect()
}

pub(super) fn compatibility_escaping(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    trace: TraceInputs<'_>,
    extra: &HashSet<String>,
) -> HashSet<String> {
    if ssa.point_contexts.is_some() {
        return HashSet::new();
    }
    let metadata = trace.source_metadata_input.and_then(|input| {
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            trace.registry,
            input,
        )
    });
    // No completed alias query exists without actual metadata. A tokenless
    // compatibility graph therefore treats every named value as observable.
    let Some(metadata) = metadata else {
        return ssa.var_names().iter().cloned().collect();
    };
    let mut names = crate::var_observability::analyse_var_observability_with_metadata_context(
        cfg,
        trace.registry,
        Some(metadata),
    )
    .escaping_var_names();
    names.extend(extra.iter().cloned());
    trace.extend_module_escaping(&mut names);
    names
}

fn seed_parameter_constants(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    registry: &CommandRegistry,
    values: &mut HashMap<ValueKey, LatticeValue>,
) {
    if let Some(seed) = param_constants {
        // The interprocedural seed keys on the parameter *name* (a stable,
        // cache-safe identity); resolve each to this build's interned symbol.
        // A param never read in the body isn't interned, and its seed slot
        // would never be consulted, so dropping it is behaviour-neutral.
        for ((name, version), v) in seed {
            let symbol = ssa.point_contexts.as_ref().map_or_else(
                || ssa.var_symbol(name),
                |points| {
                    crate::var_resolve::canonical_variable_key(
                        name,
                        points.before_statement(cfg.entry, 0),
                        registry,
                    )
                    .and_then(|cell| ssa.cell_symbol(&cell))
                },
            );
            if let Some(symbol) = symbol {
                values.insert((symbol, *version), v.clone());
            }
        }
    }
}

/// Seed live-in roots to `Overdefined`: a value *used* but never *defined*
/// anywhere in this function (a proc parameter, a global / namespace read, an
/// upvar target, or an undefined-variable read) holds a runtime-unknown value,
/// so it is `Overdefined`, not the `Unknown` join-identity (which would
/// silently vanish from any phi it feeds — folding `join(const, $runtime)` to
/// the constant).
pub(super) fn seed_live_in_roots<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &mut HashMap<ValueKey, LatticeValue, S>,
    grammar: tcl_dialect::LexerGrammar,
) {
    let mut defined_keys: FxHashSet<ValueKey> = FxHashSet::default();
    let mut used_keys: FxHashSet<ValueKey> = FxHashSet::default();
    for ssa_block in ssa.blocks.values() {
        for phi in &ssa_block.phis {
            defined_keys.insert((phi.name, phi.version));
            for inc in phi.incoming.values() {
                // Record every phi feed, including the version-0 (entry /
                // live-in) incoming: it is never a def, so it drops through
                // to the `used_keys.difference(&defined_keys)` seeding below
                // and is pinned `Overdefined`. This is what lets the phi
                // join at [`sccp_process_phis`] see the caller's value
                // instead of silently dropping it and folding to the
                // defined-arm constant.
                used_keys.insert((phi.name, *inc));
            }
        }
        for s in &ssa_block.statements {
            for (&var, ver) in &s.defs {
                defined_keys.insert((var, *ver));
            }
            for (&var, ver) in &s.uses {
                used_keys.insert((var, *ver));
            }
        }
    }
    // Branch-condition reads, resolved at each block's exit versions.
    for (bn, block) in &cfg.blocks {
        if let Some(Terminator::Branch { condition, .. }) = &block.terminator
            && let Some(sb) = ssa.blocks.get(bn)
        {
            for var in crate::var_refs::vars_in_expr(condition, grammar) {
                let Some(sym) = ssa.var_symbol_at_terminator(*bn, &var) else {
                    continue;
                };
                let ver = sb.exit_versions.get(&sym).copied().unwrap_or(0);
                used_keys.insert((sym, ver));
            }
        }
    }
    for key in used_keys.difference(&defined_keys) {
        values.entry(*key).or_insert(LatticeValue::Overdefined);
    }
}

/// Optimistic (Wegman–Zadeck) deferral test for a non-constant branch.
///
/// Returns `true` when the condition *may still fold* on a later sweep:
/// some operand defined in this function (SSA exit-version > 0) is still
/// `Unknown` (not yet computed) and none is `Overdefined`. Such a branch
/// opens neither arm until either the operand resolves to a constant or an
/// `Overdefined` operand proves the condition genuinely non-constant.
///
/// Operands read at version 0 (proc parameters, globals, and other
/// live-in roots) are excluded: [`seed_live_in_roots`] already seeds them
/// `Overdefined`, so they are never "not yet computed".
pub(super) fn branch_deferrable(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
    ssa: &SsaFunction,
    grammar: tcl_dialect::LexerGrammar,
) -> bool {
    let block = ssa.block_id(&ssa_block.name).unwrap_or(BlockId(u32::MAX));
    let lookup = if ssa.point_contexts.is_some() {
        SsaSourceView::at_terminator(ssa, block)
    } else {
        SsaSourceView::unpositioned(ssa)
    };
    let mut any_operand = false;
    let mut any_unknown = false;
    for name in crate::var_refs::vars_in_expr(condition, grammar) {
        let (sym, ver) = if lookup.is_positioned() {
            let Some(read) = lookup.read_spelling(&name) else {
                return false;
            };
            let Some(version) = read.version else {
                return false;
            };
            (read.symbol, version)
        } else {
            let Some(symbol) = lookup.symbol(&name) else {
                continue;
            };
            (
                symbol,
                ssa_block.exit_versions.get(&symbol).copied().unwrap_or(0),
            )
        };
        if ver == 0 {
            continue;
        }
        any_operand = true;
        match values.get(&(sym, ver)) {
            Some(LatticeValue::Overdefined) => return false,
            Some(LatticeValue::Unknown) | None => any_unknown = true,
            _ => {}
        }
    }
    any_operand && any_unknown
}

/// Join phi values from edge-executable predecessors for one block. Returns
/// `true` if any lattice value changed. Used by [`execution_value_facts`].
pub(super) fn sccp_process_phis(
    values: &mut HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    incoming_exec: &[BlockId],
) -> bool {
    if incoming_exec.is_empty() {
        return false;
    }
    let mut changed = false;
    for phi in &ssa_block.phis {
        let mut phi_val = LatticeValue::Unknown;
        for pred in incoming_exec {
            let incoming_ver = phi.incoming.get(pred).copied().unwrap_or(0);
            // A version-0 incoming is the entry / live-in root (a proc
            // parameter, global, or other caller-supplied value). Its
            // runtime value is unknown at compile time, so it joins in as
            // `Overdefined` — never skipped. Skipping it would let a phi
            // that merges a live-in with a defined-arm constant fold to
            // that constant, miscompiling any `if {$param} { set x k }`
            // followed by a test on `x`. This mirrors the interval pass,
            // which joins `TOP` for `inc == 0` (intervals.rs:570-574).
            // `seed_live_in_roots` pins `(name, 0)` `Overdefined`; the
            // explicit default keeps the join correct even if a feed was
            // never seeded.
            let key: ValueKey = (phi.name, incoming_ver);
            let candidate = values.get(&key).cloned().unwrap_or(if incoming_ver == 0 {
                LatticeValue::Overdefined
            } else {
                LatticeValue::Unknown
            });
            phi_val = join(&phi_val, &candidate);
        }
        if set_value(values, (phi.name, phi.version), &phi_val) {
            changed = true;
        }
    }
    changed
}
