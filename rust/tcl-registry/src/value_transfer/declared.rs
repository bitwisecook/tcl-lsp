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

//! A pack's value-transfer declarations: what the `semantics` and `evaluate`
//! statements of a `.tclspec` state at one command, subcommand or form scope
//! (`docs/design/compiler/value-evaluation.md` § *The `semantics`,
//! `evaluate`, and `facts` rows*), as the registry-owned specialisation the
//! driver resolves exactly as it resolves a shipped one. Nothing here names
//! a pack command: the loader builds one [`DeclaredSemantics`] per declaring
//! scope, and the driver reads it through [`CommandSemantics`].
//!
//! An operand index a declaration writes (`arg N`, `-targets {N …}`) counts
//! from the resolved form's first argument, as an `arg` row at the same
//! scope does, so a declaration reads the same whether its command is
//! spelled `tenant::label NAME` or `tenant label NAME`.

use crate::completion::CompletionCode;
use crate::invocation_words::InvocationWordKind;
use crate::pack_hooks::{self, EvaluationAnswer, HookAnswer, HookCall, HookSlot, HookWord};
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    Binder, BinderName, BindingKind, BodyPlan, CompletionOutcome, CompletionProtocol,
    DependencyEvidence, EvalAnswer, ExactValue, ExactValueOrUnavailable, Existence, ExitRule,
    FactBounds, InvocationOutcome, IterableKind, IterationPlan, PlanAnswer, RouteIdentity,
    StoreOutcome, TypeFacts,
};
use super::const_ops::{ConstOps, TargetSemantics};
use super::context::Budget;
use super::decline::{Axis, DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, FactDomain, InvocationLayout, OperandId, PlaceRef, TargetId};
use super::route::{ContextDependency, DeclaredInput, EvalRoute, EvaluatorCapability};

/// The completion codes a declared loop body absorbs.
const LOOP_ABSORBED: &[CompletionCode] = &[CompletionCode::Break, CompletionCode::Continue];

/// One effect a `semantics { effects {…} }` row declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeclaredEffect {
    /// The command writes no variable (`no_store_writes`).
    NoStoreWrites,
    /// The command touches nothing outside the interpreter
    /// (`no_external_io`).
    NoExternalIo,
}

impl DeclaredEffect {
    /// Every effect word.
    pub const ALL: &'static [Self] = &[Self::NoStoreWrites, Self::NoExternalIo];

    /// The DSL spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoStoreWrites => "no_store_writes",
            Self::NoExternalIo => "no_external_io",
        }
    }
}

/// A declared semantic type (`result -semantic T`, `yield -semantic T`): a
/// Tcl type, or a vendor semantic the analyser carries and never reads as a
/// Tcl type (`vendor.object_handle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticType {
    /// A Tcl value type, spelled in lower case (`string`, `int`).
    Tcl(TclType),
    /// A vendor semantic, spelled `vendor.NAME`.
    Vendor(&'static str),
}

/// How a declared target is written (`stores -targets {…} -outcome O`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutcomeKind {
    /// Every target is written on every normal completion (`write`).
    Write,
    /// Each target is written or keeps its prior value and existence
    /// (`write_or_preserve`).
    WriteOrPreserve,
    /// Each target may have been written; nothing more is known
    /// (`may_write`).
    MayWrite,
    /// Each target is unbound afterwards (`unbind`).
    Unbind,
}

impl OutcomeKind {
    /// Every outcome word.
    pub const ALL: &'static [Self] = &[
        Self::Write,
        Self::WriteOrPreserve,
        Self::MayWrite,
        Self::Unbind,
    ];

    /// The DSL spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::WriteOrPreserve => "write_or_preserve",
            Self::MayWrite => "may_write",
            Self::Unbind => "unbind",
        }
    }
}

/// The places a declaration writes and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclaredStores {
    /// The target operand indices, in declaration order.
    pub targets: &'static [usize],
    /// How each is written.
    pub outcome: OutcomeKind,
}

/// What a declared loop iterates (`iterable -arg N -kind K`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IterableWord {
    /// A Tcl list (`list`).
    List,
    /// A dict (`dict`).
    Dict,
    /// A vendor collection handle, never read as a Tcl list
    /// (`vendor.NAME`).
    Vendor(&'static str),
}

