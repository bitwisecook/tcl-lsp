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

//! The read-only view a specialisation is given
//! (`docs/design/compiler/value-transfers.md` § *The interface*).
//!
//! The analyser implements [`AnalysisInputs`] once and hands it to every
//! specialisation. Nothing here mutates; the registry never receives the
//! analyser itself.

use tcl_lexer::Span;

use crate::arg_role::ArgRole;
use crate::completion::CompletionCodeDomain;
use crate::frame_effect::FrameLevel;
use crate::invocation_words::InvocationWordKind;
use crate::taint::TaintColour;
use crate::types::TclType;
use crate::world_effect::EffectFootprint;

use super::answers::{
    DependencyEvidence, EvalAnswer, ExactValue, Existence, RepresentationEvidence, SegmentFacts,
    SelectionFact, StoreOutcome, ValueShape,
};
use super::context::{AnalysisContext, BindingIdentity};
use super::decline::DeclineReason;

/// One operand of a resolved invocation, in the resolver's coordinate
/// system: post-head, counting a subcommand word. Adequate for
/// arbitrary-length argv, never a truncating `u8`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperandId(pub usize);

/// An operand the structural plan has validated as a place-bearing target.
/// Never a hook-selected SSA id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetId(pub OperandId);

/// The kind of storage a place is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    /// A scalar variable.
    Scalar,
    /// One element of an array, with a literal key.
    Element {
        /// The array's name.
        base: String,
        /// The element key.
        key: String,
    },
    /// A whole array.
    Array,
}

/// A Tcl storage location as the analyser resolves it. Names are resolved
/// to places before any outcome is composed, so two spellings of one cell
/// are one target.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlaceRef {
    /// The normalised name of the place — `x`, `a(k)`, `::ns::v`.
    pub name: String,
    /// What kind of storage it is.
    pub kind: PlaceKind,
}

impl PlaceRef {
    /// A scalar place.
    #[must_use]
    pub fn scalar(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: PlaceKind::Scalar,
        }
    }
}

/// The SSA identity that correlates uses of one value: two operands reading
/// the same identity are one distinct value. Opaque to the registry; the
/// driver encodes its own key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueIdentity(pub u64);

/// The domains a specialisation can be asked about. One owner each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactDomain {
    /// `Const` / `ConstSet` over exact strings — the value lattice.
    ExactValue,
    /// Semantic type and shape.
    Type,
    /// Bound, unbound, or may-bound, scalar or array.
    Existence,
    /// Integer intervals.
    Range,
    /// Exact prefix, suffix, or length bound of a partly known value.
    Segments,
    /// Colour and sanitiser identity.
    Taint,
    /// Internal-representation evidence.
    Representation,
    /// Which arm or edge a proven operand selects.
    Selection,
    /// Reads, writes, and observable operations.
    Effects,
    /// The completion codes a construct can take.
    Completion,
}

/// A non-value domain's answer.
#[derive(Debug, Clone, PartialEq)]
pub enum DomainFact {
    /// The existence fact at a place.
    Existence(Existence),
    /// The semantic type and shape.
    Type {
        /// The internal representation, when proven.
        intrep: Option<TclType>,
        /// The value's shape, when proven.
        shape: Option<ValueShape>,
    },
    /// An integer interval.
    Range {
        /// The lower bound, when known.
        lo: Option<i64>,
        /// The upper bound, when known.
        hi: Option<i64>,
    },
    /// Segment facts around an unknown part.
    Segments(SegmentFacts),
    /// The taint colour.
    Taint(TaintColour),
    /// Representation evidence.
    Representation(RepresentationEvidence),
    /// A selected arm or edge.
    Selection(SelectionFact),
    /// The effect footprint.
    Effects(EffectFootprint),
    /// The completion domain.
    Completion(CompletionCodeDomain),
}

/// One answer from a fact domain. Produced by the solver that owns the
/// domain; consumed by the specialisation that asked.
#[derive(Debug, Clone, PartialEq)]
pub enum FactView {
    /// The input is the optimistic bottom: the answer stays pending.
    Pending,
    /// One exact value, with the SSA identity that correlates its uses
    /// (`None` for a literal word).
    Exact(ExactValue, Option<ValueIdentity>),
    /// A bounded set of exact values, correlated by identity.
    Finite(Vec<ExactValue>, Option<ValueIdentity>),
    /// A non-value domain's fact.
    Domain(DomainFact),
    /// The domain cannot answer for this input, for a recorded reason.
    Top(DeclineReason),
}

impl FactView {
    /// The exact value this view holds, or the answer that stands in for
    /// one it does not: a pending input stays pending, a finite set the lift
    /// could not pin is correlated, a non-value domain fact is malformed,
    /// and a top declines with its own reason. The one reading every route
    /// takes of an input it needs exactly.
    ///
    /// # Errors
    ///
    /// The stand-in answer, as above.
    pub fn exact(self) -> Result<ExactValue, EvalAnswer> {
        match self {
            Self::Pending => Err(EvalAnswer::Pending),
            Self::Exact(value, _) => Ok(value),
            Self::Finite(..) => Err(EvalAnswer::Declined(DeclineReason::CorrelatedSets)),
            Self::Domain(_) => Err(EvalAnswer::Declined(DeclineReason::MalformedAnswer)),
            Self::Top(reason) => Err(EvalAnswer::Declined(reason)),
        }
    }
}

