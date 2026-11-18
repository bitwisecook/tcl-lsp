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

//! Computing every summary, callees before callers: the local call graph, its strongly connected components and each component's completion, effects and evidence.

use super::super::ReturnKind;
use super::outer_places::aliases_unnamed_place;
use super::parameter_roles::{linked_roles, links_of};
use super::{
    ANY_OUTCOME, ERROR_ONLY, ModuleInputs, ModuleProcedures, NORMAL_OR_ERROR, ParamRole,
    TransferSummary,
};
use crate::cfg_builder::upvar_info::UpvarInfo;
use crate::ir::{CommandTokens, Statement, SyntheticMarker};
use crate::sccp::FoldTrust;
use std::collections::{BTreeSet, HashMap, HashSet};
use tcl_core_types::RecursionLimit;
use tcl_registry::completion::CompletionCodeDomain;
use tcl_registry::value_transfer::{
    BindingIdentity, DependencyEvidence, ExistenceOutcome, PlaceRef,
};
use tcl_registry::world_effect::EffectFootprint;

impl ModuleProcedures<'_> {
    /// Compute every procedure's summary, callees before callers.
    pub(super) fn summarise(&self, inputs: &ModuleInputs<'_>, rounds: RecursionLimit) {
        // A rebinding whose subject the scan cannot name may have moved any
        // command a typed statement stands for.
        let Some(rebound) = inputs.mutations.nameable_rebound_builtins() else {
            return;
        };
        if self.source.is_none() {
            return;
        }
        let mut names: Vec<crate::ir::ProcedureId> = inputs.ir.procedures.keys().copied().collect();
        names.sort_unstable();
        let mut calls: HashMap<crate::ir::ProcedureId, BTreeSet<crate::ir::ProcedureId>> = names
            .iter()
            .filter(|qname| !spells_a_rebound_builtin(inputs.ir, qname, rebound))
            .filter_map(|&qname| Some((qname, self.local_calls(&qname, inputs)?)))
            .collect();
        // A procedure that calls a barrier is one.
        loop {
            let barred: Vec<crate::ir::ProcedureId> = calls
                .iter()
                .filter(|(_, callees)| callees.iter().any(|callee| !calls.contains_key(callee)))
                .map(|(&qname, _)| qname)
                .collect();
            if barred.is_empty() {
                break;
            }
            for qname in barred {
                calls.remove(&qname);
            }
        }
        for component in components(&names, &calls) {
            self.summarise_component(&component, &calls, inputs, rounds);
        }
    }

    /// The procedures of the module `qname` calls, from its statements, the
    /// commands their words substitute and its terminators' words; `None`
    /// when it reaches code or a frame the module cannot see, names a
    /// variable it computes, or is too large to analyse.
    fn local_calls(
        &self,
        qname: &crate::ir::ProcedureId,
        inputs: &ModuleInputs<'_>,
    ) -> Option<BTreeSet<crate::ir::ProcedureId>> {
        let cfg = inputs.summary_cfg(qname)?;
        let source = self.source.as_ref()?;
        let declared = source.procedure(qname)?;
        if !source.walked_in_declared_frame(qname) {
            return None;
        }
        // Count/formal syntax is not an activation. This conditional scalar
        // model also needs the existing selected binder's local scalar closure.
        crate::registry_invocation::original_procedure_scalar_bindings(
            self.ir,
            declared,
            self.registry,
        )?;
        let body_bytes = declared.span.end().saturating_sub(declared.span.start()) as usize;
        if body_bytes > crate::ssa::DEEP_ANALYSIS_BODY_BYTES
            || crate::ssa::is_complexity_guarded(cfg)
            || self
                .frame_facts
                .get(qname)
                .is_some_and(|frame| !frame_is_closed(frame, &declared.params))
            || self
                .outer_facts
                .get(qname)
                .is_some_and(|writes| writes.opaque_global_frame)
        {
            return None;
        }
        let computed =
            crate::dynamic_names::dynamic_name_barrier(cfg, inputs.registry, inputs.config);
        if computed.writes || computed.destroys {
            return None;
        }
        let mut callees = BTreeSet::new();
        for block in cfg.blocks.values() {
            for statement in &block.statements {
                match statement {
                    Statement::Call { tokens, .. }
                        if tokens
                            .as_ref()
                            .and_then(|tokens| tokens.synthetic)
                            .is_some() =>
                    {
                        if matches!(
                            statement.synthetic_marker(),
                            Some(
                                SyntheticMarker::UnseenCall
                                    | SyntheticMarker::GlobalFrameScript
                                    | SyntheticMarker::CallerFrameOpaque
                                    | SyntheticMarker::ArmWrites
                            )
                        ) {
                            return None;
                        }
                    }
                    Statement::Call { .. }
                    | Statement::AssignConst { .. }
                    | Statement::AssignExpr { .. }
                    | Statement::AssignValue { .. }
                    | Statement::Incr { .. }
                    | Statement::ExprEval { .. }
                    | Statement::Return { .. } => {
                        let original = source.statement_tokens(Some(*qname), statement)?;
                        if crate::ir_helpers::evaluated_command_substitutions_with_metadata_context(
                            statement,
                            self.registry,
                            Some(source.metadata(&original)?),
                            self.config,
                        )
                        .opaque
                        {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
        }
        for tokens in source.body_commands(qname)? {
            if aliases_unnamed_place(&tokens, source)? {
                return None;
            }
            self.callee_at(&tokens, &mut callees)?;
        }
        self.outer_arguments(qname, inputs)?;
        Some(callees)
    }

    /// Classify only original selected source operations and original
    /// procedure allocations. Catalogue assistance cannot close this graph.
    fn callee_at(
        &self,
        tokens: &CommandTokens,
        callees: &mut BTreeSet<crate::ir::ProcedureId>,
    ) -> Option<()> {
        let source = self.source.as_ref()?;
        if source.procedure_call(tokens).is_some() {
            callees.insert(
                self.resolve_call_at(tokens, FoldTrust::ObservedBindings)?
                    .id,
            );
            return Some(());
        }
        source.operation(tokens).map(|_| ())
    }

    /// Summarise one strongly connected set of procedures, whose callees
    /// outside it are summarised already.
    fn summarise_component(
        &self,
        component: &[crate::ir::ProcedureId],
        calls: &HashMap<crate::ir::ProcedureId, BTreeSet<crate::ir::ProcedureId>>,
        inputs: &ModuleInputs<'_>,
        rounds: RecursionLimit,
    ) {
        let recursive = component.len() > 1
            || component
                .first()
                .is_some_and(|qname| calls[qname].contains(qname));
        // The component's outer writes rest only on the summaries of its
        // callees outside it, so they are known before its rounds run.
        let Some(outer) = self.component_outer_writes(component, calls, inputs) else {
            return;
        };
        let mut roles: Vec<Option<Vec<ParamRole>>> = vec![None; component.len()];
        let mut settled = !recursive;
        if recursive {
            for qname in component {
                self.provisional.borrow_mut().insert(*qname, None);
                if outer.is_empty() {
                    self.provisional_closed.borrow_mut().insert(*qname);
                }
            }
            for round in 1.. {
                if rounds.exceeded(round) {
                    break;
                }
                let next: Vec<Option<Vec<ParamRole>>> = component
                    .iter()
                    .map(|qname| self.roles(qname, inputs))
                    .collect();
                let changed = next != roles;
                roles = next;
                for (qname, roles) in component.iter().zip(&roles) {
                    self.provisional.borrow_mut().insert(*qname, roles.clone());
                }
                if !changed {
                    settled = true;
                    break;
                }
            }
            for qname in component {
                self.provisional.borrow_mut().remove(qname);
                self.provisional_closed.borrow_mut().remove(qname);
            }
        } else if let Some(qname) = component.first() {
            roles[0] = self.roles(qname, inputs);
        }
        let Some(outer_report) = outer
            .iter()
            .map(|(place, outcome)| {
                Some((
                    PlaceRef::scalar(place.symbol().checked_c_namespace_key()?),
                    *outcome,
                ))
            })
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        for (index, qname) in component.iter().enumerate() {
            let Some(declared) = inputs.ir.procedures.get(qname) else {
                continue;
            };
            let (params, completion) = if !settled {
                (
                    linked_roles(qname, declared, inputs, &self.frame_facts, &ANY_OUTCOME),
                    CompletionCodeDomain::Any,
                )
            } else if let Some(roles) = roles[index].take() {
                (roles, self.completion_of(calls, qname))
            } else {
                (
                    linked_roles(
                        qname,
                        declared,
                        inputs,
                        &self.frame_facts,
                        &[ExistenceOutcome::Preserve],
                    ),
                    CompletionCodeDomain::Exact(ERROR_ONLY),
                )
            };
            let Some(effects) = self.effects_of(qname, calls, inputs) else {
                continue;
            };
            let summary = TransferSummary {
                params,
                globals: outer_report.clone(),
                global_sources: outer.clone(),
                result: ReturnKind::Other,
                completion,
                effects,
                evidence: self.evidence_of(qname, calls),
                links: links_of(qname, declared, &self.frame_facts),
            };
            self.summaries.borrow_mut().insert(*qname, summary);
        }
    }

    /// The completion codes a call to `qname` may end with: an error or a
    /// normal completion, once every callee completes so.
    fn completion_of(
        &self,
        calls: &HashMap<crate::ir::ProcedureId, BTreeSet<crate::ir::ProcedureId>>,
        qname: &crate::ir::ProcedureId,
    ) -> CompletionCodeDomain {
        let summaries = self.summaries.borrow();
        let exact = calls[qname].iter().all(|callee| {
            callee == qname
                || summaries.get(callee).is_none_or(|summary| {
                    matches!(summary.completion, CompletionCodeDomain::Exact(_))
                })
        });
        if exact {
            CompletionCodeDomain::Exact(NORMAL_OR_ERROR)
        } else {
            CompletionCodeDomain::Any
        }
    }

    /// The world effects of the commands `qname` runs as statements, with
    /// its callees' summarised effects.
    fn effects_of(
        &self,
        qname: &crate::ir::ProcedureId,
        calls: &HashMap<crate::ir::ProcedureId, BTreeSet<crate::ir::ProcedureId>>,
        _inputs: &ModuleInputs<'_>,
    ) -> Option<EffectFootprint> {
        let source = self.source.as_ref()?;
        let mut effects = EffectFootprint::default();
        for tokens in source.body_commands(qname)? {
            if source.procedure_call(&tokens).is_none() {
                effects.extend(
                    source
                        .operation(&tokens)?
                        .facts
                        .world_state_effects()
                        .clone(),
                );
            }
        }
        let summaries = self.summaries.borrow();
        for callee in &calls[qname] {
            if let Some(summary) = summaries.get(callee) {
                effects.extend(summary.effects.clone());
            }
        }
        Some(effects)
    }

    /// The bindings `qname`'s summary rests on: each procedure it calls,
    /// and what each of their summaries rests on.
    fn evidence_of(
        &self,
        qname: &crate::ir::ProcedureId,
        calls: &HashMap<crate::ir::ProcedureId, BTreeSet<crate::ir::ProcedureId>>,
    ) -> DependencyEvidence {
        let summaries = self.summaries.borrow();
        let mut bindings: Vec<BindingIdentity> = Vec::new();
        for callee in &calls[qname] {
            if let Some(procedure) = self.procedures.get(callee) {
                bindings.push(procedure_binding(&procedure.qualified_name));
            }
            if let Some(summary) = summaries.get(callee) {
                bindings.extend(summary.evidence.bindings.iter().cloned());
            }
        }
        bindings.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        bindings.dedup();
        DependencyEvidence {
            bindings,
            ..DependencyEvidence::default()
        }
    }
}

/// Whether a procedure's caller-frame effects are all `Name` links its
/// summary can follow: each a level-1 `upvar` of one parameter's value onto
/// one local of its own, with no other frame reached.
fn frame_is_closed(frame: &UpvarInfo, params: &[String]) -> bool {
    let mut linked: HashSet<&str> = HashSet::new();
    let one_local_each = frame
        .param_targets
        .iter()
        .all(|(local, param)| !params.contains(local) && linked.insert(param.as_str()));
    one_local_each
        && !frame.has_unresolvable_caller_target
        && !frame.caller_frame_opaque_writes
        && !frame.caller_frame_opaque_reads
        && !frame.reaches_past_the_caller()
        && frame.literal_targets.is_empty()
        && frame.args_tail_upvar.is_empty()
        && frame.uplevel_literal_writes.is_empty()
        && frame.uplevel_param_writes.is_empty()
        && frame.uplevel_forwarded_calls.is_empty()
        && frame.unnameable_local_aliases.is_empty()
}

/// The binding a call to `qname` rests on.
fn procedure_binding(qname: &str) -> BindingIdentity {
    BindingIdentity {
        resolution_namespace: String::new(),
        name: qname.to_owned(),
        identity: format!("proc {qname}"),
    }
}

/// Whether `qname`'s body may run a core builtin the module rebinds: the
/// lowering turns a builtin into a typed statement with no call in it, so a
/// call to the replacement there is invisible to the summary's call walk. Any
/// spelling of the builtin's tail in the body counts, which also covers a
/// qualified or substituted spelling; a body without retained source text may
/// run any of them.
fn spells_a_rebound_builtin(
    module: &crate::ir::Module,
    qname: &crate::ir::ProcedureId,
    rebound: &HashSet<String>,
) -> bool {
    if rebound.is_empty() {
        return false;
    }
    let Some(body) = module
        .procedures
        .get(qname)
        .and_then(|procedure| procedure.body_source.as_deref())
    else {
        return true;
    };
    rebound
        .iter()
        .any(|name| body.contains(tcl_syntax::naming::key_tail(name)))
}

/// The strongly connected sets of `calls`' procedures, each after every set
/// it calls (Tarjan's algorithm, iterative).
fn components(
    names: &[crate::ir::ProcedureId],
    calls: &HashMap<crate::ir::ProcedureId, BTreeSet<crate::ir::ProcedureId>>,
) -> Vec<Vec<crate::ir::ProcedureId>> {
    struct Node {
        index: usize,
        low: usize,
        on_stack: bool,
    }
    let mut nodes: HashMap<crate::ir::ProcedureId, Node> = HashMap::new();
    let mut stack: Vec<crate::ir::ProcedureId> = Vec::new();
    let mut out: Vec<Vec<crate::ir::ProcedureId>> = Vec::new();
    let mut next = 0usize;
    for &root in names {
        if !calls.contains_key(&root) || nodes.contains_key(&root) {
            continue;
        }
        // Each frame: the node and the successors still to visit.
        let mut work: Vec<(crate::ir::ProcedureId, Vec<crate::ir::ProcedureId>)> = Vec::new();
        let successors = |qname: &crate::ir::ProcedureId| -> Vec<crate::ir::ProcedureId> {
            calls[qname]
                .iter()
                .rev()
                .filter_map(|callee| calls.get_key_value(callee).map(|(key, _)| *key))
                .collect()
        };
        nodes.insert(
            root,
            Node {
                index: next,
                low: next,
                on_stack: true,
            },
        );
        next += 1;
        stack.push(root);
        work.push((root, successors(&root)));
        while let Some((node, pending)) = work.last_mut() {
            let node = *node;
            if let Some(callee) = pending.pop() {
                match nodes.get(&callee) {
                    None => {
                        nodes.insert(
                            callee,
                            Node {
                                index: next,
                                low: next,
                                on_stack: true,
                            },
                        );
                        next += 1;
                        stack.push(callee);
                        work.push((callee, successors(&callee)));
                    }
                    Some(seen) if seen.on_stack => {
                        let index = seen.index;
                        let entry = nodes.get_mut(&node).expect("visited");
                        entry.low = entry.low.min(index);
                    }
                    Some(_) => {}
                }
                continue;
            }
            work.pop();
            let (index, low) = {
                let entry = &nodes[&node];
                (entry.index, entry.low)
            };
            if let Some((parent, _)) = work.last() {
                let parent = nodes.get_mut(parent).expect("visited");
                parent.low = parent.low.min(low);
            }
            if low == index {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    nodes.get_mut(&member).expect("visited").on_stack = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                component.sort_unstable();
                out.push(component);
            }
        }
    }
    out
}
