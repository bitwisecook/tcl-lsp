// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original class operands read the existing source walk at their own parent.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    OriginalSourceCataloguePurpose, OriginalSourceClassDeclaration,
    OriginalSourceCommandTransitionAdvice,
};
use crate::{
    registry_invocation::{OriginalSourceScriptBody, source_structure::OriginalRegistrySource},
    signature_scan::scope::{
        SignatureNamespaceScope, SignatureSourceLookup, SignatureSourceNameInput,
    },
};
use std::{cell::RefCell, sync::Arc};
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::NativeWord;
use tcl_registry::{definer::DefinerFamily, model::ContextRegistry};
use tcl_syntax::naming::NativeNameProtocol;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ClassProvider {
    Source(Arc<OriginalSourceClassDeclaration>),
    Registry(String),
}

/// Conditional provider selected by the actual source graph at one original
/// definition parent. This supplies no class object, method dispatch or entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassRelationProvider {
    body: OriginalSourceScriptBody,
    parent: Arc<OriginalSourceCommandTransitionAdvice>,
    member: Arc<[NativeWord]>,
    input: SignatureSourceNameInput,
    lookup: SignatureSourceLookup,
    slot: ByteCommandSlot,
    provider: ClassProvider,
    receiver: Option<Arc<OriginalSourceClassDeclaration>>,
}

impl OriginalSourceClassRelationProvider {
    /// Exact original operand and caller source recipe, without rendered-name joins.
    #[must_use]
    pub fn matches_relation(
        &self,
        input: &SignatureSourceNameInput,
        lookup: &SignatureSourceLookup,
    ) -> bool {
        &self.input == input && &self.lookup == lookup
    }

    /// Current whole analysis input and retained source allocation agree.
    /// This does not establish a class object or entered definition worker.
    #[must_use]
    pub fn matches_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        let Some(input) = analysis.resolved_input.as_ref() else {
            return false;
        };
        let Some(seed) = analysis.retained_command_realm().and_then(|realm| {
            realm
                .source_bindings_ref()
                .original_source_transition_seed()
        }) else {
            return false;
        };
        let image = self.body.original_container().image();
        seed.origin == self.parent.site().source
            && seed.config == input.lexer_config()
            && seed.logical_input.as_ref() == self.parent.logical_source_input()
            && self.body.matches_source(image, input.lexer_config())
            && self.parent.matches_source(image, input.lexer_config())
            && self.parent.matches_context(&input.context_registry())
            && analysis.matches_original_source_image(image, input.lexer_config())
    }

    /// Actual selected source declaration; aliases cannot supply a class object.
    #[must_use]
    pub fn source_class(&self) -> Option<&OriginalSourceClassDeclaration> {
        match &self.provider {
            ClassProvider::Source(class) => Some(class),
            ClassProvider::Registry(_) => None,
        }
    }

    /// Registry metadata is reborrowed under the same actual source context.
    #[must_use]
    pub fn registry_descriptor(
        &self,
        context: &ContextRegistry,
    ) -> Option<&'static tcl_registry::CommandSpec> {
        self.body.matches_context(context).then_some(())?;
        self.parent.matches_context(context).then_some(())?;
        let ClassProvider::Registry(command) = &self.provider else {
            return None;
        };
        let descriptor = context
            .context()
            .resolve_spec(context.commands(), command)?;
        is_registry_class_root(descriptor).then_some(descriptor)
    }

    /// Conditional source coordinate selected at this parent horizon.
    #[must_use]
    pub const fn source_slot(&self) -> &ByteCommandSlot {
        &self.slot
    }

    /// Independently selected source naming recipe, never a physical cell policy.
    #[must_use]
    pub fn policy(&self) -> tcl_syntax::naming::NamePolicyProtocol {
        self.input.policy()
    }

    pub(crate) fn matches_definition_parent(&self, body: &OriginalSourceScriptBody) -> bool {
        &self.body == body
    }

    pub(crate) fn contains_definition_site(
        &self,
        site: &crate::command_binding::CommandAllocationSite,
    ) -> bool {
        let content = self.body.content_span();
        site.source == self.parent.site().source
            && content.start() <= site.offset
            && site.offset < content.end()
    }

    /// Exact fresh declaration/header/body correspondence. Configuration
    /// parents need their own target association before supplying this view.
    #[must_use]
    pub fn matches_class_definition(&self, class: &crate::analyser::ClassDef) -> bool {
        let Some(receiver) = &self.receiver else {
            return false;
        };
        !class.source_name_ambiguous.is_observed()
            && class.metaclass_provenance == crate::analyser::MetaclassProvenance::Observed
            && receiver.factory().site() == self.parent.site()
            && receiver
                .name_input()
                .original_word()
                .and_then(|word| word.tokens().first())
                .is_some_and(|token| token.span == class.name_span)
            && self
                .body
                .original_container()
                .tokens()
                .first()
                .is_some_and(|token| token.span == class.body_span)
            && receiver
                .factory()
                .arguments()
                .iter()
                .any(|argument| &argument.original == self.body.original_container())
            && class.source_name.as_ref().is_none_or(|name| {
                name.slot() == receiver.source_slot() && name.policy() == self.policy()
            })
    }
}

