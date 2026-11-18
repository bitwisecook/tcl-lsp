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

//! Running a `[...]` script on its declared route: resolving its command, lifting the route's answer and placing its writes.

use super::argument_words::{OperandSource, view_of};
use super::expressions::Expression;
use super::folded_type::FoldedType;
use super::lattice_inputs::{Words, exact_to_lattice};
use super::procedure_calls::effective_procedure_arguments;
use super::run_records::answer_label;
use super::{
    ArgWord, ExpressionEvaluation, Lattice, LatticeDriver, LatticeInputs, ScriptRun, binding_of,
    lattice_const_text, lattice_of_outcomes, result_of, split_head,
};
use crate::analyses::{LatticeValue, MAX_CONSTSET_SIZE};
use crate::ir::{CommandTokens, Statement};
use crate::ssa::{SsaFunction, Symbol, ValueKey, Version};
use std::cell::RefCell;
use std::collections::HashMap;
use tcl_lexer::TokenType;
use tcl_registry::value_transfer::builtins::ExpressionRoute;
use tcl_registry::value_transfer::{
    AnalysisInputs, BindingIdentity, Budget, BudgetLimit, CommandSemantics, DeclineReason,
    EvalAnswer, EvalRoute, EvaluationState, EvaluatorOwner, ExactValue, InvocationLayout,
    LiftedAnswer, NestedPolicy, PlaceRef, StoreOutcome, evaluate_lifted, validate_outcome,
};
use tcl_registry::{InvocationWordKind, ResolvedInvocation};

