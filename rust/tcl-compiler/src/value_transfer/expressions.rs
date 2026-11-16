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

//! The expression route: `expr` statements and conditions over the lattice, the shared engine, and evaluation outside a run.

use super::lattice_inputs::Words;
use super::run_records::{answer_label, outcome_paths};
use super::{
    ExpressionEvaluation, Lattice, LatticeDriver, LatticeInputs, binding_of, lattice_of_outcomes,
    record_binding, result_of,
};
use crate::analyses::{LatticeValue, MAX_CONSTSET_SIZE};
use crate::expr_ast::ExprNode;
use crate::ir::CommandTokens;
use crate::sccp::{BuiltinFoldInputs, DefValues};
use crate::ssa::{SsaFunction, Symbol, ValueKey, Version};
use crate::tcl_expr_eval::{ExprAnswer, ExprServices, FoldPolicy, evaluate_expression};
use std::cell::RefCell;
use std::collections::HashMap;
use tcl_lexer::LexerConfig;
use tcl_registry::value_transfer::builtins::ExpressionRoute;
use tcl_registry::value_transfer::{
    AnalysisContext, AnalysisInputs, AnalysisTier, BindingIdentity, BodyRegion, Budget,
    CommandSemantics, CompletionOutcome, DeclineReason, DependencyEvidence, EvalAnswer, EvalRoute,
    EvaluationState, ExactValue, ExactValueOrUnavailable, Existence, FactBounds, FactDomain,
    FactView, InvocationLayout, InvocationOutcome, LanguageProfileId, LiftedAnswer, NestedPolicy,
    NumericValue, OperandId, PlaceRef, ResolvedInvocationView, RouteIdentity, TypeFacts,
    WordStructure, evaluate_lifted,
};

