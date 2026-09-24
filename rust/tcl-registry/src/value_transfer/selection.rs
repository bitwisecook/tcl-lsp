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

//! The `switch` selection contract (`docs/design/compiler/value-transfers.md`
//! § *`switch`*, step 2): the case-list plan the command's `CaseListSpec`
//! reads, and the arm each member of a proven subject selects through the
//! shared switch core (`tcl_cmd_core::switch::{parse_options,
//! select_analysis}`).
//!
//! The selection is ordered first match, a final `default` matching
//! anything, and a fall-through body running the next arm's body; the
//! regexp mode runs the ARE engine on the analysis path, where only a
//! completed match or a completed no-match selects; `-indexvar` and
//! `-matchvar` are `Write` outcomes on their variable words. Every word the
//! selection reads must be an exact value — an option, a pattern, a body,
//! the clause list — and a pattern that cannot be evaluated declines the
//! whole fact, so a consumer never reads a selection one member of the
//! subject might not make.

use tcl_cmd_core::switch::{Mode, Options, Selection, parse_options, select_analysis};
use tcl_dialect::model::SpecSurface;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_regex::cmd_core::AreEngine;
use tcl_syntax::value::ValueOps;

use crate::hover::OptionSpec;
use crate::invocation_words::InvocationWordKind;
use crate::spec::{CaseListSpec, CaseMatchMode};

use super::CommandSemantics;
use super::answers::{
    CaseArms, EvalAnswer, PlanAnswer, SelectionContract, SelectionFact, StoreOutcome,
    TransferAnswer,
};
use super::const_ops::{ConstOps, ConstValue, Needs, TargetSemantics};
use super::context::{AnalysisContext, Budget};
use super::decline::{DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, FactDomain, FactView, InvocationLayout, OperandId, TargetId};
use super::regex::{failure_reason, metered};
use super::route::EvalRoute;

/// The axes the selection reads: the ARE feature set and its limits (the
/// regexp mode), case folding (`-nocase`), the source decoding of a
/// non-ASCII word, and the list rules the clause-list word is split and the
/// `-indexvar` / `-matchvar` values are rendered under.
const NEEDS: Needs = Needs::REGEXP_FEATURES
    .union(Needs::COLLATION)
    .union(Needs::SOURCE_ENCODING)
    .union(Needs::LIST_RENDERING);

/// A case-list command that declares `switch`'s execution contract: its
/// grammar (`case_list`), the option rows whose effects select the mode and
/// case folding (`options`), and the command's own surface, which an option
/// row without one inherits. The shipped `switch` declares it; a pack's
/// dispatch-table command names the same contract rather than having it
/// inferred from its grammar.
#[derive(Debug, Clone, Copy)]
pub struct SwitchSemantics {
    /// The case-list grammar the plan reads.
    pub case_list: CaseListSpec,
    /// The command's option table.
    pub options: &'static [OptionSpec],
    /// The command's own surface.
    pub surface: Option<&'static [SpecSurface]>,
}

impl CommandSemantics for SwitchSemantics {
    fn identity(&self) -> &'static str {
        "case-list:switch"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Unauthored,
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        self.plan(input)
    }

    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        budget: &mut Budget,
    ) -> TransferAnswer {
        if domain != FactDomain::Selection {
            return TransferAnswer::Generic;
        }
        match self.selection(input, budget) {
            Ok(fact) => TransferAnswer::Selection(fact),
            Err(reason) => TransferAnswer::Declined(reason),
        }
    }

    /// The command has structure but no value of its own: its result is
    /// the selected body's.
    fn evaluate(&self, _input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        EvalAnswer::Declined(DeclineReason::NotAValue)
    }
}

