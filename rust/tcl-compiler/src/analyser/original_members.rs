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

//! Original member metadata from the definition walk's complete source words.
//! This is a source inventory, independent of handler execution and native MRO.

use super::{
    state::Analyser,
    types::{
        ClassDef, MemberSide, MethodDef, OriginalSourceClassRelation,
        OriginalSourceClassRelationEffect, OriginalSourceClassRelationKind,
        OriginalSourceMemberDeclaration, OriginalSourceMemberEffect,
        OriginalSourceMemberEffectKind, OriginalSourceMethodMetadata,
        OriginalSourcePropertyMetadata, OriginalSourceSpecialMemberMetadata, PropertyDef,
    },
};
use crate::{
    command_binding::{CommandAllocationSite, SourceOriginId},
    ir::CommandTokens,
    signature_scan::{
        original_name::{OriginalSourceNamePurpose, SourceOriginalNameOccurrence},
        scope::{
            SignatureNamespaceScope, SignatureSourceLookup, SignatureSourceNameInput,
            SignatureSourceNameKey, SignatureSourceNameValue,
        },
    },
};
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage, SourceMap, Token};
use tcl_registry::{
    arg_role::ArgRole,
    definer::{
        DeclaredMemberVisibility, DefinitionBodyGrammar, MemberKind, MemberSpec, MemberVisibility,
    },
};

fn counted_body_schema_matches(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
    grammar: &'static DefinitionBodyGrammar,
) -> bool {
    words.with_source_schema(context, |schema| {
        schema
            .authored_source_definition_body_grammar()
            .is_some_and(|selected| std::ptr::eq(selected, grammar))
    }) == Some(true)
}

fn counted_body_method(
    entry: &OriginalSourceMethodMetadata,
) -> Option<super::types::OriginalCountedBodyMethod> {
    let declaration = entry.declaration();
    declaration.source_installer().is_none().then_some(())?;
    let name = declaration.static_occurrence()?.name_input().clone();
    Some(super::types::OriginalCountedBodyMethod {
        name,
        worker_word: declaration.original_word().clone(),
        site: declaration.site().clone(),
        side: entry.side(),
        parameters: entry.parameters_word().cloned(),
        body: entry.body_word().cloned(),
    })
}

