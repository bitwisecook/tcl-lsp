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

//! The regexp owner's direct routes: `regexp` and `regsub` over the shared
//! command plumbing's analysis path (`tcl_cmd_core::regex::regexp_analysis`,
//! `regsub_analysis`) and the Tcl ARE engine
//! (`docs/design/compiler/value-evaluation.md` § *The typed precision
//! result*).
//!
//! A match writes each match variable; a completed no-match preserves them —
//! the only answer that proves a negative; a search that established
//! neither declines the whole answer, so an exhausted search is never a
//! no-match. The engine's work is charged to the evaluation's budget: the
//! compile as the pattern's length squared, each search as the fuel it
//! spent, every search of one call drawing on one allowance.

use std::cell::Cell;

use tcl_cmd_core::regex::{
    AnalysisMatch, MatchLimits, PrecisionDecline, RegexEngine, RegexFailure, RegexpResult,
    regexp_analysis, regsub_analysis,
};
use tcl_dialect::TclVersion;
use tcl_regex::cmd_core::AreEngine;

use crate::arg_role::ArgRole;
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    CompletionOutcome, DependencyEvidence, EvalAnswer, ExactValueOrUnavailable, InvocationOutcome,
    RepresentationEvidence, RouteIdentity, StoreOutcome, TypeFacts,
};
use super::builtins::exact_operands;
use super::const_ops::{ConstOps, ConstValue, Needs, TargetSemantics};
use super::context::Budget;
use super::decline::{BudgetLimit, DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, OperandId, TargetId};
use super::route::{EvalRoute, NativeEvalId};

/// This module's revision of the two routes; the engine's own revision is
/// folded in beside it ([`route_revision`]).
const ROUTE_REVISION: u64 = 1;

/// The axes the two routes read: the ARE feature set and its limits, the
/// character indices `-indices` and `-start` count in, the source decoding
/// of a non-ASCII operand, and the list rendering of an `-inline`,
/// `-indices` or `-about` answer.
const NEEDS: Needs = Needs::REGEXP_FEATURES
    .union(Needs::CHAR_INDEXING)
    .union(Needs::SOURCE_ENCODING)
    .union(Needs::LIST_RENDERING);

/// The routes' revision: this module's in the high half and the engine's in
/// the low, so a change to either invalidates an answer computed under the
/// other.
fn route_revision() -> u64 {
    (ROUTE_REVISION << 32) | u64::from(<AreEngine as RegexEngine>::IDENTITY.revision)
}

/// Every operand as text the target reads alike: its exact value, admitted
/// by [`ConstOps::admissible_text`].
fn admissible_words(
    ops: &mut ConstOps<'_>,
    words: &[ConstValue],
) -> Result<Vec<String>, DeclineReason> {
    words
        .iter()
        .map(|word| ops.admissible_text(word).map(|text| text.to_string()))
        .collect()
}

/// Where `args`' option run ends and the `-start` values it holds, scanned
/// as both commands scan it: `--` (consumed) or the first word without a
/// leading `-` ends the run, and `-start` takes the word after it. A word
/// the command rejects as an option is its error to raise; the scan reads
/// past it.
fn option_run(args: &[String]) -> (usize, Vec<&str>) {
    let mut starts = Vec::new();
    let mut end = 0;
    while let Some(word) = args.get(end) {
        if !word.starts_with('-') {
            break;
        }
        end += 1;
        if word == "--" {
            break;
        }
        if word == "-start" {
            match args.get(end) {
                Some(value) => {
                    starts.push(value.as_str());
                    end += 1;
                }
                None => break,
            }
        }
    }
    (end, starts)
}

/// Whether every release reads `value` as the same `-start` index: a
/// decimal integer, `-` its only sign, with no leading zero and no
/// whitespace. 8.4 reads `-start` as an integer (`-start 010` is 8 there,
/// `-start end` an error), 8.5 and 8.6 as an index with octal numerals
/// (`010` is 8), 9.0 on as a decimal index (`010` is 10) — measured on
/// tclsh 8.4.20 to 9.1b0.
fn start_is_release_stable(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && digits.len() <= 18
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
}

/// The decline for an option run whose `-start` index the releases may read
/// differently.
fn start_decline(args: &[String]) -> Option<DeclineReason> {
    option_run(args)
        .1
        .into_iter()
        .any(|value| !start_is_release_stable(value))
        .then_some(DeclineReason::Unsupported)
}

