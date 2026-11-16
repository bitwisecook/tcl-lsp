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

//! Which operands of a command are script bodies under each capture purpose, and the geometry of declared bodies.

use super::{CapturePurpose, PendingSourceRegion, SourceScriptContext, SourceStructure};
use std::collections::VecDeque;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_lexer::{
    ExecutablePart, ExecutableText, LexerConfig, NativeWord, SourceImage, Span, WordKind,
};
use tcl_registry::ArgRole;

#[derive(Clone, Copy)]
pub(super) struct BodyGeometry {
    pub(super) extent: Span,
    pub(super) content: Span,
}

impl SourceStructure {
    fn readonly_command_bodies(
        &mut self,
        source: &str,
        command: &SegmentedCommand,
        script: &SourceScriptContext<'_>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) -> bool {
        let SourceScriptContext {
            analysis,
            depth,
            purpose,
            definition_members,
            ..
        } = *script;
        if matches!(
            purpose,
            CapturePurpose::Readonly | CapturePurpose::DefinitionCursor
        ) && script.definition_parent.is_some()
        {
            use tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberDisposition as D;
            let Some(members) = definition_members else {
                return true;
            };
            let Some(words) = command
                .argv
                .first()
                .and_then(|head| members.original_command_words_at(head.span.start()))
            else {
                return true;
            };
            let Some(input) = analysis.resolved_input.as_ref() else {
                return true;
            };
            match members.script_body_disposition(&input.context_registry(), words) {
                D::Unavailable => return true,
                D::CompleteNonMember => {}
                D::CompleteMember(bodies) => {
                    for body in bodies {
                        self.bodies.push(body.extent_span());
                        self.caller_context_unavailable_bodies
                            .push(body.content_span());
                        queue.push_back((
                            body.content_span(),
                            depth + 1,
                            body.definition_parent().cloned(),
                        ));
                    }
                    return true;
                }
            }
        }
        if matches!(
            purpose,
            CapturePurpose::Readonly | CapturePurpose::DefinitionCursor
        ) && let Some(declared) =
            tcl_compiler::registry_invocation::source_structure::source_declared_command_words(
                source, analysis, command,
            )
        {
            for body in declared.source_script_bodies_for(
                tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::Syntax,
            ) {
                self.bodies.push(body.extent_span());
                self.caller_context_unavailable_bodies
                    .push(body.content_span());
                queue.push_back((body.content_span(), depth + 1, None));
            }
            return true;
        }
        false
    }

