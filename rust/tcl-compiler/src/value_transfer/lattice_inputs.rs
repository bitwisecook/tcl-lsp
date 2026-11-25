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

//! `LatticeInputs`: the analyser's inputs over the SCCP lattice at one statement, and the lattice-to-exact conversions.

use super::argument_words::OperandSource;
use super::{LatticeInputs, place_named, record_binding};
use crate::analyses::{ConstValue, LatticeValue};
use crate::expr_ast::ExprNode;
use crate::ssa::ValueKey;
use std::cell::RefCell;
use tcl_lexer::{LexerConfig, Span};
use tcl_registry::value_transfer::{
    AnalysisContext, AnalysisInputs, BindingIdentity, BindingKind, BodyRegion, CompletionOutcome,
    DeclineReason, DomainFact, EvalAnswer, EvalRoute, EvaluationState, ExactValue,
    ExactValueOrUnavailable, Existence, FactDomain, FactView, LiftedAnswer, NestedPolicy,
    NumericValue, OperandId, ParameterDefault, PlaceRef, ResolvedInvocationView, StoreOutcome,
    ValueIdentity, WordPart, WordStructure, WrittenPlace, written_in,
};
use tcl_registry::{ArgRole, FrameLevel, InvocationWordKind, ResolvedInvocation};

/// The lattice's answer for one SSA value as a fact view.
fn lattice_to_fact(value: &LatticeValue, identity: Option<ValueIdentity>) -> FactView {
    match value {
        LatticeValue::Unknown => FactView::Pending,
        LatticeValue::Const(c) => FactView::Exact(const_to_exact(c), identity),
        LatticeValue::ConstSet(vs) => {
            FactView::Finite(vs.iter().map(const_to_exact).collect(), identity)
        }
        LatticeValue::Overdefined => FactView::Top(DeclineReason::NotExact),
    }
}

/// A lattice constant as an exact value: the text the constant renders
/// as, with its classification as the additional fact. A double is spelled
/// as Tcl spells it (`3.0`, `1e+301`), never as Rust does (`3`).
pub(crate) fn const_to_exact(c: &ConstValue) -> ExactValue {
    const_into_exact(c.clone())
}

/// Materialise a lattice constant when the consumer owns its text.
pub(crate) fn const_into_exact(c: ConstValue) -> ExactValue {
    match c {
        ConstValue::Int(i) => ExactValue::int(i),
        ConstValue::Float(f) => ExactValue {
            bytes: tcl_syntax::number::format_double(f).into_bytes(),
            numeric: Some(NumericValue::Float(f)),
            representation: tcl_registry::value_transfer::RepresentationEvidence::Unknown,
        },
        ConstValue::Bool(b) => ExactValue {
            bytes: (if b { "1" } else { "0" }).as_bytes().to_vec(),
            numeric: Some(NumericValue::Bool(b)),
            representation: tcl_registry::value_transfer::RepresentationEvidence::Unknown,
        },
        ConstValue::String(s) => ExactValue::text(s),
    }
}

/// The text a lattice constant renders as, when it is text.
pub(crate) fn const_text(c: &ConstValue) -> Option<String> {
    String::from_utf8(const_to_exact(c).bytes).ok()
}

/// An exact value as a lattice constant: the classification when it has
/// one, else the exact text. A double is kept as one only where its text is
/// [`tcl_syntax::number::format_double`]'s, which is what
/// [`const_to_exact`] spells it back as; a release that spells it otherwise
/// (`1e-05` under 8.4 and Jim) keeps the text, which reads back as the same
/// double.
pub(crate) fn exact_to_const(value: &ExactValue) -> ConstValue {
    match value.numeric {
        Some(NumericValue::Int(i)) => ConstValue::Int(i),
        Some(NumericValue::Float(f))
            if value.bytes == tcl_syntax::number::format_double(f).as_bytes() =>
        {
            ConstValue::Float(f)
        }
        Some(NumericValue::Float(_)) => {
            ConstValue::String(String::from_utf8_lossy(&value.bytes).into_owned())
        }
        Some(NumericValue::Bool(b)) => ConstValue::Bool(b),
        None => ConstValue::String(String::from_utf8_lossy(&value.bytes).into_owned()),
    }
}