impl Analyser {
    /// Initial body syntax only; no installed or complete class membership.
    pub(super) fn counted_initial_body_methods(
        &self,
        declaration: &crate::command_binding::OriginalSourceClassDeclaration,
        body: &crate::registry_invocation::OriginalSourceScriptBody,
        scope_path: &[usize],
    ) -> Option<Vec<super::types::OriginalCountedBodyMethod>> {
        let input = self.resolved_analysis_input();
        let context = input.context_registry();
        let image = SourceImage::document(&self.source);
        let config = self.file_lexer_config();
        if !self.result.matches_original_source_image(&image, config)
            || !body.matches_source(&image, config)
            || !body.matches_context(&context)
            || !declaration.factory().matches_source(&image, config)
            || !declaration.factory().matches_context(&context)
        {
            return None;
        }
        let grammar = declaration.grammar(&context)?;
        (grammar.profile.has_native_definition_workers()).then_some(())?;
        let factory = declaration.factory();
        let descriptor = context
            .context()
            .resolve_spec(context.commands(), factory.command())?;
        descriptor
            .traits
            .contains(tcl_registry::Traits::IS_OO_METACLASS)
            .then_some(())?;
        let arguments = factory.arguments();
        let selector = std::str::from_utf8(arguments.first()?.value.as_deref()?).ok()?;
        let method = tcl_registry::CommandRegistry::manufacturer_method_for_descriptor(
            descriptor, selector,
        )?;
        (method.visibility == MemberVisibility::Exported).then_some(())?;
        let body_index = usize::from(method.definition_body_at?);
        let name_index = usize::from(method.names_instance_at?);
        let name = arguments.get(name_index)?.input.as_ref()?.native_input()?;
        (declaration.name_input().native_input()? == name).then_some(())?;
        let original_key = name.original_word_key()?;
        let caller_scope = self.declaration_namespace_scope(scope_path)?;
        let publication = crate::signature_scan::scope::SignatureSourceCommand::object_from_key(
            &caller_scope,
            original_key,
        )?;
        (publication.slot() == declaration.source_slot()).then_some(())?;
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            &self.source,
            &self.result,
            factory.site().offset,
        )?;
        if body.source_words() != &words
            || words.original_argument_word(body_index) != Some(body.original_container())
            || arguments.get(body_index)?.original != *body.original_container()
            || !counted_body_schema_matches(&words, &context, grammar)
        {
            return None;
        }
        let origin = &factory.site().source;
        (origin.source_image() == &image).then_some(())?;
        let bindings = self.head_identities.source_bindings_ref();
        (bindings.source_origin()? == origin).then_some(())?;
        let realm = self.result.retained_command_realm()?;
        realm
            .matches_resolved_analysis_input(&input)
            .then_some(())?;
        let registry = self.registry.as_ref()?;
        Arc::ptr_eq(registry, context.commands()).then_some(())?;
        let policy = self.declaration_name_policy()?;
        (policy == name.policy()).then_some(())?;
        let dialect = super::oo::MemberDialect {
            surface: Some(context.context().authoring_query()),
            rules: self.word_rules(),
        };
        let mut producer = Producer {
            grammar,
            origin,
            dialect,
            policy,
            name_purpose: OriginalSourceNamePurpose::for_analysis_input(
                &input,
                self.source_analysis_entry.as_deref(),
                policy,
            ),
            resolved_input: self.resolved_analysis_input(),
            source_dialect: self.original_member_source_dialect(),
            bindings,
            registry: Some(registry.as_ref()),
            caller_scope: Some(caller_scope),
            readonly_names: Vec::new(),
            source_installer: None,
            class_relation_body: Some(body),
        };
        let mut scratch = ClassDef::default();
        producer.configuration_body(
            std::slice::from_ref(body.original_container()),
            MemberSide::Instance,
            &mut scratch,
            config,
        )?;
        scratch
            .original_members
            .declarations()
            .map(counted_body_method)
            .collect()
    }

    /// Accumulate source metadata only under its authentic original class definer.
    /// Callers supply a class freshly walked from a sealed source call/reference;
    /// the reporting map never selects the declaration or donates a class token.
    pub(super) fn retain_original_source_class_update(
        &mut self,
        source_class: &crate::command_binding::OriginalSourceClassDeclaration,
        class: &ClassDef,
    ) -> bool {
        let Some(original) = source_class.source_class(&self.result) else {
            return false;
        };
        let canonical = original.metadata();
        if class.source_name.as_ref() != Some(original.name())
            || class.source_name_ambiguous.is_observed()
            || canonical.source_name_ambiguous.is_observed()
            || class.name_span != canonical.name_span
            || class.body_span != canonical.body_span
        {
            return false;
        }
        let Some(updated) = crate::signature_scan::original_name::SourceDeclarationMetadata::new(
            original.original_occurrence(),
            original.name().clone(),
            class.clone(),
        ) else {
            return false;
        };
        let index = self
            .result
            .original_class_metadata
            .iter()
            .position(|record| std::ptr::eq(record, original));
        let Some(index) = index else {
            return false;
        };
        self.result.original_class_metadata[index] = updated;
        true
    }

    /// Retain a readonly two-word procedure member under its exact source join.
    /// The original procedure schema supplies parameter/body words and dialect;
    /// its readonly list child never becomes a whole-word method naming token.
    pub(super) fn retain_original_two_word_source_member(
        &self,
        reference: &crate::command_binding::OriginalSourceClassReference,
        procedure: &crate::signature_scan::original_name::SourceDeclarationMetadata<
            super::types::ProcDef,
        >,
        member: &SignatureSourceNameInput,
        class: &mut ClassDef,
    ) -> bool {
        let Some(metadata) =
            self.original_two_word_source_member(reference, procedure, member, class)
        else {
            return false;
        };
        class.original_members.declare(metadata);
        true
    }

    fn original_two_word_source_member(
        &self,
        reference: &crate::command_binding::OriginalSourceClassReference,
        procedure: &crate::signature_scan::original_name::SourceDeclarationMetadata<
            super::types::ProcDef,
        >,
        member: &SignatureSourceNameInput,
        class: &ClassDef,
    ) -> Option<OriginalSourceMethodMetadata> {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        (reference.source_procedure(&self.result)? == procedure).then_some(())?;
        let canonical = reference.class_declaration().source_class(&self.result)?;
        (class.source_name.as_ref() == Some(canonical.name())
            && class.name_span == canonical.metadata().name_span
            && class.body_span == canonical.metadata().body_span)
            .then_some(())?;
        let whole = procedure.name_input().original_word();
        let children = SignatureSourceNameInput::OriginalWord(procedure.name_input().clone())
            .original_list_elements()?;
        let [class_input, member_input] = children.as_slice() else {
            return None;
        };
        (class_input == reference.name_input() && member_input == member).then_some(())?;
        let advice = reference.procedure_declaration()?;
        let grammar = reference
            .class_declaration()
            .grammar(&self.analysis_context())?;
        let purpose = grammar.two_word_procedure_member_name_purpose()?;
        let declaration = OriginalSourceMemberDeclaration::from_original_input(
            advice.site(),
            whole,
            member.clone(),
            OriginalSourceNamePurpose::for_analysis_input(
                &self.resolved_analysis_input(),
                self.source_analysis_entry.as_deref(),
                member.policy(),
            ),
        )?;
        let proc = procedure.metadata();
        let method = MethodDef {
            name: std::str::from_utf8(member.bytes()).ok()?.to_owned(),
            params: proc.params.clone(),
            params_computed: proc.params_computed,
            formal_count: proc.formal_count.clone(),
            name_span: proc.name_span,
            body_span: proc.body_span,
            kind: "method".to_owned(),
            is_self_method: false,
            visibility: "public".to_owned(),
            doc: proc.doc.clone(),
            forward_target: None,
        };
        let parameters = original_two_word_role_word(advice, ArgRole::ParamList)?;
        let body = original_two_word_role_word(advice, ArgRole::Body)?;
        OriginalSourceMethodMetadata::from_declaration(
            declaration,
            MemberSide::Instance,
            method,
            true,
            false,
            None,
            purpose,
        )?
        .with_body_role_words(Some(parameters), Some(body), Some(advice.dialect()))
    }

    pub(super) fn original_authored_jim_procedure_member(
        &self,
        reference: &crate::command_binding::OriginalAuthoredJimProcedureMember,
        procedure: &crate::signature_scan::original_name::SourceDeclarationMetadata<
            super::types::ProcDef,
        >,
        member: &SignatureSourceNameInput,
        class: &ClassDef,
    ) -> Option<OriginalSourceMethodMetadata> {
        reference
            .supports_member_association(&self.result)
            .then_some(())?;
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        (reference.source_procedure(&self.result)? == procedure).then_some(())?;
        let canonical = reference.class_metadata(&self.result)?;
        (class.source_name.is_none()
            && class.name_span == canonical.name_span
            && class.body_span == canonical.body_span)
            .then_some(())?;
        let whole = procedure.name_input().original_word();
        let children = SignatureSourceNameInput::OriginalWord(procedure.name_input().clone())
            .original_list_elements()?;
        let [class_input, member_input] = children.as_slice() else {
            return None;
        };
        (class_input == reference.class_input() && member_input == member).then_some(())?;
        let advice = reference.procedure_advice();
        let grammar = reference.class_grammar(&self.result)?;
        let purpose = grammar.two_word_procedure_member_name_purpose()?;
        let declaration = OriginalSourceMemberDeclaration::from_original_input(
            advice.site(),
            whole,
            member.clone(),
            OriginalSourceNamePurpose::for_analysis_input(
                &self.resolved_analysis_input(),
                self.source_analysis_entry.as_deref(),
                member.policy(),
            ),
        )?;
        let proc = procedure.metadata();
        let method = MethodDef {
            name: std::str::from_utf8(member.bytes()).ok()?.to_owned(),
            params: proc.params.clone(),
            params_computed: proc.params_computed,
            formal_count: proc.formal_count.clone(),
            name_span: proc.name_span,
            body_span: proc.body_span,
            kind: "method".to_owned(),
            is_self_method: false,
            visibility: "public".to_owned(),
            doc: proc.doc.clone(),
            forward_target: None,
        };
        let parameters = original_two_word_role_word(advice, ArgRole::ParamList)?;
        let body = original_two_word_role_word(advice, ArgRole::Body)?;
        OriginalSourceMethodMetadata::from_declaration(
            declaration,
            MemberSide::Instance,
            method,
            true,
            false,
            None,
            purpose,
        )?
        .with_body_role_words(Some(parameters), Some(body), Some(advice.dialect()))
    }

    fn original_member_source_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.original_source_invocation_dialect()
    }

    pub(super) fn retain_original_class_configuration_metadata(
        &mut self,
        target_span: tcl_lexer::Span,
    ) {
        self.retain_original_class_configuration_metadata_with_source(target_span, None);
    }

    pub(super) fn retain_original_class_configuration_metadata_with_source(
        &mut self,
        target_span: tcl_lexer::Span,
        source_class: Option<&crate::command_binding::OriginalSourceClassDeclaration>,
    ) {
        if let Some(configuration) = self.original_object_configuration_metadata(target_span) {
            self.result
                .original_object_configuration_metadata
                .push(configuration);
            return;
        }
        let configuration = self.original_class_configuration_metadata(target_span);
        let Some(configuration) = configuration else {
            // A selected source target retains its conditional authored ledger.
            // Missing allocation-backed configuration is not evidence that the
            // original class operand was unknown. The member walker supplies
            // original effects; this grants no entered worker or own-table state.
            if source_class
                .is_some_and(|declaration| declaration.source_class(&self.result).is_some())
            {
                return;
            }
            // An unowned target can affect any current class. Retained birth
            // declarations survive, while their effective own views withdraw.
            for declaration in &mut self.result.original_class_metadata {
                let mut class = declaration.metadata().clone();
                class.original_members.withdraw();
                class.original_relations.withdraw();
                class.original_properties.withdraw(MemberSide::Instance);
                class.original_properties.withdraw(MemberSide::ClassObject);
                if let Some(withdrawn) =
                    crate::signature_scan::original_name::SourceDeclarationMetadata::new(
                        declaration.original_occurrence(),
                        declaration.name().clone(),
                        class,
                    )
                {
                    *declaration = withdrawn;
                }
            }
            return;
        };
        if let Some(allocation) = configuration.target().local_allocation() {
            let matches = self
                .result
                .original_class_metadata
                .iter()
                .enumerate()
                .filter(|(_, declaration)| declaration.declaration_site() == &allocation.site)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if let [index] = matches.as_slice() {
                let declaration = &self.result.original_class_metadata[*index];
                let mut class = declaration.metadata().clone();
                class.original_members.absorb(configuration.members(), true);
                class
                    .original_special_members
                    .absorb(configuration.special_members(), true);
                class
                    .original_relations
                    .absorb(configuration.relations(), true);
                class
                    .original_properties
                    .absorb(configuration.properties(), true);
                if let Some(updated) =
                    crate::signature_scan::original_name::SourceDeclarationMetadata::new(
                        declaration.original_occurrence(),
                        declaration.name().clone(),
                        class,
                    )
                {
                    self.result.original_class_metadata[*index] = updated;
                }
            }
        }
        self.result
            .original_class_configuration_metadata
            .push(configuration);
    }

    fn original_class_configuration_metadata(
        &self,
        target_span: tcl_lexer::Span,
    ) -> Option<super::types::OriginalSourceClassConfiguration> {
        let registry = self.registry.as_deref()?;
        let bindings = self.head_identities.source_bindings_ref();
        let target = bindings.original_class_configuration_target_at_span(
            target_span,
            self.file_lexer_config(),
            registry,
        )?;
        let side = match target.layer() {
            tcl_registry::ObjectDispatchLayer::Class => MemberSide::Instance,
            tcl_registry::ObjectDispatchLayer::Object => MemberSide::ClassObject,
        };
        let (members, relations, properties, special_members) = self.original_configuration_delta(
            target.site(),
            target.registry_identity(),
            target.written_ordinal(),
            target.lookup().original_naming_scope().cloned(),
            side,
        )?;
        Some(
            super::types::OriginalSourceClassConfiguration::new(target, members, relations)
                .with_properties(properties)
                .with_special_members(special_members),
        )
    }

    fn original_object_configuration_metadata(
        &self,
        target_span: tcl_lexer::Span,
    ) -> Option<super::types::OriginalSourceObjectConfiguration> {
        let registry = self.registry.as_deref()?;
        let bindings = self.head_identities.source_bindings_ref();
        let target = bindings.original_object_configuration_target(
            target_span,
            self.file_lexer_config(),
            registry,
        )?;
        let (members, relations, properties, special_members) = self.original_configuration_delta(
            target.site(),
            target.registry_identity(),
            target.written_ordinal(),
            target.original_naming_scope().cloned(),
            MemberSide::ClassObject,
        )?;
        Some(
            super::types::OriginalSourceObjectConfiguration::new(target, members, relations)
                .with_properties(properties)
                .with_special_members(special_members),
        )
    }

    fn original_configuration_delta(
        &self,
        allocation_site: &CommandAllocationSite,
        registry_identity: &str,
        target_index: usize,
        caller_scope: Option<SignatureNamespaceScope>,
        side: MemberSide,
    ) -> Option<(
        super::types::OriginalSourceMemberLedger,
        super::types::OriginalSourceClassRelationLedger,
        super::types::OriginalSourcePropertyLedger,
        super::types::OriginalSourceSpecialMemberLedger,
    )> {
        let registry = self.registry.as_deref()?;
        let policy = self.declaration_name_policy()?;
        let grammar = registry.get(registry_identity)?.definition_body?;
        if !grammar.profile.has_native_definition_workers() {
            return None;
        }
        let binding = self
            .head_identities
            .source_bindings_ref()
            .invocation_at_source("", allocation_site.offset);
        let (command, tokens) = binding.original_recorded_command()?;
        let origin = &allocation_site.source;
        let native = crate::registry_invocation::original_native_compiler_words(
            origin.source_image(),
            tokens.words(),
            allocation_site.offset,
            self.file_lexer_config(),
        )?;
        let first_member = target_index.checked_add(1)?;
        let words = native.get(first_member..)?;
        let texts = tokens.argv_texts.get(first_member..)?;
        let arg_tokens = command.argv.get(first_member..)?;
        let analysis_context = self.analysis_context();
        let dialect = super::oo::MemberDialect {
            surface: Some(analysis_context.context().authoring_query()),
            rules: self.word_rules(),
        };
        let mut producer = Producer {
            grammar,
            origin,
            dialect,
            policy,
            name_purpose: OriginalSourceNamePurpose::for_analysis_input(
                &self.resolved_analysis_input(),
                self.source_analysis_entry.as_deref(),
                policy,
            ),
            resolved_input: self.resolved_analysis_input(),
            source_dialect: self.original_member_source_dialect(),
            bindings: self.head_identities.source_bindings_ref(),
            registry: self.registry.as_deref(),
            caller_scope,
            readonly_names: Vec::new(),
            source_installer: None,
            class_relation_body: self.original_class_definition_body.as_deref(),
        };
        let mut delta = ClassDef::default();
        let inline = words
            .first()
            .and_then(|word| producer.key(word))
            .and_then(|key| key.display().and_then(|name| grammar.member(name)))
            .is_some();
        let complete = if inline {
            producer.member(
                MemberCommand::new(words, texts, arg_tokens),
                side,
                None,
                &mut delta,
                0,
            )
        } else {
            producer.configuration_body(words, side, &mut delta, self.file_lexer_config())
        };
        if complete.is_none() {
            delta.original_members.withdraw();
            delta.original_relations.withdraw();
            delta.original_properties.withdraw(side);
        }
        Some((
            delta.original_members,
            delta.original_relations,
            delta.original_properties,
            delta.original_special_members,
        ))
    }

    pub(super) fn retain_original_member_metadata(
        &self,
        grammar: &DefinitionBodyGrammar,
        command: &crate::segmenter::SegmentedCommand,
        class: &mut ClassDef,
        scope_path: &[usize],
    ) {
        self.retain_logical_member_name_operand(grammar, command, class);
        self.retain_logical_member_visibility_operands(grammar, command, class);
        let Some(policy) = self.declaration_name_policy() else {
            class.original_members.withdraw();
            class.original_relations.withdraw();
            class.original_properties.withdraw(MemberSide::Instance);
            class.original_properties.withdraw(MemberSide::ClassObject);
            return;
        };
        let bindings = self.head_identities.source_bindings_ref();
        let Some(origin) = bindings.source_origin() else {
            class.original_members.withdraw();
            class.original_relations.withdraw();
            class.original_properties.withdraw(MemberSide::Instance);
            class.original_properties.withdraw(MemberSide::ClassObject);
            return;
        };
        let image = origin.source_image();
        if image != &SourceImage::document(&self.source) {
            class.original_members.withdraw();
            class.original_relations.withdraw();
            class.original_properties.withdraw(MemberSide::Instance);
            class.original_properties.withdraw(MemberSide::ClassObject);
            return;
        }
        let config = self.file_lexer_config();
        let tokens = CommandTokens::from_segmented(&SourceMap::from_image(image), config, command);
        let Some(native) = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            command.span.start(),
            config,
        ) else {
            class.original_members.withdraw();
            class.original_relations.withdraw();
            class.original_properties.withdraw(MemberSide::Instance);
            class.original_properties.withdraw(MemberSide::ClassObject);
            return;
        };
        let analysis_context = self.analysis_context();
        let dialect = super::oo::MemberDialect {
            surface: Some(analysis_context.context().authoring_query()),
            rules: self.word_rules(),
        };
        let mut producer = Producer {
            grammar,
            origin,
            dialect,
            policy,
            name_purpose: OriginalSourceNamePurpose::for_analysis_input(
                &self.resolved_analysis_input(),
                self.source_analysis_entry.as_deref(),
                policy,
            ),
            resolved_input: self.resolved_analysis_input(),
            source_dialect: self.original_member_source_dialect(),
            bindings,
            registry: self.registry.as_deref(),
            caller_scope: self.declaration_namespace_scope(scope_path),
            readonly_names: Vec::new(),
            source_installer: None,
            class_relation_body: self.original_class_definition_body.as_deref(),
        };
        if producer
            .member(
                MemberCommand::new(&native, &command.texts, &command.argv),
                MemberSide::Instance,
                None,
                class,
                0,
            )
            .is_none()
        {
            #[cfg(debug_assertions)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
                eprintln!(
                    "ORIGINAL_MEMBER_DECLINED offset={} head={:?}",
                    command.span.start(),
                    native
                        .first()
                        .and_then(|word| producer.key(word))
                        .map(|key| key.bytes().to_vec())
                );
            }
            class.original_members.withdraw();
            class.original_relations.withdraw();
            class.original_properties.withdraw(MemberSide::Instance);
            class.original_properties.withdraw(MemberSide::ClassObject);
        }
    }

    /// Inline members use the same immutable original command layout inventory;
    /// the compatibility argument vector never supplies a source word itself.
    pub(super) fn retain_original_inline_member_metadata(
        &self,
        grammar: &DefinitionBodyGrammar,
        texts: &[String],
        tokens: &[Token],
        class: &mut ClassDef,
        scope_path: &[usize],
    ) {
        let Some(policy) = self.declaration_name_policy() else {
            class.original_members.withdraw();
            class.original_relations.withdraw();
            class.original_properties.withdraw(MemberSide::Instance);
            class.original_properties.withdraw(MemberSide::ClassObject);
            return;
        };
        let bindings = self.head_identities.source_bindings_ref();
        let Some(origin) = bindings.source_origin() else {
            class.original_members.withdraw();
            class.original_relations.withdraw();
            class.original_properties.withdraw(MemberSide::Instance);
            class.original_properties.withdraw(MemberSide::ClassObject);
            return;
        };
        let native = if self.selected_logical_declaration_advice() {
            self.original_logical_inline_member_words(texts, tokens)
        } else {
            tokens
                .iter()
                .map(|token| {
                    bindings
                        .original_source_name_at_span(
                            token.span,
                            self.file_lexer_config(),
                            self.word_rules(),
                            policy,
                        )
                        .map(|original| original.name_input().original_word().clone())
                })
                .collect::<Option<Vec<_>>>()
        };
        let Some(native) = native else {
            class.original_members.withdraw();
            class.original_relations.withdraw();
            class.original_properties.withdraw(MemberSide::Instance);
            class.original_properties.withdraw(MemberSide::ClassObject);
            return;
        };
        let analysis_context = self.analysis_context();
        let dialect = super::oo::MemberDialect {
            surface: Some(analysis_context.context().authoring_query()),
            rules: self.word_rules(),
        };
        let mut producer = Producer {
            grammar,
            origin,
            dialect,
            policy,
            name_purpose: OriginalSourceNamePurpose::for_analysis_input(
                &self.resolved_analysis_input(),
                self.source_analysis_entry.as_deref(),
                policy,
            ),
            resolved_input: self.resolved_analysis_input(),
            source_dialect: self.original_member_source_dialect(),
            bindings,
            registry: self.registry.as_deref(),
            caller_scope: self.declaration_namespace_scope(scope_path),
            readonly_names: Vec::new(),
            source_installer: None,
            class_relation_body: self.original_class_definition_body.as_deref(),
        };
        if producer
            .member(
                MemberCommand::new(&native, texts, tokens),
                MemberSide::Instance,
                None,
                class,
                0,
            )
            .is_none()
        {
            class.original_members.withdraw();
            class.original_relations.withdraw();
            class.original_properties.withdraw(MemberSide::Instance);
            class.original_properties.withdraw(MemberSide::ClassObject);
        }
    }

    /// Inline arguments must be the actual suffix of one retained Logical
    /// command. Individual source words cannot assemble a new member vector.
    fn original_logical_inline_member_words(
        &self,
        texts: &[String],
        tokens: &[Token],
    ) -> Option<Vec<NativeWord>> {
        self.selected_logical_declaration_advice().then_some(())?;
        let input = self.result.resolved_input.as_ref()?;
        let image = SourceImage::document(&self.source);
        if !self
            .result
            .matches_original_source_image(&image, input.lexer_config())
            || !self.head_identities.matches_resolved_analysis_input(input)
        {
            return None;
        }
        let (_, offset) = self
            .head_identities
            .original_written_word_at_span_in_source(
                &image,
                tokens.first()?.span,
                input.lexer_config(),
            )?;
        let original = self
            .head_identities
            .source_bindings_ref()
            .original_logical_source_command_at_in_source(
                input,
                &image,
                offset,
                input.lexer_config(),
            )?;
        let first = original.native_words().len().checked_sub(tokens.len())?;
        if original.command().argv.get(first..) != Some(tokens)
            || original.command().texts.get(first..) != Some(texts)
        {
            return None;
        }
        Some(original.native_words().get(first..)?.to_vec())
    }
}