fn is_registry_class_root(descriptor: &tcl_registry::CommandSpec) -> bool {
    descriptor
        .traits
        .contains(tcl_registry::Traits::IS_OO_METACLASS)
        && descriptor
            .definition_body
            .is_some_and(|grammar| grammar.family == DefinerFamily::TclOo)
}

struct ClassRelationRequest {
    body: OriginalSourceScriptBody,
    parent: Arc<OriginalSourceCommandTransitionAdvice>,
    member: Arc<[NativeWord]>,
    input: SignatureSourceNameInput,
    lookup: SignatureSourceLookup,
}

impl ClassRelationRequest {
    fn capture(
        body: &OriginalSourceScriptBody,
        member: &[NativeWord],
        input: &SignatureSourceNameInput,
        lookup: &SignatureSourceLookup,
        context: &ContextRegistry,
    ) -> Option<Self> {
        let OriginalRegistrySource::SourceTransitions(parent) = body.source_words().source() else {
            // Geometry alone cannot supply the parent's source allocation.
            return None;
        };
        if lookup.original_name_input() != Some(input)
            || !matches!(input.policy().recipe(), NativeNameProtocol::C(_))
            || !body.matches_context(context)
            || !body.matches_source(parent.site().source.source_image(), parent.config)
        {
            return None;
        }
        let original = input.original_word_key()?.original_word();
        let region = body.definition_member_region(context, body.content_span())?;
        if region.original_command_words_at(member.first()?.span().start())? != member
            || !region.references(member)?.iter().any(|reference| {
                reference.kind() == tcl_registry::definer::MemberRefKind::Class
                    && reference.original_word() == original
                    && reference.value() == input.bytes()
            })
        {
            return None;
        }
        Some(Self {
            body: body.clone(),
            parent: Arc::clone(parent),
            member: Arc::from(member),
            input: input.clone(),
            lookup: lookup.clone(),
        })
    }
}

#[derive(Clone, Copy)]
pub(super) struct ClassRelationQuery<'a> {
    request: &'a ClassRelationRequest,
    reply: &'a RefCell<Option<Option<OriginalSourceClassRelationProvider>>>,
}

