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
use tcl_registry::representation::ConditionalSourceListIndexCoercion;
use tcl_registry::value_transfer::{ConditionalSourceRepresentation, OperandId, TargetId};

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
    LastResult,
    Variable {
        symbol: SignatureSourceVariableSymbol,
        span: Span,
    },
}

/// Events are issued only after the parent authenticates the same source scope,
/// original operands, registry relation and ordinary-cell applicability.
pub(super) enum Event {
    ClearResult,
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
        source: Arc<OriginalRegistryWords>,
    },
    ListIndex {
        site: u32,
        span: Span,
        coercion: ConditionalSourceListIndexCoercion,
        source: Arc<OriginalRegistryWords>,
    },
    Barrier,
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
}

impl std::fmt::Debug for SourceConversionObservation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SourceConversionObservation")
            .field("span", &self.span)
            .field("producer", &self.producer)
            .field("premises", &self.premises.len())
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

    pub(crate) const fn prior_type(&self) -> TclType {
        self.from_type
    }

    pub(crate) const fn to_type(&self) -> TclType {
        self.to_type
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

#[derive(Clone, Default, PartialEq, Eq)]
struct State {
    cells: HashMap<SignatureSourceVariableSymbol, Value>,
    snapshots: HashMap<OperandSnapshot, CapturedValue>,
    last_result: Option<Value>,
    producers: HashSet<Span>,
    blocked: bool,
}

impl State {
    fn forget_values(&mut self) {
        self.cells.clear();
        self.snapshots.clear();
        self.last_result = None;
    }

    fn invalidate(&mut self) {
        self.forget_values();
        self.blocked = true;
    }

    fn join(&mut self, other: &Self) -> bool {
        let before = self.clone();
        self.producers.extend(other.producers.iter().copied());
        if self.blocked || other.blocked {
            self.invalidate();
        } else {
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
        let captured = match from {
            CaptureSource::Unknown => None,
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
        source: &Arc<OriginalRegistryWords>,
    ) {
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
            self.forget_values();
            return None;
        };
        if !captured.value.integer_result || coercion.may_preserve_integer_result() {
            self.forget_values();
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
            Event::ClearResult => self.last_result = None,
            Event::Capture { into, from, source } => self.capture(*into, from, source),
            Event::Apply {
                site,
                span,
                relation,
                target,
                source,
            } => {
                self.apply(*site, *span, *relation, target.as_ref(), source);
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
                return observation;
            }
            Event::Barrier => self.invalidate(),
        }
        None
    }
}

/// Reuse the declaration graph's ordering and edges. Findings are collected
/// only after all incoming states have joined, so a late predecessor cannot
/// leave a warning based on an earlier, stronger state.
pub(super) fn project(
    builder: &super::Builder<'_>,
    entry: usize,
) -> Vec<SourceConversionObservation> {
    let mut states = vec![None; builder.nodes.len()];
    let Some(initial) = states.get_mut(entry) else {
        return Vec::new();
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
    let mut observations = Vec::new();
    for (node, incoming) in builder.nodes.iter().zip(states) {
        let Some(mut state) = incoming else {
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
    observations.sort_by_key(|observation| (observation.span.start(), observation.span.end()));
    observations
}