/// An exact value's lattice projection; bytes that are not text decline.
pub(crate) fn exact_to_lattice(value: &ExactValue) -> LatticeValue {
    if value.numeric.is_none() && std::str::from_utf8(&value.bytes).is_err() {
        return LatticeValue::Overdefined;
    }
    LatticeValue::Const(exact_to_const(value))
}

/// The SSA identity of a value key, packed.
fn identity_of(key: ValueKey) -> ValueIdentity {
    ValueIdentity((u64::from(key.0.0) << 32) | u64::from(key.1))
}

/// How an invocation's substituting operand words are evaluated.
pub(super) enum Words {
    /// Each word on its own, under the effect-free policy.
    Independent,
    /// Every substituting word once, in operand order, under one ordered
    /// state, evaluated when the first is read: the writes one word's
    /// commands make are seen by the next word and by the route that
    /// assembles them, and a word is evaluated once however often the route
    /// reads it. A route reads every operand before it reads a cell
    /// (`evaluate_lifted` pins them first), so the words have run, in order,
    /// before anything of the command reads what they wrote.
    Ordered(RefCell<Option<Box<OrderedWords>>>),
}

impl Words {
    /// How an invocation's words are evaluated under `policy`: each on its
    /// own where it runs effect-free, else in order under one state.
    pub(super) fn under(policy: NestedPolicy) -> Self {
        match policy {
            NestedPolicy::EffectFreeOnly => Self::Independent,
            NestedPolicy::LocalWrites | NestedPolicy::Protected => {
                Self::Ordered(RefCell::new(None))
            }
        }
    }

    /// The writes the words made, in order, where they were evaluated under
    /// one ordered state.
    pub(super) fn writes(&self) -> Vec<(PlaceRef, StoreOutcome)> {
        match self {
            Self::Ordered(evaluated) => evaluated
                .borrow()
                .as_ref()
                .map_or_default(|words| words.state.writes.clone()),
            Self::Independent => Vec::new(),
        }
    }
}

/// An invocation's substituting operand words after their evaluation.
pub(super) struct OrderedWords {
    /// Each operand's fact; `None` for an operand no substitution reaches.
    pub(super) facts: Vec<Option<FactView>>,
    /// The state the words ran under: its writes are what they stored.
    pub(super) state: EvaluationState,
}