impl super::super::SourceCommandBindings {
    /// Read one genuine class-reference operand at its retained definition
    /// parent's source horizon. Missing or unrepresented providers refuse.
    #[must_use]
    pub fn original_class_relation_provider(
        &self,
        body: &OriginalSourceScriptBody,
        member: &[NativeWord],
        input: &SignatureSourceNameInput,
        lookup: &SignatureSourceLookup,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalSourceClassRelationProvider> {
        let seed = self.original_source_transition_seed()?;
        let context = seed.source_context(registry)?;
        let request = ClassRelationRequest::capture(body, member, input, lookup, &context)?;
        if request.parent.site().source != seed.origin || request.parent.config != seed.config {
            return None;
        }
        let reply = RefCell::new(None);
        seed.prepare_for_query(
            &context,
            OriginalSourceCataloguePurpose::CompletePrefix,
            Some(super::SourceAdviceQuery::ClassRelation(
                ClassRelationQuery {
                    request: &request,
                    reply: &reply,
                },
            )),
        )?;
        reply.into_inner().flatten()
    }
}

impl AdviceInvocationContext<'_> {
    pub(super) fn retain_requested_class_relation(
        &self,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        self.retain_requested_logical_class_relation(invocation, graph, schema);
        self.retain_requested_logical_class_relation_candidates(invocation, graph, schema);
        let Some(super::SourceAdviceQuery::ClassRelation(query)) = self.query else {
            return;
        };
        if invocation
            .native
            .first()
            .map(NativeWord::span)
            .map(|span| span.start())
            != Some(query.request.parent.site().offset)
        {
            return;
        }
        let receipt = (|| {
            let current = self.schema_advice(invocation, graph, schema)?;
            if current != *query.request.parent
                || !graph.uncertain_operations.is_empty()
                || !super::body_effects::original_operand_effects(invocation.native).is_empty()
            {
                return None;
            }
            let policy = query.request.input.policy();
            match &graph.policy {
                AdviceNamingPolicy::Native(selected) => (*selected == policy).then_some(())?,
                AdviceNamingPolicy::Logical(input) => {
                    (tcl_registry::InvocationDialect::of_profile(input.unit_profile())
                        .authored_name_policy()
                        == Some(policy))
                    .then_some(())?;
                }
            }
            let namespace = if graph.c_source_namespace.is_some()
                && !graph.native_baseline
                && graph.procedure_namespace.is_none()
            {
                graph.c_source_namespace.clone()?
            } else {
                graph
                    .procedure_namespace
                    .as_ref()
                    .or(graph.logical_namespace.as_ref())
                    .cloned()
                    .unwrap_or_else(tcl_core_types::ByteNamespacePath::root)
            };
            let selected_lookup = SignatureSourceLookup::from_input(
                SignatureNamespaceScope::C(namespace),
                &query.request.input,
            )?;
            (selected_lookup == query.request.lookup).then_some(())?;
            let slot = graph.selected_head_key(query.request.input.bytes())?;
            let provider = self.class_relation_provider_at(graph, &slot)?;
            Some(OriginalSourceClassRelationProvider {
                body: query.request.body.clone(),
                parent: Arc::clone(&query.request.parent),
                member: Arc::clone(&query.request.member),
                input: query.request.input.clone(),
                lookup: query.request.lookup.clone(),
                slot,
                provider,
                receiver: self.source_class_declaration(graph, invocation, schema),
            })
        })();
        let mut reply = query.reply.borrow_mut();
        match &*reply {
            None => *reply = Some(receipt),
            Some(previous) if previous != &receipt => *reply = Some(None),
            _ => {}
        }
    }
}

impl AdviceInvocationContext<'_> {
    fn class_relation_provider_at(
        &self,
        graph: &AdviceGraph,
        slot: &ByteCommandSlot,
    ) -> Option<ClassProvider> {
        match graph.cells.get(slot)? {
            AdviceCell::SourceClass(class) => Some(ClassProvider::Source(Arc::clone(class))),
            AdviceCell::Descriptor(command, _) => {
                let descriptor = self
                    .context
                    .context()
                    .resolve_spec(self.context.commands(), command)?;
                is_registry_class_root(descriptor).then(|| ClassProvider::Registry(command.clone()))
            }
            _ => None,
        }
    }
}