impl SwitchSemantics {
    /// The option rows `context`'s target reads alike: under a named
    /// release, the rows its surface admits; with none named, only the
    /// rows every release has, because a row some releases lack is a bad
    /// option to them.
    fn options_for(&self, context: &AnalysisContext) -> Vec<&'static OptionSpec> {
        let named = TargetSemantics::of(context.profile).release.is_some();
        let query = context.profile.map(DialectProfile::surface_query);
        self.options
            .iter()
            .filter(|option| {
                if named {
                    option.supports_dialect(query, self.surface)
                } else {
                    option.surface.is_none()
                }
            })
            .collect()
    }

    /// The case-list plan: the release-aware layout `case_list` reads over
    /// the operands' spellings — the subject, the arms, the match mode and
    /// case folding the option rows select. An expanded or opaque word, a
    /// layout the descriptor abstains on, and the synthetic loop-header
    /// layout decline.
    fn plan(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let view = input.invocation();
        if !matches!(view.layout, InvocationLayout::Source) {
            return PlanAnswer::Declined(DeclineReason::Unsupported);
        }
        let first = view.argument_offset;
        let operands = view.operands.get(first..).unwrap_or_default();
        if operands.iter().any(|operand| {
            matches!(
                operand.kind,
                InvocationWordKind::Expanded | InvocationWordKind::Opaque
            )
        }) {
            return PlanAnswer::Declined(DeclineReason::NotExact);
        }
        let words: Vec<&str> = operands.iter().map(|operand| operand.text).collect();
        let options = self.options_for(input.context());
        let query = input.context().profile.map(DialectProfile::surface_query);
        let Some(layout) = self.case_list.invocation(&words, &options, query) else {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        let Some(subject) = layout.subject_index else {
            return PlanAnswer::Declined(DeclineReason::Unsupported);
        };
        let arms = match (layout.clause_list_index, layout.inline_clause_start) {
            (Some(list), _) => CaseArms::List(OperandId(first + list)),
            (None, Some(start)) => CaseArms::Words(
                (start..words.len().saturating_sub(1))
                    .step_by(2)
                    .map(|pattern| {
                        let body = pattern + 1;
                        let falls_through = Some(words[body]) == self.case_list.fallthrough_body;
                        (
                            OperandId(first + pattern),
                            (!falls_through).then_some(OperandId(first + body)),
                        )
                    })
                    .collect(),
            ),
            (None, None) => return PlanAnswer::Declined(DeclineReason::Unsupported),
        };
        PlanAnswer::CaseList {
            subject: OperandId(first + subject),
            arms,
            fallthrough: self.case_list.fallthrough_body,
            selection: SelectionContract {
                mode: layout.mode,
                nocase: layout.nocase,
                final_default: self.case_list.keyword_patterns_require_final
                    && !self.case_list.keyword_patterns.is_empty(),
            },
        }
    }

    /// The selection fact: the plan's layout, confirmed by the shared
    /// core's own option scan over the words' values for every member of
    /// the subject, and the core's selection per member.
    fn selection(
        &self,
        input: &dyn AnalysisInputs,
        budget: &mut Budget,
    ) -> Result<SelectionFact, DeclineReason> {
        let PlanAnswer::CaseList {
            subject,
            arms,
            selection,
            ..
        } = self.plan(input)
        else {
            return Err(DeclineReason::Unsupported);
        };
        // A specialised comparison (9.1's `-integer`) is not the core's.
        if selection.mode == CaseMatchMode::Other {
            return Err(DeclineReason::Unsupported);
        }
        let members = match input.operand(subject, FactDomain::ExactValue) {
            FactView::Exact(value, _) => vec![value],
            FactView::Finite(values, _) => values,
            FactView::Top(reason) => return Err(reason),
            FactView::Pending => return Err(DeclineReason::NotExact),
            FactView::Domain(_) => return Err(DeclineReason::MalformedAnswer),
        };
        let view = input.invocation();
        let first = view.argument_offset;
        let bounded_scan = TargetSemantics::of(input.context().profile)
            .release
            .is_some_and(|release| release >= TclVersion::V8_5);
        let mut ops = ConstOps::admit(input.context(), budget, NEEDS)?;
        // The core's argv, name-stripped: every word but the subject exactly.
        let mut argv = Vec::with_capacity(view.operands.len() - first);
        for index in first..view.operands.len() {
            argv.push(if index == subject.0 {
                ConstValue::text("")
            } else {
                exact(input, OperandId(index))?
            });
        }
        let at = subject.0 - first;
        let (patterns, bodies) = clauses(&mut ops, &arms, &argv, first)?;
        for pattern in &patterns {
            ops.admissible_text(pattern)?;
        }
        let mut fact = SelectionFact {
            selected: Vec::with_capacity(members.len()),
            bodies: Vec::with_capacity(members.len()),
            writes: Vec::with_capacity(members.len()),
        };
        let mut written: Vec<Vec<(TargetId, ConstValue)>> = Vec::with_capacity(members.len());
        for member in &members {
            let member = ConstValue::from_exact(member);
            ops.admissible_text(&member)?;
            argv[at] = member.clone();
            let options = parse_options(&mut ops, &argv).map_err(|error| ops.decline(&error))?;
            // The core must read the layout the plan read: the same
            // subject, mode and case folding. Before 8.5 every leading word
            // that starts with `-` is scanned, so a subject spelled that
            // way is an option there unless `--` ended the run.
            let ended = at > 0 && argv[at - 1].as_utf8() == Some("--");
            if options.value_index != at
                || !same_mode(options.mode, selection.mode)
                || options.nocase != selection.nocase
                || (!bounded_scan && !ended && member.bytes.first() == Some(&b'-'))
            {
                return Err(DeclineReason::Unsupported);
            }
            let chosen = metered(&mut ops, |ops, analysis| {
                select_analysis::<ConstOps<'_>, AreEngine, ConstValue>(
                    ops, &options, &member, &patterns, analysis,
                )
            })?
            .map_err(|failure| failure_reason(&ops, &failure))?;
            match chosen {
                Selection::NoMatch => {
                    fact.selected.push(None);
                    fact.bodies.push(None);
                    written.push(Vec::new());
                }
                Selection::Matched { index, writes } => {
                    let body = (index..bodies.len())
                        .find(|&arm| !bodies[arm])
                        .ok_or(DeclineReason::WrongRepresentation)?;
                    fact.selected.push(Some(index));
                    fact.bodies.push(Some(body));
                    written.push(write_targets(&options, writes, first)?);
                }
            }
        }
        if let Some(fault) = ops.fault() {
            return Err(fault);
        }
        let counts: Vec<usize> = written.iter().map(Vec::len).collect();
        let (targets, values): (Vec<TargetId>, Vec<ConstValue>) =
            written.into_iter().flatten().unzip();
        let mut stores = targets
            .into_iter()
            .zip(ops.take_all(values)?)
            .map(|(target, value)| StoreOutcome::Write { target, value });
        for count in counts {
            fact.writes.push(stores.by_ref().take(count).collect());
        }
        Ok(fact)
    }
}