/// An `iterate {…}` block: one binder stepping over one iterable, with the
/// body run in the enclosing frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclaredIteration {
    /// The binder operand (`binder -arg N`).
    pub binder: usize,
    /// The iterable operand (`iterable -arg N`).
    pub iterable: usize,
    /// What the iterable is (`-kind K`).
    pub kind: IterableWord,
    /// The body operand (`body -arg N`), when the layout carries one.
    pub body: Option<usize>,
    /// What each iteration binds (`yield -semantic S`).
    pub yields: Option<SemanticType>,
    /// The operand that counts the collection (`cardinality -arg N`).
    pub cardinality: Option<usize>,
    /// Whether the binder is bound on the zero-iteration path
    /// (`zero_iterations -bindings bind`; `preserve` keeps its prior value
    /// and existence).
    pub zero_iterations_bind: bool,
}

/// The structural half of a declaration: the `semantics { … }` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DeclaredStructure {
    /// The declared effects.
    pub effects: &'static [DeclaredEffect],
    /// The result's semantic type.
    pub result: Option<SemanticType>,
    /// The places written.
    pub stores: Option<DeclaredStores>,
    /// The iteration protocol.
    pub iterate: Option<DeclaredIteration>,
}

/// A declared implementation (`evaluate -implementation ID -host HOST {…}`):
/// the capability it states, and the hook slot its body is bound to once a
/// host plan assigned one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclaredImplementation {
    /// Everything the declaration states about itself.
    pub capability: EvaluatorCapability,
    /// The body's slot, or `None` while no host plan has bound it.
    pub slot: Option<HookSlot>,
}

/// The evaluation half of a declaration: the `evaluate` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeclaredEvaluation {
    /// A route the statement names directly: `evaluate none`, `evaluate
    /// -expression ID`, or no `evaluate` statement at all
    /// (`None { Unauthored }`).
    Route(EvalRoute),
    /// A declared implementation.
    Implementation(DeclaredImplementation),
}

/// What a pack declares at one scope: the specialisation the loader leaks
/// onto the spec as `SemanticsDeclaration::Declared`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclaredSemantics {
    /// The scope the declaration hangs off, the `SCOPE` of the id rule
    /// (`tenant::label`, `tenant::label` for its `label` subcommand of
    /// `tenant`): the specialisation's identity.
    pub scope: &'static str,
    /// The `semantics { … }` block.
    pub structure: DeclaredStructure,
    /// The `evaluate` statement.
    pub evaluation: DeclaredEvaluation,
    /// The options whose presence switches the route off (`option -NAME
    /// -evaluate none ?-evaluate-reason WORD?` at this scope), each with the
    /// decline the driver records.
    pub option_declines: &'static [(&'static str, DeclineReason)],
}

impl DeclaredSemantics {
    /// The same declaration with its implementation's body bound to `slot`
    /// and its identity naming `pack`: what the host plan installs.
    #[must_use]
    pub fn bound(mut self, pack: &'static str, slot: HookSlot) -> Self {
        if let DeclaredEvaluation::Implementation(implementation) = &mut self.evaluation {
            implementation.capability.identity.pack = pack;
            implementation.slot = Some(slot);
        }
        self
    }

    /// The declared implementation, when the route is one.
    #[must_use]
    pub const fn implementation(&self) -> Option<&DeclaredImplementation> {
        match &self.evaluation {
            DeclaredEvaluation::Implementation(implementation) => Some(implementation),
            DeclaredEvaluation::Route(_) => None,
        }
    }

    /// The operand an `arg N` of this declaration names in `input`'s
    /// invocation: `N` counted from the resolved form's first argument.
    #[must_use]
    pub fn operand(input: &dyn AnalysisInputs, index: usize) -> OperandId {
        OperandId(input.invocation().argument_offset + index)
    }

