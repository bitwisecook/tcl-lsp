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

//! Conditional source-plan correspondence for original variable edits.
use crate::analyser::AnalysisResult;
use crate::registry_invocation::source_structure::{OriginalRegistrySource, source_registry_words};
use crate::signature_scan::scope::SignatureSourceNameInput;
use crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol;
use tcl_lexer::{ExecutablePart, SourceImage, Span};
use tcl_registry::resolved_invocation::{SourceCallbackRetention, SourceExecutableValueCapability};

#[derive(Debug)]
struct ConditionalCall {
    words: Vec<(Span, Vec<u8>)>,
    command: String,
    roles: Vec<(usize, tcl_registry::ArgRole)>,
    obligations: Vec<crate::command_binding::SourceCommandTransitionObligation>,
    unknowns: Vec<Vec<(Span, Vec<u8>)>>,
}

/// Exact syntax/conditional-schema inventory. This is not an executable
/// rename certificate: generated-source isomorphism and all existing alias,
/// formal, collision and edit-container guards remain independent.
fn inspect_conditional_variable_source(
    source: &str,
    analysis: &AnalysisResult,
    symbol: &SignatureSourceVariableSymbol,
    proposed_name: &[u8],
) -> Option<Vec<ConditionalCall>> {
    let image = SourceImage::document(source);
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    if !input.has_logical_source_name_context()
        || !analysis.matches_original_source_image(&image, config)
        || !analysis.original_variable_alias_source_is_complete(&image, config)
        || analysis
            .retained_command_realm()?
            .original_logical_variable_formal_name_is_unrepresented(input, symbol)
        || analysis.original_variable_symbols.iter().any(|o| {
            !o.has_source_applicability_obligation()
                || !o.matches_logical_source_view(&image, input)
        })
    {
        return None;
    }
    let selected = analysis
        .original_variable_symbols
        .iter()
        .filter(|o| o.symbol() == symbol)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return None;
    }
    let crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::C {
        namespace,
        simple,
    } = symbol.slot()
    else {
        return None;
    };
    if !namespace.is_root() {
        return None;
    }
    let string_protocol = symbol.policy().string_protocol();
    let old_text = string_protocol.counted_unicode_text(simple.as_bytes())?;
    let proposed_text = string_protocol.counted_unicode_text(proposed_name)?;
    let old_name = old_text.as_bytes();
    let proposed_name = proposed_text.as_bytes();
    if old_name.is_empty() || proposed_name.is_empty() {
        return None;
    }
    let plan = tcl_lexer::native_script_words_in(
        image.clone(),
        Span::new(0, u32::try_from(source.len()).ok()?),
        config,
    )
    .ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let segments =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)?;
    if segments.len() != plan.commands.len() {
        return None;
    }
    let mut calls = Vec::new();
    let mut covered = Vec::<Span>::new();
    let inspection = ConditionalInspection {
        source,
        analysis,
        image: &image,
        input,
        selected: &selected,
        old_name,
        proposed_name,
        plan: &plan,
    };
    for (segment, command) in segments.iter().zip(&plan.commands) {
        calls.push(inspection.inspect_command(segment, command, &mut covered)?);
    }
    covered.sort_by_key(|span| (span.start(), span.end()));
    covered.dedup();
    if analysis.original_variable_symbols.len() != covered.len()
        || analysis
            .original_variable_symbols
            .iter()
            .any(|occurrence| !covered.contains(&occurrence.span()))
    {
        return None;
    }
    Some(calls)
}

struct ConditionalInspection<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    image: &'a SourceImage,
    input: &'a super::ResolvedAnalysisInput,
    selected: &'a [&'a crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence],
    old_name: &'a [u8],
    proposed_name: &'a [u8],
    plan: &'a tcl_lexer::NativeScriptWordsPlan,
}

