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

//! Embedded substitutions: a statement's word effects evaluated in order with the host statement whose words they are.

use super::super::expressions::Expression;
use super::super::folded_type::FoldedType;
use super::super::run_records::{answer_label, outcome_paths};
use super::super::{
    DefAnswer, ExistenceStep, ExpressionEvaluation, Lattice, LatticeDriver, Ordered, binding_of,
    lattice_of_outcomes, named_defs, place_named, result_of,
};
use crate::analyses::LatticeValue;
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Statement};
use crate::sccp::{DefValues, RaisedDefs};
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey, Version};
use std::collections::HashMap;
use tcl_registry::value_transfer::{
    AnalysisInputs, CommandSemantics, CompletionOutcome, EvalRoute, LiftedAnswer, NestedPolicy,
    OperandId, PlaceRef, StoreOutcome, TargetId,
};

impl LatticeDriver<'_> {
    /// A statement's word effects and the statement whose words they are,
    /// paired by construction ([`crate::ssa::word_effects_host`]) and
    /// evaluated once (§ `expr`): the host's words run in order under one
    /// ordered state, from the versions the definition point reads. Its
    /// definitions take what the state's writes left in each place — a place
    /// no write reached keeps what it held — and the host's take the
    /// result. `None` wherever the pair is not one this evaluates or the
    /// evaluation declines, and both keep the answers they have without it.
    ///
    /// The host is an `AssignExpr` or an `ExprEval`, whose expression the
    /// engine runs, or an `AssignValue` whose word is one command
    /// substitution on the engine's or a registry-owned route, whose own
    /// substituting words and nested commands are the ones evaluated in
    /// order. Every other host — a `Call`, a `Return`, a condition, a value
    /// that is a command without such a route — keeps the effect-free
    /// policy.
    pub(crate) fn evaluate_embedded<S: std::hash::BuildHasher>(
        &self,
        block: &crate::ssa::SsaBlock,
        index: usize,
        values: &HashMap<ValueKey, LatticeValue, S>,
        ssa: &SsaFunction,
    ) -> Option<(usize, EmbeddedAnswer)> {
        let host_index = crate::ssa::word_effects_host(&block.statements, index)?;
        let (call, host) = (&block.statements[index], &block.statements[host_index]);
        let incoming = incoming_versions(block, (index, host_index), values);
        let started = self.tally.get();
        self.explaining(Some(call.statement.span()));
        let answer = self.embedded_pair(call, host, (&incoming, values, ssa));
        self.explaining(None);
        if answer.is_none() {
            self.tally.set(started);
        }
        answer.map(|answer| (host_index, answer))
    }

    /// [`Self::evaluate_embedded`] over the versions in force where the
    /// call sits.
    fn embedded_pair<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        call: &SsaStatement,
        host: &SsaStatement,
        lattice: Lattice<'_, S1, S2>,
    ) -> Option<EmbeddedAnswer> {
        let original = self.original_expression_parent(&host.statement);
        let ordered = self.ordered_embedded_host(&host.statement, original.as_ref(), lattice)?;
        let defs = named_defs(call, lattice.2);
        let outcomes = match ordered.answer {
            LiftedAnswer::Pending => {
                let pending = defs
                    .iter()
                    .map(|(_, key)| DefAnswer {
                        existence: ExistenceStep::PENDING,
                        ..DefAnswer::untyped(*key, LatticeValue::Unknown)
                    })
                    .collect();
                return Some(EmbeddedAnswer {
                    call: pending,
                    host: DefValues::Each(LatticeValue::Unknown, None),
                    raised: None,
                });
            }
            LiftedAnswer::Declined(_) => return None,
            LiftedAnswer::Evaluated(outcomes) => outcomes,
        };
        // An error completion publishes only the writes it ran (the prefix
        // rule): where a raise ends the path the call's definitions take them
        // and the host's keep what they held, and elsewhere the pair is not
        // evaluated.
        let mut raised: Option<usize> = None;
        let errors = outcomes
            .iter()
            .filter(|outcome| matches!(outcome.completion, CompletionOutcome::Error { .. }))
            .count();
        if errors > 0 {
            if errors < outcomes.len() || !self.raises_end_paths() {
                return None;
            }
            raised = outcomes
                .iter()
                .filter_map(|outcome| match outcome.completion {
                    CompletionOutcome::Error { written, .. } => Some(written),
                    _ => None,
                })
                .min();
        } else if outcomes
            .iter()
            .any(|outcome| outcome.completion != CompletionOutcome::Normal)
        {
            return None;
        }
        let inputs = self.expression_inputs(lattice);
        let mut joined: Option<Vec<DefAnswer>> = None;
        for member in &ordered.writes {
            let answers = self.embedded_defs(member, &defs, &inputs)?;
            joined = Some(match joined {
                None => answers,
                Some(earlier) => earlier
                    .into_iter()
                    .zip(&answers)
                    .map(|(left, right)| left.join(right))
                    .collect(),
            });
        }
        if let Some(written) = raised {
            return Some(EmbeddedAnswer {
                call: joined?,
                host: DefValues::Raised(Box::new(RaisedDefs {
                    prefix: None,
                    written,
                })),
                raised,
            });
        }
        let value = lattice_of_outcomes(&outcomes, result_of);
        let folded = ordered
            .folded
            .filter(|_| value != LatticeValue::Overdefined);
        Some(EmbeddedAnswer {
            call: joined?,
            host: DefValues::Each(value, folded),
            raised: None,
        })
    }

    fn ordered_embedded_host<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        statement: &Statement,
        original: Option<&CommandTokens>,
        lattice: Lattice<'_, S1, S2>,
    ) -> Option<Ordered> {
        Some(match statement {
            Statement::AssignExpr {
                expr,
                command_binding,
                expr_base,
                ..
            } => self.ordered_expression(
                expr,
                command_binding.as_ref(),
                original.zip(*expr_base),
                lattice,
            )?,
            Statement::ExprEval {
                expr,
                command_binding,
                expr_base,
                ..
            } => self.ordered_expression(
                expr,
                Some(command_binding),
                original.zip(*expr_base),
                lattice,
            )?,
            Statement::AssignValue { value, .. } => {
                self.ordered_script(value, statement, lattice)?
            }
            _ => return None,
        })
    }

    /// The parsed expression a host runs, evaluated by the engine under
    /// `LocalWrites` from the versions the call reads. `None` when the
    /// module rebinds the head.
    pub(super) fn ordered_expression<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        expr: &ExprNode,
        command_binding: Option<&tcl_runtime_api::CommandBindingIdentity>,
        original: Option<(&CommandTokens, u32)>,
        (uses, values, ssa): Lattice<'_, S1, S2>,
    ) -> Option<Ordered> {
        let head = command_binding.map_or("expr", |binding| binding.name.as_str());
        if self.folds.is_some() && !self.trusted(head) {
            return None;
        }
        self.enter_expression();
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(expr),
            policy: self.policy,
            nested: NestedPolicy::LocalWrites,
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
        let writes = match &answer {
            LiftedAnswer::Evaluated(outcomes) => outcomes
                .iter()
                .map(|outcome| outcome.nested_writes.clone())
                .collect(),
            LiftedAnswer::Pending | LiftedAnswer::Declined(_) => Vec::new(),
        };
        Some(Ordered {
            answer,
            writes,
            folded: None,
        })
    }

    /// A value word that is one command substitution, run under
    /// `LocalWrites`: the invocation's own substituting words and the
    /// commands nested in them evaluate in order, its own stores follow, and
    /// the result is the word's. A command whose route is neither the
    /// expression engine's nor a registry-owned evaluator, and a word that
    /// is more than the one substitution, stay with the effect-free policy.
    fn ordered_script<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        value: &str,
        statement: &Statement,
        lattice: Lattice<'_, S1, S2>,
    ) -> Option<Ordered> {
        use tcl_lexer::word_parts::{SubstFlags, WordBody, WordPart as Part, decompose};
        // The word is that one substitution and nothing else: `[a][b]` is two.
        let WordBody::Parts(parts) =
            decompose(value.as_bytes(), SubstFlags::default(), self.lexer_config)
        else {
            return None;
        };
        let [Part::Command(script)] = parts.as_slice() else {
            return None;
        };
        let inner = std::str::from_utf8(script).ok()?;
        let original = self.original_value_script(statement, inner);
        let run = self.run_script(
            inner,
            lattice,
            (Vec::new(), NestedPolicy::LocalWrites),
            original.as_ref(),
        )?;
        // A head the module rebinds has no route; a procedure of the module
        // is run as its summary says.
        if !run.procedure
            && !matches!(
                run.route,
                Some(EvalRoute::Expression { .. } | EvalRoute::Direct { .. })
            )
        {
            return None;
        }
        self.explain(&run.head, run.route, answer_label(&run.answer));
        let folded = match &run.answer {
            LiftedAnswer::Evaluated(outcomes) => FoldedType::join_all(
                outcomes
                    .iter()
                    .map(|outcome| FoldedType::of_result(outcome)),
            ),
            LiftedAnswer::Pending | LiftedAnswer::Declined(_) => None,
        };
        Some(Ordered {
            answer: run.answer,
            writes: run.writes,
            folded,
        })
    }

    /// The retained script of `statement`'s value word when that word is the
    /// one command substitution `inner`. A supplied module runs a nested
    /// command only through its original tokens.
    pub(in super::super) fn original_value_script(
        &self,
        statement: &Statement,
        inner: &str,
    ) -> Option<crate::interprocedural::OriginalSummaryScript> {
        let module = self.module?;
        let parent = module.original_statement_tokens(self.body_id?.procedure_id(), statement)?;
        let (_, site) = parent.words().last()?.sole_command_substitution()?;
        module.original_substitution(&parent, site, inner)
    }

    /// The call's definitions after one member's `writes`: a place the
    /// writes name takes the last value written, and one they leave alone
    /// keeps what it held. `None` when a write names a place the call does
    /// not define — no definition would carry it — or two writes name one
    /// cell as an element and its array.
    pub(in super::super) fn embedded_defs(
        &self,
        writes: &[(PlaceRef, StoreOutcome)],
        defs: &[(String, ValueKey)],
        input: &dyn AnalysisInputs,
    ) -> Option<Vec<DefAnswer>> {
        if writes
            .iter()
            .any(|(place, _)| !defs.iter().any(|(name, _)| *name == place.name))
        {
            return None;
        }
        let mut placed: Vec<(PlaceRef, &StoreOutcome)> = Vec::with_capacity(writes.len());
        for (place, store) in writes {
            if placed
                .iter()
                .any(|(seen, _)| seen.overlaps_as_element_and_base(place))
            {
                return None;
            }
            placed.push((place.clone(), store));
        }
        let untouched = StoreOutcome::Preserve {
            target: TargetId(OperandId(0)),
        };
        for (name, _) in defs {
            let place = place_named(name);
            if !placed
                .iter()
                .any(|(seen, _)| seen.shares_storage_with(&place))
            {
                placed.push((place, &untouched));
            }
        }
        Some(self.defs_from_placed(&placed, None, defs, input))
    }
}