/// Operand `id`'s exact value, or the reason it has none.
fn exact(input: &dyn AnalysisInputs, id: OperandId) -> Result<ConstValue, DeclineReason> {
    match input.operand(id, FactDomain::ExactValue) {
        FactView::Exact(value, _) => Ok(ConstValue::from_exact(&value)),
        FactView::Top(reason) => Err(reason),
        FactView::Finite(..) => Err(DeclineReason::CorrelatedSets),
        FactView::Pending => Err(DeclineReason::NotExact),
        FactView::Domain(_) => Err(DeclineReason::MalformedAnswer),
    }
}

/// The patterns, and per arm whether its body is the fall-through body,
/// as the command reads them before it matches: the clause-list word split
/// under the target's list rules, or the words themselves. An empty list,
/// a pattern with no body, and a final fall-through body are the
/// command's own errors, raised before any pattern is tried.
fn clauses(
    ops: &mut ConstOps<'_>,
    arms: &CaseArms,
    argv: &[ConstValue],
    first: usize,
) -> Result<(Vec<ConstValue>, Vec<bool>), DeclineReason> {
    let words: Vec<ConstValue> = match arms {
        CaseArms::List(list) => {
            let elements = ops
                .list_elements(&argv[list.0 - first])
                .map_err(|error| ops.decline_value(&error))?;
            if elements.is_empty() || elements.len() % 2 != 0 {
                return Err(DeclineReason::WrongRepresentation);
            }
            elements
        }
        CaseArms::Words(pairs) => pairs
            .iter()
            .flat_map(|(pattern, body)| {
                let body = body.map_or_else(
                    // An arm the plan read as spelled with the fall-through
                    // body: the word itself is that body.
                    || ConstValue::text("-"),
                    |body| argv[body.0 - first].clone(),
                );
                [argv[pattern.0 - first].clone(), body]
            })
            .collect(),
    };
    let (patterns, bodies): (Vec<ConstValue>, Vec<bool>) = words
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (pair[0].clone(), pair[1].as_utf8() == Some("-")))
        .unzip();
    if bodies.last().is_none_or(|&falls| falls) {
        return Err(DeclineReason::WrongRepresentation);
    }
    Ok((patterns, bodies))
}

/// Whether the core's mode is the plan's.
fn same_mode(core: Mode, plan: CaseMatchMode) -> bool {
    matches!(
        (core, plan),
        (Mode::Exact, CaseMatchMode::Exact)
            | (Mode::Glob, CaseMatchMode::Glob)
            | (Mode::Regexp, CaseMatchMode::Regexp)
    )
}

/// The core's `-indexvar` / `-matchvar` writes on their variable words, in
/// the order the core makes them: the index variable's, then the match
/// variable's.
fn write_targets(
    options: &Options<ConstValue>,
    writes: Vec<(ConstValue, ConstValue)>,
    first: usize,
) -> Result<Vec<(TargetId, ConstValue)>, DeclineReason> {
    let targets: Vec<TargetId> = options
        .index_var_at
        .into_iter()
        .chain(options.match_var_at)
        .map(|at| TargetId(OperandId(first + at)))
        .collect();
    if targets.len() != writes.len() {
        return Err(DeclineReason::MalformedAnswer);
    }
    Ok(targets
        .into_iter()
        .zip(writes)
        .map(|(target, (_, value))| (target, value))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_transfer::LiteralInputs;

    const SWITCH: SwitchSemantics = SwitchSemantics {
        case_list: CaseListSpec::SWITCH,
        options: &[],
        surface: None,
    };

    /// With no option rows every leading dash word is a bad option to the
    /// plan, so an option-free call is what an empty table reads.
    #[test]
    fn an_option_free_call_selects_through_the_core() {
        let inputs = LiteralInputs::new("switch", None, &["b", "a", "A", "b", "B"], None);
        let TransferAnswer::Selection(fact) =
            SWITCH.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation())
        else {
            panic!("a selection");
        };
        assert_eq!(fact.selected, [Some(1)]);
        assert_eq!(fact.bodies, [Some(1)]);
        assert!(fact.writes.iter().all(Vec::is_empty));
    }
}