impl ConditionalInspection<'_> {
    fn inspect_command(
        &self,
        segment: &crate::segmenter::SegmentedCommand,
        command: &tcl_lexer::NativeScriptCommandWords,
        covered: &mut Vec<Span>,
    ) -> Option<ConditionalCall> {
        let Self {
            source,
            analysis,
            image,
            input,
            selected,
            old_name,
            proposed_name,
            plan,
        } = *self;
        let config = input.lexer_config();
        let words = source_registry_words(source, analysis, segment)?;
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            return None;
        };
        if advice.original_words() != command.words.as_slice()
            || !advice.matches_source(image, config)
        {
            return None;
        }
        if advice.obligations().iter().any(|obligation| !matches!(obligation,
            crate::command_binding::SourceCommandTransitionObligation::UnavailableActualLookup
            | crate::command_binding::SourceCommandTransitionObligation::ExplicitLogicalSourceApplicability
            | crate::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation
        )) { return None; }
        let roles = words.roles()?.to_vec();
        if roles.iter().any(|(_, role)| {
            !matches!(
                role,
                tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
            )
        }) {
            return None;
        }
        if words.with_source_schema(input.borrowed_context_registry(), |schema| {
            let facts = schema.facts();
            facts.arity_accepts_frozen_arguments() == Some(true)
                && facts.arg_roles_complete
                && facts.source_callback_retention() == SourceCallbackRetention::Absent
                && schema.authored_source_executable_value_capability()
                    == SourceExecutableValueCapability::Absent
                && facts
                    .state_transitions
                    .declared()
                    .is_none_or(|transitions| transitions.facts().is_empty())
        }) != Some(true)
        {
            return None;
        }
        cover_role_occurrences(&words, &roles, analysis, image, input, covered)?;
        inspect_words(command, &roles, selected, old_name, proposed_name, covered)?;
        // A prior opaque call outside the selected written occurrence set
        // is not covered by this narrow authored-source recipe.
        if advice.uncertain_operations().iter().any(|operation| {
            !plan
                .commands
                .iter()
                .any(|candidate| candidate.words.as_slice() == operation.as_ref())
                || !operation.iter().any(|word| {
                    selected.iter().any(|occurrence| {
                        word.span().start() <= occurrence.span().start()
                            && occurrence.span().end() <= word.span().end()
                    })
                })
        }) {
            return None;
        }
        let vector = |words: &[tcl_lexer::NativeWord]| {
            words
                .iter()
                .map(|word| (word.span(), word.bytes().to_vec()))
                .collect()
        };
        Some(ConditionalCall {
            words: vector(&command.words),
            command: words.command().to_owned(),
            roles,
            obligations: advice.obligations().to_vec(),
            unknowns: advice
                .uncertain_operations()
                .iter()
                .map(|op| vector(op))
                .collect(),
        })
    }
}

fn cover_role_occurrences(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    roles: &[(usize, tcl_registry::ArgRole)],
    analysis: &AnalysisResult,
    image: &SourceImage,
    input: &super::ResolvedAnalysisInput,
    covered: &mut Vec<Span>,
) -> Option<()> {
    for &(arg, _) in roles {
        let word = words.original_argument_word(arg)?;
        let mut matching = analysis
            .original_variable_symbols
            .iter()
            .filter(|occurrence| {
                (occurrence.span() == word.span()
                    || word
                        .tokens()
                        .first()
                        .is_some_and(|token| token.span == occurrence.span()))
                    && occurrence.matches_logical_source_view(image, input)
            });
        let occurrence = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        covered.push(occurrence.span());
    }
    Some(())
}

