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

//! Conditional representation observations over the declaration owner's graph.
//!
//! Slots retain source naming geometry. Values and conversions remain conditional
//! on their held original-handler and ordinary-object premises; they supply no
//! physical object, reached read, Normal completion or executable fact.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use tcl_lexer::Span;
use tcl_registry::TclType;
use tcl_registry::iteration_entry::ConditionalSourceFiniteIteration;
use tcl_registry::representation::ConditionalSourceListIndexCoercion;
use tcl_registry::value_transfer::{
    ConditionalSourceRepresentation, ConditionalSourceTargetUpdate, OperandId, TargetId,
};

use crate::registry_invocation::source_structure::OriginalRegistryWords;
use crate::shimmer::commit::CommitState;
use crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol;

/// One evaluated operand snapshot, independent of later writes to its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct OperandSnapshot {
    pub site: u32,
    pub operand: OperandId,
}

pub(super) enum CaptureSource {
    Unknown,
    Literal {
        bytes: Vec<u8>,
    },
    LastResult,
    Variable {
        symbol: SignatureSourceVariableSymbol,
        span: Span,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct LoopId(pub usize);

/// The declaration owner retains its existing conditional loop topology.
pub(super) struct LoopRegion {
    pub id: LoopId,
    pub site: u32,
    pub span: Span,
    pub source: Arc<OriginalRegistryWords>,
    pub count: ConditionalSourceFiniteIteration,
    pub first_gate: usize,
    pub bindings: usize,
    pub repeated_entry: usize,
    pub header: usize,
    pub normal_exit: usize,
    pub error_exit: usize,
    pub returned_exit: usize,
    pub other_exit: usize,
}

pub(super) struct PriorTargetUpdate {
    pub relation: ConditionalSourceTargetUpdate,
    pub read_span: Span,
}

/// Events are issued only after the parent authenticates the same source scope,
/// original operands, registry relation and ordinary-cell applicability.
pub(super) enum Event {
    ClearResult,
    BodyContext {
        context: Arc<super::ConditionalRepresentationBodyContext>,
    },
    RequireBodyContext {
        context: Arc<super::ConditionalRepresentationBodyContext>,
    },
    Capture {
        into: OperandSnapshot,
        from: CaptureSource,
        source: Arc<OriginalRegistryWords>,
    },
    Apply {
        site: u32,
        span: Span,
        relation: ConditionalSourceRepresentation,
        target: Option<(TargetId, SignatureSourceVariableSymbol)>,
        prior_target: Option<PriorTargetUpdate>,
        source: Arc<OriginalRegistryWords>,
    },
    ListIndex {
        site: u32,
        span: Span,
        coercion: ConditionalSourceListIndexCoercion,
        source: Arc<OriginalRegistryWords>,
    },
    Reset {
        targets: Vec<SignatureSourceVariableSymbol>,
        source: Arc<OriginalRegistryWords>,
    },
    Control {
        source: Arc<OriginalRegistryWords>,
    },
    Barrier,
}

#[derive(Clone, PartialEq, Eq)]
struct SourceCycleObservation {
    loop_span: Span,
    iterations: usize,
    update_span: Span,
    target_read_span: Span,
}

/// A source-only conversion with every producer, copy and consumer premise.
/// The constructor is private to this projection; no runtime proof is exposed.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SourceConversionObservation {
    span: Span,
    producer: Span,
    symbol: SignatureSourceVariableSymbol,
    from_type: TclType,
    to_type: TclType,
    consumer: Arc<OriginalRegistryWords>,
    premises: Vec<Arc<OriginalRegistryWords>>,
    body_contexts: Vec<Arc<super::ConditionalRepresentationBodyContext>>,
    cycle: Option<SourceCycleObservation>,
}

impl std::fmt::Debug for SourceConversionObservation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SourceConversionObservation")
            .field("span", &self.span)
            .field("producer", &self.producer)
            .field("premises", &self.premises.len())
            .field("body_contexts", &self.body_contexts.len())
            .finish_non_exhaustive()
    }
}

impl SourceConversionObservation {
    pub(crate) const fn span(&self) -> Span {
        self.span
    }

    pub(crate) const fn producer_span(&self) -> Span {
        self.producer
    }

    pub(crate) fn symbol(&self) -> &SignatureSourceVariableSymbol {
        &self.symbol
    }

