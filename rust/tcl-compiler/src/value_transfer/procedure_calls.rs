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

//! Calls to procedures of the module: transfer summaries applied to a call statement, and `[...]` commands re-run under the caller's stance.

use super::argument_words::OperandSource;
use super::lattice_inputs::{Words, const_to_exact, exact_to_lattice};
use super::{
    ArgWord, DefAnswer, ExistenceStep, Lattice, LatticeDriver, LatticeInputs, ScriptRun,
    binding_of, bound_to, literal_words, named_defs, place_named, record_binding, waiting,
};
use crate::analyses::LatticeValue;
use crate::ir::{CommandTokens, Statement};
use crate::sccp::{DefValues, FoldTrust, RaisedDefs};
use crate::ssa::{SsaFunction, ValueKey};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use tcl_lexer::LexerConfig;
use tcl_registry::InvocationWordKind;
use tcl_registry::value_transfer::{
    AnalysisInputs, AnalysisTier, BindingIdentity, BindingKind, CompletionOutcome, DeclineReason,
    DependencyEvidence, DomainFact, ExactValue, ExactValueOrUnavailable, Existence, FactDomain,
    FactView, InvocationLayout, InvocationOutcome, LiftedAnswer, NestedPolicy, OperandId,
    OperandView, PlaceRef, ResolvedInvocationView, StoreOutcome, TargetId, TypeFacts, WrittenPlace,
};

/// Each place a call names, with the step its callee's summary takes it by.
type PlaceSteps = [(String, ExistenceStep)];

/// What one call seeds a re-run of its callee with: each argument, exact
/// where the call's words prove it, and each `Name` link's place — its value,
/// where exact, and its existence.
type CallSeeds = (
    Vec<Option<ExactValue>>,
    Vec<(Option<ExactValue>, Existence)>,
);

/// The re-run a call statement asks of its callee.
enum CallRerun {
    /// The re-run, where one can be made.
    Made(Option<std::rc::Rc<crate::interprocedural::Rerun>>),
    /// A word or a place the solver has not reached yet.
    Waiting,
}

/// The step each place a call statement's `Name` arguments name takes from
/// its callee's transfer summary, the callee resolved from `function` under
/// the shared lattice's trust: what the read-before-set check reads of the
/// call. Empty for any other statement, and for a call the summary does not
/// answer or that never completes normally.
pub(crate) fn summary_steps(
    module: &crate::interprocedural::ModuleProcedures<'_>,
    _function: &str,
    statement: &Statement,
    _config: &LexerConfig,
) -> Vec<(String, ExistenceStep)> {
    let Statement::Call {
        tokens,
        foreach_groups: None,
        ..
    } = statement
    else {
        return Vec::new();
    };
    if statement.synthetic_marker().is_some() {
        return Vec::new();
    }
    let Some(call) = tokens
        .as_ref()
        .and_then(|tokens| module.resolve_call_at(tokens, FoldTrust::ObservedBindings))
    else {
        return Vec::new();
    };
    let Some(words) = module.original_call_arguments(&call) else {
        return Vec::new();
    };
    let words: Vec<Option<&str>> = words.iter().map(Option::as_deref).collect();
    match module.call_transfer(&call.id, &words) {
        Some(crate::interprocedural::CallTransfer::Places(places)) => places,
        _ => Vec::new(),
    }
}

/// The answer for each definition of a call statement to a procedure of the
/// module: a place a `Name` argument names takes the fact and the value the
/// re-run leaves in it, where one was made, else the summary's step and no
/// value; an outer place the callee may write takes the summary's step; and
/// any other definition is left may-bound with no value.
fn place_answers(
    defs: &[(String, ValueKey)],
    (places, outer): (&PlaceSteps, &PlaceSteps),
    after: Option<&crate::interprocedural::Rerun>,
) -> Vec<DefAnswer> {
    defs.iter()
        .map(|(name, key)| {
            if let Some(slot) = places.iter().position(|(place, _)| place == name) {
                let left = after.and_then(|after| after.places.get(slot));
                let existence = left
                    .and_then(|left| left.existence)
                    .map_or(places[slot].1, ExistenceStep::Set);
                let value = left
                    .filter(|left| matches!(left.existence, Some(Existence::Bound(_))))
                    .and_then(|left| left.value.as_ref())
                    .map_or(LatticeValue::Overdefined, exact_to_lattice);
                return DefAnswer {
                    stated: true,
                    existence,
                    ..DefAnswer::untyped(*key, value)
                };
            }
            let existence = outer
                .iter()
                .find(|(place, _)| place == name)
                .map_or(ExistenceStep::UNKNOWN, |(_, step)| *step);
            DefAnswer {
                existence,
                ..DefAnswer::untyped(*key, LatticeValue::Overdefined)
            }
        })
        .collect()
}