/// What a synthetic embedded-substitution call and its host statement leave
/// in their definitions ([`LatticeDriver::evaluate_embedded`]).
pub(crate) struct EmbeddedAnswer {
    /// The call's definitions, one answer each.
    pub(crate) call: Vec<DefAnswer>,
    /// The host's definitions.
    pub(crate) host: DefValues,
    /// How many writes ran before the error, when the pair certainly raises:
    /// the call's definitions are those writes' and the host never runs.
    pub(crate) raised: Option<usize>,
}

/// The versions the host of the embedded call at `index` reads: those it
/// and the call record, with each place the call defines at the version
/// ahead of the call — the host's own records name the call's definitions,
/// and its words that run before the writes read what was there. A place no
/// definition and no root value has ahead of the call is read as unknown
/// rather than waited for.
fn incoming_versions<S: std::hash::BuildHasher>(
    block: &crate::ssa::SsaBlock,
    (index, host): (usize, usize),
    values: &HashMap<ValueKey, LatticeValue, S>,
) -> HashMap<Symbol, Version> {
    let (call, host) = (&block.statements[index], &block.statements[host]);
    let mut incoming: HashMap<Symbol, Version> = host.uses.clone();
    incoming.extend(
        call.uses
            .iter()
            .map(|(&symbol, &version)| (symbol, version)),
    );
    for &symbol in call.defs.keys() {
        let version = crate::sccp::prior_version(block, index, symbol);
        if version == 0 && !values.contains_key(&(symbol, 0)) {
            incoming.remove(&symbol);
        } else {
            incoming.insert(symbol, version);
        }
    }
    incoming
}