    /// The decline an option this scope switches off records when the
    /// invocation carries it. Every argument word must be exact to prove
    /// the option absent, so an unknown word is the stand-in answer its
    /// fact gives (`Pending`, `NotExact`); a word spelling the option in a
    /// value position declines too, which is the conservative direction.
    #[must_use]
    pub fn option_decline(&self, input: &dyn AnalysisInputs) -> Option<EvalAnswer> {
        let view = input.invocation();
        if self.option_declines.is_empty() || view.layout != InvocationLayout::Source {
            return None;
        }
        for index in view.argument_offset..view.operands.len() {
            let value = match input
                .operand(OperandId(index), FactDomain::ExactValue)
                .exact()
            {
                Ok(value) => value,
                Err(answer) => return Some(answer),
            };
            if let Some((_, reason)) = self
                .option_declines
                .iter()
                .find(|(option, _)| option.as_bytes() == value.bytes.as_slice())
            {
                return Some(EvalAnswer::Declined(*reason));
            }
        }
        None
    }

    /// The declared targets the body speaks for, as it names them.
    fn targets(&self) -> &'static [usize] {
        self.structure.stores.map_or(&[], |stores| stores.targets)
    }

    /// Each declared target's place, in declaration order: every target
    /// must be a place the analyser can own, and no two may overlap.
    fn target_places(
        &self,
        input: &dyn AnalysisInputs,
    ) -> Result<Vec<(usize, TargetId, PlaceRef)>, DeclineReason> {
        let mut places: Vec<(usize, TargetId, PlaceRef)> = Vec::new();
        for &index in self.targets() {
            let target = TargetId(Self::operand(input, index));
            let place = input.place(target.0)?;
            if places
                .iter()
                .any(|(_, _, seen)| overlaps(&seen.name, &place.name))
            {
                return Err(DeclineReason::OverlappingTargets);
            }
            places.push((index, target, place));
        }
        Ok(places)
    }

    /// One declared input as the text its body parameter binds: an
    /// operand's exact value, a target's incoming exact value, or an
    /// option's value as a list of zero or one element — empty when the
    /// option is absent, so an absent option and an empty value stay
    /// distinguishable. An input that is not exact is the stand-in answer
    /// its fact gives, never a placeholder.
    fn input_text(
        input: &dyn AnalysisInputs,
        declared: DeclaredInput,
        target: &TargetSemantics,
    ) -> Result<String, EvalAnswer> {
        let view = input.invocation();
        let exact = match declared {
            DeclaredInput::Operand { index, .. } => {
                let id = Self::operand(input, index);
                if view.operand(id).is_none() {
                    // The call has no such word: the command raises, and an
                    // error is never a value.
                    return Err(EvalAnswer::Declined(DeclineReason::Unsupported));
                }
                input.operand(id, FactDomain::ExactValue).exact()?
            }
            DeclaredInput::IncomingTarget { index } => {
                let place = input
                    .place(Self::operand(input, index))
                    .map_err(EvalAnswer::Declined)?;
                input.prior_store(&place, FactDomain::ExactValue).exact()?
            }
            DeclaredInput::OptionValue { name } => {
                let mut found: Option<usize> = None;
                for index in view.argument_offset..view.operands.len() {
                    let word = input
                        .operand(OperandId(index), FactDomain::ExactValue)
                        .exact()?;
                    if word.bytes.as_slice() == name.as_bytes() {
                        if found.is_some() {
                            // Twice: which occurrence the command reads is
                            // its own grammar's business.
                            return Err(EvalAnswer::Declined(DeclineReason::Unsupported));
                        }
                        found = Some(index);
                    }
                }
                let Some(at) = found else {
                    return Ok(String::new());
                };
                if view.operand(OperandId(at + 1)).is_none() {
                    return Err(EvalAnswer::Declined(DeclineReason::Unsupported));
                }
                let value = input
                    .operand(OperandId(at + 1), FactDomain::ExactValue)
                    .exact()?;
                let text = value.as_str().map_err(EvalAnswer::Declined)?;
                return target.render_list(&[text]).ok_or(EvalAnswer::Declined(
                    DeclineReason::ReleaseAmbiguous(Axis::ListRendering),
                ));
            }
        };
        exact
            .as_str()
            .map(str::to_owned)
            .map_err(EvalAnswer::Declined)
    }

    /// Run the declared implementation on this invocation: every declared
    /// input resolved to an exact value, the body invoked through this
    /// thread's host under the declaration's budget, and its verbs mapped
    /// to an outcome. A host that is absent or has quarantined the body
    /// declines `Transient` — a state of the host, never a verdict on the
    /// inputs; a body that raises, stays silent, or leaves a target
    /// unstated declines `Unsupported`, under the DSL's "error means
    /// abstain" rule.
    fn evaluate_implementation(
        &self,
        implementation: &DeclaredImplementation,
        input: &dyn AnalysisInputs,
        budget: &mut Budget,
    ) -> EvalAnswer {
        let capability = &implementation.capability;
        let view = input.invocation();
        // A declaration's indices are positions; once a word expands, no
        // position is known.
        if view
            .operands
            .iter()
            .any(|operand| operand.kind == InvocationWordKind::Expanded)
        {
            return EvalAnswer::Declined(DeclineReason::NotExact);
        }
        let context = input.context();
        let target = TargetSemantics::of(context.profile);
        // The body runs on an engine pinned to the analysed release; a
        // profile that names none has no engine to run it on.
        let (Some(profile), Some(release)) = (context.profile, target.release) else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        if let Err(reason) = ConstOps::admit(context, budget, capability.target) {
            return EvalAnswer::Declined(reason);
        }
        let places = match self.target_places(input) {
            Ok(places) => places,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let mut words: Vec<String> = Vec::with_capacity(capability.inputs.len());
        for &declared in capability.inputs {
            match Self::input_text(input, declared, &target) {
                Ok(text) => words.push(text),
                Err(answer) => return answer,
            }
        }
        let Some(slot) = implementation.slot else {
            // No host plan bound the body: nothing on this worker can run it.
            return EvalAnswer::Declined(DeclineReason::Transient);
        };
        if !pack_hooks::slot_available(slot) {
            return EvalAnswer::Declined(DeclineReason::Transient);
        }
        let hook_words: Vec<HookWord<'_>> = words
            .iter()
            .map(|value| HookWord {
                value,
                kind: InvocationWordKind::Literal,
            })
            .collect();
        let call = HookCall {
            words: &hook_words,
            version: Some(release),
            in_event_body: false,
            option: None,
            constraints: None,
            dialect: Some(profile.name),
            targets: self.targets(),
            budget: capability.budget,
            depends: capability.depends,
        };
        let _earlier = pack_hooks::take_commands_spent();
        let dispatched = pack_hooks::dispatch(slot, &call);
        // The body already ran, so its commands are charged whatever it
        // answered, one unit each; a request this exhausts declines the
        // evaluations after this one, not the answer it paid for.
        let _charged = budget.charge_work(pack_hooks::take_commands_spent());
        let answer = match dispatched {
            HookAnswer::Evaluation(answer) => answer,
            HookAnswer::Abstain if !pack_hooks::slot_available(slot) => {
                // The call itself cost the body its slot: its budget blew
                // or it panicked, and the host quarantined it.
                return EvalAnswer::Declined(DeclineReason::Transient);
            }
            HookAnswer::Abstain => return EvalAnswer::Declined(DeclineReason::Unsupported),
            // A host that answers in another family's shape is a host bug:
            // it degrades, never mis-answers.
            _ => return EvalAnswer::Declined(DeclineReason::MalformedAnswer),
        };
        self.outcome(capability, &places, answer, release, budget)
    }

    /// The body's verbs as an outcome: `fold` the result, `write` and
    /// `preserve` the ordered stores, each against the declared outcome.
    fn outcome(
        &self,
        capability: &EvaluatorCapability,
        places: &[(usize, TargetId, PlaceRef)],
        answer: EvaluationAnswer,
        release: tcl_dialect::TclVersion,
        budget: &mut Budget,
    ) -> EvalAnswer {
        let outcome_kind = self.structure.stores.map(|stores| stores.outcome);
        let mut ordered_stores = Vec::with_capacity(answer.stores.len());
        for (named, value) in answer.stores {
            let Some((_, target, _)) = places.iter().find(|(index, _, _)| *index == named) else {
                // The verbs refuse a non-target; one arriving here is a
                // host defect.
                return EvalAnswer::Declined(DeclineReason::MalformedAnswer);
            };
            let store = match (value, outcome_kind) {
                (_, Some(OutcomeKind::Unbind) | None) | (None, Some(OutcomeKind::Write)) => {
                    // A verb cannot state an unbind, and a `write` outcome
                    // has no preserve path: the answer contradicts the
                    // declaration.
                    return EvalAnswer::Declined(DeclineReason::MalformedAnswer);
                }
                (Some(value), _) => {
                    if let Err(reason) = budget.charge_result(byte_count(&value)) {
                        return EvalAnswer::Declined(reason);
                    }
                    StoreOutcome::Write {
                        target: *target,
                        value: ExactValue::from_literal(&value),
                    }
                }
                (None, _) => StoreOutcome::Preserve { target: *target },
            };
            ordered_stores.push(store);
        }
        let result = match answer.result {
            Some(value) => {
                if let Err(reason) = budget.charge_result(byte_count(&value)) {
                    return EvalAnswer::Declined(reason);
                }
                ExactValueOrUnavailable::Exact(ExactValue::from_literal(&value))
            }
            // A body that spoke for its targets and not its result proved
            // the path, not the text: silence is never an empty result.
            None => ExactValueOrUnavailable::Unavailable(FactBounds {
                existence: Existence::Bound(BindingKind::Scalar),
                intrep: None,
                shape: None,
                segments: None,
                taint: None,
            }),
        };
        let bindings = capability
            .depends
            .iter()
            .filter_map(|dependency| match dependency {
                ContextDependency::Binding(binding) => Some(binding.clone()),
                _ => None,
            })
            .collect();
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result,
            ordered_stores,
            types: TypeFacts {
                result: match self.structure.result {
                    Some(SemanticType::Tcl(ty)) => Some(ty),
                    Some(SemanticType::Vendor(_)) | None => None,
                },
                per_target: Vec::new(),
                shapes: Vec::new(),
            },
            evidence: DependencyEvidence {
                bindings,
                route: Some(RouteIdentity {
                    route: EvalRoute::Implementation(*capability),
                    implementation: capability.identity.id,
                    revision: capability.identity.content_hash,
                }),
                release: Some(release),
                ..DependencyEvidence::default()
            },
        }))
    }

    fn iteration_plan(iteration: &DeclaredIteration, input: &dyn AnalysisInputs) -> PlanAnswer {
        let view = input.invocation();
        let completion = CompletionProtocol::Absorb(LOOP_ABSORBED);
        let iterable_at = |operand: OperandId| match iteration.kind {
            IterableWord::List => IterableKind::List(operand),
            IterableWord::Dict => IterableKind::Dict(operand),
            IterableWord::Vendor(_) => IterableKind::Vendor {
                collection: operand,
                cardinality: iteration
                    .cardinality
                    .map(|index| Self::operand(input, index)),
            },
        };
        match view.layout {
            // The synthetic loop header carries the iterable alone, its
            // binders the names the header defines.
            InvocationLayout::LoopHeader { binders } => PlanAnswer::Iterate(IterationPlan {
                binders: binders
                    .iter()
                    .map(|name| Binder {
                        name: BinderName::Declared(name.clone()),
                        kind: BindingKind::Scalar,
                    })
                    .collect(),
                iterable: iterable_at(OperandId(0)),
                body: None,
                exit: ExitRule::Exhaustion,
                zero_iterations_bind: iteration.zero_iterations_bind,
                completion,
            }),
            InvocationLayout::Source => PlanAnswer::Iterate(IterationPlan {
                binders: vec![Binder {
                    name: BinderName::Operand(Self::operand(input, iteration.binder)),
                    kind: BindingKind::Scalar,
                }],
                iterable: iterable_at(Self::operand(input, iteration.iterable)),
                body: iteration.body.map(|index| BodyPlan {
                    body: Self::operand(input, index),
                    frame: crate::frame_effect::FrameLevel::Relative(0),
                }),
                exit: ExitRule::Exhaustion,
                zero_iterations_bind: iteration.zero_iterations_bind,
                completion,
            }),
        }
    }
}

