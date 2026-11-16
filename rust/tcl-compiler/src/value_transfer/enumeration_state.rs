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

//! Invocations, expressions and words evaluated over a loop enumeration's state.

use super::argument_words::{amount_word, typed_node_commands, view_of};
use super::expressions::{Expression, truth_of};
use super::lattice_inputs::{exact_of_bytes, variable_name};
use super::queries::statement_selection;
use super::{
    ArgWord, ExpressionEvaluation, LatticeDriver, StateInputs, binding_of, place_named, split_head,
};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use std::borrow::Cow;
use tcl_lexer::{LexerConfig, TokenType};
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::value_transfer::builtins::ExpressionRoute;
use tcl_registry::value_transfer::{
    AnalysisContext, AnalysisInputs, AnalysisTier, BindingIdentity, BindingKind, BodyRegion,
    Budget, CommandSemantics, CompletionOutcome, DeclineReason, DomainFact, EvalAnswer, EvalRoute,
    EvaluationState, EvaluatorOwner, ExactValue, ExactValueOrUnavailable, Existence, FactDomain,
    FactView, InvocationLayout, IterationPlan, NestedPolicy, OperandId, ParameterDefault, PlaceRef,
    PlanAnswer, ResolvedInvocationView, SelectionFact, StoreOutcome, WordStructure,
};
use tcl_registry::{InvocationWord, InvocationWordKind};

/// What one invocation does when a loop enumeration runs it over its state
/// ([`crate::static_loops`]): how it completes, and the stores it makes in
/// execution order, each resolved to its place.
#[derive(Debug, Clone)]
pub(crate) struct StateStep {
    /// How the invocation completes.
    pub(crate) completion: CompletionOutcome,
    /// The stores it makes, in order.
    pub(crate) stores: Vec<(PlaceRef, StoreOutcome)>,
}

impl AnalysisInputs for StateInputs<'_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        let Some(operand) = self.view.operand(id) else {
            return FactView::Top(DeclineReason::NotExact);
        };
        if operand.kind == InvocationWordKind::Literal {
            return match domain {
                FactDomain::ExactValue => {
                    FactView::Exact(ExactValue::from_literal(operand.text), None)
                }
                _ => FactView::Top(DeclineReason::Unavailable(AnalysisTier::Fast)),
            };
        }
        match tcl_syntax::naming::whole_variable_reference_literal_name(
            operand.text,
            self.driver.lexer_config,
        ) {
            Some(name) => self.variable(name, domain),
            None if domain == FactDomain::ExactValue => self
                .substituted(operand.text)
                .map_or_else(FactView::Top, |value| FactView::Exact(value, None)),
            None => FactView::Top(DeclineReason::NotExact),
        }
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        let operand = self.view.operand(id).ok_or(DeclineReason::NotExact)?;
        if operand.kind != InvocationWordKind::Literal {
            return Err(DeclineReason::DynamicName);
        }
        // The word names the place as written: `a(k)` is an element of `a`,
        // never the scalar `a`.
        Ok(place_named(operand.text))
    }

    fn variable(&self, name: &str, domain: FactDomain) -> FactView {
        use crate::static_loops::Slot;
        match (self.state.get(name), domain) {
            (Some(Slot::Value(value)), FactDomain::ExactValue) => {
                FactView::Exact(value.clone(), None)
            }
            (Some(Slot::Value(_) | Slot::Scalar), FactDomain::Existence) => {
                FactView::Domain(DomainFact::Existence(Existence::Bound(BindingKind::Scalar)))
            }
            (Some(Slot::Scalar), FactDomain::ExactValue) => FactView::Top(DeclineReason::NotExact),
            (Some(Slot::Unbound), FactDomain::Existence) => {
                FactView::Domain(DomainFact::Existence(Existence::Unbound))
            }
            (Some(Slot::Unbound), FactDomain::ExactValue) => {
                FactView::Top(DeclineReason::UnboundPlace)
            }
            (None, FactDomain::ExactValue | FactDomain::Existence) => {
                FactView::Top(DeclineReason::NotExact)
            }
            _ => FactView::Top(DeclineReason::Unavailable(AnalysisTier::Fast)),
        }
    }

    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        self.variable(&place.name, domain)
    }

    fn word_structure(&self, _id: OperandId) -> Result<WordStructure, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn body(&self, _id: OperandId) -> Result<BodyRegion, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer {
        if state.policy != NestedPolicy::EffectFreeOnly {
            return EvalAnswer::Declined(DeclineReason::StatefulNested);
        }
        self.run_nested(script)
    }

    fn nested_expression_at(
        &self,
        script: &str,
        _start: u32,
        _end: u32,
        state: &mut EvaluationState,
    ) -> EvalAnswer {
        if self.driver.module.is_some() {
            EvalAnswer::Declined(DeclineReason::NotExact)
        } else {
            self.nested(script, state)
        }
    }

    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        self.driver.math_function(name)
    }

    fn parameter_default(&self, procedure: &str, parameter: &str) -> ParameterDefault {
        self.driver.parameter_default_at(None, procedure, parameter)
    }

    fn source_lexer_config(&self) -> Option<LexerConfig> {
        Some(self.driver.lexer_config)
    }

    fn context(&self) -> &AnalysisContext {
        &self.driver.context
    }
}