/// A readonly Logical relation selected by the actual conditional source graph.
/// The original parent and whole member vector remain independent obligations;
/// this is neither a Native lookup nor an entered definition-worker receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogicalSourceClassRelationProvider {
    body: OriginalSourceScriptBody,
    parent: Arc<OriginalSourceCommandTransitionAdvice>,
    member: Arc<[NativeWord]>,
    input: crate::analyser::ResolvedAnalysisInput,
    operand: NativeWord,
    namespace: tcl_core_types::ByteNamespacePath,
    slot: ByteCommandSlot,
    provider: ClassProvider,
    receiver: Arc<OriginalSourceClassDeclaration>,
}
impl LogicalSourceClassRelationProvider {
    pub(crate) fn operand(&self) -> &NativeWord {
        &self.operand
    }
    pub(crate) fn source_slot(&self) -> &ByteCommandSlot {
        match &self.provider {
            ClassProvider::Source(class) => class.source_slot(),
            ClassProvider::Registry(_) => &self.slot,
        }
    }
    pub(crate) fn matches_receiver(
        &self,
        analysis: &crate::analyser::AnalysisResult,
        class: &crate::analyser::ClassDef,
    ) -> bool {
        let Some(realm) = analysis.retained_command_realm() else {
            return false;
        };
        let Some(registry) = analysis.resolved_registry() else {
            return false;
        };
        let Some(current) = realm.source_bindings_ref().logical_class_relation_provider(
            &self.body,
            &self.member,
            &self.operand,
            &self.input,
            registry,
        ) else {
            return false;
        };
        current.namespace == self.namespace
            && current.slot == self.slot
            && current.provider == self.provider
            && analysis.resolved_input.as_ref() == Some(&self.input)
            && self.input.has_logical_source_name_context()
            && self.parent.logical_source_input() == Some(&self.input)
            && self
                .parent
                .matches_source(self.operand.image(), self.input.lexer_config())
            && self.parent.matches_context(&self.input.context_registry())
            && self
                .body
                .matches_source(self.operand.image(), self.input.lexer_config())
            && analysis
                .matches_original_source_image(self.operand.image(), self.input.lexer_config())
            && self
                .receiver
                .logical_source_class(analysis)
                .is_some_and(|selected| std::ptr::eq(selected, class))
    }
    pub(crate) fn source_class<'a>(
        &self,
        analysis: &'a crate::analyser::AnalysisResult,
    ) -> Option<Option<&'a crate::analyser::ClassDef>> {
        match &self.provider {
            ClassProvider::Source(class) => class.logical_source_class(analysis).map(Some),
            ClassProvider::Registry(_) => Some(None),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct LogicalClassRelationRequest {
    body: OriginalSourceScriptBody,
    parent: Arc<OriginalSourceCommandTransitionAdvice>,
    member: Arc<[NativeWord]>,
    input: crate::analyser::ResolvedAnalysisInput,
    operand: NativeWord,
    value: Vec<u8>,
}
impl LogicalClassRelationRequest {
    fn capture(
        body: &OriginalSourceScriptBody,
        member: &[NativeWord],
        operand: &NativeWord,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        input.has_logical_source_name_context().then_some(())?;
        let context = input.context_registry();
        let OriginalRegistrySource::SourceTransitions(parent) = body.source_words().source() else {
            return None;
        };
        if parent.logical_source_input() != Some(input)
            || !parent.matches_source(operand.image(), input.lexer_config())
            || !parent.matches_context(&context)
            || !body.matches_context(&context)
            || !body.matches_source(operand.image(), input.lexer_config())
        {
            return None;
        }
        let region = body.definition_member_region(&context, body.content_span())?;
        for command in region.complete_original_commands()? {
            region.checked_references(&command)?;
            if !super::body_effects::original_operand_effects(&command).is_empty() {
                return None;
            }
        }
        if region.original_command_words_at(member.first()?.span().start())? != member {
            return None;
        }
        let references = region.checked_references(member)?;
        let mut selected = references.iter().filter(|reference| {
            reference.kind() == tcl_registry::definer::MemberRefKind::Class
                && reference.original_word() == operand
        });
        let reference = selected.next()?;
        selected.next().is_none().then_some(())?;
        let value = tcl_syntax::word_rules::original_static_word_unicode_value(operand)?;
        (value.as_slice() == reference.value() && !value.contains(&0)).then_some(())?;
        Some(Self {
            body: body.clone(),
            parent: Arc::clone(parent),
            member: Arc::from(member),
            input: input.clone(),
            operand: operand.clone(),
            value,
        })
    }
}
#[derive(Clone, Copy)]
pub(super) struct LogicalClassRelationQuery<'a> {
    request: &'a LogicalClassRelationRequest,
    reply: &'a RefCell<Option<Option<LogicalSourceClassRelationProvider>>>,
}
impl super::super::SourceCommandBindings {
    pub(crate) fn logical_class_relation_provider(
        &self,
        body: &OriginalSourceScriptBody,
        member: &[NativeWord],
        operand: &NativeWord,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<LogicalSourceClassRelationProvider> {
        let seed = self.original_source_transition_seed()?;
        let context = seed.source_context(registry)?;
        let request = LogicalClassRelationRequest::capture(body, member, operand, input)?;
        if request.parent.site().source != seed.origin || request.parent.config != seed.config {
            return None;
        }
        let reply = RefCell::new(None);
        seed.prepare_for_query(
            &context,
            OriginalSourceCataloguePurpose::CompletePrefix,
            Some(super::SourceAdviceQuery::LogicalClassRelation(
                LogicalClassRelationQuery {
                    request: &request,
                    reply: &reply,
                },
            )),
        )?;
        reply.into_inner().flatten()
    }
}
impl AdviceInvocationContext<'_> {
    fn retain_requested_logical_class_relation(
        &self,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let Some(super::SourceAdviceQuery::LogicalClassRelation(query)) = self.query else {
            return;
        };
        if invocation
            .native
            .first()
            .map(NativeWord::span)
            .map(|span| span.start())
            != Some(query.request.parent.site().offset)
        {
            return;
        }
        let receipt = (|| {
            let current = self.schema_advice(invocation, graph, schema)?;
            if current != *query.request.parent
                || !graph.uncertain_operations.is_empty()
                || !super::body_effects::original_operand_effects(invocation.native).is_empty()
            {
                return None;
            }
            let AdviceNamingPolicy::Logical(input) = &graph.policy else {
                return None;
            };
            if input != &query.request.input {
                return None;
            }
            let namespace = graph.logical_namespace.clone()?;
            let slot = graph.selected_head_key(&query.request.value)?;
            let provider = self.class_relation_provider_at(graph, &slot)?;
            let receiver = self.source_class_declaration(graph, invocation, schema)?;
            Some(LogicalSourceClassRelationProvider {
                body: query.request.body.clone(),
                parent: Arc::clone(&query.request.parent),
                member: Arc::clone(&query.request.member),
                input: query.request.input.clone(),
                operand: query.request.operand.clone(),
                namespace,
                slot,
                provider,
                receiver,
            })
        })();
        let mut reply = query.reply.borrow_mut();
        match &*reply {
            None => *reply = Some(receipt),
            Some(previous) if previous != &receipt => *reply = Some(None),
            _ => {}
        }
    }
}

