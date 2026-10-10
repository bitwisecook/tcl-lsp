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
//!
//! `case`, `switch`'s obsolete 8.x ancestor, declares its own contract over
//! the same plan ([`CaseSemantics`]): no options, glob matching, pattern
//! lists, a `default` fallback wherever it stands, and no fall-through body,
//! through the shared `tcl_cmd_core::case::select`.

use tcl_cmd_core::switch::{Mode, Options, Selection, parse_options, select_analysis};
use tcl_dialect::model::SpecSurface;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_regex::cmd_core::AreEngine;
use tcl_syntax::value::ValueOps;

use super::answers::ExactValue;
use super::decline::Axis;
use super::inputs::WordPart;

use crate::hover::OptionSpec;
use crate::invocation_words::InvocationWordKind;
use crate::spec::{CaseListSpec, CaseMatchMode};

use super::CommandSemantics;
use super::answers::{
    CaseArms, EvalAnswer, PlanAnswer, SelectionContract, SelectionFact, StoreOutcome,
    TransferAnswer,
};
use super::const_ops::{ConstOps, ConstValue, Needs, TargetSemantics, WorkUnits};
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
        case_list_plan(&self.case_list, self.options, self.surface, input)
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

/// The option rows `context`'s target reads alike: under a named release,
/// the rows its surface admits; with none named, only the rows every
/// release has, because a row some releases lack is a bad option to them.
fn options_for(
    options: &'static [OptionSpec],
    surface: Option<&'static [SpecSurface]>,
    context: &AnalysisContext,
) -> Vec<&'static OptionSpec> {
    let named = TargetSemantics::of(context.profile).release.is_some();
    let query = context.profile.map(DialectProfile::surface_query);
    options
        .iter()
        .filter(|option| {
            if named {
                option.supports_dialect(query, surface)
            } else {
                option.surface.is_none()
            }
        })
        .collect()
}

/// The case-list plan: the release-aware layout `case_list` reads over the
/// operands' spellings, with the option rows the target admits — the
/// subject, the arms, the match mode and case folding the rows select, or
/// the descriptor's own comparison. An expanded or opaque word, a layout
/// the descriptor abstains on, and the synthetic loop-header layout
/// decline.
fn case_list_plan(
    case_list: &CaseListSpec,
    options: &'static [OptionSpec],
    surface: Option<&'static [SpecSurface]>,
    input: &dyn AnalysisInputs,
) -> PlanAnswer {
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
    let options = options_for(options, surface, input.context());
    let query = input.context().profile.map(DialectProfile::surface_query);
    let Some(layout) = case_list.invocation(&words, &options, query) else {
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
                    let falls_through = Some(words[body]) == case_list.fallthrough_body;
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
        fallthrough: case_list.fallthrough_body,
        selection: SelectionContract {
            mode: layout.mode,
            nocase: layout.nocase,
            final_default: !case_list.keyword_patterns.is_empty(),
        },
    }
}

/// The members of a proven subject: one for an exact value, each of a
/// finite set's, or the reason there are none.
fn subject_members(
    input: &dyn AnalysisInputs,
    subject: OperandId,
) -> Result<Vec<ExactValue>, DeclineReason> {
    match input.operand(subject, FactDomain::ExactValue) {
        FactView::Exact(value, _) => Ok(vec![value]),
        FactView::Finite(values, _) => Ok(values),
        FactView::Top(reason) => Err(reason),
        FactView::Pending => Err(DeclineReason::NotExact),
        FactView::Domain(_) => Err(DeclineReason::MalformedAnswer),
    }
}

/// Whether operand `id`'s word reads as its value on every path the
/// releases run it on. 9.1b0's byte-compiled `switch` recognises a
/// fall-through body only as a bare word (`IsFallthroughToken`,
/// `tclCompCmdsSZ.c`, measures the word token with its quotes or braces),
/// its interpreted path by value, and a word with a substitution sends the
/// whole command to the interpreted path. So a literal written braced or
/// quoted — or a word whose structure the inputs do not give — reads two
/// ways there.
fn read_by_value_everywhere(input: &dyn AnalysisInputs, id: OperandId) -> bool {
    input.word_structure(id).is_ok_and(|structure| {
        !(structure.braced || structure.quoted)
            || structure.parts.iter().any(|part| {
                matches!(
                    part,
                    WordPart::VariableRead { .. } | WordPart::Script { .. }
                )
            })
    })
}

