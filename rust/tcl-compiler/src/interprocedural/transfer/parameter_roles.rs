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

//! A procedure's parameter roles: values the body reads, and names of caller places with the outcomes two summary runs leave at the normal exits.

use super::outer_places::links_local;
use super::source_context::SourceSummaryContext;
use super::{ANY_OUTCOME, ModuleInputs, ModuleProcedures, ParamRole};
use crate::cfg::{Function as CfgFunction, Terminator};
use crate::cfg_builder::upvar_info::UpvarInfo;
use crate::sccp::{
    BuiltinFoldInputs, CallerPlaces, ExistenceEntry, FoldTrust, ModuleLevel, ModuleRun, SccpResult,
    SolveInputs, TraceInputs,
};
use crate::ssa::SsaFunction;
use std::collections::{HashMap, HashSet};
use tcl_registry::frame_effect::FrameLevel;
use tcl_registry::value_transfer::{BindingKind, Existence, ExistenceOutcome};

impl ModuleProcedures<'_> {
    /// `qname`'s parameter roles under the summaries in hand: `None` when no
    /// normal exit of its body is reached.
    pub(super) fn roles(
        &self,
        qname: &crate::ir::ProcedureId,
        inputs: &ModuleInputs<'_>,
    ) -> Option<Vec<ParamRole>> {
        let declared = inputs.ir.procedures.get(qname)?;
        let cfg = inputs.cfg.procedures.get(qname)?;
        let links = links_of(qname, declared, &self.frame_facts);
        if links.is_empty() {
            return Some(value_roles(qname, declared, inputs));
        }
        let ssa = crate::ssa::build_ssa_with_context_for_entry_and_metadata(
            cfg,
            inputs.registry,
            inputs.config,
            crate::compilation_unit::function_source_entry(cfg, None),
            Some(&declared.params),
            crate::registry_invocation::InvocationMetadataContext::for_module(
                inputs.registry,
                inputs.ir,
            ),
            cfg.metadata_context.is_standalone(),
        );
        let owned: HashSet<String> = links.iter().map(|(_, local)| local.clone()).collect();
        let runs = [Existence::Bound(BindingKind::Either), Existence::Unbound]
            .map(|entry| self.summary_run(qname, (cfg, &ssa), &owned, entry, inputs));
        // Where each linked local stands at the normal exits of each run:
        // `None` for a run that completes nowhere — no normal exit reached,
        // or every path to one through a callee that never completes — and
        // a `None` slot where a run completes but cannot state the local
        // (a release unit's SSA sends a write through a caller link to the
        // link's target rather than defining the local).
        let facts = runs.each_ref().map(|run| {
            let exits = normal_exits(cfg, run);
            if exits.is_empty() {
                return None;
            }
            Some(
                links
                    .iter()
                    .map(|(_, local)| exit_fact(&ssa, run, &exits, local))
                    .collect::<Vec<Option<Existence>>>(),
            )
        });
        if facts.iter().all(Option::is_none) {
            return None;
        }
        let reached: HashSet<crate::cfg::BlockId> = runs
            .iter()
            .flat_map(|run| run.executable_blocks.iter().copied())
            .collect();
        let mut roles = value_roles(qname, declared, inputs);
        for (slot, (index, local)) in links.iter().enumerate() {
            let param = &declared.params[*index];
            // A release unit's SSA sends a write through a caller link to the
            // link's target rather than defining the local, so its exit facts
            // cannot state the place's outcome.
            let source = self.source.as_ref()?;
            let slot_facts = facts
                .each_ref()
                .map(|facts| facts.as_ref().map(|facts| facts[slot]));
            let stated = slot_facts.iter().all(|fact| !matches!(fact, Some(None)));
            let outcomes = if stated
                && source.reads_linked_locals()
                && linked_cleanly(cfg, &ssa, local, param, qname, source)
            {
                let [bound, unbound] = slot_facts.map(Option::flatten);
                outcomes_of(bound, unbound, defined_in(&ssa, &reached, local))
            } else {
                ANY_OUTCOME.to_vec()
            };
            roles[*index] = ParamRole::Name {
                level: FrameLevel::Relative(1),
                outcomes,
            };
        }
        Some(roles)
    }

    /// One run of `qname`'s body for its summary, with its linked locals
    /// its own and entering as `entry` says.
    fn summary_run(
        &self,
        qname: &crate::ir::ProcedureId,
        (cfg, ssa): (&CfgFunction, &SsaFunction),
        owned: &HashSet<String>,
        entry: Existence,
        inputs: &ModuleInputs<'_>,
    ) -> SccpResult {
        let declared = inputs
            .ir
            .procedures
            .get(qname)
            .expect("the selected analytical summary retains an unambiguous declaration");
        let places = CallerPlaces::each(owned, entry);
        let trace = inputs.trace;
        crate::sccp::sccp_in_module(&SolveInputs {
            cfg,
            ssa,
            param_constants: None,
            policy: crate::tcl_expr_eval::FoldPolicy::from_registry(inputs.registry),
            extra_escaping: &HashSet::new(),
            trace: TraceInputs {
                source_metadata_input: inputs.ir.source_metadata_input.as_ref(),
                registry: inputs.registry,
                traced_variables: trace.traced_variables,
                has_dynamic_variable_trace: trace.has_dynamic_variable_trace,
                deferred_writes: trace.deferred_writes,
                analysis_context: Some(inputs.analysis_context),
                existence: Some(ExistenceEntry {
                    initial_state: crate::sccp::ExistenceInitialState::DeclaredFrame,
                    params: &declared.params,
                    object_state: None,
                    initial_global: false,
                    connection_scoped: None,
                    dynamic_trace: trace.has_dynamic_variable_trace || trace.deferred_writes.any,
                    config: inputs.config,
                    caller_places: Some(&places),
                }),
            },
            folds: Some(BuiltinFoldInputs {
                source_metadata_input: inputs.ir.source_metadata_input.as_ref(),
                registry: inputs.registry,
                mutations: inputs.mutations,
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::ObservedBindings,
                proven_pure_parameters: false,
            }),
            module: ModuleRun {
                analysis_body: Some(crate::sccp::AnalysisBody::Procedure(*qname)),
                procedures: Some(self),
                owned: Some(owned),
                level: ModuleLevel::Outcomes,
                reads_exits: true,
            },
        })
    }
}

