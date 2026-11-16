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

//! Per-block facts a run records for later passes: the template-word plan of each call and the selection of each case-list statement.

use super::super::argument_words::{
    binding_at, call_arguments, plan_clause_count, switch_arguments, view_of,
};
use super::super::lattice_inputs::Words;
use super::super::queries::written_before;
use super::super::{ArgWord, LatticeDriver, LatticeInputs};
use crate::analyses::LatticeValue;
use crate::ir::{CommandTokens, Statement};
use crate::ssa::{SsaFunction, SsaStatement, ValueKey};
use std::cell::RefCell;
use std::collections::HashMap;
use tcl_registry::InvocationWord;
use tcl_registry::value_transfer::{
    AnalysisInputs, FactDomain, FactView, InvocationLayout, OperandId, PlanAnswer, TransferAnswer,
};
use tcl_syntax::word_rules::WordValueRules;

impl LatticeDriver<'_> {
    /// The template-word plan each call of `block` declares, over the
    /// settled lattice — a switch's proven value reads as its spelling —
    /// with the template word's span. Only a trusted call to a command that
    /// performs substitution is asked.
    pub(crate) fn template_plans_in<S: std::hash::BuildHasher>(
        &self,
        ssa: &SsaFunction,
        (block_id, block): (crate::cfg::BlockId, &crate::ssa::SsaBlock),
        values: &HashMap<ValueKey, LatticeValue, S>,
    ) -> Vec<crate::sccp::TemplatePlanRecord> {
        let mut records = Vec::new();
        for (index, stmt_ssa) in block.statements.iter().enumerate() {
            let point = (block_id, index);
            let host = match &stmt_ssa.statement {
                Statement::Call {
                    command,
                    args,
                    tokens: Some(tokens),
                    foreach_groups: None,
                    ..
                } => {
                    let head = stmt_ssa.statement.canonical_command_or_source();
                    if tokens.argv.len() == args.len() + 1 {
                        records.extend(self.template_plan_of(
                            (head, command),
                            args,
                            (tokens, false),
                            (stmt_ssa, point),
                            (values, ssa),
                        ));
                    }
                    tokens
                }
                Statement::AssignValue {
                    tokens: Some(tokens),
                    ..
                } => tokens,
                _ => continue,
            };
            // A `subst` a word substitutes reads the statement's use versions
            // as the statement's own words do, while no earlier substitution
            // in it may have written one (#2451).
            if host.synthetic.is_some() {
                continue;
            }
            let calls = crate::word_subst::lifted_calls(Some(host), self.lexer_config);
            for call in &calls {
                let Some(words) = &call.words else {
                    continue;
                };
                if !written_before(stmt_ssa, Some(call.span.start()), &calls).is_empty() {
                    continue;
                }
                let Some(args) = words.argv_texts.get(1..) else {
                    continue;
                };
                records.extend(self.template_plan_of(
                    (&call.command, &call.command),
                    args,
                    (words, true),
                    (stmt_ssa, point),
                    (values, ssa),
                ));
            }
        }
        records
    }

    /// The template-word plan one trusted call to a command that performs
    /// substitution declares, read at `stmt_ssa`'s use versions.
    fn template_plan_of<S: std::hash::BuildHasher>(
        &self,
        (head, command): (&str, &String),
        args: &[String],
        (tokens, nested): (&CommandTokens, bool),
        (stmt_ssa, point): (&crate::ssa::SsaStatement, (crate::cfg::BlockId, usize)),
        (values, ssa): (&HashMap<ValueKey, LatticeValue, S>, &SsaFunction),
    ) -> Option<crate::sccp::TemplatePlanRecord> {
        if !self.trusted(head) {
            return None;
        }
        // A substituted call's words are read by their spelling: its lifted
        // tokens carry no lowering snapshot of their own.
        let cooked = call_arguments(args, (!nested).then_some(tokens), &self.lexer_config);
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let resolved = self.resolve(head, &words)?;
        if !resolved
            .semantics
            .traits
            .contains(tcl_registry::Traits::PERFORMS_SUBSTITUTION)
        {
            return None;
        }
        let semantics = resolved.semantics.value.semantics()?;
        let inputs = LatticeInputs {
            original: None,
            written_arguments: Vec::new(),
            expression_base: None,
            expression_source: RefCell::new(None),
            nested_source: RefCell::new(None),
            driver: self,
            prior_writes: Vec::new(),
            words: Words::Independent,
            view: view_of(&resolved, &texts, &words, InvocationLayout::Source),
            uses: &stmt_ssa.uses,
            values,
            ssa,
            point: Some(point),
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        let PlanAnswer::TemplateWord(plan) = semantics.structure(&inputs) else {
            return None;
        };
        let &span = tokens.argv.get(plan.operand.0 + 1)?;
        // The spelling each switch reads as, when the lattice proves every
        // one exactly.
        let switches = (0..plan.operand.0)
            .map(
                |index| match inputs.operand(OperandId(index), FactDomain::ExactValue) {
                    FactView::Exact(value, _) => String::from_utf8(value.bytes).ok(),
                    _ => None,
                },
            )
            .collect();
        Some(crate::sccp::TemplatePlanRecord {
            span,
            command: command.clone(),
            switches,
            plan,
        })
    }

    /// The selection each opaque case-list statement of `block` makes over
    /// the settled lattice ([`Self::selection_of`]), and the `Selected`
    /// branch fact of every arm that no member of a statement's subject
    /// reaches ([`unreached_arm_facts`]).
    pub(crate) fn selection_facts_in<S: std::hash::BuildHasher>(
        &self,
        cfg: &crate::cfg::Function,
        ssa: &SsaFunction,
        (block_id, block): (crate::cfg::BlockId, &crate::ssa::SsaBlock),
        values: &HashMap<ValueKey, LatticeValue, S>,
    ) -> (
        Vec<crate::sccp::SelectionRecord>,
        Vec<crate::sccp::ConstantBranch>,
    ) {
        let mut records = Vec::new();
        let mut unreached = Vec::new();
        for stmt_ssa in &block.statements {
            let Some(record) = self.selection_of(cfg, ssa, values, stmt_ssa) else {
                continue;
            };
            if let Statement::Switch { arms, .. } = &stmt_ssa.statement {
                unreached.extend(unreached_arm_facts(cfg.block_name(block_id), &record, arms));
            }
            records.push(record);
        }
        (records, unreached)
    }

    /// The selection one case-list statement makes over the settled
    /// lattice: the command its binding site names ([`binding_at`]) — trusted,
    /// and resolved over the statement's words — asked for its `Selection`
    /// transfer, the words read as the lowering recorded them
    /// ([`switch_arguments`]) and the variables at the statement's use
    /// versions. The fact's arm indices count the command's pattern and body
    /// pairs, which the record reads against the statement's arms and its
    /// final `default`, so a statement whose plan does not read the
    /// statement's own subject (an inliner's renamed one) or its own clause
    /// count, whose words' delimiters the lowering did not record, or whose
    /// transfer declines, has none.
    fn selection_of<S: std::hash::BuildHasher>(
        &self,
        cfg: &crate::cfg::Function,
        ssa: &SsaFunction,
        values: &HashMap<ValueKey, LatticeValue, S>,
        stmt_ssa: &SsaStatement,
    ) -> Option<crate::sccp::SelectionRecord> {
        let Statement::Switch {
            span,
            subject,
            arms,
            default_body,
            raw_args,
            raw_arg_braced,
            raw_arg_quoted,
            ..
        } = &stmt_ssa.statement
        else {
            return None;
        };
        if raw_arg_braced.len() != raw_args.len() || raw_arg_quoted.len() != raw_args.len() {
            return None;
        }
        let head = binding_at(cfg, *span)?;
        if !self.trusted(head) {
            return None;
        }
        let cooked = switch_arguments(
            raw_args,
            (raw_arg_braced, raw_arg_quoted),
            &self.lexer_config,
        );
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let resolved = self.resolve(head, &words)?;
        let semantics = resolved.semantics.value.semantics()?;
        let inputs = LatticeInputs {
            original: None,
            written_arguments: Vec::new(),
            expression_base: None,
            expression_source: RefCell::new(None),
            nested_source: RefCell::new(None),
            driver: self,
            prior_writes: Vec::new(),
            words: Words::Independent,
            view: view_of(&resolved, &texts, &words, InvocationLayout::Source),
            uses: &stmt_ssa.uses,
            values,
            ssa,
            point: None,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        let PlanAnswer::CaseList {
            subject: at,
            arms: plan_arms,
            ..
        } = semantics.structure(&inputs)
        else {
            return None;
        };
        let clauses = arms.len() + usize::from(default_body.is_some());
        if raw_args.get(at.0) != Some(subject)
            || plan_clause_count(
                &inputs,
                &plan_arms,
                WordValueRules::from_config(&self.lexer_config),
            ) != Some(clauses)
        {
            return None;
        }
        let TransferAnswer::Selection(fact) =
            semantics.transfer(FactDomain::Selection, &inputs, &mut self.budget())
        else {
            return None;
        };
        Some(crate::sccp::SelectionRecord {
            span: *span,
            arm_pattern_spans: arms.iter().map(|arm| arm.pattern_span).collect(),
            fact,
        })
    }
}

/// The `Selected` branch fact of each arm of `arms` whose body no member of
/// `record`'s subject runs: `block` holds the statement, the fact's span is
/// the arm's pattern, its condition the pattern's text, its value `false`, and
/// it has no target — no block stands for an arm. An arm that passes its body
/// on with `-` is judged by the body it leads to, so the alternate patterns of
/// a running body are not reported, and the final `default`, which has no
/// pattern, never is.
fn unreached_arm_facts(
    block: &str,
    record: &crate::sccp::SelectionRecord,
    arms: &[crate::ir::SwitchArm],
) -> Vec<crate::sccp::ConstantBranch> {
    if record.fact.selected.is_empty() {
        return Vec::new();
    }
    let body_of = |index: usize| {
        arms[index..]
            .iter()
            .position(|arm| !arm.fallthrough)
            .map_or(arms.len(), |offset| index + offset)
    };
    arms.iter()
        .enumerate()
        .zip(&record.arm_pattern_spans)
        .filter(|((index, _), _)| !record.fact.bodies.contains(&Some(body_of(*index))))
        .map(|((_, arm), span)| crate::sccp::ConstantBranch {
            block: block.to_owned(),
            span: Some(*span),
            condition: arm.pattern.clone(),
            value: false,
            taken_target: String::new(),
            not_taken_target: String::new(),
            kind: crate::sccp::BranchFactKind::Selected,
        })
        .collect()
}