/// Run one analysis-path command under `ops`' budget: the compile charge and
/// every search's fuel drawn from one allowance — the work the budget has
/// left, each search's fuel at most the engine's own per-search budget —
/// and the work spent charged once the run returns.
///
/// # Errors
///
/// The budget's limit when the work spent exceeds what it had left.
fn metered<R>(
    ops: &mut ConstOps<'_>,
    run: impl FnOnce(&mut ConstOps<'_>, &mut AnalysisMatch<'_>) -> R,
) -> Result<R, DeclineReason> {
    let remaining = ops.remaining_work();
    let target: TargetSemantics = *ops.target();
    let spent = Cell::new(0_u64);
    let mut charge = |units: u64| {
        let total = spent.get().saturating_add(units);
        spent.set(total);
        total <= remaining
    };
    let answer = {
        let mut analysis = AnalysisMatch {
            target: (target.character_model, target.byte_strings),
            charge: &mut charge,
            limits: MatchLimits {
                fuel: Some(remaining.min(tcl_regex::MATCH_FUEL)),
                cancel: None,
                spent: Some(&spent),
            },
        };
        run(ops, &mut analysis)
    };
    ops.charge(spent.get())?;
    Ok(answer)
}

/// The decline a run that did not complete normally answers with
/// (`value-evaluation.md` § *The typed precision result*): the run's own
/// fault when one was recorded; an error the command raises — a malformed
/// pattern among them — is never a value, and before the completion slice
/// it is a decline; an exhausted, too-deep or approximate search is
/// `Approximate`; a form the core does not implement is `Unsupported`; a
/// stopped one is the cancelled request's.
fn failure_reason(ops: &ConstOps<'_>, failure: &RegexFailure) -> DeclineReason {
    if let Some(fault) = ops.fault() {
        return fault;
    }
    match failure {
        RegexFailure::Error(_) | RegexFailure::Declined(PrecisionDecline::PatternError(_)) => {
            DeclineReason::WrongRepresentation
        }
        RegexFailure::Declined(
            PrecisionDecline::FuelExhausted { .. }
            | PrecisionDecline::DepthExhausted { .. }
            | PrecisionDecline::ApproximateCapture { .. },
        ) => DeclineReason::Approximate,
        RegexFailure::Declined(PrecisionDecline::FormUnsupported { .. }) => {
            DeclineReason::Unsupported
        }
        RegexFailure::Declined(PrecisionDecline::Cancelled) => {
            DeclineReason::Budget(BudgetLimit::Cancelled)
        }
    }
}

/// What one evaluation publishes, before its values are taken: the result,
/// then each store in execution order, a write carrying its value.
struct Publication {
    result: ConstValue,
    result_type: TclType,
    stores: Vec<(TargetId, Option<ConstValue>)>,
}

impl Publication {
    /// A result and a `Preserve` for every declared target: the call wrote
    /// nothing.
    fn preserving(result: ConstValue, result_type: TclType, targets: &[OperandId]) -> Self {
        Self {
            result,
            result_type,
            stores: targets.iter().map(|id| (TargetId(*id), None)).collect(),
        }
    }

    /// Charge the published bytes as work — one unit per capture or output
    /// byte — take every value, and build the outcome.
    fn publish(self, mut ops: ConstOps<'_>, id: NativeEvalId) -> EvalAnswer {
        let target = *ops.target();
        let bytes = self.result.bytes.len()
            + self
                .stores
                .iter()
                .filter_map(|(_, value)| value.as_ref())
                .map(|value| value.bytes.len())
                .sum::<usize>();
        if let Err(reason) = ops.charge(u64::try_from(bytes).unwrap_or(u64::MAX)) {
            return EvalAnswer::Declined(reason);
        }
        let written: Vec<ConstValue> = self
            .stores
            .iter()
            .filter_map(|(_, value)| value.clone())
            .collect();
        let mut taken = match ops.take_all(
            std::iter::once(self.result)
                .chain(written)
                .collect::<Vec<_>>(),
        ) {
            Ok(taken) => taken.into_iter(),
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let Some(result) = taken.next() else {
            return EvalAnswer::Declined(DeclineReason::MalformedAnswer);
        };
        let mut ordered_stores = Vec::with_capacity(self.stores.len());
        let mut per_target = Vec::new();
        for (target, value) in self.stores {
            if value.is_none() {
                ordered_stores.push(StoreOutcome::Preserve { target });
                continue;
            }
            let Some(value) = taken.next() else {
                return EvalAnswer::Declined(DeclineReason::MalformedAnswer);
            };
            if let RepresentationEvidence::Constructed(built) = value.representation {
                per_target.push((target, built));
            }
            ordered_stores.push(StoreOutcome::Write { target, value });
        }
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result: ExactValueOrUnavailable::Exact(result),
            ordered_stores,
            types: TypeFacts {
                result: Some(self.result_type),
                per_target,
                shapes: Vec::new(),
            },
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: EvalRoute::Direct { id },
                    implementation: id.as_str(),
                    revision: route_revision(),
                }),
                characters: target.character_model,
                release: target.release,
                ..DependencyEvidence::default()
            },
        }))
    }
}

