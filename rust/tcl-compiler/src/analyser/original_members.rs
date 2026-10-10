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
        original_name::SourceOriginalNameOccurrence,
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
        DeclaredMemberVisibility, DefinerFamily, DefinitionBodyGrammar, MemberKind, MemberSpec,
        MemberVisibility,
    },
};

impl Analyser {
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
            self.lexer_config(),
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
            self.lexer_config(),
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
        if grammar.family != DefinerFamily::TclOo {
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
            self.lexer_config(),
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
            source_dialect: self.original_member_source_dialect(),
            bindings: self.head_identities.source_bindings_ref(),
            registry: self.registry.as_deref(),
            caller_scope,
            readonly_names: Vec::new(),
            source_installer: None,
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
            producer.configuration_body(words, side, &mut delta, self.lexer_config())
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
        let config = self.lexer_config();
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
            source_dialect: self.original_member_source_dialect(),
            bindings,
            registry: self.registry.as_deref(),
            caller_scope: self.declaration_namespace_scope(scope_path),
            readonly_names: Vec::new(),
            source_installer: None,
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
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
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
        let native = tokens
            .iter()
            .map(|token| {
                bindings
                    .original_source_name_at_span(
                        token.span,
                        self.lexer_config(),
                        self.word_rules(),
                        policy,
                    )
                    .map(|original| original.name_input().original_word().clone())
            })
            .collect::<Option<Vec<_>>>();
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
            source_dialect: self.original_member_source_dialect(),
            bindings,
            registry: self.registry.as_deref(),
            caller_scope: self.declaration_namespace_scope(scope_path),
            readonly_names: Vec::new(),
            source_installer: None,
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
    source_dialect: Option<tcl_registry::InvocationDialect>,
    caller_scope: Option<SignatureNamespaceScope>,
    bindings: &'a crate::command_binding::SourceCommandBindings,
    readonly_names: Vec<(tcl_lexer::Span, SignatureSourceNameInput)>,
    source_installer: Option<Arc<crate::command_binding::OriginalCatalogueSourceCandidate>>,
    registry: Option<&'a tcl_registry::CommandRegistry>,
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
        SignatureSourceNameKey::from_original_native_word(word, self.dialect.rules, self.policy)
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
                body.lexer_config(),
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
            && std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some()
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
        if self.relation_member(member, arg_words, &allocation_site, side, class)? {
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
        arg_words: &[NativeWord],
        allocation_site: &CommandAllocationSite,
        side: MemberSide,
        class: &mut ClassDef,
    ) -> Option<bool> {
        if let Some(slot) = member.slot {
            // These are the analyser's class-field routes. The selected
            // Registry member owns layout and slot behavior; keyword reports
            // never select or reconstruct an operand's bytes.
            let relation = match member.keyword {
                "superclass" if side == MemberSide::Instance => {
                    Some(OriginalSourceClassRelationKind::Superclass)
                }
                "mixin" => Some(OriginalSourceClassRelationKind::Mixin),
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
                        let scope = if input.bytes().starts_with(b"::") {
                            SignatureNamespaceScope::root(Some(input.policy()))
                        } else {
                            self.caller_scope.clone()?
                        };
                        let lookup = SignatureSourceLookup::from_input(scope, &input)?;
                        OriginalSourceClassRelation::new(input, lookup)
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
        )?
        .with_source_installer(self.source_installer.clone())?;
        let delegate = self
            .grammar
            .native_classmethod_declaration(member, self.dialect.surface);
        let side = if delegate {
            MemberSide::ClassObject
        } else {
            side
        };
        let exported = member
            .declared_visibility_for_in(args, self.dialect.surface)
            .or(wrapper_visibility)
            .map_or_else(
                || self.grammar.member_default_exported_bytes(name.bytes()),
                |visibility| visibility == DeclaredMemberVisibility::Public,
            );
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
        } = self.method_parts(
            member,
            command,
            &occurrence,
            MemberFlags {
                side,
                exported,
                delegate,
            },
        )?;
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
                .with_body_role_words(
                    parameters.clone(),
                    body.clone(),
                    self.source_dialect,
                )?,
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
            .with_body_role_words(parameters, body, self.source_dialect)?,
        );
        Some(())
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
            body.lexer_config(),
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
            config,
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
                    self.grammar
                        .source_property_accessor_advice(self.policy.recipe()),
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
mod tests {
    use super::*;

    #[test]
    fn original_readonly_method_names_keep_the_selected_option_position() {
        // naming.tcloo.original-member-body-source-roles
        // docs/design/analysis/name-resolution-proofs/tcloo-original-member-body-source-roles.md
        let source = r"oo::class create C {foreach m {p\uD800 p\uD801} {method $m -private {arg} {return $arg}}; method zero\x00tail {} {return zero}}";
        for dialect in ["tcl9.0", "tcl9.1"] {
            let analysis = super::super::Analyser::new().analyse(source, dialect);
            let class = analysis
                .original_class_declarations()
                .next()
                .unwrap()
                .metadata();
            let methods = class
                .original_members
                .methods(MemberSide::Instance)
                .unwrap();
            assert_eq!(methods.len(), 3, "{dialect}");
            for bytes in [b"p\xed\xa0\x80".as_slice(), b"p\xed\xa0\x81".as_slice()] {
                let method = methods
                    .iter()
                    .find(|method| method.original_name_input().bytes() == bytes)
                    .unwrap();
                assert!(method.declaration().static_occurrence().is_none());
                assert!(method.declaration().source_installer().is_some());
                assert!(!method.exported());
                let parameters = method.parameters_word().unwrap();
                assert_eq!(
                    parameters
                        .image()
                        .bytes()
                        .get(parameters.content_span().unwrap().as_range()),
                    Some(b"arg".as_slice())
                );
                assert!(method.body_word().is_some());
            }
            let zero = methods
                .iter()
                .find(|method| method.original_name_input().bytes() == b"zero\xc0\x80tail")
                .unwrap();
            assert!(zero.declaration().static_occurrence().is_some());
            assert!(zero.parameters_word().is_some());
            assert!(zero.body_word().is_some());
        }
    }

    fn foreign_member_body_word() -> tcl_lexer::NativeWord {
        let foreign = super::super::Analyser::new().analyse(
            "oo::class create Foreign {method pick {arg} {return $arg}}",
            "tcl8.6",
        );
        foreign
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata()
            .original_members
            .declarations()
            .next()
            .unwrap()
            .body_word()
            .unwrap()
            .clone()
    }

    #[test]
    fn original_method_source_roles_retain_complete_words_across_routes_and_readonly_names() {
        // Implementation contract: naming.tcloo.original-member-body-source-roles
        // docs/design/analysis/name-resolution-proofs/tcloo-original-member-body-source-roles.md
        let source = "oo::class create C {method pick {arg} {return $arg}; renamemethod pick moved; foreach m {one two} {method $m {} {return body}}; forward proxy target fixed}";
        let analysis = super::super::Analyser::new().analyse(source, "tcl8.6");
        let class = analysis
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata();
        let methods = class
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        let moved = methods
            .iter()
            .find(|method| method.original_name_input().bytes() == b"moved")
            .unwrap();
        assert_eq!(moved.declaration().original_name_input().bytes(), b"pick");
        for method in methods.iter().filter(|method| method.body_word().is_some()) {
            let body = method.body_word().unwrap();
            assert_eq!(body.image(), &SourceImage::document(source));
            assert_eq!(body.config(), method.declaration().original_word().config());
            assert!(method.parameters_word().is_some());
            assert!(!body.group().expand);
            let dialect = method.source_dialect().unwrap();
            assert_eq!(dialect.tcl_version, Some(tcl_dialect::TclVersion::V8_6));
            assert_eq!(
                dialect.parameter_grammar(),
                Some(tcl_dialect::ParameterGrammar::Tcl)
            );
            assert_eq!(
                body.config().grammar_over(dialect.lexer_grammar),
                dialect.lexer_grammar
            );
        }
        let readonly = methods
            .iter()
            .filter(|method| method.declaration().static_occurrence().is_none())
            .collect::<Vec<_>>();
        assert_eq!(readonly.len(), 2);
        assert!(
            readonly
                .iter()
                .all(|method| method.parameters_word().is_some() && method.body_word().is_some())
        );
        for method in &readonly {
            let installer = method.declaration().source_installer().unwrap();
            assert!(installer.matches_source(
                &SourceImage::document(source),
                method.declaration().original_word().config()
            ));
            assert_eq!(installer.candidate().name, "foreach");
            assert!(!installer.obligations().is_empty());
            assert!(installer.site().offset < method.declaration().site().offset);
        }
        assert!(moved.declaration().source_installer().is_none());
        let forward = methods
            .iter()
            .find(|method| method.original_name_input().bytes() == b"proxy")
            .unwrap();
        assert!(forward.parameters_word().is_none());
        assert!(forward.body_word().is_none());
        let foreign_word = foreign_member_body_word();
        assert!(
            moved
                .clone()
                .with_body_role_words(None, Some(foreign_word), moved.source_dialect())
                .is_none()
        );
        let unknown = moved
            .clone()
            .with_body_role_words(
                moved.parameters_word().cloned(),
                moved.body_word().cloned(),
                None,
            )
            .unwrap();
        assert!(unknown.parameters_word().is_some());
        assert!(unknown.body_word().is_some());
        assert!(unknown.source_dialect().is_none());
        let jim = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        let foreign_dialect = moved
            .clone()
            .with_body_role_words(
                moved.parameters_word().cloned(),
                moved.body_word().cloned(),
                Some(jim),
            )
            .unwrap();
        assert!(foreign_dialect.source_dialect().is_none());
    }

    #[test]
    fn original_literal_member_installer_keeps_known_shadow_and_shape_barriers() {
        // Implementation contract: naming.tcloo.original-member-body-source-roles
        // docs/design/analysis/name-resolution-proofs/tcloo-original-member-body-source-roles.md
        for source in [
            "proc foreach {args} {}; oo::class create C {foreach m {one two} {method $m {} {return body}}}",
            "oo::class create C {foreach {m n} {one two} {method $m {} {return body}}}",
            "oo::class create C {foreach m $unknown {method $m {} {return body}}}",
        ] {
            let analysis = super::super::Analyser::new().analyse(source, "tcl8.6");
            let class = analysis
                .original_class_declarations()
                .next()
                .unwrap()
                .metadata();
            assert!(
                class
                    .original_members
                    .methods(MemberSide::Instance)
                    .is_none(),
                "{source}"
            );
            assert_eq!(class.original_members.declarations().count(), 0, "{source}");
        }
    }

    #[test]
    fn original_special_members_keep_keyword_body_and_definition_owners_separate() {
        let source = "oo::class create C {constructor {arg} {set local $arg}; destructor {puts CLEAN}; method p {} {}}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let class = class(source, dialect);
            let entries = class
                .original_special_members
                .declarations()
                .collect::<Vec<_>>();
            assert_eq!(entries.len(), 2, "{dialect}");
            assert_eq!(
                entries[0].kind(),
                tcl_registry::definer::DefinitionSpecialMemberKind::Constructor
            );
            assert_eq!(
                entries[1].kind(),
                tcl_registry::definer::DefinitionSpecialMemberKind::Destructor
            );
            for entry in &entries {
                assert_eq!(entry.keyword_word().image(), &SourceImage::document(source));
                assert_eq!(entry.body_word().image(), &SourceImage::document(source));
                assert_eq!(
                    entry.site().offset,
                    entry.keyword_word().group().span.start()
                );
                assert_eq!(entry.metadata().body_span, entry.body_word().span());
                assert!(entry.site().offset < entry.body_word().span().start());
            }
            assert_eq!(entries[0].parameters_word().unwrap().bytes(), b"{arg}");
            assert!(entries[1].parameters_word().is_none());
            assert_eq!(class.original_members.declarations().count(), 1);
        }
    }

    #[test]
    fn original_special_members_retain_empty_replacements_without_inferring_absence() {
        let source = "oo::class create C {constructor args {}; destructor {}}; oo::define C {constructor {} {}; destructor {}}";
        let result = Analyser::new().analyse(source, "tcl9.0");
        let class = result
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata();
        let declarations = class
            .original_special_members
            .declarations()
            .collect::<Vec<_>>();
        assert_eq!(declarations.len(), 4);
        assert!(declarations.iter().all(|entry| {
            entry
                .body_word()
                .content_span()
                .unwrap()
                .as_range()
                .is_empty()
        }));
        let delta = result.original_class_configurations().next().unwrap();
        assert_eq!(delta.special_members().declarations().count(), 2);
        let configuration_offset = u32::try_from(source.find("oo::define").unwrap()).unwrap();
        assert!(
            delta
                .special_members()
                .declarations()
                .all(|entry| entry.site().offset > configuration_offset)
        );
    }

    #[test]
    fn original_property_options_retain_exact_declaration_policy_and_source() {
        // Implementation contract: naming.tcloo.original-property-accessor-source-advice
        // docs/design/analysis/name-resolution-proofs/tcloo-original-property-accessor-source-advice.md
        let source = r"oo::configurable create C {property p\uD800 p\uD801 zero\x00tail}";
        for dialect in ["tcl9.0", "tcl9.1"] {
            let class = class(source, dialect);
            let properties = class
                .original_properties
                .properties(MemberSide::Instance)
                .unwrap();
            assert_eq!(properties.len(), 3);
            let options = properties
                .iter()
                .map(|property| property.option_name().unwrap().selected().to_vec())
                .collect::<Vec<_>>();
            assert_eq!(
                options,
                vec![
                    b"-p\xed\xa0\x80".to_vec(),
                    b"-p\xed\xa0\x81".to_vec(),
                    b"-zero\xc0\x80tail".to_vec()
                ]
            );
            for property in properties {
                let option = property.option_name().unwrap();
                assert_eq!(option.original(), property.original_name_input().bytes());
                assert_eq!(
                    property.declaration().name_input().source_image(),
                    &SourceImage::document(source)
                );
            }
        }
    }

    #[test]
    fn original_literal_loop_members_keep_readonly_list_children_and_worker_words() {
        let source = "oo::class create C {foreach m {p\\uD800 p\\uD801} {method $m {args} {return $args}}; method fetch {} {}}";
        let class = class(source, "tcl9.0");
        let methods = class
            .original_members
            .methods(MemberSide::Instance)
            .expect("closed source candidate fold");
        assert_eq!(methods.len(), 3);
        let generated = methods
            .iter()
            .filter(|method| method.declaration().static_occurrence().is_none())
            .collect::<Vec<_>>();
        assert_eq!(generated.len(), 2);
        assert_eq!(
            generated
                .iter()
                .map(|method| method.original_name_input().bytes())
                .collect::<Vec<_>>(),
            vec![b"p\xed\xa0\x80".as_slice(), b"p\xed\xa0\x81".as_slice()]
        );
        for method in generated {
            let declaration = method.declaration();
            assert!(matches!(
                declaration.original_name_input(),
                SignatureSourceNameInput::OriginalValue(_)
            ));
            assert_eq!(declaration.original_word().try_text().unwrap(), "$m");
            assert_eq!(
                declaration.original_word().image(),
                &SourceImage::document(source)
            );
            assert!(declaration.site().offset < declaration.original_word().span().start());
            assert!(method.metadata().params_computed && method.metadata().params.is_empty());
            assert!(method.original_name_input().original_word_key().is_none());
        }
        let direct = methods
            .iter()
            .find(|method| method.original_name_input().bytes() == b"fetch")
            .unwrap();
        assert!(direct.declaration().static_occurrence().is_some());
    }

    #[test]
    fn original_literal_loop_member_candidates_require_registry_shape_and_single_variable() {
        for source in [
            "proc foreach args {}; oo::class create C {foreach m {fake} {method $m {} {}}}",
            "oo::class create C {foreach {a b} {fake alternate} {method $a {} {}}}",
            "oo::class create C {foreach m $names {method $m {} {}}}",
            "oo::class create C {foreach m {fake} {method prefix$m {} {}}}",
        ] {
            let class = class(source, "tcl9.0");
            assert!(
                class
                    .original_members
                    .declarations()
                    .all(|method| method.declaration().static_occurrence().is_some()),
                "{source}"
            );
        }
    }

    fn class(source: &str, dialect: &str) -> ClassDef {
        let result = Analyser::new().analyse(source, dialect);
        result
            .original_class_declarations()
            .next()
            .expect("genuine original class publication")
            .metadata()
            .clone()
    }

    #[test]
    fn original_opaque_member_names_survive_reporting_collisions_and_moves() {
        let source = r"oo::class create C {method p\uD800 {} {return A}; method p\uD801 {} {return B}; renamemethod p\uD800 q\uD800; unexport p\uD801}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let class = class(source, dialect);
            let methods = class
                .original_members
                .methods(MemberSide::Instance)
                .expect("complete static original metadata");
            assert_eq!(methods.len(), 2, "{dialect}");
            let moved = methods
                .iter()
                .find(|method| method.original_name_input().bytes() == b"q\xed\xa0\x80")
                .unwrap();
            assert_eq!(
                moved.declaration().original_name_input().bytes(),
                b"p\xed\xa0\x80"
            );
            assert!(moved.exported());
            let private = methods
                .iter()
                .find(|method| method.original_name_input().bytes() == b"p\xed\xa0\x81")
                .unwrap();
            assert!(!private.exported());
            assert!(std::str::from_utf8(private.original_name_input().bytes()).is_err());
            assert_eq!(class.original_members.effects().count(), 2);
        }
    }