impl LatticeDriver<'_> {
    /// The inputs of an expression that has no operand words of its own:
    /// it reads every variable by name, at the versions `lattice` selects.
    pub(super) fn expression_inputs<'i, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &'i self,
        (uses, values, ssa): Lattice<'i, S1, S2>,
    ) -> LatticeInputs<'i, S1, S2> {
        LatticeInputs {
            original: None,
            written_arguments: Vec::new(),
            expression_base: None,
            expression_source: RefCell::new(None),
            nested_source: RefCell::new(None),
            driver: self,
            prior_writes: Vec::new(),
            words: Words::Independent,
            view: ResolvedInvocationView {
                canonical_command: "expr",
                subcommand: None,
                form: None,
                layout: InvocationLayout::Source,
                operands: Vec::new(),
                argument_offset: 0,
                arity: None,
            },
            uses,
            values,
            ssa,
            point: None,
            sources: Vec::new(),
        }
    }

    /// A fused `set name [expr {…}]` (`AssignExpr`): the parsed expression
    /// evaluated by the shared engine over this statement's lattice inputs,
    /// lifted over one finite input. The nested `expr` is the head whose
    /// binding the answer rests on; without a trust fact the fused node's
    /// own lowering stands.
    pub(crate) fn evaluate_assign_expr<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        expr: &ExprNode,
        command_binding: Option<&tcl_runtime_api::CommandBindingIdentity>,
        original: Option<(&CommandTokens, u32)>,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> DefValues {
        let head = command_binding.map_or("expr", |binding| binding.name.as_str());
        if self.folds.is_some() && !self.trusted(head) {
            self.explain(head, None, "declined: rebinding-suspected".to_owned());
            return DefValues::Each(LatticeValue::Overdefined, None);
        }
        self.enter_expression();
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(expr),
            policy: self.policy,
            nested: NestedPolicy::EffectFreeOnly,
            head: Some(binding_of(
                head,
                command_binding.map_or("expr", |binding| binding.identity.as_str()),
            )),
        };
        let answer = self.evaluate_expression_at(&expression, original, uses, values, ssa);
        self.explain(head, Some(expression.route()), answer_label(&answer));
        if let LiftedAnswer::Evaluated(outcomes) = &answer {
            self.explain_paths(outcome_paths(outcomes, None));
        }
        if self.raises_end_paths()
            && let Some(raised) = Self::raised_expression(&answer)
        {
            return DefValues::Raised(Box::new(raised));
        }
        let value = match answer {
            LiftedAnswer::Pending => LatticeValue::Unknown,
            LiftedAnswer::Declined(_) => LatticeValue::Overdefined,
            LiftedAnswer::Evaluated(outcomes) => {
                let preparation = self
                    .explaining
                    .get()
                    .and_then(|span| self.expression_preparations.get(&span).copied().flatten());
                let admitted = preparation.is_some_and(|preparation| {
                    outcomes.iter().all(|outcome| {
                        let ExactValueOrUnavailable::Exact(value) = &outcome.result else {
                            return false;
                        };
                        let numeric = match value.numeric {
                            Some(NumericValue::Int(value)) => {
                                Some(crate::tcl_expr_eval::TclValue::Int(value))
                            }
                            Some(NumericValue::Float(value)) => {
                                Some(crate::tcl_expr_eval::TclValue::Float(value))
                            }
                            Some(NumericValue::Bool(_)) | None => None,
                        };
                        numeric.as_ref().map_or_else(
                            || crate::expression_rewrite::expression_result_protocol_equivalence(
                                &preparation.witness,
                                None,
                            ),
                            |numeric| crate::expression_rewrite::contents_only_result_equivalence(
                                &preparation.witness,
                                numeric,
                            ),
                        ).is_ok()
                    })
                });
                if admitted {
                    lattice_of_outcomes(&outcomes, result_of)
                } else {
                    LatticeValue::Overdefined
                }
            }
        };
        DefValues::Each(value, None)
    }

    /// A branch condition's truth over the lattice inputs `uses` selects:
    /// the shared engine's full value read as Tcl reads a condition, per
    /// member of one finite input. `Some` only when every member agrees; an
    /// undecided, mixed, pending or declined answer is `None`, and the
    /// answer is recorded against the condition.
    pub(crate) fn evaluate_condition<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        condition: &ExprNode,
        original: Option<(&CommandTokens, u32)>,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<bool> {
        self.enter_expression();
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(condition),
            policy: self.policy,
            nested: NestedPolicy::EffectFreeOnly,
            head: None,
        };
        let answer = self.evaluate_expression_at(&expression, original, uses, values, ssa);
        self.explain("condition", Some(expression.route()), answer_label(&answer));
        let LiftedAnswer::Evaluated(outcomes) = answer else {
            return None;
        };
        let mut decided = None;
        for outcome in &outcomes {
            let truth = truth_of(&outcome.result)?;
            match decided {
                None => decided = Some(truth),
                Some(earlier) if earlier != truth => return None,
                Some(_) => {}
            }
        }
        decided
    }

    /// The math-function service: the binding an `expr` call `name(…)`
    /// dispatches to. The function must exist in the target's `expr`
    /// grammar (`min` is 8.5's, the `is…` classes 9.0's); a name the
    /// grammar does not have is the program's error. Where the functions are
    /// commands (`::tcl::mathfunc::NAME`, from 8.5) the module can rebind
    /// one, and a rebound or shadowed wrapper is `RebindingSuspected` under
    /// the run's trust stance; under 8.4 the grammar dispatches internally,
    /// so no `proc` reaches it. Without a trust fact the builtin table
    /// stands, as it does for the fused node's own lowering.
    pub(super) fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        let profile = self.context.profile;
        // A profile naming no release calls only what every release has.
        let available = tcl_syntax::expr::mathfunc::added_in(name).is_some_and(|since| {
            profile.is_none_or(|profile| since <= crate::tcl_expr_eval::fold_math_ceiling(profile))
        });
        if !available {
            return Err(DeclineReason::Unsupported);
        }
        let qualified = tcl_registry::mathfunc::qualified_name(name);
        let wrappers = profile.is_none_or(tcl_registry::mathfunc::command_wrappers_available);
        if wrappers && self.folds.is_some() && !self.trusted(&qualified) {
            return Err(DeclineReason::RebindingSuspected);
        }
        Ok(binding_of(name, &qualified))
    }

    /// `expression` evaluated over the lattice inputs `uses` selects, with
    /// no operand words of its own: the parsed expression reads every
    /// variable by name.
    pub(super) fn evaluate_expression_at<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        expression: &ExpressionEvaluation<'_>,
        original: Option<(&CommandTokens, u32)>,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> LiftedAnswer {
        let mut inputs = self.expression_inputs((uses, values, ssa));
        if let Some((parent, base)) = original {
            inputs.original = Some(parent.clone());
            inputs.expression_base = Some(base);
        }
        evaluate_lifted(expression, &inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
    }
}