/// Each parameter of `declared` whose value names the place one frame up
/// its body links a local to, by index, with that local.
pub(super) fn links_of(
    id: &crate::ir::ProcedureId,
    declared: &crate::ir::Procedure,
    frames: &HashMap<crate::ir::ProcedureId, UpvarInfo>,
) -> Vec<(usize, String)> {
    let mut links: Vec<(usize, String)> = frames.get(id).map_or_default(|frame| {
        frame
            .param_targets
            .iter()
            .filter_map(|(local, param)| {
                let index = declared.params.iter().position(|p| p == param)?;
                Some((index, local.clone()))
            })
            .collect()
    });
    links.sort_unstable();
    links
}

/// Parameter uses from the existing SSA owner under the complete retained
/// metadata and lexer configuration. This is reached only after the source
/// graph/frame/dynamic-name guards close the summary; a missing graph cannot
/// establish that a parameter is unused.
fn value_roles(
    qname: &crate::ir::ProcedureId,
    declared: &crate::ir::Procedure,
    inputs: &ModuleInputs<'_>,
) -> Vec<ParamRole> {
    let Some(cfg) = inputs.cfg.procedures.get(qname) else {
        return vec![ParamRole::Value; declared.params.len()];
    };
    let ssa = crate::ssa::build_ssa_with_context_for_entry_and_metadata(
        cfg,
        inputs.registry,
        inputs.config,
        crate::compilation_unit::function_source_entry(cfg, None),
        Some(&declared.params),
        crate::registry_invocation::InvocationMetadataContext::for_module(
            inputs.registry,
            inputs.ir,
        ),
        cfg.metadata_context.is_standalone(),
    );
    declared
        .params
        .iter()
        .map(|parameter| {
            if ssa.var_symbol(parameter).is_some() {
                ParamRole::Value
            } else {
                ParamRole::Unused
            }
        })
        .collect()
}

/// [`value_roles`], with every parameter the body links a local to a place
/// the body applies `outcomes` to: the roles of a procedure that never
/// completes normally, or of a cycle that did not settle.
pub(super) fn linked_roles(
    qname: &crate::ir::ProcedureId,
    declared: &crate::ir::Procedure,
    inputs: &ModuleInputs<'_>,
    frames: &HashMap<crate::ir::ProcedureId, UpvarInfo>,
    outcomes: &[ExistenceOutcome],
) -> Vec<ParamRole> {
    let linked: HashSet<&str> = frames
        .get(qname)
        .map_or_default(|frame| frame.param_targets.values().map(String::as_str).collect());
    value_roles(qname, declared, inputs)
        .into_iter()
        .zip(&declared.params)
        .map(|(role, param)| {
            if linked.contains(param.as_str()) {
                ParamRole::Name {
                    level: FrameLevel::Relative(1),
                    outcomes: outcomes.to_vec(),
                }
            } else {
                role
            }
        })
        .collect()
}