/// Per arm, whether its body is the fall-through spelling a release that
/// may be 9.1 reads two ways: only a body word of the separate-words
/// form can be delimited, and only under a profile that may be 9.1.
fn delimited_fallthroughs(
    input: &dyn AnalysisInputs,
    arms: &CaseArms,
    bodies: &[bool],
    may_be_91: bool,
) -> Vec<bool> {
    match arms {
        CaseArms::Words(pairs) if may_be_91 => pairs
            .iter()
            .zip(bodies)
            .map(|((pattern, _), &falls)| {
                falls && !read_by_value_everywhere(input, OperandId(pattern.0 + 1))
            })
            .collect(),
        _ => vec![false; bodies.len()],
    }
}

impl SwitchSemantics {
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
        } = self.structure(input)
        else {
            return Err(DeclineReason::Unsupported);
        };
        // A specialised comparison (9.1's `-integer`) is not the core's.
        if selection.mode == CaseMatchMode::Other {
            return Err(DeclineReason::Unsupported);
        }
        let members = subject_members(input, subject)?;
        let view = input.invocation();
        let first = view.argument_offset;
        let release = TargetSemantics::of(input.context().profile).release;
        let bounded_scan = release.is_some_and(|release| release >= TclVersion::V8_5);
        let may_be_91 = release.is_none_or(|release| release >= TclVersion::V9_1);
        // With no release named the core reads 9.0's options and patterns: a
        // `-regexp` pattern only 9.1 compiles (`\z`) then fails to compile,
        // which declines here and is never taken for a raise.
        let version = release.unwrap_or(TclVersion::V9_0);
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
        let fallthrough = self.case_list.fallthrough_body;
        let (patterns, bodies) = clauses(&mut ops, &arms, (&argv, first), fallthrough)?;
        for pattern in &patterns {
            ops.admissible_text(pattern)?;
        }
        let two_ways = delimited_fallthroughs(input, &arms, &bodies, may_be_91);
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
            let options =
                parse_options(&mut ops, &argv, version).map_err(|error| ops.decline(&error))?;
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
                    ops, &options, &member, &patterns, version, analysis,
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
                    if two_ways[index..body].contains(&true) {
                        return Err(DeclineReason::ReleaseAmbiguous(Axis::Availability(
                            SpecSurface::TCL91[0],
                        )));
                    }
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

/// The axes `case`'s selection reads: the source decoding of a non-ASCII
/// word, and the list rules its clause-list word and its pattern lists
/// split under.
const CASE_NEEDS: Needs = Needs::SOURCE_ENCODING.union(Needs::LIST_RENDERING);

/// `case`'s selection contract (Tcl 8.4 to 8.6, and iRules on their 8.4
/// base): the same case-list plan as `switch`, read with no option rows,
/// and the clause `Tcl_CaseObjCmd` runs per member of a proven subject
/// through the shared `tcl_cmd_core::case::select` — glob matching, a
/// pattern word holding whitespace or a backslash a list of patterns, a
/// `default` fallback wherever it stands, and no fall-through body, so the
/// arm selected is the arm whose body runs and nothing is written.
#[derive(Debug, Clone, Copy)]
pub struct CaseSemantics {
    /// The case-list grammar the plan reads.
    pub case_list: CaseListSpec,
    /// The command's own surface.
    pub surface: Option<&'static [SpecSurface]>,
}