/// The exact words of every operand and the operands the resolver gives the
/// `VarWrite` role, with a value model admitted for the routes' axes — or
/// the answer that stands in for an input that is not exact.
fn open<'b>(
    input: &dyn AnalysisInputs,
    budget: &'b mut Budget,
) -> Result<(ConstOps<'b>, Vec<String>, Vec<OperandId>), EvalAnswer> {
    let words = exact_operands(input, 0..input.invocation().operands.len())?;
    let targets: Vec<OperandId> = input
        .invocation()
        .operands_with_role(ArgRole::VarWrite)
        .collect();
    let mut ops = ConstOps::admit(input.context(), budget, NEEDS).map_err(EvalAnswer::Declined)?;
    let texts = admissible_words(&mut ops, &words).map_err(EvalAnswer::Declined)?;
    if let Some(reason) = start_decline(&texts) {
        return Err(EvalAnswer::Declined(reason));
    }
    Ok((ops, texts, targets))
}

/// `regexp ?switches? exp string ?matchVar …?` on the direct route: on a
/// match the count as the result and one `Write` per match variable (an
/// unmatched subgroup writes the empty string, or `-1 -1` with
/// `-indices`); on a completed no-match 0 and one `Preserve` per match
/// variable, because the command leaves them untouched; with `-inline` the
/// list as the result and nothing written; with `-all` the count of
/// matches, the variables holding the last; with `-about` the
/// subexpression count and the engine's `re_info` names. Any
/// `PrecisionDecline` declines the whole answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegexpSemantics;

/// `regexp`.
pub static REGEXP: RegexpSemantics = RegexpSemantics;

impl RegexpSemantics {
    /// The axes the route reads.
    pub const NEEDS: Needs = NEEDS;

    fn evaluate_regexp(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let (mut ops, texts, targets) = match open(input, budget) {
            Ok(opened) => opened,
            Err(answer) => return answer,
        };
        let args: Vec<&[u8]> = texts.iter().map(String::as_bytes).collect();
        let run = metered(&mut ops, |ops, analysis| {
            regexp_analysis::<ConstOps<'_>, AreEngine>(ops, &args, analysis)
        });
        let publication = match run {
            Err(reason) => return EvalAnswer::Declined(reason),
            Ok(Err(failure)) => return EvalAnswer::Declined(failure_reason(&ops, &failure)),
            Ok(Ok(RegexpResult::Inline(list))) => {
                Publication::preserving(list, TclType::List, &targets)
            }
            Ok(Ok(RegexpResult::Count {
                assign: None,
                count,
            })) => Publication::preserving(ConstValue::int(count), TclType::Int, &targets),
            Ok(Ok(RegexpResult::Count {
                assign: Some(pairs),
                count,
            })) => {
                // The core names the variables it writes; each must be the
                // operand the resolver gave the role, in order, or the
                // stores would land on the wrong places.
                if pairs.len() != targets.len()
                    || pairs
                        .iter()
                        .zip(&targets)
                        .any(|((name, _), id)| name.as_slice() != texts[id.0].as_bytes())
                {
                    return EvalAnswer::Declined(DeclineReason::Unsupported);
                }
                Publication {
                    result: ConstValue::int(count),
                    result_type: TclType::Int,
                    stores: targets
                        .iter()
                        .zip(pairs)
                        .map(|(id, (_, value))| (TargetId(*id), Some(value)))
                        .collect(),
                }
            }
        };
        publication.publish(ops, NativeEvalId::RegexpMatch)
    }
}