/// The view a `[…]` command to the procedure `callee` presents its words
/// through: each operand's text and kind, with no registry role.
fn procedure_view<'a>(
    callee: &'a str,
    texts: &[&'a str],
    cooked: &[ArgWord<'_>],
) -> ResolvedInvocationView<'a> {
    ResolvedInvocationView {
        canonical_command: callee,
        subcommand: None,
        form: None,
        layout: InvocationLayout::Source,
        operands: texts
            .iter()
            .zip(cooked)
            .map(|(text, word)| OperandView {
                text,
                kind: word.kind,
                role: None,
            })
            .collect(),
        argument_offset: 0,
        arity: None,
    }
}

/// The writes a `[…]` command answers under `policy`: none where it runs
/// effect-free, else `writes` as its one member.
fn member_writes(
    policy: NestedPolicy,
    writes: Vec<(PlaceRef, StoreOutcome)>,
) -> Vec<Vec<(PlaceRef, StoreOutcome)>> {
    if policy == NestedPolicy::EffectFreeOnly {
        Vec::new()
    } else {
        vec![writes]
    }
}

/// A `[…]` command to a procedure of the module that never completes
/// normally: an error before any store, resting on the procedure's binding.
fn procedure_raises(binding: &BindingIdentity) -> LiftedAnswer {
    let mut evidence = DependencyEvidence::default();
    record_binding(&mut evidence, binding.clone());
    LiftedAnswer::Evaluated(vec![Box::new(InvocationOutcome {
        completion: CompletionOutcome::error_unproven(0),
        result: ExactValueOrUnavailable::unproven_string(),
        nested_writes: Vec::new(),
        ordered_stores: Vec::new(),
        types: TypeFacts::default(),
        evidence,
    })])
}

/// The stores a `[…]` command to a procedure of the module makes: each place
/// a `Name` argument names takes what the re-run leaves in it, where one was
/// made, else the summary's step with no value, and each outer place the
/// callee may write takes the summary's step.
fn place_stores(
    (places, links): (&PlaceSteps, &[(usize, String)]),
    outer: &PlaceSteps,
    after: Option<&crate::interprocedural::Rerun>,
) -> Vec<(PlaceRef, StoreOutcome)> {
    let mut stores = Vec::new();
    for (slot, ((name, step), (param, _))) in places.iter().zip(links).enumerate() {
        let target = TargetId(OperandId(*param));
        let left = after.and_then(|after| after.places.get(slot));
        let made = match (
            left.and_then(|left| left.existence),
            left.and_then(|left| left.value.clone()),
        ) {
            (Some(Existence::Bound(_)), Some(value)) => vec![StoreOutcome::Write { target, value }],
            (Some(Existence::Bound(kind)), None) => vec![StoreOutcome::WriteUnavailable {
                target,
                facts: bound_to(kind),
            }],
            (Some(Existence::Unbound), _) => vec![StoreOutcome::Unbind { target }],
            _ => stores_of(*step, target),
        };
        stores.extend(made.into_iter().map(|store| (place_named(name), store)));
    }
    for (name, step) in outer {
        stores.extend(
            stores_of(*step, TargetId(OperandId(0)))
                .into_iter()
                .map(|store| (place_named(name), store)),
        );
    }
    stores
}

