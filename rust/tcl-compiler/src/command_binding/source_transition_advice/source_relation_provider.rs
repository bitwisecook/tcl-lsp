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

    /// Exact fresh declaration/header/body correspondence. Configuration
    /// parents need their own target association before supplying this view.
    #[must_use]
    pub fn matches_class_definition(&self, class: &crate::analyser::ClassDef) -> bool {
        let Some(receiver) = &self.receiver else {
            return false;
        };
        receiver.factory().site() == self.parent.site()
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
        seed.prepare_for_class_relation(
            &context,
            OriginalSourceCataloguePurpose::CompletePrefix,
            Some(ClassRelationQuery {
                request: &request,
                reply: &reply,
            }),
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
        let Some(query) = self.class_relation_query else {
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
            let slot = graph.lookup_head_key(query.request.input.bytes())?;
            let provider = match graph.cells.get(&slot)? {
                AdviceCell::SourceClass(class) => ClassProvider::Source(Arc::clone(class)),
                AdviceCell::Descriptor(command, _) => {
                    let descriptor = self
                        .context
                        .context()
                        .resolve_spec(self.context.commands(), command)?;
                    is_registry_class_root(descriptor).then_some(())?;
                    ClassProvider::Registry(command.clone())
                }
                _ => return None,
            };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyser::Analyser, signature_scan::scope::SignatureSourceNameKey};

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
            SignatureSourceNameKey::from_original_native_word(
                reference.original_word(),
                tcl_syntax::word_rules::WordValueRules::from_config(input.lexer_config()),
                policy,
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