impl<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher> LatticeInputs<'_, S1, S2> {
    /// Re-resolve the view's roles over the words' lattice values, when the
    /// command's roles depend on its words (`arg_role_resolver`) and a
    /// substituted word kept the resolver from reading them: the command
    /// sees the values, so a subject the lattice proves (`regexp {(a+)b} $s
    /// -> g`, `lassign $l a b`) no longer hides its targets' roles. Only
    /// when every word is exact, and every word the values make a target
    /// is a literal name — a computed name is no place.
    pub(super) fn resolve_roles_over_values(&mut self, resolved: &ResolvedInvocation<'_, '_>) {
        let semantics = &resolved.semantics;
        let Some(derive_roles) = semantics.arg_role_resolver else {
            return;
        };
        if self
            .view
            .operands
            .iter()
            .all(|operand| operand.kind == InvocationWordKind::Literal)
        {
            return;
        }
        let values: Option<Vec<String>> = (0..self.view.operands.len())
            .map(
                |index| match self.operand(OperandId(index), FactDomain::ExactValue) {
                    FactView::Exact(value, _) => String::from_utf8(value.bytes).ok(),
                    _ => None,
                },
            )
            .collect();
        let Some(values) = values else {
            return;
        };
        let words: Vec<&str> = values.iter().map(String::as_str).collect();
        let offset = semantics.argument_offset;
        let Some(arguments) = words.get(offset..) else {
            return;
        };
        let roles = derive_roles(arguments);
        let role_at = |index: usize| {
            index
                .checked_sub(offset)
                .and_then(|i| u8::try_from(i).ok())
                .and_then(|i| roles.iter().find(|(at, _)| *at == i).map(|(_, role)| *role))
        };
        let names_are_literal = self
            .view
            .operands
            .iter()
            .enumerate()
            .all(|(index, operand)| {
                role_at(index) != Some(ArgRole::VarWrite)
                    || operand.kind == InvocationWordKind::Literal
            });
        if !names_are_literal {
            return;
        }
        for (index, operand) in self.view.operands.iter_mut().enumerate() {
            operand.role = role_at(index);
        }
    }

    /// The lattice fact for `name` at this statement's use version, after
    /// the writes the enclosing evaluations made: the last of them decides
    /// what the name holds.
    pub(super) fn named_fact(&self, name: &str) -> FactView {
        match written_in(&self.prior_writes, name) {
            WrittenPlace::Exact(value) => return FactView::Exact(value, None),
            WrittenPlace::Unknown => return FactView::Top(DeclineReason::NotExact),
            WrittenPlace::Untouched => {}
        }
        let sym = match self.point {
            Some((block, index)) if self.ssa.point_contexts.is_some() => {
                self.ssa.var_symbol_at(block, index, name)
            }
            _ => self.ssa.var_symbol(name),
        };
        let Some(sym) = sym else {
            return FactView::Top(DeclineReason::NotExact);
        };
        let Some(&ver) = self.uses.get(&sym) else {
            return FactView::Top(DeclineReason::NotExact);
        };
        self.values
            .get(&(sym, ver))
            .map_or(FactView::Pending, |value| {
                lattice_to_fact(value, Some(identity_of((sym, ver))))
            })
    }

    /// What the writes the invocation's own substituting words made say of
    /// `name`: Tcl substitutes every word before the command runs, so the
    /// command's reads come after them.
    pub(super) fn words_written(&self, name: &str) -> WrittenPlace {
        match &self.words {
            Words::Independent => WrittenPlace::Untouched,
            Words::Ordered(evaluated) => evaluated
                .borrow()
                .as_ref()
                .map_or(WrittenPlace::Untouched, |words| words.state.written(name)),
        }
    }

    /// The existence fact the writes made ahead of the invocation — its own
    /// words' and then the enclosing evaluations' — leave `name` in, when
    /// they reached it: a write binds it, and any other touch (an unbind, an
    /// element of its array) states nothing.
    pub(super) fn written_existence(&self, name: &str) -> Option<FactView> {
        let written = match self.words_written(name) {
            WrittenPlace::Untouched => written_in(&self.prior_writes, name),
            later => later,
        };
        match written {
            WrittenPlace::Untouched => None,
            WrittenPlace::Exact(_) => Some(FactView::Domain(DomainFact::Existence(
                Existence::Bound(BindingKind::Scalar),
            ))),
            WrittenPlace::Unknown => Some(FactView::Top(DeclineReason::StatefulNested)),
        }
    }

    /// `answer` with the writes the words made ahead of each outcome's own,
    /// for a route that does not carry them itself: the expression engine
    /// starts from the words' state, and every other route's outcome says
    /// nothing of what a word's command wrote.
    pub(super) fn carrying_word_writes(
        &self,
        answer: LiftedAnswer,
        route: EvalRoute,
    ) -> LiftedAnswer {
        let Words::Ordered(evaluated) = &self.words else {
            return answer;
        };
        if matches!(route, EvalRoute::Expression { .. }) {
            return answer;
        }
        let Some(state) = evaluated.borrow().as_ref().map(|words| words.state.clone()) else {
            return answer;
        };
        let LiftedAnswer::Evaluated(mut outcomes) = answer else {
            return answer;
        };
        for outcome in &mut outcomes {
            let mut writes = state.writes.clone();
            writes.append(&mut outcome.nested_writes);
            outcome.nested_writes = writes;
            // The prefix an error completion ran counts the writes ahead of
            // it.
            if let CompletionOutcome::Error { written, .. } = &mut outcome.completion {
                *written += state.writes.len();
            }
            for binding in &state.evidence.bindings {
                record_binding(&mut outcome.evidence, binding.clone());
            }
        }
        LiftedAnswer::Evaluated(outcomes)
    }
}

