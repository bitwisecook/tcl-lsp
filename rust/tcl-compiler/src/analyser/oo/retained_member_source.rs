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

//! The original source operands kept for a member: its name, visibility, relation words, formal parameters and provider class.

use super::super::state::Analyser;
use super::super::types::ClassDef;
use super::{MemberStatement, member_side};
use tcl_lexer::{Span, Token};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::{DefinitionBodyGrammar, MemberEffect, MemberRefKind, MemberSpec};

impl Analyser {
    pub(in super::super) fn original_provider_class_name(
        &self,
        span: Span,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNameOccurrence> {
        let policy = self.declaration_name_policy()?;
        self.head_identities
            .original_source_name_at_span(span, self.file_lexer_config(), self.word_rules(), policy)
            .or_else(|| {
                // The actual walk retains this complete original static word
                // independently of invocation entry. It supplies naming syntax
                // only; the caller still selects the definer and naming scope.
                let original = self.original_static_source_name_at_span(span)?;
                let input = original.name_input();
                (input.lexer_config() == self.file_lexer_config()
                    && input.word_value_rules() == self.word_rules()
                    && input.policy() == policy)
                    .then(|| original.clone())
            })
    }

    pub(in super::super) fn retain_original_provider_class(
        &mut self,
        original: Option<&crate::signature_scan::original_name::SourceOriginalNameOccurrence>,
        class: &ClassDef,
    ) {
        if self.selected_logical_declaration_advice()
            && let Some(image) = self
                .result
                .retained_command_realm()
                .filter(|realm| {
                    realm.matches_resolved_analysis_input(&self.resolved_analysis_input())
                })
                .and_then(|realm| realm.original_source_image())
                .filter(|image| image.bytes() == self.source.as_bytes())
                .cloned()
            && let Some(declaration) = super::super::types::LogicalSourceClassDeclaration::capture(
                image,
                self.resolved_analysis_input(),
                class,
            )
        {
            self.result.logical_class_declarations.push(declaration);
        }
        if let Some(original) = original
            && let Some(name) = class.source_name.clone()
            && let Some(metadata) =
                crate::signature_scan::original_name::SourceDeclarationMetadata::new(
                    original,
                    name,
                    class.clone(),
                )
        {
            self.result.original_class_metadata.push(metadata);
        }
        // The selected definer supplies the class role. Hosted source words
        // retain their own context and unavailable materialisation; no C/Jim
        // command slot is recovered from the reporting class name.
        if let Some(original) = self.result.original_vendor_source_name_in_source(
            &tcl_lexer::SourceImage::document(&self.source),
            self.file_lexer_config(),
            class.name_span,
        ) {
            let record = crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata::new(
                original,
                tcl_syntax::naming::VendorSourceNamePurpose::SourceName,
                class.clone(),
            );
            if !self.result.original_vendor_class_metadata.contains(&record) {
                self.result.original_vendor_class_metadata.push(record);
            }
        }
    }

    pub(super) fn original_logical_member_words<'a>(
        &self,
        grammar: &'a DefinitionBodyGrammar,
        cmd: &'a crate::segmenter::SegmentedCommand,
    ) -> Option<(
        MemberStatement<'a>,
        super::super::ResolvedAnalysisInput,
        Vec<tcl_lexer::NativeWord>,
    )> {
        if !self.selected_logical_declaration_advice() || cmd.is_partial {
            return None;
        }
        let input = self.resolved_analysis_input();
        let image = self
            .result
            .retained_command_realm()
            .filter(|realm| realm.matches_resolved_analysis_input(&input))
            .and_then(|realm| realm.original_source_image())
            .filter(|image| image.bytes() == self.source.as_bytes())?;
        let config = self.file_lexer_config().nested();
        if config != input.lexer_config().nested() {
            return None;
        }
        let context = self.analysis_context();
        let statement = MemberStatement::read(
            grammar,
            &cmd.texts,
            &cmd.argv,
            Some(context.context().authoring_query()),
        )?;
        let groups = tcl_lexer::group_commands_bytes(&cmd.all_tokens, image.bytes(), config);
        let [group] = groups.as_slice() else {
            return None;
        };
        if group.words.len() != cmd.argv.len() || group.words.iter().any(|word| word.expand) {
            return None;
        }
        let words = group
            .words
            .iter()
            .map(|group| {
                tcl_lexer::NativeWord::from_group(image.clone(), config, &cmd.all_tokens, group)
            })
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        if !words.iter().zip(&cmd.argv).all(|(word, token)| {
            word.group().span == token.span
                && word.tokens().first().is_some_and(|original| {
                    original.kind == token.kind
                        && original.content_offset == token.content_offset
                        && original.in_quote == token.in_quote
                })
        }) || words[..=statement.row.keyword_index]
            .iter()
            .zip(&cmd.texts)
            .any(|(word, text)| {
                tcl_syntax::word_rules::original_static_word_ascii_presentation(word).as_deref()
                    != Some(text.as_bytes())
            })
        {
            return None;
        }
        Some((statement, input, words))
    }