fn inspect_words(
    command: &tcl_lexer::NativeScriptCommandWords,
    roles: &[(usize, tcl_registry::ArgRole)],
    selected: &[&crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence],
    old_name: &[u8],
    proposed_name: &[u8],
    covered: &mut Vec<Span>,
) -> Option<()> {
    for (ordinal, word) in command.words.iter().enumerate() {
        if word.group().expand {
            return None;
        }
        let is_role = ordinal
            .checked_sub(1)
            .is_some_and(|arg| roles.iter().any(|(position, _)| *position == arg));
        if ordinal > 0
            && !is_role
            && let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word)
            && (value
                .iter()
                .any(|byte| matches!(*byte, b'\\' | b'{' | b'}' | b'$' | b'[' | b']'))
                || value.windows(old_name.len()).any(|w| w == old_name)
                || value
                    .windows(proposed_name.len())
                    .any(|w| w == proposed_name))
        {
            return None;
        }
        for part in word.executable_parts().all_parts() {
            match part.part {
                ExecutablePart::Text(_) => {}
                ExecutablePart::Variable { index: None, .. } if ordinal > 0 && !is_role => {
                    let mut matching = selected.iter().filter(|occurrence| matches!(occurrence.original_name_input(), SignatureSourceNameInput::OriginalVariableRoot(root) if root.part_span() == part.span));
                    let occurrence = matching.next()?;
                    if matching.next().is_some() {
                        return None;
                    }
                    covered.push(occurrence.span());
                }
                _ => return None,
            }
        }
    }
    Some(())
}

/// An original name edit; callers must independently authenticate the
/// selected occurrence, source container and target name before constructing it.
pub struct ConditionalNameEdit<'a> {
    /// Original source extent to replace.
    pub span: Span,
    /// Replacement bytes for the extent.
    pub replacement: &'a [u8],
}