impl<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher> AnalysisInputs
    for LatticeInputs<'_, S1, S2>
{
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        if domain != FactDomain::ExactValue {
            return FactView::Top(DeclineReason::Unavailable(self.driver.context.tier));
        }
        let Some(operand) = self.view.operand(id) else {
            return FactView::Top(DeclineReason::NotExact);
        };
        match operand.kind {
            // A literal word's text is its value, already cooked: never
            // re-read as a substitution (`{$x}` and `"\$x"` are the text
            // `$x`, not a read of `x`).
            InvocationWordKind::Literal => {
                FactView::Exact(ExactValue::from_literal(operand.text), None)
            }
            // OperandView is a text-only compatibility projection. It cannot
            // recover an independently retained non-Unicode byte payload.
            InvocationWordKind::KnownBytes => FactView::Top(DeclineReason::NotText),
            InvocationWordKind::Dynamic => match self.sources.get(id.0) {
                Some(OperandSource::Substituted) => match &self.words {
                    Words::Independent => self.substituted_argument(operand.text, id.0),
                    Words::Ordered(evaluated) => self.ordered_word(evaluated, id),
                },
                _ => tcl_syntax::naming::whole_variable_reference_literal_name(
                    operand.text,
                    self.driver.lexer_config,
                )
                .map_or(FactView::Top(DeclineReason::NotExact), |name| {
                    self.named_fact(name)
                }),
            },
            InvocationWordKind::Expanded | InvocationWordKind::Opaque => {
                FactView::Top(DeclineReason::NotExact)
            }
        }
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        let text = self
            .view
            .operand(id)
            .map(|operand| operand.text)
            .ok_or(DeclineReason::NotExact)?;
        // A dynamic-key target (`incr a($i)`) never interns a symbol — that
        // miss is permanent, so it must widen: pending would launder a
        // fanned element's stale constant through the join.
        let name = crate::naming::element_var_name(text);
        if self.ssa.var_symbol(name).is_none() {
            return Err(DeclineReason::DynamicName);
        }
        Ok(place_named(name))
    }

    fn variable(&self, name: &str, domain: FactDomain) -> FactView {
        match domain {
            FactDomain::ExactValue => self.named_fact(name),
            FactDomain::Existence => self
                .written_existence(name)
                .unwrap_or_else(|| self.driver.existence_fact(self.ssa, name)),
            _ => FactView::Top(DeclineReason::Unavailable(self.driver.context.tier)),
        }
    }

    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        match domain {
            FactDomain::ExactValue => {
                let written = match self.words_written(&place.name) {
                    WrittenPlace::Untouched => written_in(&self.prior_writes, &place.name),
                    later => later,
                };
                match written {
                    WrittenPlace::Exact(value) => return FactView::Exact(value, None),
                    WrittenPlace::Unknown => return FactView::Top(DeclineReason::NotExact),
                    WrittenPlace::Untouched => {}
                }
            }
            FactDomain::Existence => {
                return self
                    .written_existence(&place.name)
                    .unwrap_or_else(|| self.driver.existence_fact(self.ssa, &place.name));
            }
            _ => return FactView::Top(DeclineReason::Unavailable(self.driver.context.tier)),
        }
        let Some(sym) = self.ssa.var_symbol(&place.name) else {
            return FactView::Top(DeclineReason::DynamicName);
        };
        // A use the statement does not hold is a permanent miss, as a
        // dynamic key's is in `place`: a value-position cell update's read
        // sits on the synthetic call ahead of its host, so the host's uses
        // never reach it. Reading version 0 instead found no value and
        // answered a `Pending` that never resolved, which a phi then
        // laundered into the other arm's constant.
        let Some(&ver) = self.uses.get(&sym) else {
            return FactView::Top(DeclineReason::NotExact);
        };
        self.values
            .get(&(sym, ver))
            .map_or(FactView::Pending, |value| {
                lattice_to_fact(value, Some(identity_of((sym, ver))))
            })
    }

    fn word_structure(&self, id: OperandId) -> Result<WordStructure, DeclineReason> {
        let operand = self.view.operand(id).ok_or(DeclineReason::NotExact)?;
        let whole = |start: u32| {
            let end = u32::try_from(operand.text.len())
                .unwrap_or(u32::MAX)
                .saturating_add(start);
            vec![WordPart::Literal {
                span: Span::new(start, end),
                text: operand.text.to_owned(),
            }]
        };
        match self.sources.get(id.0) {
            // The content starts one past the opening brace.
            Some(OperandSource::BracedLiteral) => Ok(WordStructure {
                braced: true,
                quoted: false,
                parts: whole(1),
            }),
            Some(OperandSource::Literal) => Ok(WordStructure {
                braced: false,
                quoted: false,
                parts: whole(0),
            }),
            // The content starts one past the opening quote.
            Some(OperandSource::QuotedLiteral) => Ok(WordStructure {
                braced: false,
                quoted: true,
                parts: whole(1),
            }),
            // A substituted word's reading is its parts'; its quoting is
            // not recorded.
            Some(OperandSource::Substituted) => Ok(WordStructure {
                braced: false,
                quoted: false,
                parts: word_parts(operand.text, self.driver.lexer_config)?,
            }),
            Some(OperandSource::Unknown) | None => Err(DeclineReason::NotExact),
        }
    }

    fn body(&self, id: OperandId) -> Result<BodyRegion, DeclineReason> {
        let operand = self.view.operand(id).ok_or(DeclineReason::NotExact)?;
        // Only text the word is: a brace-quoted script is not re-read as a
        // substitution, and any other spelling is not the script it runs.
        match self.sources.get(id.0) {
            Some(OperandSource::BracedLiteral) => {
                let original =
                    self.original
                        .as_ref()
                        .zip(self.driver.module)
                        .and_then(|(parent, module)| {
                            let argument = self.written_arguments.get(id.0).copied().flatten()?;
                            module.original_body(parent, argument, operand.text)
                        });
                let base_offset = original.as_ref().map_or(
                    0,
                    super::super::interprocedural::OriginalSummaryScript::base,
                );
                let base_offset =
                    usize::try_from(base_offset).map_err(|_| DeclineReason::NotExact)?;
                *self.nested_source.borrow_mut() = original;
                Ok(BodyRegion {
                    script: operand.text.to_owned(),
                    base_offset,
                    frame: FrameLevel::Relative(0),
                })
            }
            _ => Err(DeclineReason::NotExact),
        }
    }

    fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer {
        self.driver.nested_answer(script, state, self)
    }

    fn prepare_expression_source(&self, expression: &ExprNode) {
        *self.expression_source.borrow_mut() = self.driver.module.and_then(|module| {
            module.original_expression(self.original.as_ref()?, expression, self.expression_base)
        });
    }

    fn nested_expression_at(
        &self,
        script: &str,
        start: u32,
        end: u32,
        state: &mut EvaluationState,
    ) -> EvalAnswer {
        if let Some(module) = self.driver.module {
            let original = self
                .expression_source
                .borrow()
                .as_ref()
                .and_then(|expression| {
                    module.original_expression_substitution(expression, script, start, end)
                });
            let Some(original) = original else {
                return EvalAnswer::Declined(DeclineReason::NotExact);
            };
            *self.nested_source.borrow_mut() = Some(original);
        }
        let answer = self.driver.nested_answer(script, state, self);
        *self.nested_source.borrow_mut() = None;
        answer
    }

    fn word_state(&self) -> Option<EvaluationState> {
        match &self.words {
            Words::Independent => None,
            Words::Ordered(evaluated) => {
                evaluated.borrow().as_ref().map(|words| words.state.clone())
            }
        }
    }

    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        self.driver.math_function(name)
    }

    fn parameter_default(&self, procedure: &str, parameter: &str) -> ParameterDefault {
        self.driver
            .parameter_default_at(self.original.as_ref(), procedure, parameter)
    }

    fn source_lexer_config(&self) -> Option<LexerConfig> {
        Some(self.driver.lexer_config)
    }

    fn context(&self) -> &AnalysisContext {
        &self.driver.context
    }
}

