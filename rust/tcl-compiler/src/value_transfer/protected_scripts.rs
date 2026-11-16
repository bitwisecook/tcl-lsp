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

//! Protected scripts and loops: every command in order under one evaluation state, with the writes the state admits.

use super::argument_words::{OperandSource, view_of};
use super::lattice_inputs::Words;
use super::{
    ArgWord, LatticeDriver, LatticeInputs, ScriptRun, binding_of, place_named, record_binding,
    split_head,
};
use crate::ir::CommandTokens;
use std::cell::RefCell;
use tcl_registry::value_transfer::{
    AnalysisInputs, BinderName, BindingKind, Budget, BudgetLimit, CompletionOutcome, DeclineReason,
    DependencyEvidence, DomainFact, EvalAnswer, EvaluationState, ExactValue,
    ExactValueOrUnavailable, Existence, ExitRule, FactDomain, FactView, InvocationLayout,
    InvocationOutcome, IterableKind, IterationPlan, LiftedAnswer, LoopStep, NestedPolicy,
    OperandId, PlaceRef, PlanAnswer, StoreOutcome, TargetId, TargetSemantics, TypeFacts,
};
use tcl_registry::{FrameLevel, ResolvedInvocation};

impl LatticeDriver<'_> {
    /// A protected script: every command it holds, in order, under one state
    /// — each as under [`NestedPolicy::LocalWrites`] — until one completes
    /// other than normally, which is the script's completion, with the writes
    /// that ran before it in the state. The result is the last command's, the
    /// empty one for a script with no command. A command that declines, a
    /// value that is not exact and a script that does not parse decline the
    /// whole: the completion of a script is the first of its commands that
    /// does not complete normally, and each must be known to be past. So must
    /// each store a command made: a route takes a store the analysis does not
    /// prove failing as the normal completion's, which a statement's normal
    /// path may, but a script whose completion is observed declines where a
    /// store's place is not proven to take it ([`proved_stores`]). A command
    /// whose plan iterates runs as its plan says ([`Self::protected_loop`]).
    ///
    /// An error a command's own word raises is not read here: that command
    /// declines, and the flag a statement's word raising sets is the
    /// statement's, kept as it was.
    pub(super) fn protected_script<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        script: &str,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> EvalAnswer {
        let original = from.nested_source.borrow().clone();
        if original
            .as_ref()
            .is_some_and(|owner| owner.text() != script)
        {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            script,
            original.as_ref().map_or(
                0,
                super::super::interprocedural::OriginalSummaryScript::base,
            ),
            tcl_lexer::SourceSegmentationPurpose::NestedScript.config(self.lexer_config),
        );
        if commands.iter().any(|seg| seg.is_partial) {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let outer_word_error = self.word_error.replace(false);
        let answer = self.run_protected(&commands, script, state, from, original.as_ref());
        self.word_error.set(outer_word_error);
        answer
    }

    fn run_protected<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        commands: &[crate::segmenter::SegmentedCommand],
        script: &str,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
        original: Option<&crate::interprocedural::OriginalSummaryScript>,
    ) -> EvalAnswer {
        let mut result = ExactValueOrUnavailable::exact_text("");
        let mut completion = CompletionOutcome::Normal;
        for seg in commands {
            // The command's own text: a head the source writes as the
            // command's name, which a `;` may follow (`break;`).
            let base = original.map_or(
                0,
                super::super::interprocedural::OriginalSummaryScript::base,
            );
            let at = usize::try_from(seg.span.start().saturating_sub(base)).unwrap_or(usize::MAX);
            let end = usize::try_from(seg.span.end().saturating_sub(base)).unwrap_or(usize::MAX);
            if script.get(at..end).map(|text| split_head(text).0) != Some(seg.name()) {
                return EvalAnswer::Declined(DeclineReason::Unsupported);
            }
            let before = state.writes.len();
            let (ended, value) = match self.protected_loop(
                seg,
                state,
                from,
                original.and_then(|owner| owner.command_at(seg.span.start())),
            ) {
                Some(Ok(ran)) => ran,
                Some(Err(answer)) => return answer,
                None => {
                    let prior: Vec<(PlaceRef, StoreOutcome)> = from
                        .prior_writes
                        .iter()
                        .chain(&state.writes)
                        .cloned()
                        .collect();
                    let Some(run) = self.run_command(
                        seg,
                        (from.uses, from.values, from.ssa),
                        (prior, NestedPolicy::LocalWrites),
                        original.and_then(|owner| owner.command_at(seg.span.start())),
                    ) else {
                        return EvalAnswer::Declined(DeclineReason::Unsupported);
                    };
                    match self.settle(run, state) {
                        Ok(outcome) => (outcome.completion, outcome.result),
                        Err(answer) => return answer,
                    }
                }
            };
            if let Err(answer) = proved_stores(&state.writes, before, from) {
                return answer;
            }
            match ended {
                CompletionOutcome::Normal => result = value,
                CompletionOutcome::Error {
                    message,
                    error_code,
                    ..
                } => {
                    completion = CompletionOutcome::Error {
                        written: state.writes.len(),
                        message,
                        error_code,
                    };
                    break;
                }
                code @ CompletionOutcome::Code { .. } => {
                    completion = code;
                    break;
                }
            }
        }
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion,
            result,
            nested_writes: Vec::new(),
            ordered_stores: Vec::new(),
            types: TypeFacts::default(),
            evidence: DependencyEvidence::default(),
        }))
    }

    /// A command of a protected script whose plan iterates — `foreach`,
    /// `lmap`, a pack's loop — run as its plan says ([`Self::run_loop`]), with
    /// its head's binding recorded in the state's evidence: the loop's
    /// completion and result. `None` for a command no iteration plan
    /// describes, which runs as any command does, and for one whose words are
    /// not text the source spells: a word's own writes run before the loop,
    /// and the runner orders none of them.
    fn protected_loop<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        seg: &crate::segmenter::SegmentedCommand,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
        original: Option<&CommandTokens>,
    ) -> Option<Result<(CompletionOutcome, ExactValueOrUnavailable), EvalAnswer>> {
        let head = seg.name();
        if let Some(module) = self.module {
            let original = original?;
            return module.with_original_operation(original, |operation, schema| {
                let cooked = self.operation_arguments(operation)?;
                let written = (0..cooked.len())
                    .map(|index| operation.effective.written_argument(index))
                    .collect();
                self.protected_loop_resolved(
                    head,
                    schema,
                    (&cooked, written, Some(original)),
                    state,
                    from,
                )
            });
        }
        if !self.trusted(head) {
            return None;
        }
        let cooked = self.cooked_args(seg);
        let words: Vec<_> = cooked.iter().map(ArgWord::word).collect();
        let resolved = self.resolve(head, &words)?;
        let written = (0..cooked.len()).map(Some).collect();
        self.protected_loop_resolved(head, &resolved, (&cooked, written, original), state, from)
    }

    fn protected_loop_resolved<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        head: &str,
        resolved: &ResolvedInvocation<'_, '_>,
        (cooked, written_arguments, original): (
            &[ArgWord<'_>],
            Vec<Option<usize>>,
            Option<&CommandTokens>,
        ),
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> Option<Result<(CompletionOutcome, ExactValueOrUnavailable), EvalAnswer>> {
        if !cooked.iter().all(|arg| {
            matches!(
                arg.source,
                OperandSource::BracedLiteral
                    | OperandSource::Literal
                    | OperandSource::QuotedLiteral
            )
        }) {
            return None;
        }
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<_> = cooked.iter().map(ArgWord::word).collect();
        let semantics = resolved.semantics.value.semantics()?;
        let inputs = LatticeInputs {
            original: original.cloned(),
            written_arguments,
            expression_base: None,
            expression_source: RefCell::new(None),
            nested_source: RefCell::new(None),
            driver: self,
            prior_writes: from
                .prior_writes
                .iter()
                .chain(&state.writes)
                .cloned()
                .collect(),
            words: Words::Independent,
            view: view_of(resolved, &texts, &words, InvocationLayout::Source),
            uses: from.uses,
            values: from.values,
            ssa: from.ssa,
            point: None,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        let PlanAnswer::Iterate(plan) = semantics.structure(&inputs) else {
            return None;
        };
        record_binding(
            &mut state.evidence,
            binding_of(head, resolved.canonical_command),
        );
        let depth = self.nesting.get();
        if depth >= Budget::EVALUATION_DEPTH {
            return Some(Err(EvalAnswer::Declined(DeclineReason::Budget(
                BudgetLimit::Depth,
            ))));
        }
        self.nesting.set(depth + 1);
        let ran = self.run_loop(&plan, &inputs, state, from);
        self.nesting.set(depth);
        Some(ran)
    }

    /// The loop `plan` describes over `inputs`, its iterations run in
    /// `state`: each binder written in order, padded with the empty string
    /// past the list's end, then the body as a protected script, whose
    /// completion the plan's rule reads ([`IterationPlan::step`]) — the next
    /// iteration, the loop's end, or the loop's own completion — and the
    /// loop's result is what its result rule makes of what the iterations
    /// yielded ([`LoopResult::of`](tcl_registry::value_transfer::LoopResult::of)).
    /// No iteration binds nothing. Each iteration is charged to the
    /// evaluation's budget by the length of its body.
    ///
    /// A plan the runner cannot follow declines: an iterable that is not a
    /// list the analysis knows exactly, a body that is not brace-quoted text
    /// run in the current frame, a binder that is not a scalar place the
    /// state owns, an exit other than the iterable's exhaustion, and binders
    /// bound on the zero-iteration path. A binder is a scalar store, taken
    /// where the script's stores are ([`held_before`]): one the analysis
    /// proves an array raises an error after the writes before it, with its
    /// message and `-errorcode` unproven, and one whose kind it does not
    /// prove declines.
    fn run_loop<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        plan: &IterationPlan,
        inputs: &LatticeInputs<'_, S1, S2>,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> Result<(CompletionOutcome, ExactValueOrUnavailable), EvalAnswer> {
        let declined = |reason| Err(EvalAnswer::Declined(reason));
        let (Some(body), IterableKind::List(list), ExitRule::Exhaustion, false) = (
            &plan.body,
            &plan.iterable,
            plan.exit,
            plan.zero_iterations_bind,
        ) else {
            return declined(DeclineReason::Unsupported);
        };
        if body.frame != FrameLevel::Relative(0) || plan.binders.is_empty() {
            return declined(DeclineReason::Unsupported);
        }
        let places = plan
            .binders
            .iter()
            .map(|binder| self.binder_place(binder, inputs))
            .collect::<Result<Vec<_>, _>>()
            .map_err(EvalAnswer::Declined)?;
        let text = match inputs.operand(*list, FactDomain::ExactValue) {
            FactView::Exact(value, _) => value.as_str().map_err(EvalAnswer::Declined)?.to_owned(),
            FactView::Pending => return Err(EvalAnswer::Pending),
            FactView::Top(reason) => return declined(reason),
            FactView::Finite(..) | FactView::Domain(_) => return declined(DeclineReason::NotExact),
        };
        // A list that does not parse is the command's error, which the
        // runner does not word.
        let Ok(items) = self.policy.word_rules.split_list(&text) else {
            return declined(DeclineReason::Unsupported);
        };
        let script = inputs.body(body.body).map_err(EvalAnswer::Declined)?.script;
        let mut budget = self.budget();
        let cost = u64::try_from(script.len()).unwrap_or(u64::MAX).max(1);
        let mut yielded = Vec::new();
        for iteration in 0..items.len().div_ceil(places.len()) {
            budget.charge_work(cost).map_err(EvalAnswer::Declined)?;
            for (at, place) in places.iter().enumerate() {
                // A binder takes its element as any store of the script does
                // ([`held_before`]): a proven array raises there.
                if held_before(place.base(), &state.writes, from)? == Held::Array {
                    return Ok((
                        CompletionOutcome::error_unproven(state.writes.len()),
                        ExactValueOrUnavailable::unproven_string(),
                    ));
                }
                let value = items
                    .get(iteration * places.len() + at)
                    .map_or("", AsRef::as_ref);
                state.writes.push((
                    place.clone(),
                    StoreOutcome::Write {
                        target: TargetId(OperandId(0)),
                        value: ExactValue::from_literal(value),
                    },
                ));
            }
            let ran = match self.protected_script(&script, state, inputs) {
                EvalAnswer::Evaluated(ran) => ran,
                other => return Err(other),
            };
            match plan.step(&ran.completion) {
                LoopStep::Next => yielded.push(ran.result),
                LoopStep::Skip => {}
                LoopStep::Exit => break,
                LoopStep::Leave => return Ok((ran.completion, ran.result)),
            }
        }
        let target = TargetSemantics::of(self.context.profile);
        Ok((CompletionOutcome::Normal, plan.result.of(&yielded, &target)))
    }

    /// The place a loop's binder names: a scalar the state owns, named by
    /// the plan or by an operand. A qualified name, an element and a place
    /// the state cannot own (traced or escaping) are refused.
    fn binder_place<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        binder: &tcl_registry::value_transfer::Binder,
        inputs: &LatticeInputs<'_, S1, S2>,
    ) -> Result<PlaceRef, DeclineReason> {
        if binder.kind != BindingKind::Scalar {
            return Err(DeclineReason::Unsupported);
        }
        let place = match &binder.name {
            BinderName::Declared(name) if !tcl_syntax::naming::is_qualified(name.as_bytes()) => {
                place_named(name)
            }
            BinderName::Declared(_) => return Err(DeclineReason::Unsupported),
            BinderName::Operand(id) => inputs.place(*id)?,
        };
        if place.is_element() || tcl_syntax::naming::is_qualified(place.name.as_bytes()) {
            return Err(DeclineReason::Unsupported);
        }
        if !self.state_owns(&place) {
            return Err(DeclineReason::StatefulNested);
        }
        Ok(place)
    }

    /// What a command run for `state` leaves in it: its binding and every
    /// binding its answer rests on recorded in the state's evidence, and its
    /// writes applied to the state in order — when the policy admits them —
    /// so the next read sees them. An outcome that differs between the
    /// members of a finite input declines as correlated: the host evaluation
    /// holds one value per operand.
    ///
    /// Under [`NestedPolicy::EffectFreeOnly`] only an effect-free outcome is
    /// admitted — any store is `StatefulNested`. Under the other policies the
    /// outcome's writes are applied when the state can own every place it
    /// writes ([`Self::state_owns`]) and its completion says how many ran
    /// ([`Self::admitted_writes`]).
    pub(super) fn settle(
        &self,
        run: ScriptRun,
        state: &mut EvaluationState,
    ) -> Result<Box<InvocationOutcome>, EvalAnswer> {
        record_binding(&mut state.evidence, run.binding);
        let outcomes = match run.answer {
            LiftedAnswer::Pending => return Err(EvalAnswer::Pending),
            LiftedAnswer::Declined(reason) => return Err(EvalAnswer::Declined(reason)),
            LiftedAnswer::Evaluated(outcomes) => outcomes,
        };
        if state.policy == NestedPolicy::EffectFreeOnly
            && outcomes.iter().any(|outcome| outcome.has_stores())
        {
            return Err(EvalAnswer::Declined(DeclineReason::StatefulNested));
        }
        // A run that placed no writes (an effect-free one) has none per
        // outcome.
        let mut placed = run.writes.into_iter();
        let mut members = outcomes
            .into_iter()
            .map(|outcome| (outcome, placed.next().unwrap_or_default()));
        let Some((first, first_writes)) = members.next() else {
            return Err(EvalAnswer::Declined(DeclineReason::Unsupported));
        };
        if members.any(|(outcome, writes)| {
            outcome.result != first.result
                || outcome.completion != first.completion
                || writes != first_writes
        }) {
            return Err(EvalAnswer::Declined(DeclineReason::CorrelatedSets));
        }
        if state.policy != NestedPolicy::EffectFreeOnly {
            match self.admitted_writes(&first_writes, &first) {
                Ok(ran) => state.writes.extend(first_writes.into_iter().take(ran)),
                Err(reason) => return Err(EvalAnswer::Declined(reason)),
            }
        }
        for binding in &first.evidence.bindings {
            record_binding(&mut state.evidence, binding.clone());
        }
        Ok(first)
    }

    /// How many of a nested outcome's `placed` writes the ordered state
    /// applies, or why it cannot apply them: a write the state can own
    /// (`Write`, `WriteElement`, `WriteUnavailable`) and a preserve, which
    /// leaves its place as it was; nothing else. The completion is exact when
    /// it is the normal one, which ran every write, an error, which ran the
    /// first `written` of them, or a code, whose command stores nothing of
    /// its own, so every write is its words'.
    ///
    /// # Errors
    ///
    /// `StatefulNested` for a write to a place the state cannot own, a
    /// may-write or an unbind, and a code completion that stores.
    fn admitted_writes(
        &self,
        placed: &[(PlaceRef, StoreOutcome)],
        outcome: &InvocationOutcome,
    ) -> Result<usize, DeclineReason> {
        let ran = match &outcome.completion {
            CompletionOutcome::Normal => placed.len(),
            CompletionOutcome::Error { written, .. } => (*written).min(placed.len()),
            CompletionOutcome::Code { .. } if outcome.ordered_stores.is_empty() => placed.len(),
            CompletionOutcome::Code { .. } => return Err(DeclineReason::StatefulNested),
        };
        for (place, store) in &placed[..ran] {
            let admitted = match store {
                StoreOutcome::Preserve { .. } => true,
                StoreOutcome::Write { .. }
                | StoreOutcome::WriteElement { .. }
                | StoreOutcome::WriteUnavailable { .. } => self.state_owns(place),
                StoreOutcome::Unbind { .. } | StoreOutcome::MayWrite { .. } => false,
            };
            if !admitted {
                return Err(DeclineReason::StatefulNested);
            }
        }
        Ok(ran)
    }
}