/// The stores that take a place as `step` says, with no value: a binding a
/// write whose value is unavailable, an unbinding, a preserve, a may-binding
/// a may-write, and a place left may-bound whatever it held an unbinding
/// then a may-write.
fn stores_of(step: ExistenceStep, target: TargetId) -> Vec<StoreOutcome> {
    match step {
        ExistenceStep::Set(Existence::Bound(kind)) => vec![StoreOutcome::WriteUnavailable {
            target,
            facts: bound_to(kind),
        }],
        ExistenceStep::Set(Existence::Unbound) => vec![StoreOutcome::Unbind { target }],
        ExistenceStep::Join(Existence::Bound(kind)) => vec![StoreOutcome::MayWrite {
            target,
            facts: bound_to(kind),
        }],
        ExistenceStep::Set(_) => vec![
            StoreOutcome::Unbind { target },
            StoreOutcome::MayWrite {
                target,
                facts: bound_to(BindingKind::Either),
            },
        ],
        ExistenceStep::Join(_) => vec![StoreOutcome::Preserve { target }],
    }
}

impl<'a> LatticeDriver<'a> {
    /// The module's procedures for a call whose head is `head`, where the
    /// run holds them. A run that holds none records that it would have read
    /// them where `head` may name a procedure a transfer summary answers for
    /// ([`crate::sccp::SccpResult::reads_module`]).
    fn procedures_for(
        &self,
        head: &str,
    ) -> Option<&'a crate::interprocedural::ModuleProcedures<'a>> {
        if self.module.is_none() && self.may_name_a_summary(head) {
            self.reads_module.set(true);
        }
        self.module
    }

    /// Whether `head` may name a procedure of the module a transfer summary
    /// answers for, as a run that holds no module can tell: the module has
    /// summaries only at the deep tier and where it rebinds no builtin,
    /// and a registry command whose binding the module leaves
    /// standing names none.
    fn may_name_a_summary(&self, head: &str) -> bool {
        let Some(folds) = self.folds else {
            return true;
        };
        self.context.tier == AnalysisTier::Deep
            && folds.mutations.nameable_rebound_builtins().is_some()
            && (!self.trusted(head) || self.registry.get(head).is_none())
    }