impl<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher> LatticeInputs<'_, S1, S2> {
    /// The fact operand `id` holds once every substituting word has been
    /// evaluated in order under one state ([`Words::Ordered`]): the first
    /// read evaluates them all, so the order of the reads never orders the
    /// writes.
    fn ordered_word(
        &self,
        evaluated: &RefCell<Option<Box<OrderedWords>>>,
        id: OperandId,
    ) -> FactView {
        let mut slot = evaluated.borrow_mut();
        let words = slot.get_or_insert_with(|| self.evaluate_words());
        words
            .facts
            .get(id.0)
            .and_then(Clone::clone)
            .unwrap_or(FactView::Top(DeclineReason::NotExact))
    }

    /// Every operand that substitutes, in operand order, under one
    /// `LocalWrites` state: Tcl substitutes a command's words left to right
    /// before it runs.
    fn evaluate_words(&self) -> Box<OrderedWords> {
        let mut state = EvaluationState::new(NestedPolicy::LocalWrites);
        let facts = self
            .view
            .operands
            .iter()
            .enumerate()
            .map(|(index, operand)| {
                (operand.kind == InvocationWordKind::Dynamic
                    && matches!(self.sources.get(index), Some(OperandSource::Substituted)))
                .then(|| self.substituted_in(operand.text, &mut state, Some(index)))
            })
            .collect();
        Box::new(OrderedWords { facts, state })
    }

    /// What `state`'s writes and then the lattice say `name` holds.
    pub(super) fn read(&self, state: &EvaluationState, name: &str) -> FactView {
        match state.written(name) {
            WrittenPlace::Exact(value) => FactView::Exact(value, None),
            WrittenPlace::Unknown => FactView::Top(DeclineReason::NotExact),
            WrittenPlace::Untouched => self.named_fact(name),
        }
    }

    fn substitution_sources(&self, argument: Option<usize>) -> Vec<crate::ir::SourceSite> {
        let Some((parent, index)) =
            self.original
                .as_ref()
                .zip(argument)
                .and_then(|(parent, index)| {
                    self.written_arguments
                        .get(index)
                        .copied()
                        .flatten()
                        .map(|index| (parent, index))
                })
        else {
            return Vec::new();
        };
        match parent.words().get(index + 1) {
            Some(crate::ir::WordExpr::CommandSubstitution { source, .. }) => vec![source.clone()],
            Some(crate::ir::WordExpr::Template { parts, .. }) => parts
                .iter()
                .filter_map(|part| match part {
                    crate::ir::WordPart::CommandSubstitution { source, .. } => Some(source.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(super) fn substituted_argument(&self, text: &str, argument: usize) -> FactView {
        let mut state = EvaluationState::new(NestedPolicy::EffectFreeOnly);
        self.substituted_in(text, &mut state, Some(argument))
    }

    /// Evaluate a genuine original argument under `state`: variable reads
    /// observe preceding writes and checked child scripts update that same state.
    fn substituted_in(
        &self,
        text: &str,
        state: &mut EvaluationState,
        argument: Option<usize>,
    ) -> FactView {
        use tcl_lexer::word_parts::{SubstFlags, WordBody, WordPart as Part, decompose};
        if let Some(name) = tcl_syntax::naming::whole_variable_reference_literal_name(
            text,
            self.driver.lexer_config,
        ) {
            return self.read(state, name);
        }
        let parts = match decompose(
            text.as_bytes(),
            SubstFlags::default(),
            self.driver.lexer_config,
        ) {
            WordBody::Literal(bytes) => {
                return FactView::Exact(exact_of_bytes(bytes.to_vec()), None);
            }
            WordBody::Parts(parts) => parts,
        };
        let mut sites = self.substitution_sources(argument).into_iter();
        let mut bytes = Vec::with_capacity(text.len());
        let mut pending = false;
        for part in parts {
            let value = match part {
                Part::Text(run) => {
                    bytes.extend_from_slice(&run);
                    continue;
                }
                Part::Variable(reference) => match variable_name(&reference) {
                    Ok(name) => self.read(state, &name).exact(),
                    Err(reason) => return FactView::Top(reason),
                },
                Part::Command(script) => match std::str::from_utf8(script) {
                    Ok(script) => {
                        let original = self.original.as_ref().zip(self.driver.module).and_then(
                            |(parent, module)| {
                                module.original_substitution(parent, &sites.next()?, script)
                            },
                        );
                        *self.nested_source.borrow_mut() = original;
                        let answer = self.nested(script, state);
                        *self.nested_source.borrow_mut() = None;
                        match answer {
                            // A command that did not complete normally ends the
                            // word with the writes so far: it has no value, and
                            // an error is one the invocation never gets past.
                            EvalAnswer::Evaluated(outcome)
                                if outcome.completion != CompletionOutcome::Normal =>
                            {
                                if matches!(outcome.completion, CompletionOutcome::Error { .. }) {
                                    self.driver.word_error.set(true);
                                }
                                Err(EvalAnswer::Declined(DeclineReason::StatefulNested))
                            }
                            EvalAnswer::Evaluated(outcome) => match outcome.result {
                                ExactValueOrUnavailable::Exact(value) => Ok(value),
                                ExactValueOrUnavailable::Unavailable(_) => {
                                    Err(EvalAnswer::Declined(DeclineReason::NotExact))
                                }
                            },
                            answer => Err(answer),
                        }
                    }
                    Err(_) => return FactView::Top(DeclineReason::NotText),
                },
                Part::Expression(_) => return FactView::Top(DeclineReason::Unsupported),
                Part::ParseError(_) => return FactView::Top(DeclineReason::WrongRepresentation),
            };
            match value {
                Ok(value) => bytes.extend_from_slice(&value.bytes),
                Err(EvalAnswer::Pending) => pending = true,
                Err(EvalAnswer::Declined(reason)) => return FactView::Top(reason),
                Err(EvalAnswer::Evaluated(_)) => {
                    return FactView::Top(DeclineReason::Unsupported);
                }
            }
        }
        if pending {
            return FactView::Pending;
        }
        FactView::Exact(exact_of_bytes(bytes), None)
    }
}

/// A variable reference's element-qualified name: the base, or `base(key)`
/// for a constant key; a key that substitutes is a computed name. The one
/// assembly for a substituted word's reads and an expression's quoted
/// operand (`ExprServices::quoted_string`).
pub(crate) fn variable_name(
    reference: &tcl_lexer::word_parts::VarRef<'_>,
) -> Result<String, DeclineReason> {
    use tcl_lexer::word_parts::WordPart as Part;
    let name = std::str::from_utf8(reference.name).map_err(|_| DeclineReason::NotText)?;
    let Some(index) = &reference.index else {
        return Ok(name.to_owned());
    };
    let mut key = Vec::new();
    for piece in index {
        let Part::Text(run) = piece else {
            return Err(DeclineReason::DynamicName);
        };
        key.extend_from_slice(run);
    }
    let key = String::from_utf8(key).map_err(|_| DeclineReason::NotText)?;
    Ok(format!("{name}({key})"))
}

/// A substituted word's structure: its literal runs (decoded), variable
/// reads and script regions, each with its span in the word.
pub(super) fn word_parts(text: &str, config: LexerConfig) -> Result<Vec<WordPart>, DeclineReason> {
    use tcl_lexer::word_parts::{SubstFlags, WordPart as Part, decompose_spanned};
    let span = |start: usize, end: usize| {
        Span::new(
            u32::try_from(start).unwrap_or(u32::MAX),
            u32::try_from(end).unwrap_or(u32::MAX),
        )
    };
    decompose_spanned(text.as_bytes(), SubstFlags::default(), config)
        .into_iter()
        .map(|spanned| {
            let at = span(spanned.start, spanned.end);
            match spanned.part {
                Part::Text(run) => Ok(WordPart::Literal {
                    span: at,
                    text: String::from_utf8(run.into_owned())
                        .map_err(|_| DeclineReason::NotText)?,
                }),
                Part::Variable(reference) => {
                    let base = std::str::from_utf8(reference.name)
                        .map_err(|_| DeclineReason::NotText)?
                        .to_owned();
                    let element = match variable_name(&reference)? {
                        name if name == base => None,
                        name => Some(name[base.len() + 1..name.len() - 1].to_owned()),
                    };
                    Ok(WordPart::VariableRead {
                        span: at,
                        name: base,
                        element,
                    })
                }
                Part::Command(script) => Ok(WordPart::Script {
                    span: at,
                    script: std::str::from_utf8(script)
                        .map_err(|_| DeclineReason::NotText)?
                        .to_owned(),
                }),
                Part::Expression(_) => Err(DeclineReason::Unsupported),
                Part::ParseError(_) => Err(DeclineReason::WrongRepresentation),
            }
        })
        .collect()
}

/// A value's bytes as an exact value, classified as a literal is when they
/// are text.
pub(super) fn exact_of_bytes(bytes: Vec<u8>) -> ExactValue {
    match std::str::from_utf8(&bytes) {
        Ok(text) => ExactValue::from_literal(text),
        Err(_) => ExactValue {
            bytes,
            numeric: None,
            representation: tcl_registry::value_transfer::RepresentationEvidence::Unknown,
        },
    }
}