impl LatticeDriver<'_> {
    /// Fold a `[cmd args…]` command substitution through the resolved
    /// command's declared route, then — when the caller holds the trust
    /// fact — through the registry's constant-fold engine. The value comes
    /// with the folded type the route's evaluation states; the engine's
    /// text states none.
    pub(crate) fn fold_cmd_subst<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        value: &str,
        statement: &Statement,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<(LatticeValue, Option<FoldedType>)> {
        // Only a word that is one whole command substitution: `[a][b]` also
        // starts with `[` and ends with `]`, but its value is the two
        // results joined (#2396).
        if !tcl_lexer::word_parts::is_whole_command_substitution(
            value.as_bytes(),
            self.lexer_config,
        ) {
            return None;
        }
        let original = value
            .strip_prefix('[')
            .and_then(|script| script.strip_suffix(']'))
            .and_then(|inner| self.original_value_script(statement, inner));
        if let Some(folded) =
            self.fold_cmd_subst_routes(value, original.as_ref(), uses, values, ssa)
        {
            return Some(folded);
        }
        // Registry const-fold fallback: the fold's `$var` words resolve at
        // this statement's use versions, so a folded value re-enters the
        // lattice and downstream statements see it — the multi-hop chain
        // the declared routes cannot close yet. Checked after them so
        // single-hop results stay byte-identical, and only for a caller that
        // turns the engine on: the shared per-unit lattice holds the trust
        // fact with the engine off, so its fold surface stays the routes'.
        let f = self.folds.filter(|f| f.registry_engine)?;
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        let trusts = |name: &str| f.mutations.trusts(name);
        let lookup = |name: &str| lattice_const_text(name, uses, values, ssa);
        let folded = crate::const_subst::ConstSubstCtx {
            registry: f.registry,
            resolution_namespace: &self.context.namespace,
            // This detached value spelling retains no original physical
            // namespace/token owner for executable dependencies.
            namespace_context: None,
            version: f
                .dialect
                .and_then(tcl_dialect::DialectProfile::const_fold_version),
            defining_class: f.defining_class,
            trusts: &trusts,
            lookup_var: &lookup,
            script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
        }
        .fold_cmd_subst(inner)?;
        Some((exact_to_lattice(&ExactValue::from_literal(&folded)), None))
    }

    /// The declared-route half of [`Self::fold_cmd_subst`]: the one command
    /// the substitution holds, run on its declared route
    /// ([`Self::run_script`]) under the effect-free nested policy, with the
    /// folded type every member's result states ([`FoldedType::join_all`]).
    pub(super) fn fold_cmd_subst_routes<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        value: &str,
        original: Option<&crate::interprocedural::OriginalSummaryScript>,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<(LatticeValue, Option<FoldedType>)> {
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        let run = self.run_script(
            inner,
            (uses, values, ssa),
            (Vec::new(), NestedPolicy::EffectFreeOnly),
            original,
        )?;
        // Binding validity comes first: after `rename list mylist` or a
        // shadowing `proc format …` anywhere in the unit, `[list a 1]` is a
        // call to something else entirely, and the const-fold engine the
        // caller falls back to asks the same question itself.
        if run.rebound {
            return None;
        }
        // In value position a nested invocation runs under the effect-free
        // policy: an outcome with stores has no definition to land on here.
        let folded = match &run.answer {
            LiftedAnswer::Evaluated(outcomes)
                if outcomes.iter().all(|outcome| !outcome.has_stores()) =>
            {
                Some(lattice_of_outcomes(outcomes, result_of))
                    .filter(|folded| *folded != LatticeValue::Overdefined)
                    .map(|folded| {
                        let typed = FoldedType::join_all(
                            outcomes
                                .iter()
                                .map(|outcome| FoldedType::of_result(outcome)),
                        );
                        (folded, typed)
                    })
            }
            LiftedAnswer::Pending => Some((LatticeValue::Unknown, None)),
            LiftedAnswer::Evaluated(_) | LiftedAnswer::Declined(_) => None,
        };
        self.explain(
            &run.head,
            run.route,
            match (&run.answer, &folded) {
                (LiftedAnswer::Evaluated(outcomes), None)
                    if outcomes.iter().any(|outcome| outcome.has_stores()) =>
                {
                    "not substituted: the outcome writes storage".to_owned()
                }
                _ => answer_label(&run.answer),
            },
        );
        folded
    }

    /// The one command a `[…]` script holds, resolved and run on its
    /// declared route over this statement's lattice inputs: a
    /// registry-owned evaluator or the expression engine. `None` when the
    /// script is not one command the registry resolves to a declaration.
    ///
    /// `prior` holds the writes the enclosing evaluations made before the
    /// command runs, which its reads consult first, and `policy` says
    /// whether the writes it makes are placed ([`ScriptRun::writes`]) for
    /// the caller to apply.
    pub(super) fn run_script<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        script: &str,
        lattice: Lattice<'_, S1, S2>,
        under: (Vec<(PlaceRef, StoreOutcome)>, NestedPolicy),
        original: Option<&crate::interprocedural::OriginalSummaryScript>,
    ) -> Option<ScriptRun> {
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            script,
            original.map_or(
                0,
                super::super::interprocedural::OriginalSummaryScript::base,
            ),
            tcl_lexer::SourceSegmentationPurpose::NestedScript.config(self.lexer_config),
        );
        let [seg] = commands.as_slice() else {
            return None;
        };
        if split_head(script).0 != seg.name() {
            return None;
        }
        if original.is_some_and(|owner| owner.text() != script) {
            return None;
        }
        self.run_command(
            seg,
            lattice,
            under,
            original.and_then(|owner| owner.command_at(seg.span.start())),
        )
    }

    /// A segmented command's arguments as the resolver and an evaluator read
    /// them, each cooked from its token. The segmenter drops a `{*}` prefix
    /// from the token and flags the word instead, so the flag decides that a
    /// word expands: `[list {*}{a b}]` has two elements, not the one `a b`.
    pub(super) fn cooked_args<'s>(
        &self,
        seg: &'s crate::segmenter::SegmentedCommand,
    ) -> Vec<ArgWord<'s>> {
        let expands = |at: usize| {
            seg.expand_word
                .as_ref()
                .and_then(|flags| flags.get(at + 1))
                .copied()
                .unwrap_or(false)
        };
        seg.arg_tokens()
            .iter()
            .zip(seg.arg_single_token())
            .zip(seg.args())
            .enumerate()
            .map(|(at, ((token, &single), text))| {
                if expands(at) {
                    return ArgWord::spelled(
                        text,
                        InvocationWordKind::Expanded,
                        OperandSource::Unknown,
                    );
                }
                // A quoted-opening token counts its `"` as a delimiter byte.
                let quoted = token.kind == TokenType::Esc && token.content_offset > 0;
                ArgWord::of_token(text, (token.kind, quoted), single, &self.lexer_config)
            })
            .collect()
    }

    /// One segmented command resolved against the registry and evaluated on
    /// its declared route, `prior` being the writes made ahead of it.
    pub(super) fn run_command<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        seg: &crate::segmenter::SegmentedCommand,
        lattice: Lattice<'_, S1, S2>,
        under: (Vec<(PlaceRef, StoreOutcome)>, NestedPolicy),
        original: Option<&CommandTokens>,
    ) -> Option<ScriptRun> {
        if let Some(run) = self.procedure_run(seg, original, lattice, (&under.0, under.1)) {
            return Some(run);
        }
        let head = seg.name();
        if let Some(module) = self.module {
            // A supplied Module is an actual-source ingress. Missing child
            // ownership cannot reopen the standalone catalogue route.
            let original = original?;
            if module.has_original_native_source() {
                let writes_land = !matches!(under.1, NestedPolicy::EffectFreeOnly);
                return module.with_original_native_value(
                    original,
                    writes_land,
                    |operation, schema| {
                        let cooked = self.operation_arguments(operation)?;
                        let written = (0..cooked.len())
                            .map(|index| operation.effective.written_argument(index))
                            .collect();
                        self.run_resolved_command(
                            head,
                            schema,
                            (&cooked, written, Some(original)),
                            lattice,
                            under,
                        )
                    },
                );
            }
            return module.with_original_operation(original, |operation, schema| {
                let cooked = self.operation_arguments(operation)?;
                let written = (0..cooked.len())
                    .map(|index| operation.effective.written_argument(index))
                    .collect();
                self.run_resolved_command(
                    head,
                    schema,
                    (&cooked, written, Some(original)),
                    lattice,
                    under,
                )
            });
        }
        if !self.trusted(head) {
            return Some(ScriptRun {
                head: head.to_owned(),
                route: None,
                answer: LiftedAnswer::Declined(DeclineReason::RebindingSuspected),
                writes: Vec::new(),
                binding: binding_of(head, head),
                rebound: true,
                procedure: false,
            });
        }
        let cooked = self.cooked_args(seg);
        let words: Vec<_> = cooked.iter().map(ArgWord::word).collect();
        let resolved = self.resolve(head, &words)?;
        let written = (0..cooked.len()).map(Some).collect();
        self.run_resolved_command(
            head,
            &resolved,
            (&cooked, written, original),
            lattice,
            under,
        )
    }

    pub(super) fn operation_arguments<'w>(
        &self,
        operation: &'w crate::registry_invocation::ResolvedStatementInvocation,
    ) -> Option<Vec<ArgWord<'w>>> {
        if operation.effective.words.iter().any(|word| {
            matches!(
                word,
                crate::ir::WordExpr::Expand { .. } | crate::ir::WordExpr::Opaque { .. }
            )
        }) {
            return None;
        }
        let literals = (0..operation.effective.words.len().checked_sub(1)?)
            .map(|index| {
                operation.effective.argument_literal(
                    index,
                    self.lexer_config.escapes,
                    tcl_syntax::word_rules::WordValueRules::from_config(&self.lexer_config),
                )
            })
            .collect();
        Some(effective_procedure_arguments(
            &operation.effective,
            literals,
        ))
    }

    fn run_resolved_command<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        head: &str,
        resolved: &ResolvedInvocation<'_, '_>,
        (cooked, written_arguments, original): (
            &[ArgWord<'_>],
            Vec<Option<usize>>,
            Option<&CommandTokens>,
        ),
        (uses, values, ssa): Lattice<'_, S1, S2>,
        (prior, policy): (Vec<(PlaceRef, StoreOutcome)>, NestedPolicy),
    ) -> Option<ScriptRun> {
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<_> = cooked.iter().map(ArgWord::word).collect();
        let binding = binding_of(head, resolved.canonical_command);
        // An existence query reads the rung, not a route.
        if let Some(kind) = crate::existence_query::kind_of(resolved.semantics.operation) {
            return Some(ScriptRun {
                head: head.to_owned(),
                route: None,
                answer: self.existence_answer(kind, (&words, &texts), (ssa, &prior)),
                writes: Vec::new(),
                binding,
                rebound: false,
                procedure: false,
            });
        }
        let semantics = resolved.semantics.value.semantics()?;
        let view = view_of(resolved, &texts, &words, InvocationLayout::Source);
        let mut inputs = LatticeInputs {
            original: original.cloned(),
            written_arguments,
            expression_base: None,
            expression_source: RefCell::new(None),
            nested_source: RefCell::new(None),
            driver: self,
            prior_writes: prior,
            words: Words::under(policy),
            view,
            uses,
            values,
            ssa,
            point: None,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        // A command that stores reads its targets' roles from its words as
        // a statement's does, so a word the lattice proves no longer hides
        // them (`set b $a`).
        if policy != NestedPolicy::EffectFreeOnly {
            inputs.resolve_roles_over_values(resolved);
        }
        let route = semantics.route();
        // A word that raises ends the invocation before the command runs,
        // whichever operands the route reads; the flag stays set for the
        // word, statement or script that holds this command.
        let outer = self.word_error.replace(false);
        let answer = self.route_answer(semantics, &inputs, (&binding, policy));
        let raised = self.word_error.get();
        self.word_error.set(outer || raised);
        let answer = if raised {
            LiftedAnswer::Declined(DeclineReason::StatefulNested)
        } else {
            answer
        };
        let answer = inputs.carrying_word_writes(answer, route);
        let answer = match original
            .and_then(|tokens| super::compiled_constants::compiled_constant(tokens, resolved))
        {
            Some(representation) => {
                super::compiled_constants::with_representation(answer, representation)
            }
            None => answer,
        };
        let (answer, writes) = match policy {
            NestedPolicy::EffectFreeOnly => (answer, Vec::new()),
            NestedPolicy::LocalWrites | NestedPolicy::Protected => {
                self.placed_answer(semantics, &inputs, answer)
            }
        };
        Some(ScriptRun {
            head: head.to_owned(),
            route: Some(route),
            answer,
            writes,
            binding,
            rebound: false,
            procedure: false,
        })
    }

    /// The answer of `semantics`'s declared route over `inputs`, lifted
    /// over one finite input: a registry-owned evaluator, the expression
    /// engine over the analysis services, or a declared implementation.
    /// `binding` is the head the answer rests on, and `nested` the policy the
    /// expression engine admits nested invocations under.
    fn route_answer(
        &self,
        semantics: &dyn CommandSemantics,
        inputs: &dyn AnalysisInputs,
        (binding, nested): (&BindingIdentity, NestedPolicy),
    ) -> LiftedAnswer {
        match semantics.route() {
            EvalRoute::Direct { id } => {
                self.enter_direct();
                match id.owner() {
                    EvaluatorOwner::Registry => {
                        evaluate_lifted(semantics, inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
                    }
                }
            }
            EvalRoute::Expression { language } => {
                self.enter_expression();
                // A pack's arity or option row can switch the declared
                // route off; the engine adapter never reads the
                // declaration, so the driver asks it first.
                match semantics
                    .as_declared()
                    .and_then(|declared| declared.invocation_decline(inputs))
                {
                    Some(EvalAnswer::Pending) => LiftedAnswer::Pending,
                    Some(EvalAnswer::Declined(reason)) => LiftedAnswer::Declined(reason),
                    Some(EvalAnswer::Evaluated(_)) | None => {
                        let expression = ExpressionEvaluation {
                            expression: Expression::Assembled(ExpressionRoute { language }),
                            policy: self.policy,
                            nested,
                            head: Some(binding.clone()),
                        };
                        evaluate_lifted(&expression, inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
                    }
                }
            }
            EvalRoute::None { reason } => LiftedAnswer::Declined(DeclineReason::NoRoute(reason)),
            // The specialisation's `evaluate` resolves the declared inputs
            // and runs the body in this thread's host, on the bindings it
            // declares it rests on.
            EvalRoute::Implementation(capability) => {
                if self.unheld_binding(&capability).is_some() {
                    return LiftedAnswer::Declined(DeclineReason::RebindingSuspected);
                }
                self.enter_implementation();
                evaluate_lifted(semantics, inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
            }
        }
    }

    /// `answer` with each evaluated outcome's writes placed, in execution
    /// order: the writes its substitutions made, then its own ordered
    /// stores, each resolved to its place after the structural check a
    /// statement's outcome passes ([`validate_outcome`]). An outcome with a
    /// store that names no place the state can name — a computed one, an
    /// element beside its array, a traced place — is `StatefulNested`.
    fn placed_answer(
        &self,
        semantics: &dyn CommandSemantics,
        inputs: &dyn AnalysisInputs,
        answer: LiftedAnswer,
    ) -> (LiftedAnswer, Vec<Vec<(PlaceRef, StoreOutcome)>>) {
        let LiftedAnswer::Evaluated(outcomes) = &answer else {
            return (answer, Vec::new());
        };
        let plan = semantics.structure(inputs);
        let targets = semantics.store_targets(inputs);
        let mut writes = Vec::with_capacity(outcomes.len());
        for outcome in outcomes {
            let own = validate_outcome(&plan, &targets, outcome).and_then(|()| {
                self.placed_stores(outcome, inputs)
                    .map_err(|_| DeclineReason::StatefulNested)
            });
            match own {
                Ok(own) => {
                    let mut all = outcome.nested_writes.clone();
                    all.extend(own.into_iter().map(|(place, store)| (place, store.clone())));
                    writes.push(all);
                }
                Err(reason) => return (LiftedAnswer::Declined(reason), Vec::new()),
            }
        }
        (answer, writes)
    }

    /// The nested-substitution service: `script` under `state`'s policy, its
    /// binding and every binding its answer rests on recorded in the state's
    /// evidence. A protected script ([`NestedPolicy::Protected`]) runs every
    /// command it holds ([`Self::protected_script`]); under any other policy
    /// the script is the one command a substitution holds
    /// ([`Self::nested_command`]).
    pub(super) fn nested_answer<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        script: &str,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> EvalAnswer {
        let depth = self.nesting.get();
        if depth >= Budget::EVALUATION_DEPTH {
            return EvalAnswer::Declined(DeclineReason::Budget(BudgetLimit::Depth));
        }
        self.nesting.set(depth + 1);
        let answer = match state.policy {
            NestedPolicy::Protected => self.protected_script(script, state, from),
            NestedPolicy::EffectFreeOnly | NestedPolicy::LocalWrites => {
                self.nested_command(script, state, from)
            }
        };
        self.nesting.set(depth);
        answer
    }

    /// The one command of `script` under `state`'s policy. The command reads
    /// what the state and `from`'s enclosing evaluations have written first.
    fn nested_command<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        script: &str,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> EvalAnswer {
        let prior: Vec<(PlaceRef, StoreOutcome)> = from
            .prior_writes
            .iter()
            .chain(&state.writes)
            .cloned()
            .collect();
        let original = from.nested_source.borrow().clone();
        let Some(run) = self.run_script(
            script,
            (from.uses, from.values, from.ssa),
            (prior, state.policy),
            original.as_ref(),
        ) else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        match self.settle(run, state) {
            Ok(outcome) => EvalAnswer::Evaluated(outcome),
            Err(answer) => answer,
        }
    }
}