    /// The stance a re-run this run asks for is solved under: its own.
    fn rerun_stance(&self) -> Option<crate::interprocedural::RerunStance<'a>> {
        Some(crate::interprocedural::RerunStance {
            policy: self.policy,
            folds: self.folds?,
        })
    }

    /// A call statement to a procedure of the module, applied as the
    /// callee's transfer summary says (§ *Proc-level transfer summaries*):
    /// each place a `Name` argument names, and each outer place the callee
    /// may write, takes the existence the summary states; and, but in a
    /// summary's own run, a place takes the fact and the value a re-run of
    /// the callee leaves in it — its parameters holding the call's arguments
    /// and its links the places' facts before the call — where the re-run
    /// can be made. A call that never completes normally — a word count the
    /// callee's parameters reject, a callee that never completes, a re-run
    /// under the call's seeds that reaches no exit — is a certain raise
    /// where the run reads how the procedure completes
    /// ([`Self::never_completes`]), and a call whose completion no re-run
    /// decided is recorded there. `None` leaves the call to the generic
    /// answer: a head naming no procedure of the module whose binding
    /// stands, a callee whose summary does not answer the call, a call that
    /// defines nothing and may complete, and a statement a throw leaves
    /// from, whose partial writes no summary states.
    pub(crate) fn procedure_answer<S: std::hash::BuildHasher>(
        &self,
        (block, index): (&crate::ssa::SsaBlock, usize),
        values: &HashMap<ValueKey, LatticeValue, S>,
        ssa: &SsaFunction,
    ) -> Option<DefValues> {
        let stmt_ssa = block.statements.get(index)?;
        let Statement::Call {
            command,
            tokens,
            foreach_groups: None,
            ..
        } = &stmt_ssa.statement
        else {
            return None;
        };
        if std::env::var_os("ZZ").is_some() { eprintln!("ZZ call {command} defs={} reads_exits={} marker={:?} throwing={}", stmt_ssa.defs.len(), self.reads_exits, stmt_ssa.statement.synthetic_marker(), self.is_throwing()); }
        if (stmt_ssa.defs.is_empty() && !self.reads_exits)
            || stmt_ssa.statement.synthetic_marker().is_some()
            || self.is_throwing()
        {
            return None;
        }
        let Some(module) = self.procedures_for(command) else { if std::env::var_os("ZZ").is_some() { eprintln!("ZZ no-module {command}"); } return None; };
        let original = tokens.as_ref()?;
        let Some(call) = module.resolve_call_at(original, self.procedure_trust()) else { if std::env::var_os("ZZ").is_some() { eprintln!("ZZ no-resolve {command}"); } return None; };
        let callee = &call.id;
        self.reads_module.set(true);
        let defs = named_defs(stmt_ssa, ssa);
        let cooked =
            effective_procedure_arguments(&call.arguments, module.original_call_arguments(&call)?);
        if cooked
            .iter()
            .any(|word| word.kind == InvocationWordKind::Expanded)
        {
            if !defs.is_empty() {
                self.record_undecided();
            }
            return None;
        }
        let words = literal_words(&cooked);
        let Some(outer) = module.outer_steps(callee) else {
            if std::env::var_os("ZZ").is_some() { eprintln!("ZZ no-outer {command}"); } 
            self.record_undecided();
            return None;
        };
        let places = match module.call_transfer(callee, &words) {
            Some(crate::interprocedural::CallTransfer::Never) => {
                return Some(self.never_completes(&defs));
            }
            _ if defs.is_empty() => return None,
            Some(crate::interprocedural::CallTransfer::Places(places)) => places,
            None => {
                if std::env::var_os("ZZ").is_some() { eprintln!("ZZ no-transfer {command}"); } 
                self.record_undecided();
                return None;
            }
        };
        let after = if self.level == crate::sccp::ModuleLevel::Outcomes {
            None
        } else {
            let Some(rerun) = self.statement_rerun(
                (module, callee),
                (block, index),
                (&cooked, &places),
                (values, ssa),
            ) else {
                if std::env::var_os("ZZ").is_some() { eprintln!("ZZ no-rerun {command}"); } 
                self.record_undecided();
                return None;
            };
            match rerun {
                CallRerun::Made(after) => after,
                CallRerun::Waiting => {
                    return Some(DefValues::PerDef(waiting(&defs, &LatticeValue::Unknown)));
                }
            }
        };
        match after.as_deref() {
            Some(after) if !after.completes => return Some(self.never_completes(&defs)),
            Some(after) if after.decided => {}
            _ if self.level != crate::sccp::ModuleLevel::Outcomes => self.record_undecided(),
            _ => {}
        }
        if std::env::var_os("ZZ").is_some() { eprintln!("ZZ answer {command} {places:?} {outer:?} after={:?}", after.as_deref().map(|a| (a.completes, a.decided, a.places.iter().map(|p| (p.existence, p.value.clone())).collect::<Vec<_>>()))); }
        Some(DefValues::PerDef(place_answers(
            &defs,
            (&places, &outer),
            after.as_deref(),
        )))
    }

    /// A call statement that never completes normally: where a raise ends
    /// the path a certain raise before any store, so no exit past it is
    /// read; elsewhere each definition waits on an existence the call never
    /// gives, and no code after the call is proved unreachable by it.
    fn never_completes(&self, defs: &[(String, ValueKey)]) -> DefValues {
        if self.raises_end_paths() {
            DefValues::Raised(Box::new(RaisedDefs {
                prefix: None,
                written: 0,
            }))
        } else {
            DefValues::PerDef(waiting(defs, &LatticeValue::Overdefined))
        }
    }

    /// The re-run of `callee` a call statement asks for: its parameters
    /// holding the statement's words, exact where the solver proves them,
    /// and its links the places the call names (`places`, in the callee's
    /// link order), as the solver holds them before the call. `None` where
    /// a place is not a variable of the frame the call spells.
    fn statement_rerun<S: std::hash::BuildHasher>(
        &self,
        (module, callee): (
            &crate::interprocedural::ModuleProcedures<'a>,
            &crate::ir::ProcedureId,
        ),
        (block, index): (&crate::ssa::SsaBlock, usize),
        (cooked, places): (&[ArgWord<'_>], &PlaceSteps),
        (values, ssa): (&HashMap<ValueKey, LatticeValue, S>, &SsaFunction),
    ) -> Option<CallRerun> {
        let original = block.statements.get(index)?.statement.tokens()?;
        let call = module.resolve_call_at(original, self.procedure_trust())?;
        let mut inputs = self.expression_inputs((&block.statements.get(index)?.uses, values, ssa));
        inputs.original = Some(original.clone());
        inputs.written_arguments = (0..cooked.len())
            .map(|argument| call.arguments.written_argument(argument))
            .collect();
        let mut arguments = Vec::with_capacity(cooked.len());
        for (index, word) in cooked.iter().enumerate() {
            arguments.push(match word.kind {
                InvocationWordKind::Literal => Some(ExactValue::from_literal(&word.text)),
                _ if word.source == OperandSource::Substituted => {
                    match inputs.substituted_argument(&word.text, index) {
                        FactView::Exact(value, _) => Some(value),
                        FactView::Pending => return Some(CallRerun::Waiting),
                        _ => None,
                    }
                }
                _ => None,
            });
        }
        let mut seeds = Vec::new();
        for (name, _) in places {
            let symbol = ssa.var_symbol(name)?;
            let prior = crate::sccp::prior_version(block, index, symbol);
            let value = match values.get(&(symbol, prior)) {
                Some(LatticeValue::Unknown) => return Some(CallRerun::Waiting),
                Some(LatticeValue::Const(value)) => Some(const_to_exact(value)),
                _ => None,
            };
            // With no existence rung, a procedure's entry version of a name
            // is bound for a parameter and unbound for any other.
            // The top level has no parameters, and no entry rule.
            let entry = self
                .body_id
                .and_then(crate::sccp::AnalysisBody::procedure_id)
                .and_then(|id| module.params_of(&id))
                .filter(|_| prior == 0)
                .map(|params| {
                    if params.iter().any(|param| param == name) {
                        Existence::Bound(BindingKind::Scalar)
                    } else {
                        Existence::Unbound
                    }
                });
            let existence = match (self.existence_now(symbol), entry) {
                (Some(Existence::Pending), _) => return Some(CallRerun::Waiting),
                (Some(fact), _) | (None, Some(fact)) => fact,
                (None, None) if value.is_some() => Existence::Bound(BindingKind::Scalar),
                (None, None) => Existence::MayBound,
            };
            seeds.push((value, existence));
        }
        Some(CallRerun::Made(self.rerun_with(
            (module, callee),
            &arguments,
            &seeds,
        )))
    }

    /// `callee` re-run under this run's stance, its parameters holding
    /// `arguments` — an omitted one its default — and its links `seeds`.
    fn rerun_with(
        &self,
        (module, callee): (
            &crate::interprocedural::ModuleProcedures<'a>,
            &crate::ir::ProcedureId,
        ),
        arguments: &[Option<ExactValue>],
        seeds: &[(Option<ExactValue>, Existence)],
    ) -> Option<std::rc::Rc<crate::interprocedural::Rerun>> {
        module
            .parameter_values(callee, arguments)
            .zip(self.rerun_stance())
            .and_then(|(params, stance)| module.rerun(callee, (&params, seeds), stance))
    }

    /// One `[…]` command naming a procedure of the module, run as the
    /// callee's transfer summary says: its words first, then a write to each
    /// place a `Name` argument names — the value a re-run of the callee
    /// leaves there where one is made, else what the summary bounds — and
    /// its result where the run takes results ([`crate::sccp::ModuleLevel::
    /// Results`]). A command that never completes normally raises before it
    /// stores, where a raise ends the path, and one whose completion no
    /// re-run decided is recorded there. `None` leaves the command to the
    /// registry: a head naming no procedure of the module; below `Results`,
    /// a command whose writes nothing would carry (the effect-free policy);
    /// and a protected script or a statement a throw leaves from, since a
    /// summary says how a call may complete, not that it completes normally.
    pub(super) fn procedure_run<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        seg: &crate::segmenter::SegmentedCommand,
        original: Option<&CommandTokens>,
        (uses, values, ssa): Lattice<'_, S1, S2>,
        (prior, policy): (&[(PlaceRef, StoreOutcome)], NestedPolicy),
    ) -> Option<ScriptRun> {
        let head = seg.name();
        let original = original?;
        if self.leaves_procedure_run(original, policy) {
            return None;
        }
        let module = self.procedures_for(head)?;
        let call = module.resolve_call_at(original, self.procedure_trust())?;
        let callee = &call.id;
        self.reads_module.set(true);
        let binding = binding_of(head, &call.procedure.qualified_name);
        let answered = |answer, writes| {
            Some(ScriptRun {
                head: head.to_owned(),
                route: None,
                answer,
                writes,
                binding: binding.clone(),
                rebound: false,
                procedure: true,
            })
        };
        let declined = |reason| answered(LiftedAnswer::Declined(reason), Vec::new());
        // A command that never completes normally raises before it stores
        // anything, where a raise ends the path.
        let raises = || {
            answered(
                procedure_raises(&binding),
                member_writes(policy, Vec::new()),
            )
        };
        let cooked =
            effective_procedure_arguments(&call.arguments, module.original_call_arguments(&call)?);
        if cooked
            .iter()
            .any(|word| word.kind == InvocationWordKind::Expanded)
        {
            self.record_undecided();
            return None;
        }
        let words = literal_words(&cooked);
        let Some(outer) = module.outer_steps(callee) else {
            self.record_undecided();
            return declined(DeclineReason::Unsupported);
        };
        let places = match module.call_transfer(callee, &words) {
            Some(crate::interprocedural::CallTransfer::Places(places)) => places,
            Some(crate::interprocedural::CallTransfer::Never) if self.raises_end_paths() => {
                return raises();
            }
            _ => {
                self.record_undecided();
                return None;
            }
        };
        if policy == NestedPolicy::EffectFreeOnly && (!places.is_empty() || !outer.is_empty()) {
            self.record_undecided();
            return declined(DeclineReason::StatefulNested);
        }
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let inputs = LatticeInputs {
            original: Some(original.clone()),
            written_arguments: (0..cooked.len())
                .map(|index| call.arguments.written_argument(index))
                .collect(),
            expression_base: None,
            expression_source: RefCell::new(None),
            nested_source: RefCell::new(None),
            driver: self,
            prior_writes: prior.to_vec(),
            words: Words::under(policy),
            view: procedure_view(&call.procedure.qualified_name, &texts, &cooked),
            uses,
            values,
            ssa,
            point: None,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        let links = module.links(callee)?;
        let (arguments, seeds) = match self.run_seeds(&inputs, (words.len(), &places), ssa) {
            Ok(seeds) => seeds,
            Err(answer) => return answered(answer, Vec::new()),
        };
        let after = self.rerun_with((module, callee), &arguments, &seeds);
        match after.as_deref() {
            Some(after) if !after.completes => {
                if self.raises_end_paths() {
                    return raises();
                }
                return declined(DeclineReason::Unsupported);
            }
            Some(after) if after.decided => {}
            _ => self.record_undecided(),
        }
        let mut writes = inputs.words.writes();
        writes.extend(place_stores((&places, &links), &outer, after.as_deref()));
        answered(
            self.procedure_outcome(after.as_deref(), &binding),
            member_writes(policy, writes),
        )
    }

    /// Whether [`Self::procedure_run`] leaves a `[…]` command headed `head`
    /// to the registry before resolving it: in a protected script or a
    /// statement a throw leaves from, and, below `Results`, where its writes
    /// nothing would carry (the effect-free policy) — there a call to a
    /// procedure of the module runs nowhere, so its completion is one no
    /// re-run decided.
    fn leaves_procedure_run(&self, original: &CommandTokens, policy: NestedPolicy) -> bool {
        if policy == NestedPolicy::Protected || self.is_throwing() {
            return true;
        }
        if policy != NestedPolicy::EffectFreeOnly || self.level == crate::sccp::ModuleLevel::Results
        {
            return false;
        }
        if self.module.is_some_and(|module| {
            module
                .resolve_call_at(original, self.procedure_trust())
                .is_some()
        }) {
            self.record_undecided();
        }
        true
    }

    /// A `[…]` command to a procedure of the module that ran: it completes
    /// normally, with the result the re-run proves where the run takes
    /// results, and otherwise one the source does not give.
    fn procedure_outcome(
        &self,
        after: Option<&crate::interprocedural::Rerun>,
        binding: &BindingIdentity,
    ) -> LiftedAnswer {
        let result = match (after, self.level) {
            (Some(after), crate::sccp::ModuleLevel::Results) => after.result.clone().map_or_else(
                || ExactValueOrUnavailable::Unavailable(bound_to(BindingKind::Scalar)),
                ExactValueOrUnavailable::Exact,
            ),
            _ => ExactValueOrUnavailable::Unavailable(bound_to(BindingKind::Scalar)),
        };
        let mut evidence = DependencyEvidence::default();
        record_binding(&mut evidence, binding.clone());
        LiftedAnswer::Evaluated(vec![Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result,
            nested_writes: Vec::new(),
            ordered_stores: Vec::new(),
            types: TypeFacts::default(),
            evidence,
        })])
    }

    /// The arguments and the `Name` places a `[…]` command's re-run is
    /// seeded with, as its words leave them: every operand of the `count`
    /// first, the substituting ones evaluated in order, then each place the
    /// call names (`places`, in the callee's link order) after their
    /// writes. `Err` with the command's answer where a word or a place waits
    /// on the solver.
    fn run_seeds<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        inputs: &LatticeInputs<'_, S1, S2>,
        (count, places): (usize, &PlaceSteps),
        ssa: &SsaFunction,
    ) -> Result<CallSeeds, LiftedAnswer> {
        let mut arguments = Vec::with_capacity(count);
        for index in 0..count {
            arguments.push(
                match inputs.operand(OperandId(index), FactDomain::ExactValue) {
                    FactView::Exact(value, _) => Some(value),
                    FactView::Pending => return Err(LiftedAnswer::Pending),
                    _ => None,
                },
            );
        }
        let mut seeds = Vec::with_capacity(places.len());
        for (name, _) in places {
            let name = name.as_str();
            let value = match inputs.words_written(name) {
                WrittenPlace::Exact(value) => FactView::Exact(value, None),
                WrittenPlace::Unknown => FactView::Top(DeclineReason::NotExact),
                WrittenPlace::Untouched => inputs.named_fact(name),
            };
            let value = match value {
                FactView::Exact(value, _) => Some(value),
                FactView::Pending => return Err(LiftedAnswer::Pending),
                _ => None,
            };
            let existence = match inputs.written_existence(name) {
                Some(FactView::Domain(DomainFact::Existence(fact))) => fact,
                Some(_) => Existence::MayBound,
                None => match self.existence_fact(ssa, name) {
                    FactView::Domain(DomainFact::Existence(Existence::Pending)) => {
                        return Err(LiftedAnswer::Pending);
                    }
                    FactView::Domain(DomainFact::Existence(fact)) => fact,
                    _ if value.is_some() => Existence::Bound(BindingKind::Scalar),
                    _ => Existence::MayBound,
                },
            };
            seeds.push((value, existence));
        }
        Ok((arguments, seeds))
    }
}

pub(super) fn effective_procedure_arguments(
    effective: &crate::registry_invocation::EffectiveCommandWords,
    literals: Vec<Option<String>>,
) -> Vec<ArgWord<'_>> {
    effective
        .words
        .iter()
        .skip(1)
        .zip(literals)
        .map(|(word, literal)| match literal {
            Some(text) => ArgWord {
                text: Cow::Owned(text),
                kind: InvocationWordKind::Literal,
                source: if matches!(word, crate::ir::WordExpr::BracedLiteral { .. }) {
                    OperandSource::BracedLiteral
                } else {
                    OperandSource::Literal
                },
            },
            None => ArgWord {
                text: Cow::Owned(word.legacy_text()),
                kind: InvocationWordKind::Dynamic,
                source: OperandSource::Substituted,
            },
        })
        .collect()
}