impl CommandSemantics for DeclaredSemantics {
    fn identity(&self) -> &'static str {
        self.scope
    }

    fn route(&self) -> EvalRoute {
        match self.evaluation {
            DeclaredEvaluation::Route(route) => route,
            DeclaredEvaluation::Implementation(implementation) => {
                EvalRoute::Implementation(implementation.capability)
            }
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        match &self.structure.iterate {
            Some(iteration) => Self::iteration_plan(iteration, input),
            None => PlanAnswer::NoStructure,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        if let Some(answer) = self.option_decline(input) {
            return answer;
        }
        match &self.evaluation {
            DeclaredEvaluation::Implementation(implementation) => {
                self.evaluate_implementation(implementation, input, budget)
            }
            // The driver runs a named route itself; what reaches here
            // evaluates nothing, with the route's own reason.
            DeclaredEvaluation::Route(EvalRoute::None { reason }) => {
                EvalAnswer::Declined(DeclineReason::NoRoute(*reason))
            }
            DeclaredEvaluation::Route(_) => EvalAnswer::Declined(DeclineReason::Unsupported),
        }
    }

    fn as_declared(&self) -> Option<&DeclaredSemantics> {
        Some(self)
    }

    fn incoming_targets(&self, input: &dyn AnalysisInputs) -> Vec<TargetId> {
        self.implementation()
            .into_iter()
            .flat_map(|implementation| implementation.capability.inputs)
            .filter_map(|declared| match declared {
                DeclaredInput::IncomingTarget { index } => {
                    Some(TargetId(Self::operand(input, *index)))
                }
                DeclaredInput::Operand { .. } | DeclaredInput::OptionValue { .. } => None,
            })
            .collect()
    }
}

/// A published value's size, for the result-bytes budget.
fn byte_count(value: &str) -> u64 {
    u64::try_from(value.len()).unwrap_or(u64::MAX)
}

/// Whether two places share storage: the same name, or an array and one
/// of its elements. Two elements of one array are distinct places.
fn overlaps(left: &str, right: &str) -> bool {
    let base = |name: &str| {
        name.split_once('(')
            .map_or(name, |(base, _)| base)
            .to_owned()
    };
    left == right || (base(left) == base(right) && (!left.contains('(') || !right.contains('(')))
}

/// A declaration with no `evaluate` statement: the structure alone, and
/// the route every pure command without an evaluator has.
pub const UNAUTHORED: DeclaredEvaluation = DeclaredEvaluation::Route(EvalRoute::None {
    reason: NoRouteReason::Unauthored,
});

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;
    use crate::pack_hooks::{HookFamily, HookInput, HookInputs, PackHookHost};
    use crate::value_transfer::const_ops::Needs;
    use crate::value_transfer::context::{AnalysisContext, BindingIdentity};
    use crate::value_transfer::inputs::{
        BodyRegion, EvaluationState, FactView, OperandView, ResolvedInvocationView, WordStructure,
    };
    use crate::value_transfer::route::{
        CompletionSupport, Exactness, HostKind, ImplementationBudget, ImplementationIdentity,
    };

    /// `kv::put KEY VAR`, with `VAR` holding `prior` before the call.
    struct PutInputs {
        view: ResolvedInvocationView<'static>,
        prior: RefCell<String>,
        context: AnalysisContext,
    }

    impl AnalysisInputs for PutInputs {
        fn invocation(&self) -> &ResolvedInvocationView<'_> {
            &self.view
        }

        fn operand(&self, id: OperandId, _domain: FactDomain) -> FactView {
            self.view
                .operand(id)
                .map_or(FactView::Top(DeclineReason::NotExact), |operand| {
                    FactView::Exact(ExactValue::from_literal(operand.text), None)
                })
        }

        fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
            self.view
                .operand(id)
                .map(|operand| PlaceRef::scalar(operand.text))
                .ok_or(DeclineReason::NotExact)
        }

        fn variable(&self, _name: &str, _domain: FactDomain) -> FactView {
            FactView::Top(DeclineReason::NotExact)
        }

        fn prior_store(&self, _place: &PlaceRef, _domain: FactDomain) -> FactView {
            FactView::Exact(ExactValue::from_literal(&self.prior.borrow()), None)
        }

        fn word_structure(&self, _id: OperandId) -> Result<WordStructure, DeclineReason> {
            Err(DeclineReason::Unsupported)
        }

        fn body(&self, _id: OperandId) -> Result<BodyRegion, DeclineReason> {
            Err(DeclineReason::Unsupported)
        }

        fn nested(&self, _script: &str, _state: &mut EvaluationState) -> EvalAnswer {
            EvalAnswer::Declined(DeclineReason::Unsupported)
        }

        fn math_function(&self, _name: &str) -> Result<BindingIdentity, DeclineReason> {
            Err(DeclineReason::Unsupported)
        }

        fn context(&self) -> &AnalysisContext {
            &self.context
        }
    }

    /// The body `{key prior} { preserve 1; fold $key=$prior }`, counting
    /// its runs.
    struct JoinHost {
        calls: Cell<u32>,
    }

    impl PackHookHost for JoinHost {
        fn invoke(&self, _slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
            self.calls.set(self.calls.get() + 1);
            HookAnswer::Evaluation(EvaluationAnswer {
                result: Some(
                    call.words
                        .iter()
                        .map(|word| word.value)
                        .collect::<Vec<_>>()
                        .join("="),
                ),
                stores: vec![(1, None)],
            })
        }
    }

    fn literal(text: &'static str) -> OperandView<'static> {
        OperandView {
            text,
            kind: InvocationWordKind::Literal,
            role: None,
        }
    }

    /// An incoming target is part of what a cached answer rests on: the
    /// same key and target value is a hit, and a changed target value is a
    /// miss that runs the body again and answers for the new value, never
    /// the old one.
    #[test]
    fn a_changed_incoming_target_misses_the_cache() {
        let slot = pack_hooks::allocate(
            HookFamily::Evaluate,
            &HookInputs::declared([HookInput::Words]),
        )
        .expect("an evaluate slot");
        let put = DeclaredSemantics {
            scope: "kv::put",
            structure: DeclaredStructure {
                stores: Some(DeclaredStores {
                    targets: &[1],
                    outcome: OutcomeKind::WriteOrPreserve,
                }),
                ..DeclaredStructure::default()
            },
            evaluation: DeclaredEvaluation::Implementation(DeclaredImplementation {
                capability: EvaluatorCapability {
                    identity: ImplementationIdentity {
                        pack: "kv",
                        id: "kv.put.v1",
                        content_hash: 1,
                    },
                    host: HostKind::BoundedTcl,
                    target: Needs::NONE,
                    inputs: &[
                        DeclaredInput::Operand {
                            index: 0,
                            exactness: Exactness::Exact,
                        },
                        DeclaredInput::IncomingTarget { index: 1 },
                    ],
                    depends: &[],
                    budget: ImplementationBudget::default(),
                    completion: CompletionSupport::NormalOnly,
                },
                slot: Some(slot),
            }),
            option_declines: &[],
        };
        let inputs = PutInputs {
            view: ResolvedInvocationView {
                canonical_command: "kv::put",
                subcommand: None,
                form: None,
                layout: InvocationLayout::Source,
                operands: vec![literal("k"), literal("v")],
                argument_offset: 0,
            },
            prior: RefCell::new("a".to_owned()),
            context: AnalysisContext::detached(tcl_dialect::DialectProfile::find("tcl9.0")),
        };
        let host = Rc::new(JoinHost {
            calls: Cell::new(0),
        });
        pack_hooks::install_host(host.clone());
        let evaluate = |inputs: &PutInputs| match put.evaluate(inputs, &mut Budget::evaluation()) {
            EvalAnswer::Evaluated(outcome) => {
                assert_eq!(
                    outcome.ordered_stores,
                    [StoreOutcome::Preserve {
                        target: TargetId(OperandId(1))
                    }]
                );
                match outcome.result {
                    ExactValueOrUnavailable::Exact(value) => {
                        String::from_utf8(value.bytes).expect("text")
                    }
                    ExactValueOrUnavailable::Unavailable(bounds) => {
                        panic!("an exact result, not {bounds:?}")
                    }
                }
            }
            other => panic!("an outcome: {other:?}"),
        };
        assert_eq!(evaluate(&inputs), "k=a");
        assert_eq!(evaluate(&inputs), "k=a");
        assert_eq!(host.calls.get(), 1, "an unchanged target is a hit");
        inputs.prior.replace("b".to_owned());
        assert_eq!(evaluate(&inputs), "k=b");
        assert_eq!(host.calls.get(), 2, "a changed target misses");
        pack_hooks::clear_host();
    }
}
