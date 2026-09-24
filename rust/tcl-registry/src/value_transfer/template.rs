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

//! The template-word plan (`docs/design/compiler/value-transfers.md` § *The
//! template-word plan*): what a substituting command runs over its final
//! argument.
//!
//! [`TemplateSemantics`] answers `PlanAnswer::TemplateWord` for a command
//! whose switches select the substitution kinds (`subst`). The kinds are the
//! switches' option effects read over the values the analysis proves for
//! them, so a computed switch with a proven value reads as its literal
//! spelling does; the option grammar stays the command's own table, read per
//! spelling and per release. The template's structure is its word structure
//! decomposed under those kinds: the `[…]` regions that run in the caller's
//! frame, the variable reads outside them, and the backslash escapes that
//! materialise.

use tcl_dialect::model::{Family, SpecSurface, SurfaceQuery};
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_lexer::word_parts::{SpannedPart, SubstFlags, VarRef, WordPart as Part};
use tcl_lexer::{LexerConfig, Span};

use crate::abbrev::PrefixMatching;
use crate::frame_effect::FrameLevel;
use crate::hover::OptionSpec;
use crate::invocation_words::InvocationArguments;
use crate::option_effect::{OptionEffectFamily, option_effects};
use crate::substitution::SubstitutionKinds;

use super::CommandSemantics;
use super::answers::{PlanAnswer, ScriptRegion, TemplateWordPlan, VariableRead};
use super::decline::{Axis, DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, BodyRegion, FactDomain, FactView, OperandId, WordPart};
use super::route::EvalRoute;

/// The most spellings the switches' proven values combine into before the
/// plan reads the call as unproven, which runs every kind.
const MAX_COMBINATIONS: usize = 64;

/// A substituting command's template plan over its own switch table: the
/// final argument is the template and every earlier word a switch. The
/// table is the one the command's spec declares, so the plan reads the
/// same options, families and trailing reservation the registry's
/// `option_effects` answers from.
#[derive(Debug)]
pub struct TemplateSemantics {
    identity: &'static str,
    options: &'static [OptionSpec],
    families: &'static [OptionEffectFamily],
    reserved_trailing_words: usize,
}

impl TemplateSemantics {
    /// The plan over `options`, `families` and the command's
    /// `reserved_trailing_words`, under the inventory spelling `identity`.
    #[must_use]
    pub const fn new(
        identity: &'static str,
        options: &'static [OptionSpec],
        families: &'static [OptionEffectFamily],
        reserved_trailing_words: usize,
    ) -> Self {
        Self {
            identity,
            options,
            families,
            reserved_trailing_words,
        }
    }

    /// The kinds the switches run, over their proven values: an exact value
    /// is its spelling, a finite set one spelling per member, and a switch
    /// the analysis does not prove runs every kind. Each combination of
    /// spellings is read at every release the profile names; a combination
    /// that raises runs nothing, the others join, a kind on in any being on.
    /// A combination the releases read differently declines
    /// `ReleaseAmbiguous`, and a call every combination of which raises is
    /// the command's error (`WrongRepresentation`).
    fn switch_kinds(
        &self,
        input: &dyn AnalysisInputs,
        switches: std::ops::Range<usize>,
        template: &str,
    ) -> Result<SubstitutionKinds, DeclineReason> {
        let mut spellings: Vec<Vec<String>> = Vec::with_capacity(switches.len());
        for index in switches {
            let members = match input.operand(OperandId(index), FactDomain::ExactValue) {
                FactView::Exact(value, _) => vec![value],
                FactView::Finite(values, _) => values,
                FactView::Pending | FactView::Domain(_) | FactView::Top(_) => {
                    return Ok(SubstitutionKinds::ALL);
                }
            };
            let Some(members) = members
                .into_iter()
                .map(|value| String::from_utf8(value.bytes).ok())
                .collect::<Option<Vec<_>>>()
            else {
                return Ok(SubstitutionKinds::ALL);
            };
            spellings.push(members);
        }
        let Some(combinations) = combinations(&spellings) else {
            return Ok(SubstitutionKinds::ALL);
        };
        let releases = releases_of(input.context().profile);
        let mut joined: Option<SubstitutionKinds> = None;
        for combination in &combinations {
            let switches: Vec<&str> = combination.iter().map(String::as_str).collect();
            let mut answers = releases
                .iter()
                .map(|&release| self.kinds_at(&switches, template, release));
            let first = answers.next().flatten();
            if answers.any(|answer| answer != first) {
                return Err(DeclineReason::ReleaseAmbiguous(Axis::Availability(
                    self.gate_of(&switches),
                )));
            }
            if let Some(kinds) = first {
                joined = Some(joined.map_or(kinds, |held| join(held, kinds)));
            }
        }
        joined.ok_or(DeclineReason::WrongRepresentation)
    }