/// What a variable holds, as a store into it needs to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    /// No binding.
    Nothing,
    /// A scalar.
    Scalar,
    /// An array.
    Array,
}

/// Whether each store a protected script's state took from `from_index` on is
/// one its place certainly takes, given the state's writes before it and the
/// fact before the script ([`held_before`]): a scalar store into a variable
/// that holds nothing or a scalar, an element store into one that holds
/// nothing or an array. Tcl raises on any other, so where the analysis does
/// not prove the variable's kind the store may fail and the script's
/// completion is not exact.
fn proved_stores<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    writes: &[(PlaceRef, StoreOutcome)],
    from_index: usize,
    from: &LatticeInputs<'_, S1, S2>,
) -> Result<(), EvalAnswer> {
    for at in from_index..writes.len() {
        let (place, store) = &writes[at];
        if matches!(store, StoreOutcome::Preserve { .. }) {
            continue;
        }
        let held = held_before(place.base(), &writes[..at], from)?;
        let takes = if place.is_element() {
            held != Held::Scalar
        } else {
            held != Held::Array
        };
        if !takes {
            return Err(EvalAnswer::Declined(DeclineReason::NotExact));
        }
    }
    Ok(())
}

/// What the variable `name` holds after `earlier`, a protected script's writes
/// so far: the last of them that reaches it decides — a scalar write a scalar,
/// an element write an array — and with none, the fact before the script.
/// A write that may have happened, and a kind no fact proves, decline.
fn held_before<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    name: &str,
    earlier: &[(PlaceRef, StoreOutcome)],
    from: &LatticeInputs<'_, S1, S2>,
) -> Result<Held, EvalAnswer> {
    let unproven = || Err(EvalAnswer::Declined(DeclineReason::NotExact));
    for (place, store) in earlier.iter().rev() {
        if place.base() != name || matches!(store, StoreOutcome::Preserve { .. }) {
            continue;
        }
        return match store {
            StoreOutcome::Unbind { .. } if !place.is_element() => Ok(Held::Nothing),
            StoreOutcome::Write { .. }
            | StoreOutcome::WriteElement { .. }
            | StoreOutcome::WriteUnavailable { .. } => Ok(if place.is_element() {
                Held::Array
            } else {
                Held::Scalar
            }),
            _ => unproven(),
        };
    }
    match from.prior_store(&PlaceRef::scalar(name), FactDomain::Existence) {
        FactView::Domain(DomainFact::Existence(Existence::Unbound)) => Ok(Held::Nothing),
        FactView::Domain(DomainFact::Existence(Existence::Bound(BindingKind::Scalar))) => {
            Ok(Held::Scalar)
        }
        FactView::Domain(DomainFact::Existence(Existence::Bound(BindingKind::Array))) => {
            Ok(Held::Array)
        }
        FactView::Pending | FactView::Domain(DomainFact::Existence(Existence::Pending)) => {
            Err(EvalAnswer::Pending)
        }
        _ => unproven(),
    }
}