/// The inputs of an expression a rewrite evaluates outside a solver run:
/// its `$name` reads answer from `constants`, a nested command declines but
/// where it calls a procedure of `module` ([`detached_procedure_result`]),
/// and a math function asks the driver's binding service under the
/// rewrite's trust stance.
struct DetachedExpressionInputs<'a, 'd> {
    pub(super) driver: &'a LatticeDriver<'d>,
    pub(super) view: ResolvedInvocationView<'a>,
    constants: &'a HashMap<String, ExactValue>,
    /// The module's procedures and the function the expression runs in,
    /// where a re-run's exit reading holds them.
    pub(super) module: Option<(
        &'a crate::interprocedural::ModuleProcedures<'a>,
        &'a crate::ir::ProcedureId,
    )>,
    /// The stance the expression is read under.
    stance: (BuiltinFoldInputs<'a>, FoldPolicy),
}

impl AnalysisInputs for DetachedExpressionInputs<'_, '_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, _id: OperandId, _domain: FactDomain) -> FactView {
        FactView::Top(DeclineReason::NotExact)
    }

    fn place(&self, _id: OperandId) -> Result<PlaceRef, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn variable(&self, name: &str, domain: FactDomain) -> FactView {
        if domain != FactDomain::ExactValue {
            return FactView::Top(DeclineReason::Unavailable(AnalysisTier::Fast));
        }
        self.constants
            .get(name)
            .map_or(FactView::Top(DeclineReason::NotExact), |value| {
                FactView::Exact(value.clone(), None)
            })
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

    fn nested(&self, script: &str, _state: &mut EvaluationState) -> EvalAnswer {
        let result = self.module.and_then(|module| {
            detached_procedure_result(module, script, self.constants, self.stance)
        });
        match result {
            Some(value) => EvalAnswer::Evaluated(Box::new(InvocationOutcome {
                completion: CompletionOutcome::Normal,
                result: ExactValueOrUnavailable::Exact(value),
                nested_writes: Vec::new(),
                ordered_stores: Vec::new(),
                types: TypeFacts::default(),
                evidence: DependencyEvidence::default(),
            })),
            None => EvalAnswer::Declined(DeclineReason::Unsupported),
        }
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

    fn source_lexer_config(&self) -> Option<LexerConfig> {
        Some(self.driver.lexer_config)
    }

    fn context(&self) -> &AnalysisContext {
        &self.driver.context
    }
}

/// Detached source text retains no original parent operand or point. A Module
/// procedure result needs that independent receipt; this interface declines.
fn detached_procedure_result(
    _module: (
        &crate::interprocedural::ModuleProcedures<'_>,
        &crate::ir::ProcedureId,
    ),
    _script: &str,
    _constants: &HashMap<String, ExactValue>,
    _stance: (BuiltinFoldInputs<'_>, FoldPolicy),
) -> Option<ExactValue> {
    // Detached text has no original parent operand or point receipt. Existing
    // registry folding has its separate contract; a Module procedure join
    // cannot be issued by reparsing equal text at offset zero.
    None
}

/// `node` evaluated by the shared expression route outside a solver run —
/// the value the lattice proves for it over `constants`, or `None` wherever
/// the route declines. It is the one evaluator a rewrite asks
/// (`docs/design/compiler/value-transfers.md` § *`expr`: the first demanding
/// client*): a math function the target lacks, the module rebinds, or Tcl
/// does not call (`ABS`); a beyond-wide integer or an infinity the target
/// does not widen to; a numeral the target's grammar cannot decide — each
/// declines as the lattice does. `folds` carries the rewrite's whole-module
/// trust, and a nested command is never evaluated.
pub(crate) fn evaluate_expression_detached(
    node: &ExprNode,
    constants: &HashMap<String, ExactValue>,
    folds: BuiltinFoldInputs<'_>,
    policy: FoldPolicy,
) -> Option<ExactValue> {
    evaluate_expression_in_module(node, constants, (folds, policy), None)
}

/// [`evaluate_expression_detached`] with descriptive Module context. Detached
/// expression text does not retain original child invocation receipts and cannot
/// consume conditional procedure completion or transfer summaries.
pub(crate) fn evaluate_expression_in_module(
    node: &ExprNode,
    constants: &HashMap<String, ExactValue>,
    (folds, policy): (BuiltinFoldInputs<'_>, FoldPolicy),
    module: Option<(
        &crate::interprocedural::ModuleProcedures<'_>,
        &crate::ir::ProcedureId,
    )>,
) -> Option<ExactValue> {
    let driver = LatticeDriver::detached(Some(folds), policy);
    let expression = ExpressionEvaluation {
        expression: Expression::Parsed(node),
        policy,
        nested: NestedPolicy::EffectFreeOnly,
        head: None,
    };
    let inputs = DetachedExpressionInputs {
        driver: &driver,
        view: ResolvedInvocationView {
            canonical_command: "expr",
            subcommand: None,
            form: None,
            layout: InvocationLayout::Source,
            operands: Vec::new(),
            argument_offset: 0,
            arity: None,
        },
        constants,
        module,
        stance: (folds, policy),
    };
    match evaluate_lifted(
        &expression,
        &inputs,
        &mut driver.budget(),
        MAX_CONSTSET_SIZE,
    ) {
        LiftedAnswer::Evaluated(outcomes) => match outcomes.as_slice() {
            [outcome] => match &outcome.result {
                ExactValueOrUnavailable::Exact(value) => Some(value.clone()),
                ExactValueOrUnavailable::Unavailable(_) => None,
            },
            _ => None,
        },
        LiftedAnswer::Pending | LiftedAnswer::Declined(_) => None,
    }
}

/// Whether the condition `node` is decided outside a solver run: its value
/// on the shared expression route ([`evaluate_expression_detached`]) read
/// as `if` reads a truth, or `None` wherever the route declines or the value
/// is no truth.
pub(crate) fn decide_condition_detached(
    node: &ExprNode,
    constants: &HashMap<String, ExactValue>,
    folds: BuiltinFoldInputs<'_>,
    policy: FoldPolicy,
) -> Option<bool> {
    let value = evaluate_expression_detached(node, constants, folds, policy)?;
    truth_of(&ExactValueOrUnavailable::Exact(value))
}

/// A condition's truth as `if` reads it: a number is true when non-zero, a
/// boolean word is its value, and anything else raises (`expected boolean
/// value`), which decides nothing.
pub(super) fn truth_of(result: &ExactValueOrUnavailable) -> Option<bool> {
    let ExactValueOrUnavailable::Exact(value) = result else {
        return None;
    };
    match value.numeric {
        Some(NumericValue::Int(i)) => Some(i != 0),
        Some(NumericValue::Float(f)) if f.is_nan() => None,
        Some(NumericValue::Float(f)) => Some(f != 0.0),
        Some(NumericValue::Bool(b)) => Some(b),
        None => {
            let text = value.as_str().ok()?;
            if let Some(word) = tcl_syntax::boolean::parse_boolean_word(text) {
                return Some(word);
            }
            // A beyond-wide integer's canonical spelling: non-zero is true.
            // Only the canonical spelling: a leading zero is a text the
            // target's grammar did not read as a number (`08` under 8.x),
            // and `if` raises `expected boolean value` on it.
            let digits = text.strip_prefix('-').unwrap_or(text);
            (!digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0')))
            .then(|| digits != "0")
        }
    }
}

/// An expression's outcome before its writes and completion: `result`, no
/// store, and the evidence of the route that computed it.
fn expression_outcome(
    result: ExactValueOrUnavailable,
    route: EvalRoute,
    implementation: &'static str,
    revision: u64,
    bindings: Vec<BindingIdentity>,
) -> InvocationOutcome {
    InvocationOutcome {
        completion: CompletionOutcome::Normal,
        nested_writes: Vec::new(),
        result,
        ordered_stores: Vec::new(),
        types: TypeFacts::default(),
        evidence: DependencyEvidence {
            bindings,
            route: Some(RouteIdentity {
                route,
                implementation,
                revision,
            }),
            ..DependencyEvidence::default()
        },
    }
}

/// Which expression an [`ExpressionEvaluation`] evaluates.
pub(super) enum Expression<'e> {
    /// The expression the route assembles from the invocation's argument
    /// words ([`ExpressionRoute::assemble`]).
    Assembled(ExpressionRoute),
    /// An expression the lowering already parsed: a fused assignment's, or
    /// a branch condition.
    Parsed(&'e ExprNode),
}

fn expression_lexer_config(input: &dyn AnalysisInputs) -> LexerConfig {
    input
        .source_lexer_config()
        .unwrap_or_else(|| LexerConfig::from_grammar(input.context().grammar))
}

pub(super) fn expression_parse_context(
    input: &dyn AnalysisInputs,
) -> tcl_syntax::expr::parser::ExprParseContext {
    use tcl_syntax::expr::parser::{ExprParseContext, NativeExprSyntax};
    let mut parser = input.context().profile.map_or(
        ExprParseContext {
            lexer_grammar: input.context().grammar,
            expr_grammar_base: None,
            f5_word_grammar: None,
            native_syntax: NativeExprSyntax::Unknown,
        },
        ExprParseContext::for_profile,
    );
    parser.lexer_grammar = expression_lexer_config(input).grammar_over(input.context().grammar);
    parser
}

impl ExpressionEvaluation<'_> {
    /// The language the engine evaluates.
    pub(super) const fn language(&self) -> LanguageProfileId {
        match &self.expression {
            Expression::Assembled(route) => route.language,
            Expression::Parsed(_) => LanguageProfileId::TclExpr,
        }
    }
}

impl CommandSemantics for ExpressionEvaluation<'_> {
    fn identity(&self) -> &'static str {
        ExpressionRoute {
            language: self.language(),
        }
        .identity()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Expression {
            language: self.language(),
        }
    }

    fn variable_reads(&self, input: &dyn AnalysisInputs) -> Vec<String> {
        match &self.expression {
            Expression::Assembled(route) => route.variable_reads(input),
            Expression::Parsed(node) => {
                let config = expression_lexer_config(input);
                let mut reads: Vec<String> = node
                    .vars_element_qualified_with_config(config)
                    .into_iter()
                    .collect();
                reads.sort_unstable();
                reads
            }
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let assembled;
        let node = match &self.expression {
            Expression::Assembled(route) => {
                let source = match route.assemble(input) {
                    Ok(source) => source,
                    Err(answer) => return answer,
                };
                let text = match source.text() {
                    Ok(text) => text,
                    Err(reason) => return EvalAnswer::Declined(reason),
                };
                assembled = crate::expr_parser::parse_expr_with_syntax_context(
                    text,
                    &expression_parse_context(input),
                );
                &assembled
            }
            Expression::Parsed(node) => *node,
        };
        input.prepare_expression_source(node);
        // The operand words substituted before the engine parses them, and
        // the writes their commands made, come first.
        let mut state = input
            .word_state()
            .unwrap_or_else(|| EvaluationState::new(self.nested));
        if let Some(head) = &self.head {
            record_binding(&mut state.evidence, head.clone());
        }
        let answer = {
            let mut services = ExprServices::new(input, &mut state, budget);
            evaluate_expression(node, &mut services, self.policy)
        };
        let (result, completion) = match answer {
            ExprAnswer::Pending => return EvalAnswer::Pending,
            ExprAnswer::Declined(reason) => return EvalAnswer::Declined(reason),
            ExprAnswer::Value(value) => (
                ExactValueOrUnavailable::Exact(value),
                CompletionOutcome::Normal,
            ),
            // The evaluation ended on a nested error: the writes the state
            // holds are the ones that ran before it, and the outcome has no
            // value.
            ExprAnswer::Ended(ended) => match *ended {
                CompletionOutcome::Error {
                    message,
                    error_code,
                    ..
                } => (
                    ExactValueOrUnavailable::Unavailable(FactBounds {
                        existence: Existence::MayBound,
                        intrep: None,
                        shape: None,
                        segments: None,
                        taint: None,
                    }),
                    CompletionOutcome::Error {
                        written: state.writes.len(),
                        message,
                        error_code,
                    },
                ),
                CompletionOutcome::Normal | CompletionOutcome::Code { .. } => {
                    return EvalAnswer::Declined(DeclineReason::StatefulNested);
                }
            },
        };
        let mut outcome = expression_outcome(
            result,
            self.route(),
            self.identity(),
            ExpressionRoute::REVISION,
            state.evidence.bindings,
        );
        outcome.completion = completion;
        outcome.nested_writes = state.writes;
        outcome.evidence.numerals = self.policy.numbers;
        outcome.evidence.release =
            tcl_registry::value_transfer::TargetSemantics::of(self.policy.dialect).release;
        EvalAnswer::Evaluated(Box::new(outcome))
    }
}