    /// The kinds one spelling of the switches runs at `release`, or `None`
    /// when the call raises there: a spelling the release lacks or that names
    /// several switches, the two families together, or a word the switch run
    /// stops at before the template.
    fn kinds_at(
        &self,
        switches: &[&str],
        template: &str,
        release: TclVersion,
    ) -> Option<SubstitutionKinds> {
        let words: Vec<&str> = switches
            .iter()
            .copied()
            .chain(std::iter::once(template))
            .collect();
        let effects = option_effects(
            self.options,
            self.families,
            InvocationArguments::literals(&words),
            self.reserved_trailing_words,
            Some(SurfaceQuery::core(Family::Tcl, release.version_string())),
        );
        (effects.complete && effects.option_end + self.reserved_trailing_words >= words.len())
            .then(|| effects.substitution_kinds())
    }

    /// The release gate a combination the releases read differently turns
    /// on: the first of its switches whose option a release gates, else the
    /// first gated family's.
    fn gate_of(&self, switches: &[&str]) -> SpecSurface {
        let options: Vec<&OptionSpec> = self.options.iter().collect();
        switches
            .iter()
            .find_map(|spelling| {
                crate::patterns::resolve_available_option_prefix_with(
                    &options,
                    spelling,
                    PrefixMatching::Enabled,
                )
                .and_then(|option| option.surface)
                .and_then(|rows| rows.first().copied())
            })
            .or_else(|| {
                self.families
                    .iter()
                    .find_map(|family| family.surface.and_then(|rows| rows.first().copied()))
            })
            .unwrap_or_else(|| SpecSurface::core(Family::Tcl))
    }
}

impl CommandSemantics for TemplateSemantics {
    fn identity(&self) -> &'static str {
        self.identity
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Unauthored,
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let view = input.invocation();
        let first = view.argument_offset;
        // The template is the final argument; a call without one is the
        // command's `wrong # args`.
        let Some(template) = view
            .operands
            .len()
            .checked_sub(1)
            .filter(|&last| last >= first)
        else {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        let operand = OperandId(template);
        let spelling = view.operand(operand).map_or("", |operand| operand.text);
        // A template `subst` rejects raises after substituting what came
        // before it: the command's error.
        match self.switch_kinds(input, first..template, spelling) {
            Ok(kinds) => template_plan(input, operand, kinds).map_or(
                PlanAnswer::Declined(DeclineReason::WrongRepresentation),
                PlanAnswer::TemplateWord,
            ),
            Err(reason) => PlanAnswer::Declined(reason),
        }
    }
}

/// Every combination of one spelling per switch, or `None` past
/// [`MAX_COMBINATIONS`].
fn combinations(spellings: &[Vec<String>]) -> Option<Vec<Vec<String>>> {
    let mut out: Vec<Vec<String>> = vec![Vec::new()];
    for members in spellings {
        if out.len().checked_mul(members.len())? > MAX_COMBINATIONS {
            return None;
        }
        out = out
            .into_iter()
            .flat_map(|prefix| {
                members.iter().map(move |member| {
                    let mut next = prefix.clone();
                    next.push(member.clone());
                    next
                })
            })
            .collect();
    }
    Some(out)
}

/// The releases `profile` evaluates under: the one it declares, or every
/// modelled release when it declares none.
fn releases_of(profile: Option<&'static DialectProfile>) -> Vec<TclVersion> {
    profile
        .and_then(DialectProfile::runtime_version)
        .map_or_else(|| TclVersion::ALL.to_vec(), |release| vec![release])
}

/// A kind on in either answer is on.
const fn join(a: SubstitutionKinds, b: SubstitutionKinds) -> SubstitutionKinds {
    SubstitutionKinds {
        backslashes: a.backslashes || b.backslashes,
        commands: a.commands || b.commands,
        variables: a.variables || b.variables,
    }
}