    pub(in super::super) fn retain_logical_member_name_operand(
        &self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        class: &mut ClassDef,
    ) {
        let Some((statement, input, words)) = self.original_logical_member_words(grammar, cmd)
        else {
            return;
        };
        let MemberEffect::Callable {
            name_slot: Some(name_slot),
            ..
        } = statement.row.effect
        else {
            return;
        };
        let (Some(name), Some(side), Some(body)) = (
            statement.row.name.as_ref(),
            member_side(statement.row.receiver),
            statement.row.body,
        ) else {
            return;
        };
        let context = self.analysis_context();
        let surface = Some(context.context().authoring_query());
        let mut names =
            statement
                .member
                .indices_for_call_in(statement.args(), surface, ArgRole::Name);
        let mut bodies =
            statement
                .member
                .indices_for_call_in(statement.args(), surface, ArgRole::Body);
        if names.next() != Some(usize::from(name_slot))
            || names.next().is_some()
            || bodies
                .next()
                .map(|index| statement.row.keyword_index + 1 + index)
                != Some(body.0)
            || bodies.next().is_some()
        {
            return;
        }
        let name_ordinal = statement.row.keyword_index + 1 + usize::from(name_slot);
        let Some(word) = words.get(name_ordinal) else {
            return;
        };
        if tcl_syntax::word_rules::original_static_word_unicode_value(word).as_deref()
            != Some(name.as_bytes())
        {
            return;
        }
        let (Some(name_token), Some(body_token)) =
            (cmd.argv.get(name_ordinal), cmd.argv.get(body.0))
        else {
            return;
        };
        let mut parameters =
            statement
                .member
                .indices_for_call_in(statement.args(), surface, ArgRole::ParamList);
        let first_parameter = parameters.next();
        let parameter_ordinal = if parameters.next().is_none() {
            first_parameter.map(|index| statement.row.keyword_index + 1 + index)
        } else {
            None
        };
        class.original_members.retain_logical_name_operand(
            input,
            words,
            (name_ordinal, body.0, parameter_ordinal),
            side,
            (name_token.span, body_token.span),
            statement.row.clone(),
        );
    }

    pub(in super::super) fn retain_logical_member_visibility_operands(
        &self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        class: &mut ClassDef,
    ) {
        let Some((statement, input, words)) = self.original_logical_member_words(grammar, cmd)
        else {
            return;
        };
        if statement.row.effect != MemberEffect::Visibility
            || statement.member.all_args_ref != Some(MemberRefKind::Method)
        {
            return;
        }
        let (Some(side), Some(effect)) = (
            member_side(statement.row.receiver),
            statement.member.visibility_effect,
        ) else {
            return;
        };
        let first = statement.row.keyword_index + 1;
        if words[first..]
            .iter()
            .zip(statement.args())
            .any(|(word, name)| {
                name.as_bytes().contains(&0)
                    || tcl_syntax::word_rules::original_static_word_unicode_value(word).as_deref()
                        != Some(name.as_bytes())
            })
        {
            return;
        }
        for ordinal in first..words.len() {
            class.original_members.retain_logical_visibility_operand(
                input.clone(),
                words.clone(),
                ordinal,
                side,
                effect,
            );
        }
    }