/// The blocks a run reaches whose end completes the procedure normally: a
/// block a statement of which certainly raises never reaches its end.
pub(super) fn normal_exits(cfg: &CfgFunction, result: &SccpResult) -> Vec<crate::cfg::BlockId> {
    let mut exits: Vec<crate::cfg::BlockId> = result
        .executable_blocks
        .iter()
        .copied()
        .filter(|id| {
            !result.completion.raises_in(*id)
                && cfg.blocks.get(id).is_some_and(|block| {
                    matches!(block.terminator, None | Some(Terminator::Return { .. }))
                })
        })
        .collect();
    exits.sort_unstable();
    exits
}

/// The fact `local` holds where a run's normal exits leave it, joined;
/// `None` where no exit was reached or the fact waits on a callee that has
/// not completed.
pub(super) fn exit_fact(
    ssa: &SsaFunction,
    result: &SccpResult,
    exits: &[crate::cfg::BlockId],
    local: &str,
) -> Option<Existence> {
    let symbol = ssa.var_symbol(local)?;
    let fact = exits
        .iter()
        .filter_map(|block| {
            result
                .existence_exits
                .get(block)?
                .get(symbol.0 as usize)
                .copied()
        })
        .fold(Existence::Pending, Existence::join);
    (fact != Existence::Pending).then_some(fact)
}

/// Whether a block a run reached defines `local`, by a statement or a φ.
fn defined_in(ssa: &SsaFunction, reached: &HashSet<crate::cfg::BlockId>, local: &str) -> bool {
    let Some(symbol) = ssa.var_symbol(local) else {
        return false;
    };
    reached
        .iter()
        .filter_map(|block| ssa.blocks.get(block))
        .any(|block| {
            block.phis.iter().any(|phi| phi.name == symbol)
                || block
                    .statements
                    .iter()
                    .any(|statement| statement.defs.contains_key(&symbol))
        })
}

/// Whether `local` is linked once, by the entry block's first statement
/// that touches it, to the place the parameter's own value names, and is
/// only ever reached as a whole variable.
pub(super) fn linked_cleanly(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    local: &str,
    param: &str,
    qname: &crate::ir::ProcedureId,
    source: &SourceSummaryContext<'_>,
) -> bool {
    if ssa.var_names().iter().any(|name| {
        tcl_syntax::naming::split_element_ref_bytes(name.as_bytes())
            .is_some_and(|(root, _)| root == local.as_bytes())
    }) {
        return false;
    }
    if cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .any(|statement| links_local(statement, local, qname, source).is_none())
    {
        return false;
    }
    let sites: Vec<(crate::cfg::BlockId, usize)> = cfg
        .blocks
        .iter()
        .flat_map(|(id, block)| {
            block
                .statements
                .iter()
                .enumerate()
                .filter(|(_, statement)| {
                    links_local(statement, local, qname, source).unwrap_or(false)
                })
                .map(move |(index, _)| (*id, index))
        })
        .collect();
    let [(block, index)] = sites[..] else {
        return false;
    };
    let (Some(symbol), Some(param_symbol), Some(entry)) = (
        ssa.var_symbol(local),
        ssa.var_symbol(param),
        ssa.blocks.get(&cfg.entry),
    ) else {
        return false;
    };
    block == cfg.entry
        && entry.statements[..index].iter().all(|statement| {
            !statement.uses.contains_key(&symbol) && !statement.defs.contains_key(&symbol)
        })
        && entry.statements[index].uses.get(&param_symbol) == Some(&0)
}

/// The outcomes a body applies to a linked place, from where the two runs
/// leave it — bound on entry, then unbound — and whether any exit holds a
/// version the body wrote.
fn outcomes_of(
    bound: Option<Existence>,
    unbound: Option<Existence>,
    written: bool,
) -> Vec<ExistenceOutcome> {
    if !written {
        return vec![ExistenceOutcome::Preserve];
    }
    let facts: Vec<Existence> = [bound, unbound].into_iter().flatten().collect();
    if facts.is_empty() {
        return vec![ExistenceOutcome::Preserve];
    }
    if facts.iter().all(|fact| matches!(fact, Existence::Bound(_))) {
        let kind = facts
            .iter()
            .filter_map(|fact| match fact {
                Existence::Bound(kind) => Some(*kind),
                _ => None,
            })
            .reduce(BindingKind::join)
            .unwrap_or(BindingKind::Either);
        return vec![ExistenceOutcome::Bind(kind)];
    }
    if facts.iter().all(|fact| *fact == Existence::Unbound) {
        return vec![ExistenceOutcome::Unbind];
    }
    if bound.is_none_or(|fact| matches!(fact, Existence::Bound(_))) {
        return vec![ExistenceOutcome::MayBind(BindingKind::Either)];
    }
    ANY_OUTCOME.to_vec()
}
