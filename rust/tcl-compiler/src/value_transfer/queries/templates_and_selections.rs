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

//! Template-word plans and case-list selections a caller asks for over words whose values it already holds.

use super::super::argument_words::{OperandSource, plan_clause_count, switch_arguments, word_of};
use super::super::lattice_inputs::word_parts;
use crate::ir::Statement;
use tcl_lexer::{LexerConfig, Span};
use tcl_registry::value_transfer::{
    AnalysisContext, AnalysisTier, Budget, FactDomain, OperandId, PlanAnswer, SelectionFact,
    TransferAnswer, WordPart, WordStructure,
};
use tcl_registry::{CommandRegistry, InvocationWord, InvocationWords, ResolvedInvocation};
use tcl_syntax::word_rules::WordValueRules;

/// The already selected descriptor's template structure under complete source
/// metadata and its original lexer policy. Literal captured operands keep their
/// effective ordinals; unknown operands supply no invented text or child span.
/// The Structure tier grants neither a current value nor successful evaluation.
pub(crate) fn source_template_plan(
    schema: &ResolvedInvocation<'_, '_>,
    registry: &CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    config: LexerConfig,
) -> Option<tcl_registry::value_transfer::TemplateWordPlan> {
    use tcl_registry::value_transfer::LiteralInputs;
    if !metadata.matches_registry(registry) {
        return None;
    }
    let input = metadata.source_analysis_input()?;
    let arguments = schema.words.arguments();
    let texts = (0..arguments.len())
        .map(|ordinal| arguments.literal_at(ordinal).unwrap_or_default())
        .collect::<Vec<_>>();
    let mut selected = AnalysisContext::detached(Some(input.unit_profile()));
    selected.grammar = config.grammar_over(input.unit_profile().grammar);
    selected.script_encoding = input.script_encoding();
    selected.registry_generation = registry.generation();
    selected.overlay_generation = registry.overlay_generation();
    selected.tier = AnalysisTier::Structure;
    let mut literals = LiteralInputs::new(
        schema.canonical_command,
        None,
        &texts,
        Some(input.unit_profile()),
    )
    .with_context(selected);
    for ordinal in 0..arguments.len() {
        literals = if arguments.literal_at(ordinal).is_some() {
            literals.with_bare(OperandId(ordinal))
        } else {
            literals.with_unproven(OperandId(ordinal))
        };
    }
    match schema.semantics.value.semantics()?.structure(&literals) {
        PlanAnswer::TemplateWord(plan) => Some(plan),
        _ => None,
    }
}

/// How one word of a call reads in its source, for a template plan asked
/// before the lattice ([`literal_template_plan`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceWord {
    /// A brace-quoted word: its content is its value and its text.
    Braced,
    /// A bare or quoted word the parser leaves as literal text.
    Literal,
    /// A word the parser substitutes: its value is computed.
    Substituted,
}

impl SourceWord {
    /// The word `text` reads as: brace-quoted when `braced`, else computed
    /// when it spells a `$` or `[`, else literal text (as when no text is
    /// given).
    #[must_use]
    pub fn of(text: Option<&str>, braced: bool) -> Self {
        if braced {
            Self::Braced
        } else if text.is_some_and(|text| text.contains('$') || text.contains('[')) {
            Self::Substituted
        } else {
            Self::Literal
        }
    }
}

/// The template-word plan the call `head args…` declares over its source
/// words, each read as `source` says — the plan a folder that runs before
/// the lattice, or the analyser's walk, reads
/// (`docs/design/compiler/value-transfers.md` § *The template-word plan*).
/// A literal word reads as its own spelling and a substituted one is
/// unproven, so a computed switch runs every kind. `None` when the call
/// declares no template plan or the plan declines.
#[must_use]
pub fn literal_template_plan(
    registry: &CommandRegistry,
    head: &str,
    args: &[&str],
    source: impl Fn(usize) -> SourceWord,
) -> Option<tcl_registry::value_transfer::TemplateWordPlan> {
    let words: Vec<InvocationWord<'_>> = args.iter().copied().map(word_of).collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()?;
    let semantics = resolved.semantics.value.semantics()?;
    let config = LexerConfig::for_profile(registry.profile());
    let mut inputs = tcl_registry::value_transfer::LiteralInputs::new(
        resolved.canonical_command,
        None,
        args,
        registry.profile(),
    );
    for (index, text) in args.iter().enumerate() {
        let id = OperandId(index);
        inputs = match source(index) {
            SourceWord::Braced => inputs.with_braced(id),
            SourceWord::Literal => inputs.with_structure(
                id,
                WordStructure {
                    braced: false,
                    quoted: false,
                    parts: vec![WordPart::Literal {
                        span: Span::new(0, u32::try_from(text.len()).unwrap_or(u32::MAX)),
                        text: (*text).to_owned(),
                    }],
                },
            ),
            SourceWord::Substituted => {
                let inputs = inputs.with_unproven(id);
                match word_parts(text, config) {
                    Ok(parts) => inputs.with_structure(
                        id,
                        WordStructure {
                            braced: false,
                            quoted: false,
                            parts,
                        },
                    ),
                    Err(_) => inputs,
                }
            }
        };
    }
    match semantics.structure(&inputs) {
        PlanAnswer::TemplateWord(plan) => Some(plan),
        _ => None,
    }
}