impl CommandSemantics for RegexpSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::RegexpMatch.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::RegexpMatch,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        Self::evaluate_regexp(input, budget)
    }
}

/// `regsub ?switches? exp string subSpec ?varName?` on the direct route: the
/// substituted text as the result, or — with a result variable — the count
/// as the result and the text written to the variable, which the command
/// writes whether or not anything matched. The callback form (`-command`,
/// from 9.0) runs a script on each match and has no route of its own:
/// `NoRoute(Callback)`. Any `PrecisionDecline` declines the whole answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegsubSemantics;

/// `regsub`.
pub static REGSUB: RegsubSemantics = RegsubSemantics;

impl RegsubSemantics {
    /// The axes the route reads.
    pub const NEEDS: Needs = NEEDS;

    fn evaluate_regsub(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let (mut ops, texts, targets) = match open(input, budget) {
            Ok(opened) => opened,
            Err(answer) => return answer,
        };
        if crate::commands::tcl::regsub_names_a_callback(&texts) {
            return EvalAnswer::Declined(DeclineReason::NoRoute(NoRouteReason::Callback));
        }
        // The output is bounded before it is built: the subject once, and
        // the substitution spec once per match — at most one more match
        // than the subject has characters — with each of its references
        // expanded to at most the subject's bytes over all the matches.
        let positional = &texts[option_run(&texts).0..];
        let length = |at: usize| {
            positional
                .get(at)
                .map_or(0, |word| u64::try_from(word.len()).unwrap_or(u64::MAX))
        };
        let bound = length(1)
            .saturating_add(1)
            .saturating_mul(length(2).saturating_mul(2).saturating_add(1));
        if let Err(reason) = ops.charge_bytes(bound) {
            return EvalAnswer::Declined(reason);
        }
        // With no release named, the 9.0 table reads every option the
        // earlier tables do; the one it adds is the callback form above.
        let version = ops.target().release.unwrap_or(TclVersion::V9_0);
        let args: Vec<&[u8]> = texts.iter().map(String::as_bytes).collect();
        let run = metered(&mut ops, |_, analysis| {
            regsub_analysis::<AreEngine>(&args, version, analysis)
        });
        let substituted = match run {
            Err(reason) => return EvalAnswer::Declined(reason),
            Ok(Err(failure)) => return EvalAnswer::Declined(failure_reason(&ops, &failure)),
            Ok(Ok(substituted)) => substituted,
        };
        let Ok(text) = String::from_utf8(substituted.text) else {
            return EvalAnswer::Declined(DeclineReason::NotText);
        };
        let publication = match substituted.var {
            None => Publication::preserving(ConstValue::text(&text), TclType::String, &targets),
            Some(name) => {
                let [id] = targets.as_slice() else {
                    return EvalAnswer::Declined(DeclineReason::Unsupported);
                };
                if name.as_slice() != texts[id.0].as_bytes() {
                    return EvalAnswer::Declined(DeclineReason::Unsupported);
                }
                Publication {
                    result: ConstValue::int(substituted.count),
                    result_type: TclType::Int,
                    stores: vec![(TargetId(*id), Some(ConstValue::text(&text)))],
                }
            }
        };
        publication.publish(ops, NativeEvalId::RegsubSubstitute)
    }
}

impl CommandSemantics for RegsubSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::RegsubSubstitute.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::RegsubSubstitute,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        Self::evaluate_regsub(input, budget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_plain_decimal_start_reads_alike_everywhere() {
        for stable in ["0", "7", "-3", "-0", "120"] {
            assert!(start_is_release_stable(stable), "{stable}");
        }
        for unstable in [
            "010", "0x2", "end", "end-1", "1_0", " 1", "+1", "", "-", "1.0",
        ] {
            assert!(!start_is_release_stable(unstable), "{unstable}");
        }
        let args = |words: &[&str]| words.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert_eq!(
            option_run(&args(&["-all", "-start", "010", "a", "b"])),
            (3, vec!["010"])
        );
        assert_eq!(
            option_run(&args(&["--", "-start", "010", "a"])),
            (1, Vec::new())
        );
        assert_eq!(option_run(&args(&["-start"])), (1, Vec::new()));
        assert_eq!(option_run(&args(&["a", "-start"])), (0, Vec::new()));
    }
}