    pub(crate) fn consumer(&self) -> &OriginalRegistryWords {
        &self.consumer
    }

    pub(crate) fn premises(&self) -> &[Arc<OriginalRegistryWords>] {
        &self.premises
    }

    #[cfg(test)]
    pub(super) fn body_contexts(&self) -> &[Arc<super::ConditionalRepresentationBodyContext>] {
        &self.body_contexts
    }

    pub(crate) const fn prior_type(&self) -> TclType {
        self.from_type
    }

    pub(crate) const fn to_type(&self) -> TclType {
        self.to_type
    }

    pub(crate) const fn is_loop_carried(&self) -> bool {
        self.cycle.is_some()
    }

    pub(crate) fn loop_span(&self) -> Option<Span> {
        self.cycle.as_ref().map(|cycle| cycle.loop_span)
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Value {
    producer: Span,
    commitment: CommitState,
    integer_result: bool,
    premises: Vec<Arc<OriginalRegistryWords>>,
}

impl Value {
    fn add_premise(&mut self, premise: &Arc<OriginalRegistryWords>) {
        if !self.premises.contains(premise) {
            self.premises.push(Arc::clone(premise));
        }
    }

    fn joined(&self, other: &Self) -> Option<Self> {
        if self.producer != other.producer {
            return None;
        }
        let mut joined = self.clone();
        joined.commitment.join(&other.commitment);
        joined.integer_result &= other.integer_result;
        for premise in &other.premises {
            joined.add_premise(premise);
        }
        Some(joined)
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Read {
    symbol: SignatureSourceVariableSymbol,
    span: Span,
}

#[derive(Clone, PartialEq, Eq)]
struct CapturedValue {
    value: Value,
    read: Option<Read>,
}

impl CapturedValue {
    fn joined(&self, other: &Self) -> Option<Self> {
        Some(Self {
            value: self.value.joined(&other.value)?,
            read: if self.read == other.read {
                self.read.clone()
            } else {
                None
            },
        })
    }
}

/// Exact source bytes supply counterevidence, never an object representation.
#[derive(Clone, PartialEq, Eq)]
struct LiteralValue {
    bytes: Vec<u8>,
    premises: Vec<Arc<OriginalRegistryWords>>,
}

impl LiteralValue {
    fn add_premise(&mut self, source: &Arc<OriginalRegistryWords>) {
        if !self.premises.contains(source) {
            self.premises.push(Arc::clone(source));
        }
    }

    fn joined(&self, other: &Self) -> Option<Self> {
        if self.bytes != other.bytes {
            return None;
        }
        let mut joined = self.clone();
        for premise in &other.premises {
            joined.add_premise(premise);
        }
        Some(joined)
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
struct LiteralState {
    cells: HashMap<SignatureSourceVariableSymbol, LiteralValue>,
    snapshots: HashMap<OperandSnapshot, LiteralValue>,
    last_result: Option<LiteralValue>,
}

impl LiteralState {
    fn join(&mut self, other: &Self) {
        self.cells.retain(|symbol, value| {
            let Some(joined) = other
                .cells
                .get(symbol)
                .and_then(|other| value.joined(other))
            else {
                return false;
            };
            *value = joined;
            true
        });
        self.snapshots.retain(|slot, value| {
            let Some(joined) = other
                .snapshots
                .get(slot)
                .and_then(|other| value.joined(other))
            else {
                return false;
            };
            *value = joined;
            true
        });
        self.last_result = self
            .last_result
            .as_ref()
            .zip(other.last_result.as_ref())
            .and_then(|(left, right)| left.joined(right));
    }

    fn capture(
        &mut self,
        into: OperandSnapshot,
        from: &CaptureSource,
        source: &Arc<OriginalRegistryWords>,
    ) {
        let captured = match from {
            CaptureSource::Unknown => None,
            CaptureSource::Literal { bytes } => Some(LiteralValue {
                bytes: bytes.clone(),
                premises: Vec::new(),
            }),
            CaptureSource::LastResult => self.last_result.clone(),
            CaptureSource::Variable { symbol, .. } => self.cells.get(symbol).cloned(),
        };
        self.snapshots.remove(&into);
        if let Some(mut value) = captured {
            value.add_premise(source);
            self.snapshots.insert(into, value);
        }
    }

    fn apply(
        &mut self,
        site: u32,
        relation: ConditionalSourceRepresentation,
        target: Option<&(TargetId, SignatureSourceVariableSymbol)>,
        source: &Arc<OriginalRegistryWords>,
    ) {
        let mut value = match relation {
            ConditionalSourceRepresentation::IntegerResult { .. } => None,
            ConditionalSourceRepresentation::ForwardValue { value, .. } => self
                .snapshots
                .get(&OperandSnapshot {
                    site,
                    operand: value,
                })
                .cloned(),
        };
        if let Some(value) = &mut value {
            value.add_premise(source);
        }
        if let Some((_, symbol)) = target {
            self.cells.remove(symbol);
            if let Some(value) = &value {
                self.cells.insert(symbol.clone(), value.clone());
            }
        }
        self.last_result = value;
        self.snapshots.retain(|slot, _| slot.site != site);
    }

    fn control(&mut self, source: &Arc<OriginalRegistryWords>) {
        self.last_result = None;
        for value in self.cells.values_mut().chain(self.snapshots.values_mut()) {
            value.add_premise(source);
        }
    }

    fn clear_temporaries(&mut self) {
        self.snapshots.clear();
        self.last_result = None;
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
struct State {
    cells: HashMap<SignatureSourceVariableSymbol, Value>,
    snapshots: HashMap<OperandSnapshot, CapturedValue>,
    last_result: Option<Value>,
    literals: LiteralState,
    producers: HashSet<Span>,
    // Required path premises, never an active naming or quietness capability.
    body_contexts: Vec<Arc<super::ConditionalRepresentationBodyContext>>,
    // Exact body entries traversed on every incoming graph path.
    entered_body_contexts: Vec<Arc<super::ConditionalRepresentationBodyContext>>,
    blocked: bool,
}

impl State {
    fn forget_values(&mut self) {
        self.forget_representations();
        self.literals = LiteralState::default();
    }

    fn forget_representations(&mut self) {
        self.cells.clear();
        self.snapshots.clear();
        self.last_result = None;
    }

    fn invalidate(&mut self) {
        self.forget_values();
        self.body_contexts.clear();
        self.entered_body_contexts.clear();
        self.blocked = true;
    }

    fn join(&mut self, other: &Self) -> bool {
        let before = self.clone();
        self.producers.extend(other.producers.iter().copied());
        if self.blocked || other.blocked {
            self.invalidate();
        } else {
            extend_body_contexts(&mut self.body_contexts, &other.body_contexts);
            self.entered_body_contexts
                .retain(|context| other.entered_body_contexts.contains(context));
            self.literals.join(&other.literals);
            self.cells.retain(|symbol, value| {
                let Some(joined) = other
                    .cells
                    .get(symbol)
                    .and_then(|other| value.joined(other))
                else {
                    return false;
                };
                *value = joined;
                true
            });
            self.snapshots.retain(|slot, value| {
                let Some(joined) = other
                    .snapshots
                    .get(slot)
                    .and_then(|other| value.joined(other))
                else {
                    return false;
                };
                *value = joined;
                true
            });
            self.last_result = self
                .last_result
                .as_ref()
                .zip(other.last_result.as_ref())
                .and_then(|(left, right)| left.joined(right));
        }
        *self != before
    }

    fn capture(
        &mut self,
        into: OperandSnapshot,
        from: &CaptureSource,
        source: &Arc<OriginalRegistryWords>,
    ) {
        self.literals.capture(into, from, source);
        let captured = match from {
            CaptureSource::Unknown | CaptureSource::Literal { .. } => None,
            CaptureSource::LastResult => self
                .last_result
                .clone()
                .map(|value| CapturedValue { value, read: None }),
            CaptureSource::Variable { symbol, span } => {
                self.cells.get(symbol).cloned().map(|value| CapturedValue {
                    value,
                    read: Some(Read {
                        symbol: symbol.clone(),
                        span: *span,
                    }),
                })
            }
        };
        self.snapshots.remove(&into);
        if let Some(mut captured) = captured {
            captured.value.add_premise(source);
            self.snapshots.insert(into, captured);
        }
    }

    fn apply(
        &mut self,
        site: u32,
        span: Span,
        relation: ConditionalSourceRepresentation,
        target: Option<&(TargetId, SignatureSourceVariableSymbol)>,
        prior_target: Option<&PriorTargetUpdate>,
        source: &Arc<OriginalRegistryWords>,
    ) {
        let prior_literal = prior_target
            .and_then(|prior| {
                target
                    .filter(|(id, _)| *id == prior.relation.target())
                    .and_then(|(_, symbol)| self.literals.cells.get(symbol))
            })
            .cloned();
        if prior_target
            .zip(prior_literal.as_ref())
            .is_some_and(|(prior, literal)| {
                prior.relation.definitely_rejects_literal(&literal.bytes)
            })
        {
            self.invalidate();
            return;
        }
        let (mut value, expected_target) = match relation {
            ConditionalSourceRepresentation::IntegerResult { target } => {
                // One source site is not a runtime allocation identity. A path
                // revisiting a producer needs separate loop-carried evidence.
                if !self.producers.insert(span) {
                    self.invalidate();
                    return;
                }
                (
                    Some(Value {
                        producer: span,
                        commitment: CommitState::committed(TclType::Int, Some(span)),
                        integer_result: true,
                        premises: vec![Arc::clone(source)],
                    }),
                    target,
                )
            }
            ConditionalSourceRepresentation::ForwardValue { value, target } => (
                self.snapshots
                    .get(&OperandSnapshot {
                        site,
                        operand: value,
                    })
                    .map(|captured| captured.value.clone()),
                Some(target),
            ),
        };
        if expected_target != target.map(|(id, _)| *id) {
            self.invalidate();
            return;
        }
        if let Some(value) = &mut value {
            value.add_premise(source);
            if let Some(literal) = prior_literal {
                for premise in &literal.premises {
                    value.add_premise(premise);
                }
            }
        }
        self.literals.apply(site, relation, target, source);
        if let Some((_, symbol)) = target {
            self.cells.remove(symbol);
            if let Some(value) = &value {
                self.cells.insert(symbol.clone(), value.clone());
            }
        }
        self.last_result = value;
        self.snapshots.retain(|slot, _| slot.site != site);
    }

    fn list_index(
        &mut self,
        site: u32,
        span: Span,
        coercion: ConditionalSourceListIndexCoercion,
        source: &Arc<OriginalRegistryWords>,
    ) -> Option<SourceConversionObservation> {
        let Some(captured) = self.snapshots.get(&OperandSnapshot {
            site,
            operand: OperandId(coercion.argument()),
        }) else {
            self.forget_representations();
            return None;
        };
        if !captured.value.integer_result || coercion.may_preserve_integer_result() {
            self.forget_representations();
            return None;
        }
        let mut value = captured.value.clone();
        value.add_premise(source);
        let observation = captured.read.as_ref().and_then(|read| {
            (value.commitment.must_pay(TclType::List)
                && value.commitment.single_committed() == Some(TclType::Int))
            .then(|| SourceConversionObservation {
                span: read.span,
                producer: value.producer,
                symbol: read.symbol.clone(),
                from_type: TclType::Int,
                to_type: TclType::List,
                consumer: Arc::clone(source),
                premises: value.premises.clone(),
                body_contexts: self.body_contexts.clone(),
                cycle: None,
            })
        });
        value.commitment.commit(TclType::List, span);
        // Forwarding retains known sharing. Distinct producer sites supply no
        // proof of disjoint objects, so other commitments become unavailable.
        self.cells
            .retain(|_, shared| shared.producer == value.producer);
        self.snapshots
            .retain(|_, captured| captured.value.producer == value.producer);
        if self
            .last_result
            .as_ref()
            .is_some_and(|last| last.producer != value.producer)
        {
            self.last_result = None;
        }
        for shared in self
            .cells
            .values_mut()
            .chain(
                self.snapshots
                    .values_mut()
                    .map(|captured| &mut captured.value),
            )
            .chain(self.last_result.iter_mut())
        {
            shared.commitment.clone_from(&value.commitment);
            shared.add_premise(source);
        }
        observation
    }

    fn step(&mut self, event: &Event) -> Option<SourceConversionObservation> {
        if self.blocked {
            return None;
        }
        match event {
            Event::ClearResult => {
                self.last_result = None;
                self.literals.last_result = None;
            }
            Event::BodyContext { context } => {
                extend_body_contexts(&mut self.body_contexts, std::slice::from_ref(context));
                extend_body_contexts(
                    &mut self.entered_body_contexts,
                    std::slice::from_ref(context),
                );
            }
            Event::RequireBodyContext { context } => {
                if !self.entered_body_contexts.contains(context) {
                    self.invalidate();
                }
            }
            Event::Capture { into, from, source } => self.capture(*into, from, source),
            Event::Apply {
                site,
                span,
                relation,
                target,
                prior_target,
                source,
            } => {
                self.apply(
                    *site,
                    *span,
                    *relation,
                    target.as_ref(),
                    prior_target.as_ref(),
                    source,
                );
            }
            Event::ListIndex {
                site,
                span,
                coercion,
                source,
            } => {
                let observation = self.list_index(*site, *span, *coercion, source);
                self.last_result = None;
                self.snapshots.retain(|slot, _| slot.site != *site);
                self.literals.control(source);
                self.literals.snapshots.retain(|slot, _| slot.site != *site);
                return observation;
            }
            Event::Reset { targets, source } => {
                self.cells.retain(|symbol, _| !targets.contains(symbol));
                self.literals
                    .cells
                    .retain(|symbol, _| !targets.contains(symbol));
                self.control(source);
            }
            Event::Control { source } => self.control(source),
            Event::Barrier => self.invalidate(),
        }
        None
    }

    fn control(&mut self, source: &Arc<OriginalRegistryWords>) {
        self.last_result = None;
        self.literals.control(source);
        for value in self.cells.values_mut().chain(
            self.snapshots
                .values_mut()
                .map(|captured| &mut captured.value),
        ) {
            value.add_premise(source);
        }
    }
}

fn extend_body_contexts(
    into: &mut Vec<Arc<super::ConditionalRepresentationBodyContext>>,
    from: &[Arc<super::ConditionalRepresentationBodyContext>],
) {
    for context in from {
        if !into.contains(context) {
            into.push(Arc::clone(context));
        }
    }
}

#[cfg(test)]
pub(super) fn assert_body_context_state_laws(
    context: &Arc<super::ConditionalRepresentationBodyContext>,
    source: &Arc<OriginalRegistryWords>,
) {
    let enter = Event::BodyContext {
        context: Arc::clone(context),
    };
    let require = Event::RequireBodyContext {
        context: Arc::clone(context),
    };
    let mut entered = State::default();
    let _ = entered.step(&enter);
    let _ = entered.step(&enter);
    assert_eq!(entered.body_contexts.len(), 1);
    assert_eq!(entered.entered_body_contexts.len(), 1);
    let _ = entered.step(&Event::Reset {
        targets: Vec::new(),
        source: Arc::clone(source),
    });
    entered.forget_representations();
    let _ = entered.step(&Event::ClearResult);
    let _ = entered.step(&require);
    assert!(!entered.blocked);
    assert!(entered.body_contexts.contains(context));
    assert!(entered.entered_body_contexts.contains(context));

    let mut same_entry = entered.clone();
    let _ = same_entry.join(&entered);
    let _ = same_entry.step(&require);
    assert!(!same_entry.blocked);
    for (mut joined, other) in [
        (entered.clone(), State::default()),
        (State::default(), entered.clone()),
    ] {
        let _ = joined.join(&other);
        assert!(joined.body_contexts.contains(context));
        assert!(joined.entered_body_contexts.is_empty());
        let _ = joined.step(&require);
        assert!(joined.blocked);
        assert!(joined.body_contexts.is_empty());
    }

    let mut missing = State::default();
    let _ = missing.step(&require);
    assert!(missing.blocked);
    let _ = entered.step(&Event::Barrier);
    let _ = entered.step(&enter);
    let _ = entered.step(&require);
    assert!(entered.blocked);
    assert!(entered.body_contexts.is_empty());
    assert!(entered.entered_body_contexts.is_empty());
}

/// Reuse the declaration graph's ordering and edges. Findings are collected
/// only after all incoming states have joined, so a late predecessor cannot
/// leave a warning based on an earlier, stronger state.
pub(super) fn project(
    builder: &super::Builder<'_>,
    entry: usize,
) -> Vec<SourceConversionObservation> {
    let states = settled_states(builder, entry);
    let mut observations = Vec::new();
    for (node, incoming) in builder.nodes.iter().zip(&states) {
        let Some(mut state) = incoming.clone() else {
            continue;
        };
        for event in &node.representation_events {
            if let Some(observation) = state.step(event)
                && !observations.contains(&observation)
            {
                observations.push(observation);
            }
        }
    }
    for cycle in project_cycles(builder, &states) {
        observations.retain(|single| single.span != cycle.span || single.symbol != cycle.symbol);
        observations.push(cycle);
    }
    observations.sort_by_key(|observation| (observation.span.start(), observation.span.end()));
    observations
}

fn settled_states(builder: &super::Builder<'_>, entry: usize) -> Vec<Option<State>> {
    let mut states = vec![None; builder.nodes.len()];
    let Some(initial) = states.get_mut(entry) else {
        return states;
    };
    *initial = Some(State::default());
    let mut pending = VecDeque::from([entry]);
    let mut queued = HashSet::from([entry]);
    while let Some(id) = pending.pop_front() {
        queued.remove(&id);
        let Some(mut state) = states[id].clone() else {
            continue;
        };
        let node = &builder.nodes[id];
        for event in &node.representation_events {
            let _ = state.step(event);
        }
        for &next in &node.successors {
            let Some(incoming) = states.get_mut(next) else {
                return Vec::new();
            };
            let changed = if let Some(incoming) = incoming {
                incoming.join(&state)
            } else {
                *incoming = Some(state.clone());
                true
            };
            if changed && queued.insert(next) {
                pending.push_back(next);
            }
        }
    }
    states
}

#[derive(Clone, PartialEq, Eq)]
struct UpdateWitness {
    span: Span,
    read_span: Span,
    source: Arc<OriginalRegistryWords>,
}

#[derive(Clone, PartialEq, Eq)]
struct FirstConversion {
    update: UpdateWitness,
    conversion: Option<SourceConversionObservation>,
}

#[derive(Clone, PartialEq, Eq)]
struct CarryWitness {
    symbol: SignatureSourceVariableSymbol,
    update: UpdateWitness,
    first: SourceConversionObservation,
    updated: bool,
    conversion: Option<SourceConversionObservation>,
}

#[derive(Clone, PartialEq, Eq)]
struct CycleState {
    values: State,
    first: HashMap<SignatureSourceVariableSymbol, FirstConversion>,
    carry: Option<CarryWitness>,
    premises: Vec<Arc<OriginalRegistryWords>>,
}

impl CycleState {
    fn join(&mut self, other: &Self) -> bool {
        let before = self.clone();
        self.values.join(&other.values);
        for premise in &other.premises {
            self.retain_premise(premise);
        }
        self.first
            .retain(|symbol, witness| other.first.get(symbol) == Some(witness));
        // Distinct path witnesses cannot be assembled into a cycle.
        if self.carry != other.carry {
            self.values.invalidate();
        }
        *self != before
    }

    fn step(&mut self, event: &Event) {
        if self.values.blocked {
            return;
        }
        match event {
            Event::Capture { source, .. }
            | Event::Apply { source, .. }
            | Event::ListIndex { source, .. }
            | Event::Reset { source, .. }
            | Event::Control { source } => self.retain_premise(source),
            Event::ClearResult
            | Event::BodyContext { .. }
            | Event::RequireBodyContext { .. }
            | Event::Barrier => {}
        }
        if let Event::Apply {
            target: Some((_, symbol)),
            ..
        } = event
        {
            self.first.remove(symbol);
            if self
                .carry
                .as_ref()
                .is_some_and(|carry| &carry.symbol == symbol)
                && !self.prepare_carried_update(event)
            {
                self.values.invalidate();
                return;
            }
        }
        let observation = self.values.step(event);
        if self.carry.is_none()
            && let Some((symbol, update)) = target_update(event)
            && self.values.cells.get(symbol).is_some_and(|value| {
                value.producer == update.span
                    && value.commitment.single_committed() == Some(TclType::Int)
            })
        {
            self.first.insert(
                symbol.clone(),
                FirstConversion {
                    update,
                    conversion: None,
                },
            );
        }
        if let Some(observation) = observation {
            self.record_conversion(observation);
        }
        self.first.retain(|symbol, witness| {
            self.values
                .cells
                .get(symbol)
                .is_some_and(|value| value.producer == witness.update.span)
        });
        if let Some(carry) = &self.carry
            && !self
                .values
                .cells
                .get(&carry.symbol)
                .is_some_and(|value| value.producer == carry.update.span)
        {
            self.values.invalidate();
        }
    }

    fn prepare_carried_update(&mut self, event: &Event) -> bool {
        let Some((symbol, update)) = target_update(event) else {
            return false;
        };
        let Event::Apply {
            prior_target: Some(prior),
            ..
        } = event
        else {
            return false;
        };
        if self
            .values
            .literals
            .cells
            .get(symbol)
            .is_some_and(|literal| prior.relation.definitely_rejects_literal(&literal.bytes))
        {
            return false;
        }
        let Some(carry) = self.carry.as_mut() else {
            return false;
        };
        if carry.updated || carry.symbol != *symbol || carry.update != update {
            return false;
        }
        let Some(value) = self.values.cells.get(symbol) else {
            return false;
        };
        if value.producer != update.span
            || !value.integer_result
            || value.commitment.single_committed() != Some(TclType::List)
            || !value.commitment.must_pay(TclType::Int)
        {
            return false;
        }
        // Only this dependency permits a repeated producer. Retire all old
        // aliases and captures before reusing its source provenance: the new
        // store is a different evaluation, not a same-span object identity.
        self.values.forget_representations();
        self.values.literals.clear_temporaries();
        self.values.producers.remove(&update.span);
        carry.updated = true;
        true
    }

    fn record_conversion(&mut self, observation: SourceConversionObservation) {
        if let Some(carry) = &mut self.carry {
            if carry.updated
                && carry.conversion.is_none()
                && observation.symbol == carry.symbol
                && observation.producer == carry.update.span
                && observation.span == carry.first.span
                && observation.consumer == carry.first.consumer
                && observation.from_type == carry.first.from_type
                && observation.to_type == carry.first.to_type
            {
                carry.conversion = Some(observation);
            }
        } else if let Some(witness) = self.first.get_mut(&observation.symbol)
            && witness.update.span == observation.producer
        {
            witness.conversion = Some(observation);
        }
    }

    fn at_boundary(&mut self) {
        self.values.snapshots.clear();
        self.values.last_result = None;
        self.values.literals.clear_temporaries();
    }

    fn retain_premise(&mut self, premise: &Arc<OriginalRegistryWords>) {
        if !self.premises.contains(premise) {
            self.premises.push(Arc::clone(premise));
        }
    }
}

fn target_update(event: &Event) -> Option<(&SignatureSourceVariableSymbol, UpdateWitness)> {
    let Event::Apply {
        span,
        relation:
            ConditionalSourceRepresentation::IntegerResult {
                target: Some(expected),
            },
        target: Some((target, symbol)),
        prior_target: Some(prior),
        source,
        ..
    } = event
    else {
        return None;
    };
    (*expected == *target
        && prior.relation.target() == *target
        && prior.relation.prior_representation_requirement() == TclType::Int
        && prior.relation.stored_representation() == TclType::Int)
        .then(|| {
            (
                symbol,
                UpdateWitness {
                    span: *span,
                    read_span: prior.read_span,
                    source: Arc::clone(source),
                },
            )
        })
}

fn valid_region(builder: &super::Builder<'_>, region: &LoopRegion) -> bool {
    if region.count.iterations() < 2
        || region.site != region.span.start()
        || region.first_gate == region.header
        || [
            region.normal_exit,
            region.returned_exit,
            region.other_exit,
            region.header,
            region.bindings,
        ]
        .contains(&region.error_exit)
    {
        return false;
    }
    let nodes = &builder.nodes;
    if [
        region.first_gate,
        region.bindings,
        region.repeated_entry,
        region.header,
        region.normal_exit,
        region.error_exit,
        region.returned_exit,
        region.other_exit,
    ]
    .iter()
    .any(|id| *id >= nodes.len())
    {
        return false;
    }
    let [test] = nodes[region.header].successors.as_slice() else {
        return false;
    };
    nodes[region.bindings].successors == [region.repeated_entry]
        && nodes[region.first_gate]
            .successors
            .contains(&region.bindings)
        && nodes.get(*test).is_some_and(|test| {
            test.successors.contains(&region.bindings)
                && test.successors.contains(&region.normal_exit)
        })
}

/// Cut only at the original owner's header and exits; the underlying graph is
/// unchanged. Each pass joins every returning route before exposing a witness.
fn traverse_iteration(
    builder: &super::Builder<'_>,
    region: &LoopRegion,
    initial: CycleState,
) -> Option<CycleState> {
    let mut states = vec![None; builder.nodes.len()];
    states[region.bindings] = Some(initial);
    let mut pending = VecDeque::from([region.bindings]);
    let mut queued = HashSet::from([region.bindings]);
    while let Some(id) = pending.pop_front() {
        queued.remove(&id);
        if id == region.header {
            continue;
        }
        if builder
            .representation_loops
            .iter()
            .any(|nested| nested.id != region.id && nested.bindings == id)
        {
            return None;
        }
        let mut state = states[id].clone()?;
        let node = &builder.nodes[id];
        for event in &node.representation_events {
            state.step(event);
        }
        if state.values.blocked || node.successors.is_empty() {
            return None;
        }
        for &next in &node.successors {
            // Error paths do not assert a successful original handler. The
            // other exits cannot donate a returning-iteration witness.
            if next == region.error_exit {
                continue;
            }
            if next == region.normal_exit
                || next == region.returned_exit
                || next == region.other_exit
                || next == region.first_gate
            {
                return None;
            }
            let incoming = states.get_mut(next)?;
            let changed = if let Some(incoming) = incoming {
                incoming.join(&state)
            } else {
                *incoming = Some(state.clone());
                true
            };
            if changed && queued.insert(next) {
                pending.push_back(next);
            }
        }
    }
    let mut returned = states[region.header].take()?;
    if returned.values.blocked {
        return None;
    }
    returned.at_boundary();
    Some(returned)
}

fn project_cycles(
    builder: &super::Builder<'_>,
    states: &[Option<State>],
) -> Vec<SourceConversionObservation> {
    let mut observations = Vec::new();
    for (index, region) in builder.representation_loops.iter().enumerate() {
        if region.id != LoopId(index) || !valid_region(builder, region) {
            continue;
        }
        let Some(initial) = states
            .get(region.first_gate)
            .and_then(Option::as_ref)
            .filter(|state| !state.blocked)
        else {
            continue;
        };
        let Some(first) = traverse_iteration(
            builder,
            region,
            CycleState {
                values: initial.clone(),
                first: HashMap::new(),
                carry: None,
                premises: Vec::new(),
            },
        ) else {
            continue;
        };
        for (symbol, witness) in &first.first {
            if let Some(observation) = repeat_conversion(builder, region, &first, symbol, witness)
                && !observations.contains(&observation)
            {
                observations.push(observation);
            }
        }
    }
    observations
}

fn repeat_conversion(
    builder: &super::Builder<'_>,
    region: &LoopRegion,
    first: &CycleState,
    symbol: &SignatureSourceVariableSymbol,
    witness: &FirstConversion,
) -> Option<SourceConversionObservation> {
    let first_observation = witness.conversion.as_ref()?;
    let value = first.values.cells.get(symbol)?;
    if value.producer != witness.update.span
        || value.commitment.single_committed() != Some(TclType::List)
    {
        return None;
    }
    let second = traverse_iteration(
        builder,
        region,
        CycleState {
            values: first.values.clone(),
            first: HashMap::new(),
            carry: Some(CarryWitness {
                symbol: symbol.clone(),
                update: witness.update.clone(),
                first: first_observation.clone(),
                updated: false,
                conversion: None,
            }),
            premises: first.premises.clone(),
        },
    )?;
    let carry = second.carry?;
    let mut observation = carry.conversion?;
    let value = second.values.cells.get(symbol)?;
    if value.producer != witness.update.span
        || value.commitment.single_committed() != Some(TclType::List)
    {
        return None;
    }
    for premise in first_observation
        .premises
        .iter()
        .chain(&value.premises)
        .chain(&second.premises)
        .chain(std::iter::once(&region.source))
    {
        if !observation.premises.contains(premise) {
            observation.premises.push(Arc::clone(premise));
        }
    }
    extend_body_contexts(
        &mut observation.body_contexts,
        &first_observation.body_contexts,
    );
    extend_body_contexts(&mut observation.body_contexts, &second.values.body_contexts);
    observation.cycle = Some(SourceCycleObservation {
        loop_span: region.span,
        iterations: region.count.iterations(),
        update_span: witness.update.span,
        target_read_span: witness.update.read_span,
    });
    Some(observation)
}