fn map_boundary(offset: u32, edits: &[ConditionalNameEdit<'_>]) -> Option<u32> {
    let mut delta = 0_i64;
    for edit in edits {
        if offset < edit.span.start() {
            break;
        }
        if offset == edit.span.start() {
            return u32::try_from(i64::from(offset) + delta).ok();
        }
        if offset < edit.span.end() {
            return None;
        }
        delta += i64::try_from(edit.replacement.len()).ok()?
            - i64::from(edit.span.end() - edit.span.start());
    }
    u32::try_from(i64::from(offset) + delta).ok()
}

fn mapped_word(
    old: &(Span, Vec<u8>),
    new: &(Span, Vec<u8>),
    edits: &[ConditionalNameEdit<'_>],
) -> bool {
    let Some(start) = map_boundary(old.0.start(), edits) else {
        return false;
    };
    let Some(end) = map_boundary(old.0.end(), edits) else {
        return false;
    };
    if new.0 != Span::new(start, end) {
        return false;
    }
    let mut expected = old.1.clone();
    for edit in edits.iter().rev() {
        if edit.span.end() <= old.0.start() || edit.span.start() >= old.0.end() {
            continue;
        }
        if edit.span.start() < old.0.start() || edit.span.end() > old.0.end() {
            return false;
        }
        let start = (edit.span.start() - old.0.start()) as usize;
        let end = (edit.span.end() - old.0.start()) as usize;
        if end > expected.len() {
            return false;
        }
        expected.splice(start..end, edit.replacement.iter().copied());
    }
    expected == new.1
}

fn mapped_words(
    old: &[(Span, Vec<u8>)],
    new: &[(Span, Vec<u8>)],
    edits: &[ConditionalNameEdit<'_>],
) -> bool {
    old.len() == new.len() && old.iter().zip(new).all(|(a, b)| mapped_word(a, b, edits))
}

/// Verifies conditional schema/unknown-operation shape only. A successful
/// comparison is still not a rename, binding, observer or execution proof.
fn conditional_inventories_correspond(
    old: &[ConditionalCall],
    new: &[ConditionalCall],
    edits: &[ConditionalNameEdit<'_>],
) -> bool {
    if edits
        .windows(2)
        .any(|pair| pair[0].span.end() > pair[1].span.start())
    {
        return false;
    }
    old.len() == new.len()
        && old.iter().zip(new).all(|(a, b)| {
            a.command == b.command
                && a.roles == b.roles
                && a.obligations == b.obligations
                && mapped_words(&a.words, &b.words, edits)
                && a.unknowns.len() == b.unknowns.len()
                && a.unknowns
                    .iter()
                    .zip(&b.unknowns)
                    .all(|(left, right)| mapped_words(left, right, edits))
        })
}

/// Correspondence of retained authored plans for two exact Logical images.
/// This is not variable edit, runtime binding, trace, command identity or Native authority.
pub struct ConditionalSourcePlanCorrespondence {
    before: SourceImage,
    after: SourceImage,
    input: crate::analyser::ResolvedAnalysisInput,
    before_symbol: SignatureSourceVariableSymbol,
    after_symbol: SignatureSourceVariableSymbol,
    edits: Vec<(Span, Vec<u8>)>,
}

impl ConditionalSourcePlanCorrespondence {
    /// Recheck both images and the complete Logical input before consuming.
    #[must_use]
    pub fn matches(
        &self,
        before: &str,
        after: &str,
        input: &crate::analyser::ResolvedAnalysisInput,
        before_symbol: &SignatureSourceVariableSymbol,
        after_symbol: &SignatureSourceVariableSymbol,
        edits: &[ConditionalNameEdit<'_>],
    ) -> bool {
        self.before == SourceImage::document(before)
            && self.after == SourceImage::document(after)
            && &self.input == input
            && &self.before_symbol == before_symbol
            && &self.after_symbol == after_symbol
            && self.edits.len() == edits.len()
            && self.edits.iter().zip(edits).all(|((span, text), edit)| {
                *span == edit.span && text.as_slice() == edit.replacement
            })
    }
}

/// Inputs for checked authored source plan correspondence. Additional callers
/// must independently prove occurrence bijection, edit ownership, alias,
/// collision and formal topology.
pub struct ConditionalSourcePlanRequest<'a> {
    /// Exact original source.
    pub before_source: &'a str,
    /// Analysis of the original source.
    pub before_analysis: &'a AnalysisResult,
    /// Selected original variable symbol.
    pub before_symbol: &'a SignatureSourceVariableSymbol,
    /// Exact source generated by applying the edits.
    pub after_source: &'a str,
    /// Analysis of the generated source.
    pub after_analysis: &'a AnalysisResult,
    /// Corresponding variable symbol in the generated source.
    pub after_symbol: &'a SignatureSourceVariableSymbol,
    /// Ordered source edits to authenticate.
    pub edits: &'a [ConditionalNameEdit<'a>],
}

/// Compare exact authored plans while retaining every unknown operation.
#[must_use]
pub fn conditional_source_plan_correspondence(
    request: ConditionalSourcePlanRequest<'_>,
) -> Option<ConditionalSourcePlanCorrespondence> {
    let ConditionalSourcePlanRequest {
        before_source,
        before_analysis,
        before_symbol,
        after_source,
        after_analysis,
        after_symbol,
        edits,
    } = request;
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    let SignatureSourceVariableSlot::C {
        simple: before_name,
        ..
    } = before_symbol.slot()
    else {
        return None;
    };
    let SignatureSourceVariableSlot::C {
        simple: after_name, ..
    } = after_symbol.slot()
    else {
        return None;
    };
    if before_symbol.renamed(after_name.as_bytes()).as_ref() != Some(after_symbol) {
        return None;
    }
    let input = before_analysis.resolved_input.as_ref()?;
    if after_analysis.resolved_input.as_ref() != Some(input) {
        return None;
    }
    let before = inspect_conditional_variable_source(
        before_source,
        before_analysis,
        before_symbol,
        after_name.as_bytes(),
    )?;
    let after = inspect_conditional_variable_source(
        after_source,
        after_analysis,
        after_symbol,
        before_name.as_bytes(),
    )?;
    if !conditional_inventories_correspond(&before, &after, edits) {
        return None;
    }
    let mut expected = before_source.as_bytes().to_vec();
    for edit in edits.iter().rev() {
        let range = edit.span.as_range();
        if range.end > expected.len() {
            return None;
        }
        expected.splice(range, edit.replacement.iter().copied());
    }
    if expected != after_source.as_bytes() {
        return None;
    }
    Some(ConditionalSourcePlanCorrespondence {
        before: SourceImage::document(before_source),
        after: SourceImage::document(after_source),
        input: input.clone(),
        before_symbol: before_symbol.clone(),
        after_symbol: after_symbol.clone(),
        edits: edits
            .iter()
            .map(|edit| (edit.span, edit.replacement.to_vec()))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{Analyser, ResolvedAnalysisInput};

    fn analyse(source: &str) -> AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let input = ResolvedAnalysisInput::new(profile, profile, context, config);
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    #[test]
    fn conditional_alpha_maps_exact_written_occurrences_and_uncertainty() {
        for name in ["x", "scalar(open", "café🙂"] {
            let before = format!("set {{{name}}} 1\nputs ${{{name}}}\n");
            let after = "set {new} 1\nputs ${new}\n";
            let old = analyse(&before);
            let new = analyse(after);
            let old_symbol = old
                .original_variable_symbols
                .iter()
                .find(|row| row.is_declaration())
                .unwrap()
                .symbol();
            let new_symbol = new
                .original_variable_symbols
                .iter()
                .find(|row| row.is_declaration())
                .unwrap()
                .symbol();
            let length = u32::try_from(name.len()).unwrap();
            let edits = [
                ConditionalNameEdit {
                    span: Span::new(5, 5 + length),
                    replacement: b"new",
                },
                ConditionalNameEdit {
                    span: Span::new(16 + length, 16 + 2 * length),
                    replacement: b"new",
                },
            ];
            let certificate =
                conditional_source_plan_correspondence(ConditionalSourcePlanRequest {
                    before_source: &before,
                    before_analysis: &old,
                    before_symbol: old_symbol,
                    after_source: after,
                    after_analysis: &new,
                    after_symbol: new_symbol,
                    edits: &edits,
                })
                .expect("conditional source alpha");
            assert!(certificate.matches(
                &before,
                after,
                old.resolved_input.as_ref().unwrap(),
                old_symbol,
                new_symbol,
                &edits
            ));
            assert!(!certificate.matches(
                &(before.clone() + " "),
                after,
                old.resolved_input.as_ref().unwrap(),
                old_symbol,
                new_symbol,
                &edits
            ));
            assert!(!old.original_variable_rename_is_complete(old_symbol));
        }
    }

    #[test]
    fn logical_c91_unicode_literal_barrier_uses_unicode_projection() {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let source = "set {café🙂} 1; puts ${café🙂}; list {café🙂}";
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let symbol = analysis
            .original_variable_symbols
            .iter()
            .find(|row| row.is_declaration())
            .expect("actual selected variable")
            .symbol();
        assert!(inspect_conditional_variable_source(source, &analysis, symbol, b"new").is_none());
    }

    #[test]
    fn conditional_alpha_declines_unaccounted_source_and_prior_operations() {
        for source in [
            "$cmd x\nset x 1\nputs $x\n",
            "mystery\nset x 1\nputs $x\n",
            "puts hello\nset x 1\nputs $x\n",
            "namespace eval N {set x 1}\nset x 1\nputs $x\n",
            "proc p {} {set x 2}\nset x 1\nputs $x\n",
            "trace add variable x write callback\nset x 1\nputs $x\n",
            "set x 1\nputs $x\neval {set x 2}\n",
            "set x 1\nputs $x\nexpr {$x + 1}\n",
            "set x 1\nputs $x\nlist x\n",
            "set x 1\nputs $x\nlist {extra x}\n",
            r"set x 1; puts $x; list {\x78}",
            "set x 1\nputs $x\nputs [command]\n",
        ] {
            let analysis = analyse(source);
            let allowed = analysis
                .original_variable_symbols
                .iter()
                .find(|row| row.original_name_input().bytes() == b"x" && row.is_declaration())
                .and_then(|row| {
                    inspect_conditional_variable_source(source, &analysis, row.symbol(), b"new")
                });
            assert!(allowed.is_none(), "{source}");
        }
    }
}