#[derive(Clone, Copy)]
struct MemberCommand<'a> {
    words: &'a [NativeWord],
    texts: &'a [String],
    tokens: &'a [Token],
}

impl<'a> MemberCommand<'a> {
    const fn new(words: &'a [NativeWord], texts: &'a [String], tokens: &'a [Token]) -> Self {
        Self {
            words,
            texts,
            tokens,
        }
    }
}

struct Producer<'a> {
    grammar: &'a DefinitionBodyGrammar,
    origin: &'a Arc<SourceOriginId>,
    dialect: super::oo::MemberDialect<'a>,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    name_purpose: OriginalSourceNamePurpose,
    resolved_input: crate::analyser::ResolvedAnalysisInput,
    source_dialect: Option<tcl_registry::InvocationDialect>,
    caller_scope: Option<SignatureNamespaceScope>,
    bindings: &'a crate::command_binding::SourceCommandBindings,
    readonly_names: Vec<(tcl_lexer::Span, SignatureSourceNameInput)>,
    source_installer: Option<Arc<crate::command_binding::OriginalCatalogueSourceCandidate>>,
    registry: Option<&'a tcl_registry::CommandRegistry>,
    class_relation_body: Option<&'a crate::registry_invocation::OriginalSourceScriptBody>,
}

struct LiteralInstaller {
    installer: Arc<crate::command_binding::OriginalCatalogueSourceCandidate>,
    variable: SignatureSourceNameValue,
    names: Vec<SignatureSourceNameValue>,
    body: SignatureSourceNameKey,
}