impl StateInputs<'_> {
    /// A substituted word's value over the state: its parts concatenated —
    /// each literal run decoded under the document's grammar, each variable
    /// read from the state, each script run as one command ([`Self::run_nested`]).
    /// Any part the state does not decide declines the word.
    pub(super) fn substituted(&self, text: &str) -> Result<ExactValue, DeclineReason> {
        use tcl_lexer::word_parts::{SubstFlags, WordBody, WordPart as Part, decompose};
        let parts = match decompose(
            text.as_bytes(),
            SubstFlags::default(),
            self.driver.lexer_config,
        ) {
            WordBody::Literal(bytes) => return Ok(exact_of_bytes(bytes.to_vec())),
            WordBody::Parts(parts) => parts,
        };
        let mut bytes = Vec::with_capacity(text.len());
        for part in parts {
            let value = match part {
                Part::Text(run) => {
                    bytes.extend_from_slice(&run);
                    continue;
                }
                Part::Variable(reference) => {
                    let name = variable_name(&reference)?;
                    match self.variable(&name, FactDomain::ExactValue) {
                        FactView::Exact(value, _) => value,
                        FactView::Top(reason) => return Err(reason),
                        _ => return Err(DeclineReason::NotExact),
                    }
                }
                Part::Command(script) => {
                    let script = std::str::from_utf8(script).map_err(|_| DeclineReason::NotText)?;
                    match self.run_nested(script) {
                        EvalAnswer::Evaluated(outcome)
                            if outcome.completion == CompletionOutcome::Normal =>
                        {
                            match outcome.result {
                                ExactValueOrUnavailable::Exact(value) => value,
                                ExactValueOrUnavailable::Unavailable(_) => {
                                    return Err(DeclineReason::NotExact);
                                }
                            }
                        }
                        // A script that does not complete normally ends the
                        // word, which the enumeration does not follow.
                        EvalAnswer::Evaluated(_) => return Err(DeclineReason::StatefulNested),
                        EvalAnswer::Pending => return Err(DeclineReason::NotExact),
                        EvalAnswer::Declined(reason) => return Err(reason),
                    }
                }
                Part::Expression(_) => return Err(DeclineReason::Unsupported),
                Part::ParseError(_) => return Err(DeclineReason::WrongRepresentation),
            };
            bytes.extend_from_slice(&value.bytes);
        }
        Ok(exact_of_bytes(bytes))
    }

    /// The one command of `script` run over the state by its declared route —
    /// a registry-owned evaluator or the expression engine — under the
    /// effect-free policy: a command that stores declines, as one with no
    /// route or a head the run may not take for the registry's does.
    pub(super) fn run_nested(&self, script: &str) -> EvalAnswer {
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            script,
            0,
            tcl_lexer::SourceSegmentationPurpose::NestedScript.config(self.driver.lexer_config),
        );
        let [seg] = commands.as_slice() else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        let head = seg.name();
        if split_head(script).0 != head {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        if !self.driver.enumeration_trusts(head) {
            return EvalAnswer::Declined(DeclineReason::RebindingSuspected);
        }
        let cooked = self.driver.cooked_args(seg);
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let Some(resolved) = self.driver.resolve(head, &words) else {
            return EvalAnswer::Declined(DeclineReason::NoSemantics);
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            return EvalAnswer::Declined(DeclineReason::NoSemantics);
        };
        let inputs = StateInputs {
            driver: self.driver,
            view: view_of(&resolved, &texts, &words, InvocationLayout::Source),
            state: self.state,
        };
        let mut budget = self.driver.budget();
        let answer = match semantics.route() {
            EvalRoute::Direct { id } if id.owner() == EvaluatorOwner::Registry => {
                semantics.evaluate(&inputs, &mut budget)
            }
            // A pack's arity or option row can switch the declared route
            // off, as the lattice driver asks before it runs the engine.
            EvalRoute::Expression { language } => match semantics
                .as_declared()
                .and_then(|declared| declared.invocation_decline(&inputs))
            {
                Some(EvalAnswer::Pending) => EvalAnswer::Pending,
                Some(EvalAnswer::Declined(reason)) => EvalAnswer::Declined(reason),
                Some(EvalAnswer::Evaluated(_)) | None => ExpressionEvaluation {
                    expression: Expression::Assembled(ExpressionRoute { language }),
                    policy: self.driver.policy,
                    nested: NestedPolicy::EffectFreeOnly,
                    head: Some(binding_of(head, resolved.canonical_command)),
                }
                .evaluate(&inputs, &mut budget),
            },
            EvalRoute::None { reason } => EvalAnswer::Declined(DeclineReason::NoRoute(reason)),
            _ => EvalAnswer::Declined(DeclineReason::Unsupported),
        };
        match answer {
            EvalAnswer::Evaluated(outcome)
                if !outcome.ordered_stores.is_empty() || !outcome.nested_writes.is_empty() =>
            {
                EvalAnswer::Declined(DeclineReason::StatefulNested)
            }
            answer => answer,
        }
    }
}

