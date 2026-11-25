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

use super::super::solver::{sccp_process_phis, seed_live_in_roots, seeded_values};
use super::super::{
    TraceInputs, cfg_order, compute_predecessors, incoming_executable_predecessors,
    settle_reachability,
};
use super::block_transfer::{
    collect_constant_branches, sccp_process_statements, sccp_process_terminator,
};
use super::{
    BranchFold, MathDependencies, SettledValueFactState, SolvedValueFacts, SolverPurpose,
    StatementInputs, StoreInputs, TerminatorInputs, ValueFactInputs,
};
use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction};
use crate::expr_ast::ExprNode;
use crate::ssa::{SsaFunction, SsaSourceView, ValueKey};
use crate::tcl_expr_eval::FoldPolicy;
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

fn seed_value_facts(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    policy: FoldPolicy,
    registry: &CommandRegistry,
) -> (HashMap<ValueKey, LatticeValue>, tcl_dialect::LexerGrammar) {
    let mut values = seeded_values(param_constants, |name| {
        ssa.point_contexts.as_ref().map_or_else(
            || ssa.var_symbol(name),
            |points| {
                crate::var_resolve::canonical_variable_key(
                    name,
                    points.before_statement(cfg.entry, 0),
                    registry,
                )
                .and_then(|cell| ssa.cell_symbol(&cell))
            },
        )
    });
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
    seed_live_in_roots(cfg, ssa, &mut values, grammar, |block, name| {
        ssa.var_symbol_at_terminator(block, name)
    });
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

    settle_reachability(|finalizing| {
        let mut changed = false;
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
        changed
    });

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