#[derive(Clone, Copy)]
struct MemberFlags {
    side: MemberSide,
    exported: bool,
    delegate: bool,
}

struct MethodParts {
    method: MethodDef,
    forward_prefix: Option<Vec<SignatureSourceNameInput>>,
    parameters: Option<NativeWord>,
    body: Option<NativeWord>,
}

impl Producer<'_> {
    fn key(&self, word: &NativeWord) -> Option<SignatureSourceNameKey> {
        self.name_purpose.key(word, self.dialect.rules)
    }
    fn input(&self, word: &NativeWord) -> Option<SignatureSourceNameInput> {
        if let Some((_, input)) = self
            .readonly_names
            .iter()
            .rev()
            .find(|(span, _)| *span == word.span())
        {
            return Some(input.clone());
        }
        if let Some(key) = self.key(word) {
            return Some(SignatureSourceNameInput::OriginalWord(key));
        }
        let input = self
            .bindings
            .original_written_name_input_at_span_in_source(
                word.image(),
                word.span(),
                word.config(),
            )?;
        (input.policy() == self.policy).then_some(input)
    }
    fn literal_installer(
        &self,
        words: &[NativeWord],
        checkpoint: &mut &'static str,
    ) -> Option<LiteralInstaller> {
        let head = self.key(words.first()?)?;
        let binding = self
            .bindings
            .invocation_at_source("", head.original_word().group().span.start());
        // The source definition walk retains a conditional installer schema;
        // runtime uncertainty does not become a selected handler or execution.
        // The shared catalogue owner independently keeps known shadow barriers.
        *checkpoint = "registry";
        let registry = self.registry?;
        *checkpoint = "recorded-command";
        let (command, mut original_tokens) = binding.original_recorded_command()?;
        if command.span.start() != head.original_word().group().span.start() {
            return None;
        }
        original_tokens.source_binding = Some(binding.clone());
        *checkpoint = "head-occurrence";
        let occurrence =
            SourceOriginalNameOccurrence::new(binding.invocation_site()?, head.clone())?;
        *checkpoint = "catalogue-schema";
        let installer = binding.original_catalogue_source_candidate(
            &original_tokens,
            &occurrence,
            self.caller_scope.as_ref(),
            registry,
        )?;
        *checkpoint = "vector-correspondence";
        if installer.original_words() != words {
            return None;
        }
        *checkpoint = "descriptor";
        let spec = registry.get_for_surface(&installer.candidate().name, self.dialect.surface)?;
        let installer = Arc::new(installer);
        *checkpoint = "loop-layout";
        let (variable, list, body) =
            super::oo::loop_installer_pair(spec, words.len().checked_sub(1)?)?;
        *checkpoint = "variable-list";
        let variable = self.key(words.get(variable + 1)?)?;
        let variable = SignatureSourceNameValue::from_original_static_word(
            variable.original_word(),
            self.dialect.rules,
            self.policy,
        )?;
        if variable.list_length()? != 1 {
            return None;
        }
        let variable = variable.list_element(0)?;
        if self
            .policy
            .recipe()
            .combined_variable_input(variable.bytes())
            .element()
            .is_some()
        {
            return None;
        }
        *checkpoint = "value-list";
        let list = self.key(words.get(list + 1)?)?;
        let list = SignatureSourceNameValue::from_original_static_word(
            list.original_word(),
            self.dialect.rules,
            self.policy,
        )?;
        let names = list.list_elements()?;
        *checkpoint = "body-source";
        let body = self.key(words.get(body + 1)?)?;
        Some(LiteralInstaller {
            installer,
            variable,
            names,
            body,
        })
    }

    fn loop_member_name<'a>(
        &self,
        native: &'a [NativeWord],
        variable: &SignatureSourceNameValue,
        image: &SourceImage,
        arguments: &[String],
        checkpoint: &mut &'static str,
    ) -> Option<&'a NativeWord> {
        *checkpoint = "worker-layout";
        let member = self.grammar.member(self.key(native.first()?)?.display()?)?;
        member
            .indices_for_call_in(arguments, self.dialect.surface, ArgRole::Body)
            .next()?;
        let ordinal = member
            .indices_for_call_in(arguments, self.dialect.surface, ArgRole::Name)
            .next()?
            .checked_add(1)?;
        *checkpoint = "variable-reference";
        let name_word = native.get(ordinal)?;
        let arena = name_word.executable_parts();
        let [part] = arena.list(arena.root()) else {
            return None;
        };
        let tcl_lexer::ExecutablePart::Variable { name, index: None } = &part.part else {
            return None;
        };
        let root = tcl_syntax::backslash::native_source_literal_bytes(
            arena.bytes(*name)?,
            image.channel(),
            self.policy.string_protocol(),
        )
        .ok()?;
        if root.as_ref() != variable.bytes() {
            return None;
        }
        Some(name_word)
    }

    fn literal_loop(
        &mut self,
        command: MemberCommand<'_>,
        side: MemberSide,
        visibility: Option<DeclaredMemberVisibility>,
        class: &mut ClassDef,
        depth: usize,
    ) -> Option<()> {
        let MemberCommand {
            words,
            texts,
            tokens,
        } = command;
        let mut checkpoint = "head";
        let retained = (|| {
            let LiteralInstaller {
                installer,
                variable,
                names,
                body,
            } = self.literal_installer(words, &mut checkpoint)?;
            let region = body.original_word().content_span().ok()?;
            let image = self.origin.source_image();
            if image.bytes().get(region.as_range())? != body.bytes() {
                return None;
            }
            let selected = SourceImage::from_bytes(body.bytes(), image.channel());
            checkpoint = "body-segmentation";
            let commands = crate::segmenter::segment_commands_image_with_offset_and_config(
                &selected,
                region.start(),
                body.lexer_config().nested(),
            )?;
            let [command] = commands.as_slice() else {
                return None;
            };
            if command.is_partial {
                return None;
            }
            let command_tokens = CommandTokens::from_segmented(
                &SourceMap::from_image(image),
                body.lexer_config(),
                command,
            );
            checkpoint = "worker-vector";
            let native = crate::registry_invocation::original_native_compiler_words(
                image,
                command_tokens.words(),
                command.span.start(),
                body.lexer_config(),
            )?;
            let name_word = self.loop_member_name(
                &native,
                &variable,
                image,
                command.texts.get(1..)?,
                &mut checkpoint,
            )?;
            checkpoint = "readonly-member";
            let before = self.readonly_names.len();
            for name in names {
                self.readonly_names.push((
                    name_word.span(),
                    SignatureSourceNameInput::OriginalValue(name),
                ));
                let previous = self.source_installer.replace(Arc::clone(&installer));
                let retained = self.member(
                    MemberCommand::new(&native, &command.texts, &command.argv),
                    side,
                    visibility,
                    class,
                    depth + 1,
                );
                self.source_installer = previous;
                self.readonly_names.truncate(before);
                retained?;
            }
            // Whole vector lengths remain an independent source-layout obligation.
            (words.len() == texts.len() && words.len() == tokens.len()).then_some(())
        })();
        if cfg!(debug_assertions)
            && retained.is_none()
            && tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some()
        {
            eprintln!(
                "ORIGINAL_MEMBER_INSTALLER_DECLINED offset={:?} stage={checkpoint} caller_scope={:?} words={}",
                words.first().map(|word| word.span().start()),
                self.caller_scope,
                words.len()
            );
        }
        retained
    }

    fn member(
        &mut self,
        command: MemberCommand<'_>,
        side: MemberSide,
        wrapper_visibility: Option<DeclaredMemberVisibility>,
        class: &mut ClassDef,
        depth: usize,
    ) -> Option<()> {
        let MemberCommand {
            words,
            texts,
            tokens,
        } = command;
        if depth >= 32 || words.len() != texts.len() || words.len() != tokens.len() {
            return None;
        }
        let head = self.key(words.first()?)?;
        let Some(member) = self.grammar.member(head.display()?) else {
            return self.literal_loop(command, side, wrapper_visibility, class, depth);
        };
        if member.surface.is_some_and(|surface| {
            !tcl_dialect::model::surface_admits(surface, self.dialect.surface.as_ref())
        }) {
            return Some(());
        }
        let args = texts.get(1..)?;
        let arg_tokens = tokens.get(1..)?;
        let arg_words = words.get(1..)?;
        if member
            .unavailable_option_for(args, self.dialect.surface)
            .is_some()
        {
            return Some(());
        }
        let allocation_site = CommandAllocationSite {
            source: Arc::clone(self.origin),
            offset: head.original_word().group().span.start(),
        };
        if member.kind == MemberKind::Wrapper {
            return self.wrapper_member(
                member,
                MemberCommand::new(arg_words, args, arg_tokens),
                side,
                wrapper_visibility,
                class,
                depth,
            );
        }
        if self.relation_member(member, words, arg_words, &allocation_site, side, class)? {
            return Some(());
        }
        if member.retraction.is_some() {
            return self.retract_member(member, arg_words, allocation_site, side, class);
        }
        if let Some(visibility) = member.visibility_effect {
            let inputs = arg_words
                .iter()
                .map(|word| self.input(word))
                .collect::<Option<Vec<_>>>()?;
            if !inputs.is_empty() {
                class
                    .original_members
                    .effect(OriginalSourceMemberEffect::new(
                        allocation_site,
                        side,
                        OriginalSourceMemberEffectKind::Visibility(
                            visibility == MemberVisibility::Exported,
                        ),
                        inputs,
                        self.grammar.member_name_purpose(member)?,
                    )?);
            }
            return Some(());
        }
        if member.kind == MemberKind::FlagKeyed {
            self.property_member(arg_words, &allocation_site, side, class);
            return Some(());
        }
        if self.grammar.source_special_member_kind(member).is_some() {
            return self.special_member(member, command, allocation_site, side, class);
        }
        self.ordinary_member(
            member,
            command,
            &allocation_site,
            side,
            wrapper_visibility,
            class,
        )
    }

    fn retract_member(
        &self,
        member: &MemberSpec,
        arg_words: &[NativeWord],
        allocation_site: CommandAllocationSite,
        side: MemberSide,
        class: &mut ClassDef,
    ) -> Option<()> {
        let retraction = member.retraction?;

        let (from, to) = retraction.argument_indices(arg_words.len());
        let mut inputs = from
            .map(|index| self.input(&arg_words[index]))
            .collect::<Option<Vec<_>>>()?;
        let kind = if let Some(to) = to {
            inputs.push(self.input(&arg_words[to])?);
            OriginalSourceMemberEffectKind::Move
        } else {
            OriginalSourceMemberEffectKind::Delete
        };
        if inputs.is_empty() {
            return None;
        }
        class
            .original_members
            .effect(OriginalSourceMemberEffect::new(
                allocation_site,
                side,
                kind,
                inputs,
                self.grammar.member_name_purpose(member)?,
            )?);
        Some(())
    }

    fn relation_member(
        &self,
        member: &MemberSpec,
        original: &[NativeWord],
        arg_words: &[NativeWord],
        allocation_site: &CommandAllocationSite,
        side: MemberSide,
        class: &mut ClassDef,
    ) -> Option<bool> {
        if let Some(slot) = member.slot {
            // These are the analyser's class-field routes. The selected
            // Registry member owns layout and slot behavior; keyword reports
            // never select or reconstruct an operand's bytes.
            let relation = match member.effect {
                tcl_registry::definer::MemberEffect::Relation {
                    slot: tcl_registry::definer::RelationSlot::Superclass,
                } if side == MemberSide::Instance => {
                    Some(OriginalSourceClassRelationKind::Superclass)
                }
                tcl_registry::definer::MemberEffect::Relation {
                    slot: tcl_registry::definer::RelationSlot::Mixin,
                } => Some(OriginalSourceClassRelationKind::Mixin),
                _ => None,
            };
            if let Some(kind) = relation {
                if arg_words.is_empty() {
                    return Some(true);
                }
                let inputs = arg_words
                    .iter()
                    .map(|word| self.input(word))
                    .collect::<Option<Vec<_>>>();
                let selected = inputs.as_ref().and_then(|inputs| {
                    let bytes = inputs
                        .iter()
                        .map(SignatureSourceNameInput::bytes)
                        .collect::<Vec<_>>();
                    let (operation, values) =
                        slot.split_original_call(&bytes, self.policy.recipe())?;
                    Some((operation, inputs.len() - values.len()))
                });
                let (operation, from) =
                    selected.map_or((None, 0), |(operation, from)| (Some(operation), from));
                let values = arg_words
                    .get(from..)?
                    .iter()
                    .map(|word| {
                        let input = self.input(word)?;
                        let scope = if tcl_syntax::naming::is_absolute_bytes(input.bytes()) {
                            SignatureNamespaceScope::root(Some(input.policy()))
                        } else {
                            self.caller_scope.clone()?
                        };
                        let lookup = SignatureSourceLookup::from_input(scope, &input)?;
                        let provider = self.class_relation_body.and_then(|body| {
                            self.bindings.original_class_relation_provider(
                                body,
                                original,
                                &input,
                                &lookup,
                                self.registry?,
                            )
                        });
                        OriginalSourceClassRelation::new(input, lookup)?.with_provider(provider)
                    })
                    .collect::<Option<Vec<_>>>();
                class
                    .original_relations
                    .effect(OriginalSourceClassRelationEffect::new(
                        allocation_site.clone(),
                        side,
                        kind,
                        slot,
                        operation,
                        values,
                    ));
                return Some(true);
            }
        }
        Some(false)
    }

    fn special_member(
        &self,
        member: &MemberSpec,
        command: MemberCommand<'_>,
        allocation_site: CommandAllocationSite,
        side: MemberSide,
        class: &mut ClassDef,
    ) -> Option<()> {
        let MemberCommand {
            words,
            texts,
            tokens,
        } = command;
        let args = texts.get(1..)?;
        let arg_words = words.get(1..)?;
        let arg_tokens = tokens.get(1..)?;
        let kind = self.grammar.source_special_member_kind(member)?;

        let body_index = member
            .indices_for_call_in(args, self.dialect.surface, ArgRole::Body)
            .next()?;
        let parameters = match member
            .indices_for_call_in(args, self.dialect.surface, ArgRole::ParamList)
            .next()
        {
            Some(index) => Some(arg_words.get(index)?.clone()),
            None => None,
        };
        let body = arg_words.get(body_index)?.clone();
        let mut metadata = super::oo::extract_method_def_in(
            member,
            args,
            arg_tokens,
            member.keyword,
            "public",
            "",
            self.dialect,
        )?;
        metadata.formal_count =
            self.member_formal_count(parameters.as_ref(), metadata.params_computed);
        metadata.name_span = words.first()?.span();
        metadata.body_span = body.span();
        metadata.is_self_method = side == MemberSide::ClassObject;
        class
            .original_special_members
            .declare(OriginalSourceSpecialMemberMetadata::new(
                allocation_site,
                words.first()?.clone(),
                parameters,
                body,
                side,
                kind,
                metadata,
            )?);
        Some(())
    }

    fn ordinary_member(
        &self,
        member: &MemberSpec,
        command: MemberCommand<'_>,
        allocation_site: &CommandAllocationSite,
        side: MemberSide,
        wrapper_visibility: Option<DeclaredMemberVisibility>,
        class: &mut ClassDef,
    ) -> Option<()> {
        let MemberCommand { words, texts, .. } = command;
        let args = texts.get(1..)?;
        let arg_words = words.get(1..)?;
        let Some(name_index) = member
            .indices_for_call_in(args, self.dialect.surface, ArgRole::Name)
            .next()
        else {
            return Some(());
        };
        // Only the Registry-selected optional position supplies its flag.
        // The independently produced name may be a readonly list child.
        if let Some(option) = member.option_for_in(args, self.dialect.surface) {
            let position = usize::from(member.optional_argument?.position);
            let input = self.input(arg_words.get(position)?)?;
            if input.bytes() != option.value.as_bytes() {
                return None;
            }
        }
        let name_word = arg_words.get(name_index)?;
        let name = self.input(name_word)?;
        let occurrence = OriginalSourceMemberDeclaration::from_original_input(
            allocation_site,
            name_word,
            name.clone(),
            self.name_purpose,
        )?
        .with_source_installer(self.source_installer.clone())?;
        let flags = self.method_member_flags(member, args, name.bytes(), side, wrapper_visibility);
        let MemberFlags {
            side,
            exported,
            delegate,
        } = flags;
        if member
            .indices_for_call_in(args, self.dialect.surface, ArgRole::Body)
            .next()
            .is_none()
            && member
                .indices_for_call_in(args, self.dialect.surface, ArgRole::CommandName)
                .next()
                .is_none()
        {
            return Some(());
        }
        let MethodParts {
            method,
            forward_prefix,
            parameters,
            body,
        } = self.method_parts(member, command, &occurrence, flags)?;
        if delegate {
            // Registry owns the two products of classmethod: the class-side
            // delegate and the instance-side forward. Neither is a native
            // allocation grant; consumers retain their delegate role.
            class.original_members.declare(
                OriginalSourceMethodMetadata::from_declaration(
                    occurrence.clone(),
                    MemberSide::Instance,
                    method.clone(),
                    exported,
                    true,
                    None,
                    self.grammar.member_name_purpose(member)?,
                )?
                .with_body_role_words(parameters.clone(), body.clone(), self.source_dialect)?
                .with_definition_parent(self.class_relation_body)?,
            );
        }
        class.original_members.declare(
            OriginalSourceMethodMetadata::from_declaration(
                occurrence,
                side,
                method,
                exported,
                delegate,
                forward_prefix,
                self.grammar.member_name_purpose(member)?,
            )?
            .with_body_role_words(parameters, body, self.source_dialect)?
            .with_definition_parent(self.class_relation_body)?,
        );
        Some(())
    }

    fn method_member_flags(
        &self,
        member: &MemberSpec,
        args: &[String],
        name_bytes: &[u8],
        side: MemberSide,
        wrapper_visibility: Option<DeclaredMemberVisibility>,
    ) -> MemberFlags {
        let delegate = self
            .grammar
            .native_classmethod_declaration(member, self.dialect.surface);
        // Registry type-object methods live in the class object's own table.
        // Itcl class-scoped procs remain reachable only through `Class::proc`.
        let type_object_method = !self.grammar.profile.class_procedures_are_qualified()
            && matches!(
                member.effect,
                tcl_registry::definer::MemberEffect::Callable {
                    receiver: tcl_registry::definer::MemberReceiver::TypeObject,
                    role: tcl_registry::definer::CallableRole::Method,
                    ..
                }
            );
        let side = if delegate || type_object_method {
            MemberSide::ClassObject
        } else {
            side
        };
        let exported = member
            .declared_visibility_for_in(args, self.dialect.surface)
            .or(wrapper_visibility)
            .map_or_else(
                || self.grammar.member_default_exported_bytes(name_bytes),
                |visibility| visibility == DeclaredMemberVisibility::Public,
            );
        MemberFlags {
            side,
            exported,
            delegate,
        }
    }

    fn member_formal_count(
        &self,
        parameters: Option<&tcl_lexer::NativeWord>,
        computed: bool,
    ) -> crate::signature_scan::formal_count::SourceFormalCount {
        use crate::signature_scan::formal_count::SourceFormalCount;
        if computed {
            return SourceFormalCount::Unknown;
        }
        match parameters {
            None => SourceFormalCount::Authored(tcl_dialect::ParameterGrammar::Tcl),
            Some(word) => self
                .source_dialect
                .map_or(SourceFormalCount::Unknown, |dialect| {
                    SourceFormalCount::from_original_word(word, dialect)
                }),
        }
    }

    fn method_parts(
        &self,
        member: &MemberSpec,
        command: MemberCommand<'_>,
        occurrence: &OriginalSourceMemberDeclaration,
        flags: MemberFlags,
    ) -> Option<MethodParts> {
        let MemberCommand {
            words,
            texts,
            tokens,
        } = command;
        let args = texts.get(1..)?;
        let arg_tokens = tokens.get(1..)?;
        let arg_words = words.get(1..)?;
        let MemberFlags {
            side,
            exported,
            delegate,
        } = flags;
        let name = occurrence.original_name_input();
        let name_index = member
            .indices_for_call_in(args, self.dialect.surface, ArgRole::Name)
            .next()?;
        let mut forward_prefix = None;
        let mut method = if member
            .indices_for_call_in(args, self.dialect.surface, ArgRole::Body)
            .next()
            .is_some()
        {
            let mut method = super::oo::extract_method_def_in(
                member,
                args,
                arg_tokens,
                member.keyword,
                if exported { "public" } else { "unexported" },
                "",
                self.dialect,
            )?;
            method.is_self_method = side == MemberSide::ClassObject && !delegate;
            if occurrence.static_occurrence().is_none() {
                std::str::from_utf8(name.bytes())
                    .unwrap_or_default()
                    .clone_into(&mut method.name);
                method.params.clear();
                method.params_computed = true;
            }
            method
        } else {
            let target = member
                .indices_for_call_in(args, self.dialect.surface, ArgRole::CommandName)
                .next()?;
            forward_prefix = Some(
                arg_words
                    .get(target..)?
                    .iter()
                    .map(|word| self.input(word))
                    .collect::<Option<Vec<_>>>()?,
            );
            MethodDef {
                name: args.get(name_index)?.clone(),
                name_span: arg_tokens.get(name_index)?.span,
                body_span: arg_tokens.get(name_index)?.span,
                params: Vec::new(),
                params_computed: false,
                formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                    tcl_dialect::ParameterGrammar::Tcl,
                ),
                kind: member.keyword.to_owned(),
                visibility: if exported { "public" } else { "unexported" }.to_owned(),
                is_self_method: side == MemberSide::ClassObject,
                doc: String::new(),
                forward_target: None,
            }
        };
        let parameters = match member
            .indices_for_call_in(args, self.dialect.surface, ArgRole::ParamList)
            .next()
        {
            Some(index) => Some(arg_words.get(index)?.clone()),
            None => None,
        };
        method.formal_count = self.member_formal_count(parameters.as_ref(), method.params_computed);
        let body = match member
            .indices_for_call_in(args, self.dialect.surface, ArgRole::Body)
            .next()
        {
            Some(index) => Some(arg_words.get(index)?.clone()),
            None => None,
        };
        Some(MethodParts {
            method,
            forward_prefix,
            parameters,
            body,
        })
    }

    fn wrapper_member(
        &mut self,
        member: &MemberSpec,
        command: MemberCommand<'_>,
        side: MemberSide,
        wrapper_visibility: Option<DeclaredMemberVisibility>,
        class: &mut ClassDef,
        depth: usize,
    ) -> Option<()> {
        let MemberCommand {
            words: arg_words,
            texts: args,
            tokens: arg_tokens,
        } = command;
        let side = if self.grammar.is_class_receiver_wrapper(member) {
            MemberSide::ClassObject
        } else {
            side
        };
        let visibility = self
            .grammar
            .wrapper_declared_visibility(member)
            .or(wrapper_visibility);
        if arg_words
            .first()
            .and_then(|word| self.key(word))
            .and_then(|key| {
                key.display()
                    .and_then(|keyword| self.grammar.member(keyword))
            })
            .is_some()
        {
            return self.member(
                MemberCommand::new(arg_words, args, arg_tokens),
                side,
                visibility,
                class,
                depth + 1,
            );
        }
        if !member.wrapper_block_body || arg_words.len() != 1 {
            return None;
        }
        let body = self.key(&arg_words[0])?;
        let region = body.original_word().content_span().ok()?;
        let image = self.origin.source_image();
        // Decoded/materialised scripts need their own source mapping. An
        // unchanged literal is the only block projection issued here.
        if image.bytes().get(region.as_range())? != body.bytes() {
            return None;
        }
        let selected = SourceImage::from_bytes(body.bytes(), image.channel());
        let commands = crate::segmenter::segment_commands_image_with_offset_and_config(
            &selected,
            region.start(),
            body.lexer_config().nested(),
        )?;
        for command in commands {
            if command.is_partial {
                return None;
            }
            let tokens = CommandTokens::from_segmented(
                &SourceMap::from_image(image),
                body.lexer_config(),
                &command,
            );
            let native = crate::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                command.span.start(),
                body.lexer_config(),
            )?;
            self.member(
                MemberCommand::new(&native, &command.texts, &command.argv),
                side,
                visibility,
                class,
                depth + 1,
            )?;
        }
        Some(())
    }

    fn configuration_body(
        &mut self,
        words: &[NativeWord],
        side: MemberSide,
        class: &mut ClassDef,
        config: tcl_lexer::LexerConfig,
    ) -> Option<()> {
        let [word] = words else {
            return None;
        };
        let body = self.key(word)?;
        let region = word.content_span().ok()?;
        if self.origin.source_image().bytes().get(region.as_range())? != body.bytes() {
            return None;
        }
        let image = SourceImage::from_bytes(body.bytes(), self.origin.source_image().channel());
        for command in crate::segmenter::segment_commands_image_with_offset_and_config(
            &image,
            region.start(),
            config.nested(),
        )? {
            if command.is_partial {
                return None;
            }
            let tokens = CommandTokens::from_segmented(
                &SourceMap::from_image(self.origin.source_image()),
                config,
                &command,
            );
            let native = crate::registry_invocation::original_native_compiler_words(
                self.origin.source_image(),
                tokens.words(),
                command.span.start(),
                config,
            )?;
            self.member(
                MemberCommand::new(&native, &command.texts, &command.argv),
                side,
                None,
                class,
                0,
            )?;
        }
        Some(())
    }

    fn property_member(
        &self,
        arg_words: &[NativeWord],
        allocation_site: &CommandAllocationSite,
        side: MemberSide,
        class: &mut ClassDef,
    ) {
        let declarations = (|| {
            let inputs = arg_words
                .iter()
                .map(|word| self.input(word))
                .collect::<Option<Vec<_>>>()?;
            let values = inputs
                .iter()
                .map(SignatureSourceNameInput::bytes)
                .collect::<Vec<_>>();
            let layouts = self
                .grammar
                .source_property_declarations_bytes(&values, self.dialect.surface)?;
            let mut declarations = Vec::new();
            for layout in layouts {
                let name = self.key(arg_words.get(layout.name_index())?)?;
                let declaration = SourceOriginalNameOccurrence::new(allocation_site, name.clone())?;
                let getter = match layout.getter_index() {
                    Some(index) => Some(inputs.get(index)?.clone()),
                    None => None,
                };
                let setter = match layout.setter_index() {
                    Some(index) => Some(inputs.get(index)?.clone()),
                    None => None,
                };
                let metadata = PropertyDef {
                    name: name.display().unwrap_or_default().to_owned(),
                    name_span: name.span(),
                    kind: layout.kind().name().to_owned(),
                    has_getter: getter.is_some(),
                    has_setter: setter.is_some(),
                };
                declarations.push(OriginalSourcePropertyMetadata::new(
                    declaration,
                    side,
                    metadata,
                    layout.kind(),
                    getter,
                    setter,
                    (
                        self.grammar
                            .source_property_accessor_advice(self.policy.recipe()),
                        self.resolved_input.clone(),
                    ),
                )?);
            }
            Some(declarations)
        })();
        if let Some(declarations) = declarations {
            for declaration in declarations {
                class.original_properties.declare(declaration);
            }
        } else {
            class.original_properties.withdraw(side);
        }
    }
}

fn original_two_word_role_word(
    advice: &crate::command_binding::OriginalSourceCommandTransitionAdvice,
    role: ArgRole,
) -> Option<NativeWord> {
    let mut ordinals = advice
        .roles()?
        .iter()
        .filter(|(_, selected)| *selected == role)
        .map(|(ordinal, _)| *ordinal);
    let ordinal = ordinals.next()?;
    if ordinals.next().is_some() {
        return None;
    }
    Some(advice.arguments().get(ordinal)?.original.clone())
}

#[cfg(test)]
mod tests;