/// Conditional class-reference navigation at its genuine factory parent.
/// Source debts remain attached; external coordinates prove no loaded class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceClassRelationCandidates {
    request: LogicalClassRelationRequest,
    namespace: tcl_core_types::ByteNamespacePath,
    provider: Option<ClassProvider>,
    external: Option<Vec<ByteCommandSlot>>,
    receiver: Arc<OriginalSourceClassDeclaration>,
    earlier_unknown_mutations: bool,
}

/// Source-only alternatives retained by the original relation lookup owner.
#[derive(Debug, Clone, Copy)]
pub enum LogicalSourceClassRelationTarget<'a> {
    /// One actual current source declaration selected in the parent document.
    Source(&'a crate::analyser::ClassDef),
    /// The genuine Registry class root has no source declaration target.
    Registry,
    /// Ordered external header coordinates, without source-loading authority.
    External(&'a [ByteCommandSlot]),
}

impl LogicalSourceClassRelationCandidates {
    /// Exact original Registry-class operand, independently of its display label.
    #[must_use]
    pub fn operand(&self) -> &NativeWord {
        &self.request.operand
    }

    /// Earlier unknown operations remain conditional source applicability debt.
    #[must_use]
    pub const fn has_earlier_unknown_mutations(&self) -> bool {
        self.earlier_unknown_mutations
    }

    /// Reissue the original lookup and reborrow its actual receiver/provider.
    /// A missing external inventory cannot become a known absent class.
    #[must_use]
    pub fn target<'a>(
        &'a self,
        analysis: &'a crate::analyser::AnalysisResult,
        receiver: &crate::analyser::ClassDef,
    ) -> Option<LogicalSourceClassRelationTarget<'a>> {
        analysis.logical_source_class_declaration(receiver)?;
        let realm = analysis.retained_command_realm()?;
        let registry = analysis.resolved_registry()?;
        let current =
            crate::command_binding::SourceCommandBindings::logical_class_relation_candidates(
                realm.source_bindings_ref(),
                &self.request.body,
                &self.request.member,
                &self.request.operand,
                &self.request.input,
                registry,
            )?;
        if current != *self
            || analysis.resolved_input.as_ref() != Some(&self.request.input)
            || !analysis.matches_original_source_image(
                self.request.operand.image(),
                self.request.input.lexer_config(),
            )
            || !self
                .receiver
                .logical_source_class(analysis)
                .is_some_and(|class| std::ptr::eq(class, receiver))
        {
            return None;
        }
        match &self.provider {
            Some(ClassProvider::Source(class)) => {
                let class = class.logical_source_class(analysis)?;
                analysis.logical_source_class_declaration(class)?;
                Some(LogicalSourceClassRelationTarget::Source(class))
            }
            Some(ClassProvider::Registry(_)) => Some(LogicalSourceClassRelationTarget::Registry),
            None => self
                .external
                .as_deref()
                .map(LogicalSourceClassRelationTarget::External),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct LogicalClassRelationCandidatesQuery<'a> {
    request: &'a LogicalClassRelationRequest,
    reply: &'a RefCell<Option<Option<LogicalSourceClassRelationCandidates>>>,
}

impl super::super::SourceCommandBindings {
    pub(crate) fn logical_class_relation_candidates(
        &self,
        body: &OriginalSourceScriptBody,
        member: &[NativeWord],
        operand: &NativeWord,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<LogicalSourceClassRelationCandidates> {
        let seed = self.original_source_transition_seed()?;
        let context = seed.source_context(registry)?;
        let request = LogicalClassRelationRequest::capture(body, member, operand, input)?;
        if request.parent.site().source != seed.origin || request.parent.config != seed.config {
            return None;
        }
        let reply = RefCell::new(None);
        seed.prepare_for_query(
            &context,
            OriginalSourceCataloguePurpose::CompletePrefix,
            Some(super::SourceAdviceQuery::LogicalClassRelationCandidates(
                LogicalClassRelationCandidatesQuery {
                    request: &request,
                    reply: &reply,
                },
            )),
        )?;
        reply.into_inner().flatten()
    }
}

impl AdviceInvocationContext<'_> {
    fn retain_requested_logical_class_relation_candidates(
        &self,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let Some(super::SourceAdviceQuery::LogicalClassRelationCandidates(query)) = self.query
        else {
            return;
        };
        if invocation
            .native
            .first()
            .map(NativeWord::span)
            .map(|span| span.start())
            != Some(query.request.parent.site().offset)
        {
            return;
        }
        let receipt = (|| {
            let current = self.schema_advice(invocation, graph, schema)?;
            if current != *query.request.parent
                || !super::body_effects::original_operand_effects(invocation.native).is_empty()
            {
                return None;
            }
            let AdviceNamingPolicy::Logical(input) = &graph.policy else {
                return None;
            };
            if input != &query.request.input {
                return None;
            }
            let namespace = graph.logical_namespace.clone()?;
            let provider = graph
                .selected_head_key(&query.request.value)
                .and_then(|slot| self.class_relation_provider_at(graph, &slot));
            let external = if provider.is_none() {
                let coordinates = super::logical_navigation::logical_coordinates(
                    graph,
                    &namespace,
                    &query.request.value,
                )?;
                Some(super::logical_navigation::external_header_candidates(
                    graph,
                    &query.request.value,
                    &coordinates,
                )?)
            } else {
                None
            };
            Some(LogicalSourceClassRelationCandidates {
                request: query.request.clone(),
                namespace,
                provider,
                external,
                receiver: self.source_class_declaration(graph, invocation, schema)?,
                earlier_unknown_mutations: !graph.uncertain_operations.is_empty(),
            })
        })();
        let mut reply = query.reply.borrow_mut();
        match &*reply {
            None => *reply = Some(receipt),
            Some(previous) if previous != &receipt => *reply = Some(None),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    fn original_operands(
        source: &str,
        analysis: &crate::analyser::AnalysisResult,
    ) -> Option<(
        OriginalSourceScriptBody,
        Vec<NativeWord>,
        SignatureSourceNameInput,
        SignatureSourceLookup,
    )> {
        let input = analysis.resolved_input.as_ref()?;
        let context = input.context_registry();
        let offset = u32::try_from(source.find("oo::class create Meta")?).ok()?;
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            source, analysis, offset,
        )?;
        let body = words.source_script_bodies(&context).into_iter().next()?;
        let region = body.definition_member_region(&context, body.content_span())?;
        let member = region.original_commands().next()?.to_vec();
        let reference = region
            .references(&member)?
            .into_iter()
            .find(|reference| reference.kind() == tcl_registry::definer::MemberRefKind::Class)?;
        let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()?;
        let name = SignatureSourceNameInput::OriginalWord(
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            )
            .key(
                reference.original_word(),
                tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
            )?,
        );
        let lookup = SignatureSourceLookup::from_input(
            SignatureNamespaceScope::C(tcl_core_types::ByteNamespacePath::root()),
            &name,
        )?;
        Some((body, member, name, lookup))
    }

    fn query(source: &str) -> Option<OriginalSourceClassRelationProvider> {
        let analysis = Analyser::new().analyse(source, "tcl");
        let (body, member, name, lookup) = original_operands(source, &analysis)?;
        analysis
            .retained_command_realm()?
            .source_bindings_ref()
            .original_class_relation_provider(
                &body,
                &member,
                &name,
                &lookup,
                analysis.resolved_registry()?,
            )
    }

    #[test]
    fn logical_class_relation_operands_keep_original_unicode_and_complete_member_values() {
        for (source, expected) in [
            (
                "oo::class create Meta {superclass {oo::class}}",
                Some("oo::class"),
            ),
            ("oo::class create Meta {superclass café}", Some("café")),
            (
                "oo::class create Meta {superclass oo::class $unknown}",
                None,
            ),
            ("oo::class create Meta {superclass [unknown]}", None),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let operands = original_operands(source, &analysis);
            assert_eq!(
                operands.as_ref().map(|(_, _, name, _)| name.bytes()),
                expected.map(str::as_bytes),
                "{source}"
            );
            if let Some((body, member, name, lookup)) = operands {
                let input = analysis.resolved_input.as_ref().unwrap();
                assert!(input.has_logical_source_name_context());
                let original = name.original_word_key().unwrap().original_word();
                assert!(
                    crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                        original,
                        tcl_syntax::word_rules::WordValueRules::from_config(&original.config()),
                        name.policy(),
                    )
                    .is_none()
                );
                if expected == Some("oo::class") {
                    let provider = analysis
                        .retained_command_realm()
                        .unwrap()
                        .source_bindings_ref()
                        .original_class_relation_provider(
                            &body,
                            &member,
                            &name,
                            &lookup,
                            analysis.resolved_registry().unwrap(),
                        )
                        .unwrap();
                    assert!(provider.matches_analysis(&analysis));
                    assert!(provider.matches_relation(&name, &lookup));
                }
            }
        }
    }

    #[test]
    fn original_class_relation_reads_an_actual_registry_occupant_at_its_parent_horizon() {
        let source = "oo::class create Meta {superclass oo::class}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let (body, member, name, lookup) = original_operands(source, &analysis).unwrap();
        assert!(matches!(
            body.source_words().source(),
            OriginalRegistrySource::SourceTransitions(_)
        ));
        let owner = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let provider = owner
            .original_class_relation_provider(&body, &member, &name, &lookup, context.commands())
            .unwrap();
        assert!(provider.matches_relation(&name, &lookup));
        assert!(provider.registry_descriptor(&context).is_some());
        assert!(provider.source_class().is_none());

        let mut changed = member.clone();
        changed.swap(0, 1);
        assert!(
            owner
                .original_class_relation_provider(
                    &body,
                    &changed,
                    &name,
                    &lookup,
                    context.commands(),
                )
                .is_none()
        );
        let foreign_scope = SignatureSourceLookup::from_input(
            SignatureNamespaceScope::C(tcl_core_types::ByteNamespacePath::from_segments([
                b"Other".as_slice(),
            ])),
            &name,
        )
        .unwrap();
        assert!(
            owner
                .original_class_relation_provider(
                    &body,
                    &member,
                    &name,
                    &foreign_scope,
                    context.commands(),
                )
                .is_none()
        );
        let changed = format!("{source} ");
        let foreign = Analyser::new().analyse(&changed, "tcl");
        assert!(
            foreign
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .original_class_relation_provider(
                    &body,
                    &member,
                    &name,
                    &lookup,
                    foreign.resolved_registry().unwrap(),
                )
                .is_none()
        );
    }

    #[test]
    fn original_class_relation_never_restores_missing_non_class_or_uncertain_providers() {
        for source in [
            "oo::class create Meta {superclass Missing}",
            "oo::class create Meta {superclass puts}",
            "unknown_effect; oo::class create Meta {superclass oo::class}",
            "oo::class create [set name] {superclass oo::class}",
            "proc Root args {}; oo::class create Meta {superclass Root}",
            "interp alias {} Root {} oo::class; oo::class create Meta {superclass Root}",
        ] {
            assert!(query(source).is_none(), "{source}");
        }
    }
}