/// The template's structure under `kinds`. Its text is readable only when
/// the source spells all of it — a braced word, or one the parser leaves as
/// literal text — and a word the parser substitutes reaches the command
/// already computed, which `dynamic` says. Spans are offsets in the word as
/// `word_structure` reports them (a braced word's content from 1), and a
/// script region's `base_offset` is its script's offset there. `None` when
/// the template holds a construct `subst` rejects.
fn template_plan(
    input: &dyn AnalysisInputs,
    operand: OperandId,
    kinds: SubstitutionKinds,
) -> Option<TemplateWordPlan> {
    let mut plan = TemplateWordPlan {
        operand,
        kinds,
        braced: false,
        dynamic: true,
        script_regions: Vec::new(),
        reads: Vec::new(),
        escapes: Vec::new(),
    };
    let Ok(structure) = input.word_structure(operand) else {
        return Some(plan);
    };
    plan.braced = structure.braced;
    let mut text = String::new();
    let mut base: Option<u32> = None;
    for part in &structure.parts {
        let WordPart::Literal { span, text: run } = part else {
            return Some(plan);
        };
        base.get_or_insert(span.start());
        text.push_str(run);
    }
    plan.dynamic = false;
    let config = LexerConfig::from_grammar(input.context().grammar);
    let flags = SubstFlags {
        vars: kinds.variables,
        cmds: kinds.commands,
        backslashes: kinds.backslashes,
        ..SubstFlags::default()
    };
    record(&text, base.unwrap_or(0), 0, flags, config, &mut plan).then_some(plan)
}

/// Record the substitutions `text` performs under `flags` into `plan`, each
/// span `base + offset` into the word; `false` when a construct `subst`
/// rejects stops it there. An array index substitutes every kind whatever `flags` say —
/// `Tcl_ParseVarName` parses it with `TCL_SUBST_ALL`, so `subst -nocommands
/// {$a([b])}` runs `b` — and its reads, scripts and escapes are recorded
/// beside the read that holds it.
fn record(
    text: &str,
    base: u32,
    offset: usize,
    flags: SubstFlags,
    config: LexerConfig,
    plan: &mut TemplateWordPlan,
) -> bool {
    for SpannedPart { part, start, end } in
        tcl_lexer::word_parts::decompose_spanned(text.as_bytes(), flags, config)
    {
        let span = offset_span(base, offset + start, offset + end);
        match part {
            Part::Text(_) if flags.backslashes => {
                let run = text.get(start..end).unwrap_or_default();
                let mut at = 0;
                while let Some(found) = run.get(at..).and_then(|rest| rest.find('\\')) {
                    let from = at + found;
                    let to =
                        tcl_lexer::backslash_escape_end_in(run, from, config.escapes).max(from + 1);
                    plan.escapes.push(offset_span(
                        base,
                        offset + start + from,
                        offset + start + to,
                    ));
                    at = to;
                }
            }
            Part::Text(_) => {}
            Part::Variable(VarRef { name, index }) => {
                // The index as the source spells it, from past the name's `(`
                // to before the reference's closing `)`; it substitutes
                // before the element is read.
                let from = start + 2 + name.len();
                let key = index
                    .as_ref()
                    .and_then(|_| text.get(from..end.saturating_sub(1)));
                if let Some(key) = key
                    && !record(
                        key,
                        base,
                        offset + from,
                        SubstFlags::default(),
                        config,
                        plan,
                    )
                {
                    return false;
                }
                plan.reads.push(VariableRead {
                    span,
                    name: String::from_utf8_lossy(name).into_owned(),
                    element: key.map(|key| literal_key(key, config)),
                });
            }
            Part::Command(script) => plan.script_regions.push(ScriptRegion {
                span,
                script: BodyRegion {
                    script: String::from_utf8_lossy(script).into_owned(),
                    base_offset: span.start() as usize + 1,
                    frame: FrameLevel::Relative(0),
                },
            }),
            Part::ParseError(_) => return false,
        }
    }
    true
}

/// An array index's key: its value when the index substitutes nothing but
/// escapes, else the index as the source spells it, which names no one key.
fn literal_key(index: &str, config: LexerConfig) -> String {
    let parts =
        tcl_lexer::word_parts::decompose_spanned(index.as_bytes(), SubstFlags::default(), config);
    let mut key = String::new();
    for SpannedPart { part, .. } in &parts {
        let Part::Text(text) = part else {
            return index.to_owned();
        };
        key.push_str(&String::from_utf8_lossy(text));
    }
    key
}

/// The span `start..end` of the template text, `base` into the word.
fn offset_span(base: u32, start: usize, end: usize) -> Span {
    let at = |offset: usize| base.saturating_add(u32::try_from(offset).unwrap_or(u32::MAX));
    Span::new(at(start), at(end))
}