impl LatticeDriver<'_> {
    /// Whether a loop enumeration may take `head` for the registry's command:
    /// under the run's trust stance when it has one, and the registry's
    /// table where it has none, as a math function's binding is read
    /// ([`Self::math_function`]).
    pub(super) fn enumeration_trusts(&self, head: &str) -> bool {
        self.folds.is_none() || self.trusted(head)
    }

    /// The commands the typed statement of `hook` stands for, when a loop
    /// enumeration may take every one for the registry's: the head it is
    /// resolved through first.
    pub(crate) fn enumeration_typed_head(&self, hook: LoweringHookId) -> Option<&str> {
        let heads = typed_node_commands(self.registry, hook);
        (heads.iter().all(|head| self.enumeration_trusts(head)))
            .then(|| heads.first().copied())
            .flatten()
    }

    /// `head` with its source words, each with whether it was braced, run
    /// over a loop enumeration's `state` through the registry's own route
    /// ([`StateStep`]). `Err` with the reason when the head may not be the
    /// registry's command, declares no registry-owned route, or the route
    /// declines over the state.
    pub(crate) fn invoke_in_state(
        &self,
        state: &crate::static_loops::LoopState,
        head: &str,
        words: &[(&str, bool)],
        budget: &mut Budget,
    ) -> Result<StateStep, DeclineReason> {
        if !self.enumeration_trusts(head) {
            return Err(DeclineReason::RebindingSuspected);
        }
        let texts: Vec<&str> = words.iter().map(|&(text, _)| text).collect();
        let classified: Vec<InvocationWord<'_>> = words
            .iter()
            .map(|&(text, braced)| amount_word((text, braced)))
            .collect();
        let resolved = self
            .resolve(head, &classified)
            .ok_or(DeclineReason::NoSemantics)?;
        let semantics = resolved
            .semantics
            .value
            .semantics()
            .ok_or(DeclineReason::NoSemantics)?;
        match semantics.route() {
            EvalRoute::Direct { id } if id.owner() == EvaluatorOwner::Registry => {}
            EvalRoute::None { reason } => return Err(DeclineReason::NoRoute(reason)),
            _ => return Err(DeclineReason::Unsupported),
        }
        let inputs = StateInputs {
            driver: self,
            view: view_of(&resolved, &texts, &classified, InvocationLayout::Source),
            state,
        };
        let outcome = match semantics.evaluate(&inputs, budget) {
            EvalAnswer::Evaluated(outcome) => outcome,
            EvalAnswer::Pending => return Err(DeclineReason::NotExact),
            EvalAnswer::Declined(reason) => return Err(reason),
        };
        let mut stores = outcome.nested_writes.clone();
        for store in &outcome.ordered_stores {
            stores.push((inputs.place(store.target().0)?, store.clone()));
        }
        Ok(StateStep {
            completion: outcome.completion.clone(),
            stores,
        })
    }

    /// `node` evaluated by the expression route over a loop enumeration's
    /// `state`: its value, how it completes when it does not complete
    /// normally, or why it declines. `head` names the command the
    /// expression's statement fused, when one did, which the run's trust
    /// stance must take for the registry's.
    pub(crate) fn expression_in_state(
        &self,
        state: &crate::static_loops::LoopState,
        node: &ExprNode,
        head: Option<(&str, &str)>,
        budget: &mut Budget,
    ) -> Result<Result<ExactValue, CompletionOutcome>, DeclineReason> {
        if let Some((head, _)) = head
            && !self.enumeration_trusts(head)
        {
            return Err(DeclineReason::RebindingSuspected);
        }
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(node),
            policy: self.policy,
            nested: NestedPolicy::EffectFreeOnly,
            head: head.map(|(head, identity)| binding_of(head, identity)),
        };
        let inputs = StateInputs {
            driver: self,
            view: ResolvedInvocationView {
                canonical_command: "expr",
                subcommand: None,
                form: None,
                layout: InvocationLayout::Source,
                operands: Vec::new(),
                argument_offset: 0,
                arity: None,
            },
            state,
        };
        match expression.evaluate(&inputs, budget) {
            EvalAnswer::Evaluated(outcome) => match (outcome.completion, outcome.result) {
                (CompletionOutcome::Normal, ExactValueOrUnavailable::Exact(value)) => Ok(Ok(value)),
                (CompletionOutcome::Normal, ExactValueOrUnavailable::Unavailable(_)) => {
                    Err(DeclineReason::NotExact)
                }
                (completion, _) => Ok(Err(completion)),
            },
            EvalAnswer::Pending => Err(DeclineReason::NotExact),
            EvalAnswer::Declined(reason) => Err(reason),
        }
    }

    /// The iteration plan `head` with its source words declares, its operands
    /// read over a loop enumeration's `state`.
    pub(crate) fn loop_plan_in_state(
        &self,
        state: &crate::static_loops::LoopState,
        head: &str,
        words: &[(&str, bool)],
    ) -> Result<IterationPlan, DeclineReason> {
        if !self.enumeration_trusts(head) {
            return Err(DeclineReason::RebindingSuspected);
        }
        let texts: Vec<&str> = words.iter().map(|&(text, _)| text).collect();
        let classified: Vec<InvocationWord<'_>> = words
            .iter()
            .map(|&(text, braced)| amount_word((text, braced)))
            .collect();
        let resolved = self
            .resolve(head, &classified)
            .ok_or(DeclineReason::NoSemantics)?;
        let semantics = resolved
            .semantics
            .value
            .semantics()
            .ok_or(DeclineReason::NoSemantics)?;
        let inputs = StateInputs {
            driver: self,
            view: view_of(&resolved, &texts, &classified, InvocationLayout::Source),
            state,
        };
        match semantics.structure(&inputs) {
            PlanAnswer::Iterate(plan) => Ok(plan),
            PlanAnswer::Declined(reason) => Err(reason),
            _ => Err(DeclineReason::Unsupported),
        }
    }

    /// A source word's value over a loop enumeration's `state`: a braced
    /// word's text, a bare word with nothing to substitute cooked as Tcl
    /// reads it, a whole-variable read of the state, or a word that
    /// substitutes, its parts read over the state and its scripts run as one
    /// command each ([`StateInputs::substituted`]).
    pub(crate) fn word_in_state(
        &self,
        state: &crate::static_loops::LoopState,
        text: &str,
        braced: bool,
    ) -> Result<ExactValue, DeclineReason> {
        use crate::static_loops::Slot;
        if braced {
            return self
                .literal_value(text, TokenType::Str)
                .map(|value| ExactValue::from_literal(&value))
                .ok_or(DeclineReason::NotExact);
        }
        if let Some(name) =
            tcl_syntax::naming::whole_variable_reference_literal_name(text, self.lexer_config)
        {
            // `${a(k)}` reads the element, never the scalar `a`.
            return match state.get(name) {
                Some(Slot::Value(value)) => Ok(value.clone()),
                Some(Slot::Unbound) => Err(DeclineReason::UnboundPlace),
                Some(Slot::Scalar) | None => Err(DeclineReason::NotExact),
            };
        }
        if text.contains(['$', '[']) {
            let inputs = StateInputs {
                driver: self,
                view: ResolvedInvocationView {
                    canonical_command: "",
                    subcommand: None,
                    form: None,
                    layout: InvocationLayout::Source,
                    operands: Vec::new(),
                    argument_offset: 0,
                    arity: None,
                },
                state,
            };
            return inputs.substituted(text);
        }
        self.literal_value(text, TokenType::Esc)
            .map(|value| ExactValue::from_literal(&value))
            .ok_or(DeclineReason::NotExact)
    }

    /// The elements of a list value under the target's list rules, or `None`
    /// for a text that is no list, on which a command reading it raises.
    pub(crate) fn list_elements(&self, text: &str) -> Option<Vec<String>> {
        self.policy
            .word_rules
            .split_list(text)
            .ok()
            .map(|elements| elements.into_iter().map(Cow::into_owned).collect())
    }

    /// The selection the lowered case-list statement `stmt` makes when its
    /// subject holds `subject` ([`statement_selection`]).
    pub(crate) fn switch_selection(
        &self,
        stmt: &Statement,
        subject: &str,
    ) -> Option<SelectionFact> {
        statement_selection(self.registry, stmt, subject)
    }

    /// The budget a loop enumeration charges every statement of every
    /// iteration to, within the run's request.
    pub(crate) fn enumeration_budget(&self) -> Budget {
        self.budget()
    }

    /// A condition's truth over a loop enumeration's `state`, as `if` and a
    /// loop read it ([`Self::expression_in_state`]): `Err` when it declines or
    /// does not complete normally, or when its value is no boolean.
    pub(crate) fn condition_in_state(
        &self,
        state: &crate::static_loops::LoopState,
        node: &ExprNode,
        budget: &mut Budget,
    ) -> Result<bool, DeclineReason> {
        match self.expression_in_state(state, node, None, budget)? {
            Ok(value) => truth_of(&ExactValueOrUnavailable::Exact(value))
                .ok_or(DeclineReason::WrongRepresentation),
            Err(_) => Err(DeclineReason::Unsupported),
        }
    }
}