    #[test]
    fn original_wrapper_tables_visibility_and_forward_inputs_remain_distinct() {
        let source = r"oo::class create C {method ping {} {}; self {method ping {} {}; renamemethod ping pong}; private {method hidden {} {}}; forward go target p\uD800; unexport ping; method ping {} {return RESET}}";
        for dialect in ["tcl9.0", "tcl9.1"] {
            let class = class(source, dialect);
            let own = class
                .original_members
                .methods(MemberSide::Instance)
                .unwrap();
            let object = class
                .original_members
                .methods(MemberSide::ClassObject)
                .unwrap();
            assert_eq!(object.len(), 1);
            assert_eq!(object[0].original_name_input().bytes(), b"pong");
            assert_eq!(
                object[0].declaration().original_name_input().bytes(),
                b"ping"
            );
            assert!(
                own.iter()
                    .find(|method| method.original_name_input().bytes() == b"ping")
                    .unwrap()
                    .exported()
            );
            assert!(
                !own.iter()
                    .find(|method| method.original_name_input().bytes() == b"hidden")
                    .unwrap()
                    .exported()
            );
            let forward = own
                .iter()
                .find(|method| method.original_name_input().bytes() == b"go")
                .unwrap();
            let inputs = forward.forward_prefix().unwrap();
            assert_eq!(inputs[0].bytes(), b"target");
            assert_eq!(inputs[1].bytes(), b"p\xed\xa0\x80");
        }
    }