    fn definition_cursor_document_bodies(
        &mut self,
        source: &str,
        command: &SegmentedCommand,
        script: &SourceScriptContext<'_>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) -> bool {
        if script.purpose != CapturePurpose::DefinitionCursor {
            return false;
        }
        let Some(input) = script.analysis.resolved_input.as_ref() else {
            return false;
        };
        let context = input.context_registry();
        let registry = context.commands();
        let Some(document) = registry.document_grammar() else {
            return false;
        };
        let current = if let Some(parent) = script.definition_parent {
            parent
                .source_words()
                .with_source_schema(&context, |schema| {
                    schema.authored_source_definition_body_grammar()
                })
                .flatten()
        } else if script.depth == 0 {
            Some(document)
        } else {
            None
        };
        let Some(current) = current.filter(|grammar| grammar.family == document.family) else {
            return false;
        };
        let words = if let Some(parent) = script.definition_parent {
            crate::original_invocation::source_registry_words_in_definition_body(
                source,
                script.analysis,
                command,
                parent,
            )
        } else {
            crate::original_invocation::source_registry_words(source, script.analysis, command)
        };
        let Some(words) = words else {
            return true;
        };
        let Some(head) = words.head_source().and_then(|head| head.word()) else {
            return true;
        };
        let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(head) else {
            return true;
        };
        let Ok(keyword) = std::str::from_utf8(&value) else {
            return true;
        };
        let Some(expected) = registry.authored_document_member_grammar(current, keyword) else {
            // Non-block members may own an ordinary retained script (a hook),
            // but a command outside this actual closed vocabulary may not.
            return !current.is_member(keyword);
        };
        let selected = words
            .with_source_schema(&context, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten();
        if !selected.is_some_and(|grammar| std::ptr::eq(grammar, expected)) {
            return true;
        }
        for body in words.source_script_bodies(&context) {
            let Some(parent) = body.definition_parent_for(&context, script.definition_parent)
            else {
                continue;
            };
            self.bodies.push(body.extent_span());
            queue.push_back((body.content_span(), script.depth + 1, Some(parent)));
        }
        true
    }

    fn compiler_expression_regions(
        &mut self,
        source: &str,
        command: &SegmentedCommand,
        script: &SourceScriptContext<'_>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) {
        let Some(input) = script.analysis.resolved_input.as_ref() else {
            return;
        };
        let context = input.context_registry();
        let bodies = self
            .compiler_body_syntax
            .as_ref()
            .and_then(|inventory| {
                inventory.expression_script_bodies(source, script.analysis, command)
            })
            .unwrap_or_default();
        for body in bodies {
            if !body.matches_source(&SourceImage::document(source), input.lexer_config())
                || !body.matches_context(&context)
            {
                continue;
            }
            self.caller_context_unavailable_bodies
                .push(body.content_span());
            queue.push_back((body.content_span(), script.depth + 1, None));
        }
    }

    pub(super) fn command_bodies(
        &mut self,
        source: &str,
        command: &SegmentedCommand,
        script: &SourceScriptContext<'_>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) {
        let SourceScriptContext {
            analysis,
            depth,
            purpose,
            definition_parent,
            ..
        } = *script;
        let Some(config) = analysis.body_lexer_config else {
            return;
        };
        if purpose == CapturePurpose::CheckedSyntax {
            self.census_command_bodies(source, command, script, queue);
            return;
        }
        if purpose == CapturePurpose::SyntaxCompaction
            && let Some(input) = analysis.resolved_input.as_ref()
        {
            self.compiler_expression_regions(source, command, script, queue);
            let context = input.context_registry();
            let bodies = self
                .compiler_body_syntax
                .as_ref()
                .and_then(|inventory| inventory.script_bodies(source, analysis, command))
                .unwrap_or_default();
            for body in bodies {
                if !body.matches_source(&SourceImage::document(source), config)
                    || !body.matches_context(&context)
                {
                    continue;
                }
                self.bodies.push(body.container_extent_span());
                self.bodies.push(body.extent_span());
                // This syntax-only path never lends caller/frame or definition
                // authority to another purpose or to descendant interpretation.
                self.caller_context_unavailable_bodies
                    .push(body.content_span());
                queue.push_back((body.content_span(), depth + 1, None));
            }
        }
        if self.definition_cursor_document_bodies(source, command, script, queue)
            || self.readonly_command_bodies(source, command, script, queue)
        {
            return;
        }
        let selected = crate::original_invocation::source_registry_words(source, analysis, command);
        let selected = if purpose == CapturePurpose::SyntaxCompaction
            && selected.as_ref().is_none_or(|words| {
                !matches!(
                    words.source,
                    crate::original_invocation::OriginalRegistrySource::Selected
                ) || !words.operands_preserve_source_lookup()
            }) {
            self.compiler_body_syntax
                .as_ref()
                .and_then(|inventory| inventory.words(source, analysis, command))
                .map(crate::original_invocation::from_source_words)
        } else {
            selected
        };
        let Some(words) = selected else {
            return;
        };
        if purpose == CapturePurpose::SyntaxCompaction
            && (!matches!(
                words.source,
                crate::original_invocation::OriginalRegistrySource::Selected
            ) || !words.operands_preserve_source_lookup())
        {
            return;
        }
        let Some(input) = analysis.resolved_input.as_ref() else {
            return;
        };
        let context = input.context_registry();
        for body in words.source_script_bodies(&context) {
            self.bodies.push(body.container_extent_span());
            self.bodies.push(body.extent_span());
            if !body.inherits_immediate_caller_source_context(&context) {
                self.caller_context_unavailable_bodies
                    .push(body.content_span());
                if body.runs_during_invocation(&context) {
                    self.invocation_scoped_bodies.push(body.content_span());
                }
            }
            let parent = matches!(
                purpose,
                CapturePurpose::Readonly | CapturePurpose::DefinitionCursor
            )
            .then(|| body.definition_parent_for(&context, definition_parent))
            .flatten();
            queue.push_back((body.content_span(), depth + 1, parent));
        }
        self.command_operand_bodies(source, (&words, &context), input, config, script, queue);
    }

    fn command_operand_bodies(
        &mut self,
        source: &str,
        words_context: (
            &crate::original_invocation::OriginalRegistryWords,
            &tcl_registry::model::ContextRegistry,
        ),
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
        config: tcl_lexer::LexerConfig,
        script: &SourceScriptContext<'_>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) {
        let (words, context) = words_context;
        let SourceScriptContext {
            depth,
            definition_parent,
            ..
        } = *script;
        let list_argument = words
            .with_source_schema(context, |schema| {
                schema
                    .authored_source_case_invocation()
                    .and_then(|(_, layout)| layout.clause_list_index)
            })
            .flatten();
        for &(argument, role) in words.roles.as_deref().unwrap_or_default() {
            if list_argument == Some(argument) {
                continue;
            }
            let Some(operand) = words.operands.get(argument).and_then(Option::as_ref) else {
                continue;
            };
            if role == ArgRole::Expr {
                if let Some(bodies) = words.source_expression_script_bodies(input) {
                    for body in bodies {
                        queue.push_back((
                            body.content_span(),
                            depth + 1,
                            definition_parent.cloned(),
                        ));
                    }
                }
            } else if role == ArgRole::LambdaLiteral {
                if let Some(body) = operand_body(source, operand, config) {
                    self.bodies.push(body.extent);
                }
                if let Some(word) = operand.word.as_ref()
                    && let Some(body) =
                        tcl_compiler::lambda_literal::split_original_lambda_literal(word)
                            .and_then(|elements| elements.braced_body())
                {
                    self.caller_context_unavailable_bodies.push(body);
                    queue.push_back((body, depth + 1, None));
                }
            } else if role.folds_as_block()
                && let Some(body) = operand_body(source, operand, config)
            {
                self.bodies.push(body.extent);
                // Script regions are issued by the shared Body/case owner.
                // Fold geometry for other authored roles remains separate.
            }
        }
    }

    fn census_definition_member_bodies(
        &mut self,
        command: &SegmentedCommand,
        script: &SourceScriptContext<'_>,
        context: &tcl_registry::model::ContextRegistry,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) -> bool {
        use tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberDisposition as D;
        let disposition = script
            .definition_members
            .zip(command.argv.first())
            .and_then(|(members, head)| {
                members
                    .original_command_words_at(head.span.start())
                    .map(|words| members.script_body_disposition(context, words))
            });
        match disposition {
            Some(D::CompleteNonMember) => {}
            Some(D::CompleteMember(bodies)) => {
                for body in bodies {
                    queue.push_back((
                        body.content_span(),
                        script.depth + 1,
                        body.definition_parent().cloned(),
                    ));
                }
                return true;
            }
            Some(D::Unavailable) | None => {
                self.package_census
                    .as_mut()
                    .unwrap()
                    .unavailable
                    .push(command.span);
                return true;
            }
        }
        false
    }

    fn census_command_bodies(
        &mut self,
        source: &str,
        command: &SegmentedCommand,
        script: &SourceScriptContext<'_>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) {
        use tcl_compiler::registry_invocation::source_structure::{
            OriginalSourcePackageReferenceSelection as Selection, source_registry_words,
        };
        let analysis = script.analysis;
        let Some(input) = analysis.resolved_input.as_ref() else {
            return;
        };
        let context = input.context_registry();
        // A definition body has its own scoped member vocabulary, which a
        // global head lookup cannot represent. Classify each command through
        // the retained grammar and complete member geometry first, as the
        // checked potential-source walker does. Missing member geometry or an
        // unavailable disposition stays incomplete without a global fallback;
        // only a complete nonmember keeps the ordinary lookup below.
        if script.definition_parent.is_some()
            && self.census_definition_member_bodies(command, script, &context, queue)
        {
            return;
        }
        let Some(words) = source_registry_words(source, analysis, command) else {
            if original_positioned_absent_head(source, analysis, command) {
                // A genuine positioned nonpackage leaf is sufficient only for
                // this source package census, never generic syntax closure.
                self.package_census
                    .as_mut()
                    .unwrap()
                    .syntax_unavailable
                    .push(command.span);
            } else {
                self.package_census
                    .as_mut()
                    .unwrap()
                    .unavailable
                    .push(command.span);
            }
            return;
        };
        match words.package_reference_selection(&context) {
            Selection::Reference(reference) => self
                .package_census
                .as_mut()
                .unwrap()
                .references
                .push(reference),
            Selection::Unavailable => self
                .package_census
                .as_mut()
                .unwrap()
                .package_unavailable
                .push(command.span),
            Selection::NotReference => {}
        }
        match words.source_script_bodies_for_syntax_coverage(&context) {
            Some(bodies) => {
                for body in bodies {
                    self.scoped_environment_regions
                        .push((body.clone(), script.depth + 1));
                    queue.push_back((
                        body.content_span(),
                        script.depth + 1,
                        body.definition_parent_for(&context, script.definition_parent),
                    ));
                }
            }
            None => self
                .package_census
                .as_mut()
                .unwrap()
                .unavailable
                .push(command.span),
        }
        if words
            .roles()
            .is_some_and(|roles| roles.iter().any(|(_, role)| *role == ArgRole::Expr))
        {
            match words.source_expression_script_bodies(input) {
                Some(bodies) => {
                    for body in bodies {
                        queue.push_back((
                            body.content_span(),
                            script.depth + 1,
                            script.definition_parent.cloned(),
                        ));
                    }
                }
                None => self
                    .package_census
                    .as_mut()
                    .unwrap()
                    .unavailable
                    .push(command.span),
            }
        }
    }
}

/// A genuine positioned source absence can exclude this exact static head
/// from selected Registry package syntax. This grants no unknown-handler,
/// runtime package, command execution or Native absence proof.
fn original_positioned_absent_head(
    source: &str,
    analysis: &AnalysisResult,
    command: &SegmentedCommand,
) -> bool {
    let Some(head) = command.argv.first() else {
        return false;
    };
    let Some(config) = analysis.body_lexer_config else {
        return false;
    };
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    let Some(realm) = analysis.retained_command_realm() else {
        return false;
    };
    if !realm.matches_resolved_analysis_input(input) {
        return false;
    }
    let image = SourceImage::document(source);
    let Some((original, owner)) =
        realm.original_written_word_at_span_in_source(&image, head.span, config)
    else {
        return false;
    };
    owner == head.span.start()
        && !original.group().expand
        && tcl_syntax::word_rules::original_static_word_unicode_value(&original).is_some()
        && realm.diagnostic_slot_presence_at(owner)
            == tcl_compiler::command_binding::SourceCommandSlotPresence::Absent
}

fn operand_body(
    source: &str,
    operand: &crate::original_invocation::OriginalOperandSource,
    config: LexerConfig,
) -> Option<BodyGeometry> {
    let image = SourceImage::document(source);
    if let Some(word) = &operand.word {
        (word.image() == &image && word.config() == config).then_some(())?;
        return word_body(word);
    }
    // An expanded child keeps its readonly extent, never a whole-word key.
    // Only the existing static-container facet and exact unchanged bytes can
    // lend this original extent to a source-only script-role walk.
    let input = operand.input.as_ref()?;
    let container = input.original_static_list_container()?;
    let parent = container.parent_word();
    if parent.image() != &image
        || parent.config() != config
        || image.bytes().get(operand.span.as_range()) != Some(input.bytes())
    {
        return None;
    }
    Some(BodyGeometry {
        extent: operand.span,
        content: operand.span,
    })
}

fn word_body(word: &NativeWord) -> Option<BodyGeometry> {
    if word.group().expand
        || (word.group().kind != WordKind::Braced
            && !(word.group().kind == WordKind::Quoted
                && word.executable_parts().all_parts().all(|part| {
                    matches!(part.part, ExecutablePart::Text(ExecutableText::Original))
                })))
    {
        return None;
    }
    let content = word.content_span().ok()?;
    // The fold ends at the original content boundary, keeping its closer visible.
    Some(BodyGeometry {
        extent: Span::new(word.group().span.start(), content.end()),
        content,
    })
}

/// Recover only lexical geometry of an independently owned declaration body.
/// The real source word is captured by the shared lexer; no name key is minted.
fn declared_body(image: &SourceImage, span: Span, config: LexerConfig) -> Option<BodyGeometry> {
    let whole = tcl_lexer::word_span_at(image.try_text().ok()?, span);
    let plan = tcl_lexer::native_script_words_in(image.clone(), whole, config).ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    (plan.fatal_tail.is_none() && (word.group().span == span || word.span() == span))
        .then_some(())?;
    word_body(word)
}

pub(super) fn declared_bodies(
    source: &str,
    analysis: &AnalysisResult,
    image: &SourceImage,
    config: LexerConfig,
) -> Option<Vec<BodyGeometry>> {
    use crate::original_declaration::OriginalDeclarationIdentity;
    let mut bodies = Vec::new();
    for declaration in analysis.original_procedure_declarations() {
        OriginalDeclarationIdentity::for_procedure("", source, analysis, declaration)?;
        if let Some(body) = declared_body(image, declaration.metadata().body_span, config) {
            bodies.push(body);
        }
    }
    for class in analysis.original_class_declarations() {
        OriginalDeclarationIdentity::for_class("", source, analysis, class)?;
        if let Some(body) = declared_body(image, class.metadata().body_span, config) {
            bodies.push(body);
        }
        for method in class.metadata().original_members.declarations() {
            OriginalDeclarationIdentity::for_method("", source, analysis, class, method)?;
        }
        for special in class.metadata().original_special_members.declarations() {
            OriginalDeclarationIdentity::for_special("", source, analysis, class, special)?;
        }
        for property in class.metadata().original_properties.declarations() {
            OriginalDeclarationIdentity::for_property("", source, analysis, class, property)?;
        }
        retain_member_bodies(
            class.metadata().original_members.declarations(),
            class.metadata().original_special_members.declarations(),
            class.metadata().original_properties.declarations(),
            analysis,
            image,
            config,
            &mut bodies,
        )?;
    }
    for row in analysis.original_class_configurations() {
        (row.target().site().source.source_image() == image).then_some(())?;
        retain_member_bodies(
            row.members().declarations(),
            row.special_members().declarations(),
            row.properties().declarations(),
            analysis,
            image,
            config,
            &mut bodies,
        )?;
    }
    for row in analysis.original_object_configurations() {
        (row.target().site().source.source_image() == image).then_some(())?;
        retain_member_bodies(
            row.members().declarations(),
            row.special_members().declarations(),
            row.properties().declarations(),
            analysis,
            image,
            config,
            &mut bodies,
        )?;
    }
    for declaration in analysis.original_vendor_procedure_declarations() {
        declaration
            .name_input()
            .matches_source(image, config)
            .then_some(())?;
        if let Some(body) = declared_body(image, declaration.metadata().body_span, config) {
            bodies.push(body);
        }
    }
    for declaration in analysis.original_vendor_class_declarations() {
        declaration
            .name_input()
            .matches_source(image, config)
            .then_some(())?;
        if let Some(body) = declared_body(image, declaration.metadata().body_span, config) {
            bodies.push(body);
        }
    }
    if analysis.allows_lexical_declaration_advice() {
        bodies.extend(logical_declared_bodies(analysis, image)?);
    }
    Some(bodies)
}

pub(super) fn logical_declared_bodies(
    analysis: &AnalysisResult,
    image: &SourceImage,
) -> Option<Vec<BodyGeometry>> {
    analysis.allows_lexical_declaration_advice().then_some(())?;
    let mut bodies = Vec::new();
    let input = analysis.resolved_input.as_ref()?;
    for procedure in analysis.all_procs.values() {
        let Some(word) = analysis
            .logical_source_procedure_declaration(procedure)
            .and_then(|declaration| declaration.original_body_word(image, input))
        else {
            continue;
        };
        if let Some(body) = word_body(word) {
            bodies.push(body);
        }
    }
    for class in analysis.all_classes.values() {
        let Some(word) = analysis
            .logical_source_class_declaration(class)
            .and_then(|declaration| declaration.original_body_word(image, input))
        else {
            continue;
        };
        if let Some(body) = word_body(word) {
            bodies.push(body);
        }
    }
    Some(bodies)
}

fn retain_member_bodies<'a>(
    methods: impl Iterator<Item = &'a tcl_compiler::analyser::types::OriginalSourceMethodMetadata>,
    specials: impl Iterator<
        Item = &'a tcl_compiler::analyser::types::OriginalSourceSpecialMemberMetadata,
    >,
    properties: impl Iterator<Item = &'a tcl_compiler::analyser::types::OriginalSourcePropertyMetadata>,
    analysis: &AnalysisResult,
    image: &SourceImage,
    config: LexerConfig,
    bodies: &mut Vec<BodyGeometry>,
) -> Option<()> {
    for method in methods {
        let declaration = method.declaration();
        let word = declaration.original_word();
        (word.image() == image
            && word.config() == config
            && declaration.site().source.source_image() == image)
            .then_some(())?;
        if method.forward_prefix().is_none()
            && let Some(word) = method.body_word()
        {
            (word.image() == image && word.config() == config).then_some(())?;
            if let Some(body) = word_body(word) {
                bodies.push(body);
            }
        }
    }
    for special in specials {
        let word = special.body_word();
        (word.image() == image
            && word.config() == config
            && special.site().source.source_image() == image)
            .then_some(())?;
        if let Some(body) = word_body(word) {
            bodies.push(body);
        }
    }
    for property in properties {
        property.matches_producer_input(analysis).then_some(())?;
        let declaration = property.declaration();
        (declaration.site().source.source_image() == image
            && declaration.name_input().lexer_config() == config)
            .then_some(())?;
        for input in property.getter().into_iter().chain(property.setter()) {
            let tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key) =
                input
            else {
                continue;
            };
            let word = key.original_word();
            (word.image() == image && word.config() == config).then_some(())?;
            if let Some(body) = word_body(word) {
                bodies.push(body);
            }
        }
    }
    Some(())
}