#[cfg(test)]
mod logical_relation_candidate_currency_tests {
    use crate::analyser::{Analyser, AnalysisResult};
    use crate::command_binding::{
        LogicalSourceClassRelationCandidates, LogicalSourceClassRelationTarget,
    };

    fn retained() -> (AnalysisResult, LogicalSourceClassRelationCandidates) {
        let source = "oo::class create Base {method work {} {}}; oo::class create Child {superclass Base; method run {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let child = &analysis.all_classes["::Child"];
        let statements = analysis
            .logical_source_class_relation_candidates(child)
            .unwrap();
        let candidate = statements[0].operands()[0].clone();
        assert!(
            matches!(candidate.target(&analysis, child), Some(LogicalSourceClassRelationTarget::Source(base)) if std::ptr::eq(base, &analysis.all_classes["::Base"]))
        );
        (analysis, candidate)
    }

    #[test]
    fn logical_relation_candidate_rejects_changed_completed_receiver_metadata() {
        let (analysis, candidate) = retained();
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::Child")
            .unwrap()
            .superclasses
            .push("untrusted report".into());
        assert!(
            candidate
                .target(&changed, &changed.all_classes["::Child"])
                .is_none()
        );
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::Child")
            .unwrap()
            .methods
            .get_mut("run")
            .unwrap()
            .visibility
            .push_str("-changed");
        assert!(
            candidate
                .target(&changed, &changed.all_classes["::Child"])
                .is_none()
        );
        assert!(
            candidate
                .target(&analysis, &analysis.all_classes["::Child"])
                .is_some()
        );
    }

    #[test]
    fn logical_relation_candidate_rejects_changed_completed_source_provider_metadata() {
        let (analysis, candidate) = retained();
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::Base")
            .unwrap()
            .methods
            .get_mut("work")
            .unwrap()
            .visibility
            .push_str("-changed");
        assert!(
            candidate
                .target(&changed, &changed.all_classes["::Child"])
                .is_none()
        );
        assert!(matches!(
            candidate.target(&analysis, &analysis.all_classes["::Child"]),
            Some(LogicalSourceClassRelationTarget::Source(_))
        ));
    }
}