/// How one word of a case-list call was written, for a consumer that reads
/// the call's words rather than the lattice ([`literal_selection`]). The
/// releases part on a fall-through body's delimiters, so a word whose value
/// the caller proves says how it was spelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WordForm {
    /// A bare word with nothing to substitute.
    Bare,
    /// A double-quoted word with nothing to substitute.
    Quoted,
    /// A brace-quoted word.
    Braced,
    /// A word whose value the caller does not prove.
    Unproven,
}

/// The selection a case-list call makes over words whose values the caller
/// proves, with what its indices read against.
#[derive(Debug, Clone)]
pub(crate) struct LiteralSelection {
    /// The selection: one entry, for the one subject value the words state.
    pub fact: SelectionFact,
    /// The plan's subject word, an index into the call's words.
    pub subject: usize,
    /// How many pattern and body pairs the plan read.
    pub clauses: usize,
}

/// The inputs over `words` for the resolved case-list command `command`:
/// every word exact but the unproven ones, each with the structure its form
/// states.
fn literal_selection_inputs<'a>(
    command: &'a str,
    words: &[(&'a str, WordForm)],
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> tcl_registry::value_transfer::LiteralInputs<'a> {
    let texts: Vec<&str> = words.iter().map(|&(text, _)| text).collect();
    let mut inputs =
        tcl_registry::value_transfer::LiteralInputs::new(command, None, &texts, profile);
    for (index, &(_, form)) in words.iter().enumerate() {
        let id = OperandId(index);
        inputs = match form {
            WordForm::Bare => inputs.with_bare(id),
            WordForm::Quoted => inputs.with_quoted(id),
            WordForm::Braced => inputs.with_braced(id),
            WordForm::Unproven => inputs.with_unproven(id),
        };
    }
    inputs
}

/// The selection the case-list call `head words…` makes when the caller
/// proves the words' values (`docs/design/compiler/value-transfers.md`
/// § *`switch`*): the registry's declared `Selection` transfer for the
/// resolved command, the shared core choosing the arm. `subject` gives the
/// plan's subject word the value the caller proves for it when the words
/// spell it otherwise (`$mode`). `None` where the head declares no
/// selection, or the plan or the transfer declines.
pub(crate) fn literal_selection(
    registry: &CommandRegistry,
    head: &str,
    words: &[(&str, WordForm)],
    subject: Option<&str>,
) -> Option<LiteralSelection> {
    let invocation: Vec<InvocationWord<'_>> = words
        .iter()
        .map(|&(text, form)| match form {
            WordForm::Unproven => InvocationWord::Dynamic,
            _ => InvocationWord::Literal(text),
        })
        .collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &invocation),
            registry.own_surface_query(),
        )
        .resolved()?;
    let semantics = resolved.semantics.value.semantics()?;
    let command = resolved.canonical_command;
    let profile = registry.profile();
    let mut inputs = literal_selection_inputs(command, words, profile);
    let mut plan = semantics.structure(&inputs);
    if let (Some(value), PlanAnswer::CaseList { subject: at, .. }) = (subject, &plan) {
        let mut proven = words.to_vec();
        *proven.get_mut(at.0)? = (value, WordForm::Bare);
        inputs = literal_selection_inputs(command, &proven, profile);
        plan = semantics.structure(&inputs);
    }
    let PlanAnswer::CaseList {
        subject: at, arms, ..
    } = plan
    else {
        return None;
    };
    let clauses = plan_clause_count(
        &inputs,
        &arms,
        WordValueRules::from_config(&LexerConfig::for_profile(profile)),
    )?;
    let TransferAnswer::Selection(fact) =
        semantics.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    Some(LiteralSelection {
        fact,
        subject: at.0,
        clauses,
    })
}

/// The form a lowered word's source says it has.
const fn form_of(source: OperandSource) -> WordForm {
    match source {
        OperandSource::Literal => WordForm::Bare,
        OperandSource::QuotedLiteral => WordForm::Quoted,
        OperandSource::BracedLiteral => WordForm::Braced,
        OperandSource::Substituted | OperandSource::Unknown => WordForm::Unproven,
    }
}

/// The selection the lowered case-list statement `stmt` makes when its
/// subject word holds `subject`: the words as the lowering recorded them
/// (`switch_arguments` over `raw_args` and the delimiter flags), through
/// [`literal_selection`] for the command the statement names. Only where the
/// plan reads the statement's own subject and clause count — an inliner
/// renames `subject` and leaves `raw_args` — and where the words'
/// delimiters were recorded; `None` otherwise.
pub(crate) fn statement_selection(
    registry: &CommandRegistry,
    stmt: &Statement,
    subject: &str,
) -> Option<SelectionFact> {
    let Statement::Switch {
        subject: written,
        arms,
        default_body,
        raw_args,
        raw_arg_braced,
        raw_arg_quoted,
        command,
        ..
    } = stmt
    else {
        return None;
    };
    if raw_arg_braced.len() != raw_args.len() || raw_arg_quoted.len() != raw_args.len() {
        return None;
    }
    let config = LexerConfig::for_profile(registry.profile());
    let cooked = switch_arguments(raw_args, (raw_arg_braced, raw_arg_quoted), &config);
    let words: Vec<(&str, WordForm)> = cooked
        .iter()
        .map(|word| (word.text.as_ref(), form_of(word.source)))
        .collect();
    let selection = literal_selection(registry, command, &words, Some(subject))?;
    let clauses = arms.len() + usize::from(default_body.is_some());
    (raw_args.get(selection.subject) == Some(written) && selection.clauses == clauses)
        .then_some(selection.fact)
}