    #[test]
    fn original_computed_member_name_withdraws_own_metadata_view() {
        let class = class(
            "oo::class create C {method fixed {} {}; method $name {} {}}",
            "tcl8.6",
        );
        assert!(
            class
                .original_members
                .methods(MemberSide::Instance)
                .is_none()
        );
        assert!(
            class
                .original_members
                .declarations()
                .any(|method| method.original_name_input().bytes() == b"fixed")
        );
    }

    #[test]
    fn original_relations_preserve_opaque_operands_caller_scope_and_receiver_side() {
        // Implementation contract: naming.tcloo.original-relation-provider-fold-contract
        // docs/design/analysis/name-resolution-proofs/tcloo-original-relation-provider-fold-contract.md
        let source = r"namespace eval N {oo::class create C {superclass B\uD800 B\uD801; mixin B\uD800; self mixin B\uD801}}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let class = class(source, dialect);
            let supers = class
                .original_relations
                .resolve(
                    MemberSide::Instance,
                    OriginalSourceClassRelationKind::Superclass,
                    |relation| Some(relation.name_input().bytes().to_vec()),
                )
                .unwrap();
            assert_eq!(
                supers,
                vec![b"B\xed\xa0\x80".to_vec(), b"B\xed\xa0\x81".to_vec()]
            );
            let mixes = class
                .original_relations
                .resolve(
                    MemberSide::ClassObject,
                    OriginalSourceClassRelationKind::Mixin,
                    |relation| Some(relation.name_input().bytes().to_vec()),
                )
                .unwrap();
            assert_eq!(mixes, vec![b"B\xed\xa0\x81".to_vec()]);
            let relation = class
                .original_relations
                .effects()
                .next()
                .unwrap()
                .values()
                .unwrap()
                .first()
                .unwrap();
            let candidates = relation.lookup().candidates().unwrap();
            assert_eq!(
                candidates[0].namespace,
                tcl_core_types::ByteNamespacePath::from_segments([b"N".as_slice()])
            );
            assert_eq!(candidates[0].simple.as_bytes(), b"B\xed\xa0\x80");
        }
    }

    #[test]
    fn original_relation_removal_compares_joined_providers_and_renews_after_unknown() {
        // Implementation contract: naming.tcloo.original-relation-provider-fold-contract
        // docs/design/analysis/name-resolution-proofs/tcloo-original-relation-provider-fold-contract.md
        let replaced = class(
            "oo::class create C {mixin First; mixin -remove Other; mixin $unknown; mixin -set Last}",
            "tcl9.1",
        );
        let resolved = replaced.original_relations.resolve(
            MemberSide::Instance,
            OriginalSourceClassRelationKind::Mixin,
            |relation| match relation.name_input().bytes() {
                b"First" | b"Other" => Some(1),
                b"Last" => Some(2),
                _ => None,
            },
        );
        assert_eq!(
            resolved,
            Some(vec![2]),
            "an exact replacement renews the relation list after incomplete operand coverage"
        );
        let class = class(
            "oo::class create C {mixin First; mixin -remove Other}",
            "tcl9.1",
        );
        assert_eq!(
            class.original_relations.resolve(
                MemberSide::Instance,
                OriginalSourceClassRelationKind::Mixin,
                |_| Some(1)
            ),
            Some(Vec::<i32>::new()),
            "different spellings can independently join the same actual class provider"
        );
    }

    #[test]
    fn original_class_configuration_joins_opaque_target_allocation_and_own_delta() {
        // Implementation contract: naming.tcloo.original-configuration-allocation-join-contract
        // docs/design/analysis/name-resolution-proofs/tcloo-original-configuration-allocation-join-contract.md
        let source = r"oo::class create C\uD800 {method p\uD800 {} {}}; oo::class create C\uD801 {method p\uD801 {} {}}; oo::define C\uD800 {renamemethod p\uD800 moved; mixin -set M\uD800}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = Analyser::new().analyse(source, dialect);
            let declarations = result.original_class_declarations().collect::<Vec<_>>();
            assert_eq!(declarations.len(), 2, "{dialect}");
            let configuration = result
                .original_class_configurations()
                .next()
                .expect("original target point and full member delta");
            assert_eq!(
                configuration.target().name_input().bytes(),
                b"C\xed\xa0\x80"
            );
            assert_eq!(
                configuration.target().local_allocation().unwrap().site,
                *declarations[0].declaration_site()
            );
            assert!(!configuration.target().is_external_candidate());
            let first = declarations[0]
                .metadata()
                .original_members
                .methods(MemberSide::Instance)
                .unwrap();
            assert_eq!(first[0].original_name_input().bytes(), b"moved");
            assert_eq!(
                first[0].declaration().original_name_input().bytes(),
                b"p\xed\xa0\x80"
            );
            let second = declarations[1]
                .metadata()
                .original_members
                .methods(MemberSide::Instance)
                .unwrap();
            assert_eq!(second[0].original_name_input().bytes(), b"p\xed\xa0\x81");
            assert_eq!(
                configuration
                    .relations()
                    .effects()
                    .next()
                    .unwrap()
                    .values()
                    .unwrap()[0]
                    .name_input()
                    .bytes(),
                b"M\xed\xa0\x80"
            );
        }
    }

    #[test]
    fn original_class_configuration_preserves_moved_identity_and_external_obligation() {
        // Implementation contract: naming.tcloo.original-configuration-allocation-join-contract
        // docs/design/analysis/name-resolution-proofs/tcloo-original-configuration-allocation-join-contract.md
        let source = "oo::class create C {method old {} {}}; rename C D; oo::define D {renamemethod old changed}";
        let result = Analyser::new().analyse(source, "tcl9.1");
        let declaration = result.original_class_declarations().next().unwrap();
        let configuration = result.original_class_configurations().next().unwrap();
        assert_eq!(
            configuration.target().local_allocation().unwrap().site,
            *declaration.declaration_site()
        );
        assert_eq!(
            declaration
                .metadata()
                .original_members
                .methods(MemberSide::Instance)
                .unwrap()[0]
                .original_name_input()
                .bytes(),
            b"changed"
        );
        let result =
            Analyser::new().analyse(r"oo::define Foreign {method p\uD800 {} {}}", "tcl9.1");
        assert_eq!(result.original_class_declarations().count(), 0);
        let configuration = result
            .original_class_configurations()
            .next()
            .expect("independent external target obligation");
        assert!(configuration.target().is_external_candidate());
        assert!(configuration.target().local_allocation().is_none());
        assert_eq!(
            configuration
                .members()
                .methods(MemberSide::Instance)
                .unwrap()[0]
                .original_name_input()
                .bytes(),
            b"p\xed\xa0\x80"
        );
    }

    #[test]
    fn original_class_configuration_does_not_promote_alias_or_computed_target() {
        // Implementation contract: naming.tcloo.original-configuration-allocation-join-contract
        // docs/design/analysis/name-resolution-proofs/tcloo-original-configuration-allocation-join-contract.md
        for source in [
            "oo::class create C {method old {} {}}; interp alias {} A {} C; oo::define A {method extra {} {}}",
            "oo::class create C {method old {} {}}; oo::define $unknown {method extra {} {}}",
        ] {
            let result = Analyser::new().analyse(source, "tcl9.1");
            assert_eq!(result.original_class_configurations().count(), 0);
            let class = result
                .original_class_declarations()
                .next()
                .unwrap()
                .metadata();
            assert!(
                class
                    .original_members
                    .methods(MemberSide::Instance)
                    .is_none()
            );
            assert!(
                class
                    .original_members
                    .declarations()
                    .any(|method| method.original_name_input().bytes() == b"old")
            );
        }
    }

    #[test]
    fn original_object_configuration_keeps_class_object_and_instance_tables_separate() {
        // Implementation contract: naming.tcloo.original-configuration-allocation-join-contract
        // docs/design/analysis/name-resolution-proofs/tcloo-original-configuration-allocation-join-contract.md
        let source = r"oo::class create C {method same {} {}; self method same {} {}}; oo::objdefine C {renamemethod same own; mixin -set ObjectMixin}";
        let result = Analyser::new().analyse(source, "tcl9.1");
        let declaration = result.original_class_declarations().next().unwrap();
        let configuration = result.original_class_configurations().next().unwrap();
        assert_eq!(
            configuration.target().layer(),
            tcl_registry::ObjectDispatchLayer::Object
        );
        assert_eq!(
            configuration.target().local_allocation().unwrap().site,
            *declaration.declaration_site()
        );
        let class = declaration.metadata();
        assert_eq!(
            class
                .original_members
                .methods(MemberSide::Instance)
                .unwrap()[0]
                .original_name_input()
                .bytes(),
            b"same"
        );
        assert_eq!(
            class
                .original_members
                .methods(MemberSide::ClassObject)
                .unwrap()[0]
                .original_name_input()
                .bytes(),
            b"own"
        );
        assert_eq!(
            class.original_relations.resolve(
                MemberSide::Instance,
                OriginalSourceClassRelationKind::Mixin,
                |_| Some(1)
            ),
            Some(Vec::<i32>::new())
        );
        assert_eq!(
            class.original_relations.resolve(
                MemberSide::ClassObject,
                OriginalSourceClassRelationKind::Mixin,
                |_| Some(1)
            ),
            Some(vec![1])
        );
    }

    #[test]
    fn original_properties_retain_counted_names_and_per_declaration_options() {
        let source = r"oo::configurable create C {property p\uD800 -kind writable p\uD801 -get {return custom} q -kind readable -set {set value 1}}";
        let result = crate::analyser::Analyser::new().analyse(source, "tcl9.1");
        let declaration = result
            .original_class_declarations()
            .next()
            .expect("genuine original class declaration");
        let properties = declaration
            .metadata()
            .original_properties
            .properties(MemberSide::Instance)
            .expect("complete own original property words");
        assert_eq!(properties.len(), 3);
        assert!(
            properties
                .iter()
                .all(|property| property.accessor_methods() == ["configure"])
        );
        assert_eq!(
            properties[0].original_name_input().bytes(),
            b"p\xed\xa0\x80"
        );
        assert_eq!(
            properties[0].kind(),
            tcl_registry::commands::tcl::TclOoPropertyKind::Writable
        );
        assert!(properties[0].getter().is_none());
        assert_eq!(
            properties[1].original_name_input().bytes(),
            b"p\xed\xa0\x81"
        );
        assert_eq!(
            properties[1].kind(),
            tcl_registry::commands::tcl::TclOoPropertyKind::ReadWrite
        );
        assert!(properties[1].getter().is_some());
        assert_eq!(
            properties[2].kind(),
            tcl_registry::commands::tcl::TclOoPropertyKind::Readable
        );
        assert!(properties[2].setter().is_some());
        assert!(
            declaration
                .metadata()
                .original_properties
                .properties(MemberSide::ClassObject)
                .unwrap()
                .is_empty()
        );
        assert!(properties.iter().all(
            |property| property.declaration().name_input().source_image()
                == &tcl_lexer::SourceImage::document(source)
        ));
    }

    #[test]
    fn original_object_configuration_retains_instance_allocation_without_class_table_donation() {
        let source = r"oo::class create C {method base {} {}}; C create d; oo::objdefine d {method own\uD800 {} {}; method own\uD801 {} {}}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = Analyser::new().analyse(source, dialect);
            let classes = result.original_class_declarations().collect::<Vec<_>>();
            assert_eq!(classes.len(), 1, "{dialect}");
            let own = result
                .original_object_configurations()
                .next()
                .expect("actual named-instance operand and independent delta");
            assert_eq!(own.target().name_input().unwrap().bytes(), b"d");
            assert_eq!(
                own.target().instance().allocation().site.offset,
                u32::try_from(source.find("C create d").unwrap()).unwrap()
            );
            let members = own
                .members()
                .methods(MemberSide::ClassObject)
                .expect("complete own-object source metadata");
            assert_eq!(members.len(), 2);
            assert_eq!(members[0].original_name_input().bytes(), b"own\xed\xa0\x80");
            assert_eq!(members[1].original_name_input().bytes(), b"own\xed\xa0\x81");
            assert!(
                classes[0]
                    .metadata()
                    .original_members
                    .methods(MemberSide::ClassObject)
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                classes[0]
                    .metadata()
                    .original_members
                    .methods(MemberSide::Instance)
                    .unwrap()
                    .len(),
                1
            );
            assert!(result.original_class_configurations().next().is_none());
        }
    }

    #[test]
    fn original_object_configuration_uses_actual_variable_operand_read() {
        let source =
            r"oo::class create C {}; set object [C new]; oo::objdefine $object {method own {} {}}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = Analyser::new().analyse(source, dialect);
            let own = result
                .original_object_configurations()
                .next()
                .expect("reached object read, independently of generated spelling");
            assert_eq!(
                own.target().instance().allocation().site.offset,
                u32::try_from(source.find("C new").unwrap()).unwrap()
            );
            assert_eq!(
                own.members()
                    .methods(MemberSide::ClassObject)
                    .unwrap()
                    .len(),
                1
            );
            assert!(result.original_class_configurations().next().is_none());
        }
        for source in [
            "oo::class create C {}; C create d; interp alias {} wrapper {} d; oo::objdefine wrapper {method own {} {}}",
            "oo::class create C {}; set object TEXT; oo::objdefine $object {method own {} {}}",
            "oo::class create C {}; set object [C new]; set object TEXT; oo::objdefine $object {method own {} {}}",
        ] {
            let result = Analyser::new().analyse(source, "tcl8.6");
            assert!(
                result.original_object_configurations().next().is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_generic_definer_members_use_source_value_purpose() {
        let source = r"snit::type T {method p\uD800 {} {}; method p\uD801 {} {}}";
        let result = Analyser::new().analyse(source, "tcl8.5");
        let class = result
            .original_class_declarations()
            .next()
            .expect("original generic definer publication");
        let members = class
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .expect("Registry-selected source metadata purpose");
        assert_eq!(members.len(), 2);
        assert!(members.iter().all(|member| member.name_purpose()
            == tcl_registry::definer::DefinitionMemberNamePurpose::SourceValue));
        assert_ne!(
            members[0].original_name_input().bytes(),
            members[1].original_name_input().bytes()
        );
    }

    #[test]
    fn original_readonly_class_configuration_joins_actual_name_value() {
        let source = r"oo::class create C\uD800 {method base {} {}}; set target C\uD800; oo::define $target {method added {} {}}";
        let result = Analyser::new().analyse(source, "tcl9.1");
        let declaration = result.original_class_declarations().next().unwrap();
        let configuration = result
            .original_class_configurations()
            .next()
            .expect("exact readonly target producer and actual class allocation");
        assert!(
            configuration
                .target()
                .name_input()
                .original_word_key()
                .is_none()
        );
        assert_eq!(
            configuration.target().name_input().bytes(),
            b"C\xed\xa0\x80"
        );
        assert_eq!(
            configuration.target().local_allocation().unwrap().site,
            *declaration.declaration_site()
        );
        assert_eq!(
            declaration
                .metadata()
                .original_members
                .methods(MemberSide::Instance)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn original_class_source_inventory_keeps_later_opaque_relation_declarations() {
        let source = "oo::class create B\\uD800 {}\noo::class create B\\uD801 {}\noo::class create C {superclass B\\uD800; mixin B\\uD801}\n";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = Analyser::new().analyse(source, dialect);
            let declarations = result.original_class_declarations().collect::<Vec<_>>();
            assert_eq!(declarations.len(), 3, "{dialect}");
            assert_eq!(declarations[0].name_input().bytes(), b"B\xed\xa0\x80");
            assert_eq!(declarations[1].name_input().bytes(), b"B\xed\xa0\x81");
            assert_eq!(declarations[2].name_input().bytes(), b"C");
            let relations = declarations[2]
                .metadata()
                .original_relations
                .effects()
                .flat_map(|effect| effect.values().unwrap().iter())
                .collect::<Vec<_>>();
            assert_eq!(relations.len(), 2, "{dialect}");
            assert_ne!(
                relations[0].name_input().bytes(),
                relations[1].name_input().bytes()
            );
        }
    }

    #[test]
    fn original_classmethod_source_inventory_is_independent_of_instance_ancestry() {
        let source = "oo::class create B {classmethod shared {} {}}\noo::class create C {superclass B; classmethod shared {} {}}\n";
        let result = Analyser::new().analyse(source, "tcl9.0");
        let declarations = result.original_class_declarations().collect::<Vec<_>>();
        assert_eq!(declarations.len(), 2);
        for declaration in declarations {
            let methods = declaration
                .metadata()
                .original_members
                .methods(MemberSide::ClassObject)
                .expect("source-only original classmethod own table");
            assert_eq!(methods.len(), 1);
            assert_eq!(methods[0].original_name_input().bytes(), b"shared");
            assert!(methods[0].native_class_delegate());
        }
    }

    #[test]
    fn original_static_generic_class_names_retain_source_bytes_without_entered_handler() {
        let source =
            r"snit::type T\uD800 {method base {} {}}; snit::type T\uD801 {method other {} {}}";
        let result = Analyser::new().analyse(source, "tcl8.5");
        let declarations = result.original_class_declarations().collect::<Vec<_>>();
        assert_eq!(declarations.len(), 2);
        assert_eq!(declarations[0].name_input().bytes(), b"T\xed\xa0\x80");
        assert_eq!(declarations[1].name_input().bytes(), b"T\xed\xa0\x81");
        assert_ne!(declarations[0].name().slot(), declarations[1].name().slot());
        for declaration in declarations {
            assert_eq!(
                declaration.name_input().source_image().bytes(),
                source.as_bytes()
            );
            assert_eq!(
                declaration
                    .metadata()
                    .original_members
                    .methods(MemberSide::Instance)
                    .unwrap()
                    .len(),
                1
            );
        }
        let computed = Analyser::new().analyse("snit::type $name {method p {} {}}", "tcl8.5");
        assert_eq!(computed.original_class_declarations().count(), 0);
    }
}
