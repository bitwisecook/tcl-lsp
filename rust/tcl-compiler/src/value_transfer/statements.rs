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

//! Typed statements over the lattice: calls, `incr`, `expr`, loop headers, `catch` ends and embedded substitutions.

use super::argument_words::{
    OperandSource, amount_word, call_arguments, typed_node_commands, view_of, word_of,
};
use super::lattice_inputs::{Words, exact_to_const};
use super::{
    ArgWord, CatchEndRecord, DefAnswer, ExistenceStep, Lattice, LatticeDriver, LatticeInputs,
    named_defs, widened,
};
use crate::analyses::{ConstValue, LatticeValue};
use crate::codegen_helpers::split_list_values;
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Statement};
use crate::sccp::{
    DefValues, RaisedDefs, extract_foreach_elements, resolve_foreach_list_via_lattice,
};
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey, Version};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use tcl_lexer::LexerConfig;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::value_transfer::{
    BinderName, BindingKind, CompletionOutcome, ExactValue, Existence, InvocationLayout,
    IterableKind, LiftedAnswer, PlanAnswer,
};
use tcl_registry::{InvocationWord, ResolvedInvocation};

mod embedded;
mod outcome_defs;
mod plans_and_selections;

impl LatticeDriver<'_> {
    /// Hand the driver the function's flattened `catch` regions.
    pub(crate) fn catch_ends(&self, cfg: &crate::cfg::Function) {
        *self.catch_ends.borrow_mut() = cfg
            .catch_ends
            .iter()
            .filter_map(|end| {
                let name = cfg.blocks.get(&end.end)?.name.clone();
                let record = CatchEndRecord {
                    entry: end.entry,
                    call: end.call.clone(),
                };
                Some((name, record))
            })
            .collect();
    }

    /// The result and options variables of a flattened `catch`, which the
    /// statement that ends its region defines. That statement stands where the
    /// body's flow has joined and has no words, so the `catch` the analysis
    /// build kept beside it is evaluated as the statement form is, over the
    /// state before the body — the versions the block before the body exits
    /// with, and the existence `exit_of` gives for that block: the body's own
    /// statements are in the flow, so what the script wrote is not applied
    /// again, and a script that reads what it writes finds it as it was.
    /// `None` for every other statement.
    pub(crate) fn evaluate_catch_end<S: std::hash::BuildHasher>(
        &self,
        (block, index): (&crate::ssa::SsaBlock, usize),
        values: &HashMap<ValueKey, LatticeValue, S>,
        ssa: &SsaFunction,
        exit_of: impl FnOnce(crate::cfg::BlockId) -> Option<Vec<Existence>>,
    ) -> Option<DefValues> {
        if index != 0 {
            return None;
        }
        let (entry, call) = {
            let ends = self.catch_ends.borrow();
            let end = ends.get(block.name.as_str())?;
            (end.entry, end.call.clone())
        };
        let marker = block.statements.first()?;
        let before = ssa.blocks.get(&entry)?.exit_versions.clone();
        let shadow = SsaStatement {
            statement: call,
            uses: before.clone(),
            defs: marker.defs.clone(),
            may_defs: marker.may_defs.clone(),
            destruction_defs: marker.destruction_defs.clone(),
            quoted_uses: HashSet::new(),
            name_only_uses: HashSet::new(),
        };
        // The script runs over the existence the block before the body
        // leaves, as it does over that block's versions.
        let _entry = self.body_entry_scope(exit_of(entry));
        self.explaining(Some(shadow.statement.span()));
        let answer = self.evaluate_call(&shadow, values, ssa, &before);
        self.explaining(None);
        Some(answer)
    }

    /// The typed `Incr` statement: its node projects to the invocation view
    /// of the command whose lowering it is (`incr name ?amount?`), and the
    /// registry's derived cell update evaluates it. A braced amount is its
    /// own text (`incr x {$n}` raises in every release), never a read.
    /// Answers each of `defs`, the statement's definitions.
    pub(crate) fn evaluate_incr<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        name: &str,
        amount: Option<(&str, bool)>,
        defs: &[(String, ValueKey)],
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> DefValues {
        let widen = || DefValues::PerDef(widened(defs));
        let heads = typed_node_commands(self.registry, LoweringHookId::Incr);
        let Some(&head) = heads.first() else {
            return widen();
        };
        if !heads.iter().all(|head| self.trusted(head)) {
            self.explain(head, None, "declined: rebinding-suspected".to_owned());
            return widen();
        }
        let texts: Vec<&str> = std::iter::once(name)
            .chain(amount.map(|(text, _)| text))
            .collect();
        let words: Vec<InvocationWord<'_>> = std::iter::once(InvocationWord::Literal(name))
            .chain(amount.map(amount_word))
            .collect();
        let Some(resolved) = self.resolve(head, &words) else {
            return widen();
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            return widen();
        };
        let sources = std::iter::once(OperandSource::Literal)
            .chain(amount.map(|(text, braced)| {
                if braced {
                    OperandSource::BracedLiteral
                } else if matches!(word_of(text), InvocationWord::Dynamic) {
                    OperandSource::Substituted
                } else {
                    OperandSource::Literal
                }
            }))
            .collect();
        let view = view_of(&resolved, &texts, &words, InvocationLayout::Source);
        let inputs = LatticeInputs {
            original: None,
            written_arguments: Vec::new(),
            expression_base: None,
            expression_source: RefCell::new(None),
            nested_source: RefCell::new(None),
            driver: self,
            prior_writes: Vec::new(),
            words: Words::Independent,
            view,
            uses,
            values,
            ssa,
            point: None,
            sources,
        };
        self.call_defs(head, semantics, defs, &inputs)
    }

    pub(crate) const fn source_lexer_config(&self) -> LexerConfig {
        self.lexer_config
    }

    /// Original reference grammar needs a valid retained metadata mode and profile.
    /// Withdrawn or availability-only source ownership cannot use a parser default.
    pub(crate) fn source_reference_lexer_config(&self) -> Option<LexerConfig> {
        self.metadata_context?;
        self.context.profile?;
        Some(self.lexer_config)
    }

    pub(crate) fn original_expression_parent(
        &self,
        statement: &Statement,
    ) -> Option<CommandTokens> {
        self.module?
            .original_statement_tokens(self.body_id?.procedure_id(), statement)
    }

    /// A `Call` statement's defs, each with its value. The synthetic loop
    /// header the CFG builder emits — the one `Call` carrying
    /// `foreach_groups` — has a declared structure the solver applies: its
    /// iteration plan's single list binder takes the set of the list's
    /// elements. Any other resolved call takes the ordered stores its
    /// registry-owned evaluation makes ([`Self::apply_outcome`]); every
    /// other call keeps the conservative answer.
    pub(crate) fn evaluate_call<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        stmt_ssa: &SsaStatement,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
        uses: &HashMap<Symbol, Version, S1>,
    ) -> DefValues {
        let defs = named_defs(stmt_ssa, ssa);
        let Statement::Call {
            args,
            defs: binders,
            tokens,
            foreach_groups,
            ..
        } = &stmt_ssa.statement
        else {
            return DefValues::PerDef(widened(&defs));
        };
        let head = stmt_ssa.statement.canonical_command_or_source();
        if !self.trusted(head) {
            self.explain(head, None, "declined: rebinding-suspected".to_owned());
            return DefValues::PerDef(widened(&defs));
        }
        if foreach_groups.is_none() {
            let cooked = call_arguments(args, tokens.as_ref(), &self.lexer_config);
            return self.evaluate_source_call(
                head,
                &cooked,
                &defs,
                (uses, values, ssa),
                (true, tokens.as_ref()),
            );
        }
        let (bound, existence) = self.evaluate_loop_header(head, args, binders, uses, values, ssa);
        DefValues::PerDef(
            defs.iter()
                .map(|(name, key)| {
                    let value = bound
                        .iter()
                        .find(|(binder, _)| binder == name)
                        .map_or(LatticeValue::Overdefined, |(_, value)| value.clone());
                    DefAnswer {
                        existence,
                        ..DefAnswer::untyped(*key, value)
                    }
                })
                .collect(),
        )
    }

    /// Whether a statement with no definitions certainly raises, where a
    /// throw leaves from: the answer is `Some` only for a statement the
    /// registry's routes prove an error. A call to a command with no
    /// semantics records nothing, as the solver evaluates no statement
    /// that has no definition anywhere else.
    pub(crate) fn probe_completion<S: std::hash::BuildHasher>(
        &self,
        stmt_ssa: &SsaStatement,
        values: &HashMap<ValueKey, LatticeValue, S>,
        ssa: &SsaFunction,
    ) -> Option<DefValues> {
        self.explaining(Some(stmt_ssa.statement.span()));
        let answer = match &stmt_ssa.statement {
            Statement::Call {
                args,
                tokens,
                foreach_groups: None,
                ..
            } => {
                let head = stmt_ssa.statement.canonical_command_or_source();
                self.trusted(head).then(|| {
                    let cooked = call_arguments(args, tokens.as_ref(), &self.lexer_config);
                    self.evaluate_source_call(
                        head,
                        &cooked,
                        &[],
                        (&stmt_ssa.uses, values, ssa),
                        (false, tokens.as_ref()),
                    )
                })
            }
            Statement::ExprEval {
                expr,
                command_binding,
                expr_base,
                ..
            } => {
                let original = self.original_expression_parent(&stmt_ssa.statement);
                Some(self.evaluate_expr_eval(
                    expr,
                    command_binding,
                    original.as_ref().zip(*expr_base),
                    &stmt_ssa.uses,
                    values,
                    ssa,
                ))
            }
            _ => None,
        };
        self.explaining(None);
        answer.filter(|answer| matches!(answer, DefValues::Raised(_)))
    }

    /// An `expr` statement's expression run by the engine, which has no
    /// definition to give: it raises or it does not.
    pub(crate) fn evaluate_expr_eval<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        expr: &ExprNode,
        command_binding: &tcl_runtime_api::CommandBindingIdentity,
        original: Option<(&CommandTokens, u32)>,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> DefValues {
        let normal = || DefValues::Each(LatticeValue::Overdefined, None);
        if self.folds.is_some() && !self.trusted(&command_binding.name) {
            return normal();
        }
        let Some(ordered) =
            self.ordered_expression(expr, Some(command_binding), original, (uses, values, ssa))
        else {
            return normal();
        };
        match Self::raised_expression(&ordered.answer) {
            Some(raised) => DefValues::Raised(Box::new(raised)),
            None => normal(),
        }
    }

    /// The error an expression's answer certainly ends in, for a statement
    /// that stores nothing of its own: the writes the expression's commands
    /// made before it are the call's definitions, never this statement's.
    pub(super) fn raised_expression(answer: &LiftedAnswer) -> Option<RaisedDefs> {
        let LiftedAnswer::Evaluated(outcomes) = answer else {
            return None;
        };
        let mut written = usize::MAX;
        for outcome in outcomes {
            let CompletionOutcome::Error { written: ran, .. } = outcome.completion else {
                return None;
            };
            written = written.min(ran);
        }
        (!outcomes.is_empty()).then_some(RaisedDefs {
            prefix: None,
            written,
        })
    }

    /// The synthetic loop header's value for each binder of its iteration
    /// plan over one list: the set of the elements it takes — binder `i` of
    /// `n` the elements at `i`, `i + n`, …, and the empty string where the
    /// last iteration runs past the list's end. A repeated binder takes its
    /// last position's. A multi-list header, an empty list, or a list the
    /// analysis cannot read leaves every binder `Overdefined`.
    ///
    /// Beside the values, the existence step every binder takes at the
    /// header: bound once the list is proven to have an element — every
    /// later visit of the header follows an iteration that bound it —
    /// left as it was when the list is proven empty and the plan binds
    /// nothing on zero iterations, and otherwise a may-bind.
    fn evaluate_loop_header<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        head: &str,
        args: &[String],
        defs: &[String],
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> (Vec<(String, LatticeValue)>, ExistenceStep) {
        const MAY_BIND: ExistenceStep = ExistenceStep::Join(Existence::Bound(BindingKind::Scalar));
        const BINDS: ExistenceStep = ExistenceStep::Set(Existence::Bound(BindingKind::Scalar));
        let texts: Vec<&str> = args.iter().map(String::as_str).collect();
        let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
        let Some(resolved) = self.resolve(head, &words) else {
            return (Vec::new(), MAY_BIND);
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            self.explain(head, None, "declined: no-semantics".to_owned());
            return (Vec::new(), MAY_BIND);
        };
        let view = view_of(
            &resolved,
            &texts,
            &words,
            InvocationLayout::LoopHeader { binders: defs },
        );
        let inputs = LatticeInputs {
            original: None,
            written_arguments: Vec::new(),
            expression_base: None,
            expression_source: RefCell::new(None),
            nested_source: RefCell::new(None),
            driver: self,
            prior_writes: Vec::new(),
            words: Words::Independent,
            view,
            uses,
            values,
            ssa,
            point: None,
            sources: words
                .iter()
                .map(|word| match word {
                    InvocationWord::Literal(_) => OperandSource::Literal,
                    _ => OperandSource::Unknown,
                })
                .collect(),
        };
        let PlanAnswer::Iterate(plan) = semantics.structure(&inputs) else {
            return (Vec::new(), MAY_BIND);
        };
        let IterableKind::List(iterable) = &plan.iterable else {
            return (Vec::new(), MAY_BIND);
        };
        let Some(list) = texts.get(iterable.0) else {
            return (Vec::new(), MAY_BIND);
        };
        let binders: Vec<&str> = plan
            .binders
            .iter()
            .filter_map(|binder| match &binder.name {
                BinderName::Declared(name) => Some(name.as_str()),
                BinderName::Operand(id) => texts.get(id.0).copied(),
            })
            .collect();
        if binders.len() != plan.binders.len() || binders.is_empty() {
            return (Vec::new(), MAY_BIND);
        }
        let rules = self.policy.word_rules;
        let elements = extract_foreach_elements(list, rules)
            .or_else(|| {
                let config = self.source_reference_lexer_config()?;
                resolve_foreach_list_via_lattice(list, uses, values, ssa, rules, config)
            })
            .or_else(|| {
                // `foreach v [list a b c]` — fold the command substitution
                // to a constant list string, then split into elements.
                let arg = list.trim();
                if arg.starts_with('[')
                    && arg.ends_with(']')
                    && let Some((LatticeValue::Const(ConstValue::String(s)), _)) =
                        self.fold_cmd_subst_routes(arg, None, uses, values, ssa)
                {
                    return Some(split_list_values(&s, rules));
                }
                None
            });
        let existence = match &elements {
            Some(items) if !items.is_empty() || plan.zero_iterations_bind => BINDS,
            Some(_) => ExistenceStep::PRESERVE,
            None => MAY_BIND,
        };
        let Some(items) = elements.filter(|items| !items.is_empty()) else {
            return (Vec::new(), existence);
        };
        let bound = Self::loop_binder_values(&binders, &items);
        (bound, existence)
    }

    fn loop_binder_values(binders: &[&str], items: &[String]) -> Vec<(String, LatticeValue)> {
        let stride = binders.len();
        binders
            .iter()
            .enumerate()
            .map(|(at, name)| {
                // A repeated binder is assigned at each of its positions, so
                // it holds its last one's element.
                let position = binders
                    .iter()
                    .rposition(|other| other == name)
                    .unwrap_or(at);
                let consts: Vec<ConstValue> = (0..items.len().div_ceil(stride))
                    .map(|iteration| {
                        let element = items
                            .get(iteration * stride + position)
                            .map_or("", String::as_str);
                        exact_to_const(&ExactValue::from_literal(element))
                    })
                    .collect();
                let value = match consts.as_slice() {
                    [only] => LatticeValue::Const(only.clone()),
                    _ => LatticeValue::constset(consts),
                };
                ((*name).to_owned(), value)
            })
            .collect()
    }

    /// A call in its source layout, its arguments read as source words: the
    /// stores its registry-owned evaluation makes, applied to the variables
    /// it defines ([`Self::call_defs`]).
    fn evaluate_source_call<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        head: &str,
        cooked: &[ArgWord<'_>],
        defs: &[(String, ValueKey)],
        (uses, values, ssa): Lattice<'_, S1, S2>,
        (explain_missing, original): (bool, Option<&CommandTokens>),
    ) -> DefValues {
        if let Some(module) = self.module {
            let Some(original) = original else {
                return DefValues::PerDef(widened(defs));
            };
            if module.has_original_native_source() {
                let Some(body) = self.body_id else {
                    return DefValues::PerDef(widened(defs));
                };
                return module
                    .with_original_native_store(
                        body.procedure_id(),
                        original,
                        defs,
                        ssa,
                        |operation, schema| {
                            let cooked = self.operation_arguments(operation)?;
                            let written = (0..cooked.len())
                                .map(|index| operation.effective.written_argument(index))
                                .collect();
                            Some(self.evaluate_resolved_source_call(
                                head,
                                schema,
                                (&cooked, written, Some(original)),
                                defs,
                                (uses, values, ssa),
                                explain_missing,
                            ))
                        },
                    )
                    .unwrap_or_else(|| DefValues::PerDef(widened(defs)));
            }
            return module
                .with_original_operation(original, |operation, schema| {
                    let cooked = self.operation_arguments(operation)?;
                    let written = (0..cooked.len())
                        .map(|index| operation.effective.written_argument(index))
                        .collect();
                    Some(self.evaluate_resolved_source_call(
                        head,
                        schema,
                        (&cooked, written, Some(original)),
                        defs,
                        (uses, values, ssa),
                        explain_missing,
                    ))
                })
                .unwrap_or_else(|| DefValues::PerDef(widened(defs)));
        }
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let Some(resolved) = self.resolve(head, &words) else {
            return DefValues::PerDef(widened(defs));
        };
        let written = (0..cooked.len()).map(Some).collect();
        self.evaluate_resolved_source_call(
            head,
            &resolved,
            (cooked, written, original),
            defs,
            (uses, values, ssa),
            explain_missing,
        )
    }

    fn evaluate_resolved_source_call<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        head: &str,
        resolved: &ResolvedInvocation<'_, '_>,
        (cooked, written_arguments, original): (
            &[ArgWord<'_>],
            Vec<Option<usize>>,
            Option<&CommandTokens>,
        ),
        defs: &[(String, ValueKey)],
        (uses, values, ssa): Lattice<'_, S1, S2>,
        explain_missing: bool,
    ) -> DefValues {
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let Some(semantics) = resolved.semantics.value.semantics() else {
            if explain_missing {
                self.explain(head, None, "declined: no-semantics".to_owned());
            }
            return DefValues::PerDef(widened(defs));
        };
        let view = view_of(resolved, &texts, &words, InvocationLayout::Source);
        let mut inputs = LatticeInputs {
            original: original.cloned(),
            written_arguments,
            expression_base: None,
            expression_source: RefCell::new(None),
            nested_source: RefCell::new(None),
            driver: self,
            prior_writes: Vec::new(),
            words: Words::Independent,
            view,
            uses,
            values,
            ssa,
            point: None,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        inputs.resolve_roles_over_values(resolved);
        self.call_defs(head, semantics, defs, &inputs)
    }
}