    /// Selected source-member formal applicability and exact authored ordinals.
    /// The original class ledger supplies any lifecycle body; representative
    /// tokens and reporting strings cannot stand in for that whole word.
    pub(super) fn member_formal_parameter_indices(
        &self,
        grammar: &DefinitionBodyGrammar,
        member: &MemberSpec,
        args: &[String],
        tokens: &[Token],
        class: &ClassDef,
    ) -> Vec<usize> {
        use tcl_registry::definer::SourceFormalValidationApplicability;
        // naming.tcloo.empty-lifecycle-definition
        // docs/design/analysis/name-resolution-proofs/tcloo-empty-lifecycle-definition.md
        let context = self.analysis_context();
        let surface = Some(context.context().authoring_query());
        let applicability = grammar
            .source_member_formal_validation_applicability(member, None)
            .or_else(|| {
                let kind = grammar.source_special_member_kind(member)?;
                let body_index = member
                    .indices_for_call_in(args, surface, ArgRole::Body)
                    .next()?;
                let body_token = tokens.get(body_index)?;
                let image = tcl_lexer::SourceImage::document(&self.source);
                let config = self.file_lexer_config();
                let mut selected = class
                    .original_special_members
                    .declarations()
                    .filter(|entry| {
                        let word = entry.body_word();
                        entry.kind() == kind
                            && word.image() == &image
                            && word.config() == config
                            && word.group().span.start() == body_token.span.start()
                    });
                let entry = selected.next()?;
                if selected.next().is_some() {
                    return None;
                }
                let values = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                    std::slice::from_ref(entry.body_word()),
                    self.declaration_name_policy()?.string_protocol(),
                )
                .ok()?;
                grammar
                    .source_member_formal_validation_applicability(member, Some(values.literal(0)?))
            });
        if applicability != Some(SourceFormalValidationApplicability::Required) {
            return Vec::new();
        }
        member
            .indices_for_call_in(args, surface, ArgRole::ParamList)
            .collect()
    }

    pub(super) fn retain_logical_relation_command(
        &self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        class: &mut ClassDef,
    ) {
        let Some((statement, input, words)) = self.original_logical_member_words(grammar, cmd)
        else {
            return;
        };
        if matches!(
            statement.row.effect,
            MemberEffect::Configuration | MemberEffect::InitScript { .. }
        ) {
            class
                .original_members
                .mark_logical_relation_coverage_unavailable();
            return;
        }
        let MemberEffect::Relation { slot: relation } = statement.row.effect else {
            return;
        };
        let side = member_side(statement.row.receiver);
        let slot = statement.member.slot;
        class.original_members.retain_logical_relation_command(
            input,
            words,
            (side, relation, slot, statement.row.slot_op),
        );
    }

    pub(in super::super) fn retain_class_relation_lookups(
        &self,
        texts: &[String],
        class: &mut ClassDef,
        scope_path: &[usize],
    ) {
        // These are the same analyser-local field routes as apply_oo_subcommand_in.
        let names = match texts.first().map(String::as_str) {
            // registry-axis-ok: definition_body — class-definition relation keywords; until the definition-body grammar exposes relation kinds
            Some("superclass" | "inherit") => &class.superclasses,
            // registry-axis-ok: definition_body — class-definition relation keywords; until the definition-body grammar exposes relation kinds
            Some("mixin") => &class.mixins,
            _ => return,
        };
        for written in names.iter().filter(|name| texts[1..].contains(name)) {
            let lookup = self.declaration_name_policy().and_then(|policy| {
                crate::signature_scan::scope::SignatureSourceLookup::new(
                    policy,
                    self.declaration_namespace_scope(scope_path)?,
                    written.clone(),
                )
            });
            match class.relation_lookups.entry(written.clone()) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(lookup);
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    if entry.get() != &lookup {
                        entry.insert(None);
                    }
                }
            }
        }
    }
}