impl CommandSemantics for CaseSemantics {
    fn identity(&self) -> &'static str {
        "case-list:case"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Unauthored,
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        case_list_plan(&self.case_list, &[], self.surface, input)
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

impl CaseSemantics {
    /// The selection fact: every clause word exact, the clause-list word
    /// split under the target's list rules, and the core's clause per
    /// member. A word after the subject that the plan read as a pattern
    /// but whose value is the separator (`in`) is one the command skips,
    /// so the plan is not the command's and the fact declines.
    fn selection(
        &self,
        input: &dyn AnalysisInputs,
        budget: &mut Budget,
    ) -> Result<SelectionFact, DeclineReason> {
        let PlanAnswer::CaseList { subject, arms, .. } = self.structure(input) else {
            return Err(DeclineReason::Unsupported);
        };
        let members = subject_members(input, subject)?;
        let first = input.invocation().argument_offset;
        let next = OperandId(subject.0 + 1);
        if let (Some(separator), CaseArms::Words(pairs)) =
            (self.case_list.optional_subject_separator, &arms)
            && pairs.first().is_some_and(|(pattern, _)| *pattern == next)
            && exact(input, next)?.as_utf8() == Some(separator)
        {
            return Err(DeclineReason::Unsupported);
        }
        let mut ops = ConstOps::admit(input.context(), budget, CASE_NEEDS)?;
        let argv = (first..input.invocation().operands.len())
            .map(|index| {
                if index == subject.0 {
                    Ok(ConstValue::text(""))
                } else {
                    exact(input, OperandId(index))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let clauses = clause_words(&mut ops, &arms, (&argv, first), None)?;
        for pattern in clauses.iter().step_by(2) {
            ops.admissible_text(pattern)?;
        }
        let mut fact = SelectionFact {
            selected: Vec::with_capacity(members.len()),
            bodies: Vec::with_capacity(members.len()),
            writes: Vec::with_capacity(members.len()),
        };
        // One unit per pattern word each member may try, beside what the
        // core's pattern-list splits charge.
        let patterns = WorkUnits::try_from(clauses.len().div_ceil(2)).unwrap_or(WorkUnits::MAX);
        for member in &members {
            let member = ConstValue::from_exact(member);
            ops.admissible_text(&member)?;
            ops.charge(patterns)?;
            let chosen = tcl_cmd_core::case::select(&mut ops, &member, &clauses)
                .map_err(|error| ops.decline(&error))?;
            fact.selected.push(chosen);
            fact.bodies.push(chosen);
            fact.writes.push(Vec::new());
        }
        if let Some(fault) = ops.fault() {
            return Err(fault);
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

/// The clause words as the command reads them: the clause-list word split
/// under the target's list rules, or the pattern and body words
/// themselves, an arm the plan read as spelled with the fall-through body
/// carrying that spelling. An empty list and a pattern with no body are
/// the command's own errors.
fn clause_words(
    ops: &mut ConstOps<'_>,
    arms: &CaseArms,
    (argv, first): (&[ConstValue], usize),
    fallthrough: Option<&str>,
) -> Result<Vec<ConstValue>, DeclineReason> {
    Ok(match arms {
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
                    || ConstValue::text(fallthrough.unwrap_or_default()),
                    |body| argv[body.0 - first].clone(),
                );
                [argv[pattern.0 - first].clone(), body]
            })
            .collect(),
    })
}

/// The patterns, and per arm whether its body is the descriptor's
/// fall-through spelling, as the command reads them before it matches. A
/// final fall-through body is the command's own error, raised before any
/// pattern is tried.
fn clauses(
    ops: &mut ConstOps<'_>,
    arms: &CaseArms,
    argv: (&[ConstValue], usize),
    fallthrough: Option<&str>,
) -> Result<(Vec<ConstValue>, Vec<bool>), DeclineReason> {
    let words = clause_words(ops, arms, argv, fallthrough)?;
    let (patterns, bodies): (Vec<ConstValue>, Vec<bool>) = words
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let falls = fallthrough.is_some() && pair[1].as_utf8() == fallthrough;
            (pair[0].clone(), falls)
        })
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

    /// An element of the case list collapses its backslashes under the target
    /// release's escape grammar: before 8.6 a `\x` takes every hex digit that
    /// follows, so `a\x41b` is not `aAb` and the default arm is the one run.
    #[test]
    fn an_element_of_the_case_list_is_collapsed_under_the_targets_escapes() {
        let selected = |dialect: &str| {
            let profile = crate::model::ingress::resolve_environment(dialect).analyser_profile();
            let inputs = LiteralInputs::new(
                "switch",
                None,
                &["aAb", r#""a\x41b" {puts hit} default {puts miss}"#],
                Some(profile),
            );
            let TransferAnswer::Selection(fact) =
                SWITCH.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation())
            else {
                panic!("a selection");
            };
            fact.selected
        };
        assert_eq!(selected("tcl8.4"), [Some(1)]);
        assert_eq!(selected("tcl8.5"), [Some(1)]);
        assert_eq!(selected("tcl8.6"), [Some(0)]);
        assert_eq!(selected("tcl9.0"), [Some(0)]);
    }
}