/// Which argument layout an invocation view describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationLayout<'a> {
    /// The source words after the head, as written.
    Source,
    /// The synthetic loop header the CFG builder emits for a list-iterating
    /// loop: the operands are the iterable words in group order and the
    /// binders are the names defined per iteration. Operand indices refer
    /// to this layout, never to the source var-list.
    LoopHeader {
        /// The names bound per iteration, in binding order.
        binders: &'a [String],
    },
}

/// One operand of the invocation view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperandView<'a> {
    /// The word's text, delimiters stripped.
    pub text: &'a str,
    /// What the source proves about the word's value.
    pub kind: InvocationWordKind,
    /// The registry role at this position, when the resolver assigned one.
    pub role: Option<ArgRole>,
}

/// The resolver's projection of one call: the binding, the selected
/// subcommand and form, and the operands with their substitution kinds and
/// roles. Produced from `ResolvedInvocation` by the driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInvocationView<'a> {
    /// The registry's canonical command name.
    pub canonical_command: &'a str,
    /// The selected subcommand's canonical name, when one was selected.
    pub subcommand: Option<&'a str>,
    /// The selected form's name, when one was selected.
    pub form: Option<&'a str>,
    /// Which layout the operands follow.
    pub layout: InvocationLayout<'a>,
    /// The operands, in layout order.
    pub operands: Vec<OperandView<'a>>,
}

impl<'a> ResolvedInvocationView<'a> {
    /// The operand at `id`, when the invocation has one.
    #[must_use]
    pub fn operand(&self, id: OperandId) -> Option<&OperandView<'a>> {
        self.operands.get(id.0)
    }

    /// The operands carrying `role`, in operand order.
    pub fn operands_with_role(&self, role: ArgRole) -> impl Iterator<Item = OperandId> + '_ {
        self.operands
            .iter()
            .enumerate()
            .filter(move |(_, operand)| operand.role == Some(role))
            .map(|(index, _)| OperandId(index))
    }
}

/// One part of a source word's substitution structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordPart {
    /// A literal run.
    Literal {
        /// The run's span in the word.
        span: Span,
        /// The run's text.
        text: String,
    },
    /// A `$name`, `${name}`, or `$arr(k)` read.
    VariableRead {
        /// The read's span in the word.
        span: Span,
        /// The variable name.
        name: String,
        /// The element key, for an array read.
        element: Option<String>,
    },
    /// A `[…]` script region.
    Script {
        /// The region's span in the word.
        span: Span,
        /// The script text inside the brackets.
        script: String,
    },
    /// A backslash escape.
    Escape {
        /// The escape's span in the word.
        span: Span,
    },
}

/// A source word's substitution structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordStructure {
    /// Whether the word is brace-quoted.
    pub braced: bool,
    /// The word's parts, in order.
    pub parts: Vec<WordPart>,
}

/// A body operand as a script region with its base offset and the frame it
/// runs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyRegion {
    /// The script text.
    pub script: String,
    /// The script's offset in the document.
    pub base_offset: usize,
    /// The frame the script runs in.
    pub frame: FrameLevel,
}

/// Which nested operations an evaluation admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NestedPolicy {
    /// Only effect-free nested invocations.
    EffectFreeOnly,
    /// Nested invocations whose ordered stores name only places the state
    /// can own, with an exact completion.
    LocalWrites,
}

/// The ordered evaluation state of one expression or one word
/// substitution. Owned by the driver for one evaluation; its writes become
/// the invocation's ordered stores.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationState {
    /// Writes applied so far, in order.
    pub writes: Vec<(PlaceRef, StoreOutcome)>,
    /// The evidence every nested outcome contributed.
    pub evidence: DependencyEvidence,
    /// Which nested operations this evaluation admits.
    pub policy: NestedPolicy,
}

impl EvaluationState {
    /// A fresh state under `policy`.
    #[must_use]
    pub fn new(policy: NestedPolicy) -> Self {
        Self {
            writes: Vec::new(),
            evidence: DependencyEvidence::default(),
            policy,
        }
    }
}

/// Read-only view of what the analyser has proven at this program point.
/// Implemented once by the transfer driver; read by every specialisation.
pub trait AnalysisInputs {
    /// The resolver's projection of this call.
    fn invocation(&self) -> &ResolvedInvocationView<'_>;
    /// One operand's fact in one domain: pending while the solver has not
    /// reached the input, `Top` with the reason when it never will.
    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView;
    /// The storage place a name operand denotes, or why it cannot be one.
    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason>;
    /// A variable read by name at this program point.
    fn variable(&self, name: &str, domain: FactDomain) -> FactView;
    /// The fact that holds at a place before the invocation runs.
    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView;
    /// A source word's substitution structure.
    fn word_structure(&self, id: OperandId) -> Result<WordStructure, DeclineReason>;
    /// A body operand as a script region.
    fn body(&self, id: OperandId) -> Result<BodyRegion, DeclineReason>;
    /// A nested `[…]` script evaluated under the ordered evaluation state.
    fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer;
    /// A math function with the binding evidence for what the analysed
    /// program calls.
    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason>;
    /// The immutable identity every answer is memoised under.
    fn context(&self) -> &AnalysisContext;
}
