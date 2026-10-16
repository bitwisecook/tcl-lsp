// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original class factories and constructor syntax without allocated receivers.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    AdvicePublicationPurpose, OriginalSourceCommandTransition,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceAdviceNameInput, SourceAdviceWord, SourceCommandTransitionObligation,
};
use crate::analyser::{AnalysisResult, ClassDef};
use crate::signature_scan::original_name::SourceDeclarationMetadata;
use std::{collections::BTreeMap, sync::Arc};
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::{
    definer::{DefinerFamily, DefinitionBodyGrammar, ManufacturerMethod, MemberVisibility},
    model::ContextRegistry,
};

/// Exact original selected definer and naming operand, without a runtime class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassDeclaration {
    factory: Arc<OriginalSourceCommandTransitionAdvice>,
    name: SourceAdviceNameInput,
    slot: ByteCommandSlot,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceClassDeclarationFacet {
    factory: Arc<OriginalSourceCommandTransitionAdvice>,
    class: Option<Arc<OriginalSourceClassDeclaration>>,
}

impl OriginalSourceClassDeclaration {
    /// Whole actual original definer argv and applicability.
    #[must_use]
    pub fn factory(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.factory
    }
    /// Actual class-name source operand with its selected naming policy.
    #[must_use]
    pub const fn name_input(&self) -> &SourceAdviceNameInput {
        &self.name
    }
    /// Conditional publication geometry, separate from an allocation.
    #[must_use]
    pub const fn source_slot(&self) -> &ByteCommandSlot {
        &self.slot
    }
    /// Actual original moves and aliases selecting this source factory.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.lineage
    }
    /// Family grammar selected by the retained original definer schema.
    #[must_use]
    pub fn grammar(&self, context: &ContextRegistry) -> Option<&'static DefinitionBodyGrammar> {
        self.factory.matches_context(context).then_some(())?;
        context
            .context()
            .resolve_spec(context.commands(), self.factory.command())?
            .definition_body
    }
    /// Exact original factory/name occurrence correspondence. This is a
    /// canonical metadata join, not current command occupation or execution.
    #[must_use]
    pub fn matches_source_declaration(&self, record: &SourceDeclarationMetadata<ClassDef>) -> bool {
        record.declaration_site() == self.factory.site()
            && self
                .name
                .original_word()
                .is_some_and(|word| record.name_input().original_word() == word)
            && record.metadata().source_name.as_ref() == Some(record.name())
    }
    /// Canonical original class declaration; duplicates and foreign images refuse.
    /// Reporting class names and final class maps never supply this identity.
    #[must_use]
    pub fn source_class<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a SourceDeclarationMetadata<ClassDef>> {
        let input = analysis.resolved_input.as_ref()?;
        let context = input.context_registry();
        let original = self
            .name
            .native_input()?
            .original_word_key()?
            .original_word();
        self.factory
            .matches_source(original.image(), input.lexer_config())
            .then_some(())?;
        self.factory.matches_context(&context).then_some(())?;
        analysis
            .matches_original_source_image(original.image(), input.lexer_config())
            .then_some(())?;
        let mut records = analysis
            .original_class_declarations()
            .filter(|record| self.matches_source_declaration(record));
        let record = records.next()?;
        records.next().is_none().then_some(record)
    }
    /// Readonly metadata correspondence in the positively retained Logical
    /// model. Exact factory input and original naming/body tokens are required;
    /// reporting names cannot select a moved or deleted source class.
    #[must_use]
    pub fn logical_source_class<'a>(&self, analysis: &'a AnalysisResult) -> Option<&'a ClassDef> {
        analysis
            .allows_retained_logical_declaration_advice()
            .then_some(())?;
        let input = analysis.resolved_input.as_ref()?;
        (self.factory.logical_source_input() == Some(input)).then_some(())?;
        let name = self.name.logical_input()?.original_word();
        let context = input.context_registry();
        (self
            .factory
            .matches_source(name.image(), input.lexer_config())
            && self.factory.matches_context(&context)
            && analysis.matches_original_source_image(name.image(), input.lexer_config()))
        .then_some(())?;
        let grammar = self.grammar(&context)?;
        let body_at = if let Some(installation) = grammar.command_installation() {
            usize::from(installation.body_at)
        } else if grammar.family == DefinerFamily::TclOo {
            let selector =
                std::str::from_utf8(self.factory.arguments.first()?.value.as_deref()?).ok()?;
            let spec = context
                .context()
                .resolve_spec(context.commands(), self.factory.command())?;
            usize::from(
                tcl_registry::CommandRegistry::manufacturer_method_for_descriptor(spec, selector)?
                    .definition_body_at?,
            )
        } else {
            return None;
        };
        let body = &self.factory.arguments.get(body_at)?.original;
        let name_span = name.tokens().first()?.span;
        let body_span = body.tokens().first()?.span;
        let mut declarations = analysis
            .original_class_declarations()
            .filter(|record| self.matches_source_declaration(record));
        let declaration = declarations.next();
        declarations.next().is_none().then_some(())?;
        let mut matching = analysis.all_classes.values().filter(|class| {
            class.metaclass_provenance == crate::analyser::MetaclassProvenance::Observed
                && class.source_name.as_ref().is_none_or(|key| {
                    declaration.is_some_and(|record| {
                        record.name() == key
                            && record.metadata().name_span == name_span
                            && record.metadata().body_span == body_span
                    })
                })
                && class.name_span == name_span
                && class.body_span == body_span
        });
        let metadata = matching.next()?;
        matching.next().is_none().then_some(metadata)
    }
    /// Readonly explicit constructor-body coverage under the same genuine
    /// class definition and retained full analysis input. No constructor-chain,
    /// default absence, unchanged lookup or Native entry follows.
    #[must_use]
    pub fn constructor_body_source_coverage(
        &self,
        analysis: &AnalysisResult,
    ) -> Option<super::OriginalSourceConstructorBodyCoverage> {
        super::source_constructor_effects::capture(self, analysis)
    }

    /// Same original definer receipt for a canonical source declaration.
    #[must_use]
    pub fn from_class(
        source: &str,
        analysis: &AnalysisResult,
        class: &SourceDeclarationMetadata<ClassDef>,
    ) -> Option<Self> {
        let receipt = crate::registry_invocation::source_structure::source_class_declaration_at(
            source,
            analysis,
            class.declaration_site().offset,
        )?;
        (receipt.source_class(analysis)? == class).then_some(receipt)
    }
    pub(super) fn moved(
        &self,
        slot: ByteCommandSlot,
        step: &Arc<OriginalSourceCommandTransition>,
    ) -> Self {
        let mut moved = self.clone();
        moved.slot = slot;
        moved.lineage.push(Arc::clone(step));
        moved
    }
}

/// Source construction shape selected by the genuine source family grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalSourceConstructorShape {
    /// Actual manufacturer descriptor, with its original positional layout.
    Method(&'static ManufacturerMethod),
    /// Provider-authored bare name syntax, without a fabricated keyword.
    BareWord {
        /// Effective ordinal at which constructor arguments start.
        constructor_args_from: usize,
    },
}
impl OriginalSourceConstructorShape {
    /// Start of constructor arguments in the complete effective post-head argv.
    #[must_use]
    pub fn constructor_args_from(self) -> usize {
        match self {
            Self::Method(method) => usize::from(method.constructor_args_from),
            Self::BareWord {
                constructor_args_from,
            } => constructor_args_from,
        }
    }
}

/// A selected original factory operation considered as a source attempt.
/// Constructor visibility, completed invocation, allocated object and entered
/// Native handler are independent of this diagnostic metadata projection.
#[derive(Debug, Clone, Copy)]
pub struct OriginalSourceClassFactoryAttempt<'a> {
    class: &'a OriginalSourceClassDeclaration,
    operation: &'static ManufacturerMethod,
    factory_traits: tcl_registry::Traits,
}
impl OriginalSourceClassFactoryAttempt<'_> {
    /// Canonical original definer and current conditional source incarnation.
    #[must_use]
    pub const fn class_declaration(&self) -> &OriginalSourceClassDeclaration {
        self.class
    }
    /// Actual selected manufacturer, without successful or exported dispatch.
    #[must_use]
    pub const fn operation(&self) -> &'static ManufacturerMethod {
        self.operation
    }
    /// Authored factory metadata, without runtime trait or handler authority.
    #[must_use]
    pub const fn factory_traits(&self) -> tcl_registry::Traits {
        self.factory_traits
    }
}

/// Whole original call joined to its exact original conditional class factory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceConstructorCall {
    site: super::super::CommandAllocationSite,
    original: Arc<[NativeWord]>,
    arguments: Vec<SourceAdviceWord>,
    class: Arc<OriginalSourceClassDeclaration>,
    source_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    obligations: Vec<SourceCommandTransitionObligation>,
}
impl OriginalSourceConstructorCall {
    /// Exact written call including its head, grouping and expansions.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Original occurrence, without a fabricated runtime lookup point.
    #[must_use]
    pub const fn site(&self) -> &super::super::CommandAllocationSite {
        &self.site
    }
    /// Canonical original class definer and naming operand.
    #[must_use]
    pub fn class_declaration(&self) -> &OriginalSourceClassDeclaration {
        &self.class
    }
    /// Every unresolved execution and source premise remains explicit.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Effective original source operands; unknown expansion length stays unknown.
    #[must_use]
    pub fn arguments(&self) -> Vec<crate::registry_invocation::EffectiveInvocationWord> {
        self.arguments
            .iter()
            .map(|word| {
                if word.original.group().expand {
                    crate::registry_invocation::EffectiveInvocationWord::Expanded
                } else {
                    word.value.as_deref().map_or(
                        crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                        crate::registry_invocation::EffectiveInvocationWord::from_bytes,
                    )
                }
            })
            .collect()
    }
    /// Actual whole producer word at an effective ordinal, including captures.
    #[must_use]
    pub fn argument_word(&self, ordinal: usize) -> Option<&NativeWord> {
        Some(&self.arguments.get(ordinal)?.original)
    }
    /// Genuine source naming input at an effective ordinal, when static.
    /// Logical inputs never become Native naming units or value objects.
    #[must_use]
    pub fn argument_input(&self, ordinal: usize) -> Option<&SourceAdviceNameInput> {
        self.arguments.get(ordinal)?.input.as_ref()
    }
    /// Only direct written operands have a call-site source ordinal.
    #[must_use]
    pub fn written_argument(&self, ordinal: usize) -> Option<usize> {
        match self.arguments.get(ordinal)?.origin {
            crate::registry_invocation::InvocationWordOrigin::Written(written) => {
                written.checked_sub(1)
            }
            _ => None,
        }
    }
    /// Genuine selected enclosing body, without entered-frame authority.
    #[must_use]
    pub fn original_source_body(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceScriptBody> {
        self.source_body.as_deref()
    }
    /// Full immutable image, grammar, Registry and availability all agree.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.site.source.source_image() == image
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == config)
            && self.source_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config) && body.matches_context(context)
            })
            && self.class.factory.matches_source(image, config)
            && self.class.factory.matches_context(context)
    }
    /// Diagnostic source attempt under the exact retained factory and input.
    /// The genuine source tape owns alias prefixes, moves and class incarnation;
    /// unknown earlier effects and method/factory replacement remain terminal.
    /// This does not require an exported or successfully entered constructor.
    #[must_use]
    pub fn source_factory_attempt(
        &self,
        analysis: &AnalysisResult,
    ) -> Option<OriginalSourceClassFactoryAttempt<'_>> {
        #[cfg(test)]
        let trace_enabled = std::env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some();
        #[cfg(test)]
        let trace = |stage: &str, passed: bool| {
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some() {
                let offset = self.site.offset;
                eprintln!("ORIGINAL_FACTORY_GUARD stage={stage} offset={offset} passed={passed}");
            }
        };
        #[cfg(test)]
        if trace_enabled {
            trace("attempt.input", analysis.resolved_input.is_some());
        }
        let input = analysis.resolved_input.as_ref()?;
        let context = input.context_registry();
        let current = self.matches_source_context(
            self.original.first()?.image(),
            input.lexer_config(),
            &context,
        );
        #[cfg(test)]
        if trace_enabled {
            trace("attempt.context", current);
        }
        current.then_some(())?;
        let current = analysis
            .matches_original_source_image(self.original.first()?.image(), input.lexer_config());
        #[cfg(test)]
        if trace_enabled {
            trace("attempt.analysis_image", current);
        }
        current.then_some(())?;
        #[cfg(test)]
        if trace_enabled {
            trace(
                "attempt.quiet",
                !self
                    .obligations
                    .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
                    && self.class.factory.uncertain_operations().is_empty(),
            );
        }
        if self
            .obligations
            .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
            || !self.class.factory.uncertain_operations().is_empty()
        {
            return None;
        }
        let first = self.arguments.first()?;
        if first.original.group().expand {
            return None;
        }
        let bytes = first.value.as_deref()?;
        let selector = std::str::from_utf8(bytes).ok()?;
        if bytes.contains(&0) {
            return None;
        }
        let factory = &self.class.factory;
        if factory.logical_source_input().is_some() {
            (factory.logical_source_input() == Some(input)).then_some(())?;
            let class = self.class.logical_source_class(analysis);
            #[cfg(test)]
            if trace_enabled {
                trace("attempt.logical_class", class.is_some());
            }
            let class = class?;
            if class.class_methods.contains_key(selector)
                || class.class_methods.contains_key("unknown")
            {
                return None;
            }
        } else {
            let class = self.class.source_class(analysis);
            #[cfg(test)]
            if trace_enabled {
                trace("attempt.native_source_class", class.is_some());
            }
            let class = class?;
            let members = &class.metadata().original_members;
            let deferred = self
                .obligations
                .contains(&SourceCommandTransitionObligation::OriginalProcedureBodyApplicability)
                || self
                    .obligations
                    .contains(&SourceCommandTransitionObligation::DeferredLogicalBodyApplicability);
            let methods = if deferred {
                members.methods(crate::analyser::MemberSide::ClassObject)
            } else {
                members.methods_before_source_call(
                    crate::analyser::MemberSide::ClassObject,
                    self.site(),
                )
            };
            #[cfg(test)]
            if trace_enabled {
                trace("attempt.original_members", methods.is_some());
            }
            let methods = methods?;
            if methods.iter().any(|method| {
                let name = method.original_name_input().bytes();
                name == bytes || name == b"unknown"
            }) {
                return None;
            }
        }
        let spec = context
            .context()
            .resolve_spec(context.commands(), factory.command());
        #[cfg(test)]
        if trace_enabled {
            trace("attempt.selected_spec", spec.is_some());
        }
        let spec = spec?;
        let operation = spec.definition_body?.manufacturer(selector);
        #[cfg(test)]
        if trace_enabled {
            trace("attempt.manufacturer", operation.is_some());
        }
        let operation = operation?;
        Some(OriginalSourceClassFactoryAttempt {
            class: &self.class,
            operation,
            factory_traits: spec.traits,
        })
    }

    /// Conditional construction syntax in the positively retained Logical
    /// source model. The canonical factory and selected family own this shape;
    /// it supplies no Native name, method lookup, receiver or allocated value.
    #[must_use]
    pub fn logical_constructor_shape(
        &self,
        analysis: &AnalysisResult,
    ) -> Option<OriginalSourceConstructorShape> {
        let input = analysis.resolved_input.as_ref()?;
        let context = input.context_registry();
        self.matches_source_context(
            self.original.first()?.image(),
            input.lexer_config(),
            &context,
        )
        .then_some(())?;
        let class = self.class.logical_source_class(analysis)?;
        let first = self.arguments.first()?;
        if first.original.group().expand {
            return None;
        }
        self.class
            .display_constructor_shape(class, first.input.as_ref()?, &context, None)
    }
    /// Exact source construction shape. Unexported methods, declared provider
    /// type methods and ambiguous or opaque selectors have no such projection.
    #[must_use]
    pub fn constructor_shape(
        &self,
        analysis: &AnalysisResult,
    ) -> Option<OriginalSourceConstructorShape> {
        let input = analysis.resolved_input.as_ref()?;
        let context = input.context_registry();
        self.matches_source_context(
            self.original.first()?.image(),
            input.lexer_config(),
            &context,
        )
        .then_some(())?;
        let first = self.arguments.first()?;
        if first.original.group().expand {
            return None;
        }
        let class = self.class.source_class(analysis)?;
        let deferred = self
            .obligations
            .contains(&SourceCommandTransitionObligation::OriginalProcedureBodyApplicability)
            || self
                .obligations
                .contains(&SourceCommandTransitionObligation::DeferredLogicalBodyApplicability);
        self.class.display_constructor_shape(
            class.metadata(),
            first.input.as_ref()?,
            &context,
            (!deferred).then_some(self.site()),
        )
    }
}

impl AdviceGraph {
    fn resolve_source_class(
        &self,
        head: &SourceAdviceNameInput,
    ) -> Option<(Arc<OriginalSourceClassDeclaration>, Vec<SourceAdviceWord>)> {
        let mut key = self.lookup_head_key(head.bytes())?;
        let mut seen = Vec::new();
        let mut prefix = Vec::new();
        let mut lineage = Vec::new();
        loop {
            if seen.contains(&key) {
                return None;
            }
            seen.push(key.clone());
            match self.cells.get(&key)? {
                AdviceCell::SourceClass(class) => {
                    let mut class = class.as_ref().clone();
                    class.lineage.extend(lineage);
                    return Some((Arc::new(class), prefix));
                }
                AdviceCell::Alias {
                    target,
                    prefix: captured,
                    lookup,
                    lineage: steps,
                } => {
                    let mut next = captured.clone();
                    next.extend(prefix);
                    prefix = next;
                    lineage.extend(steps.iter().cloned());
                    key = self.alias_target_key(target, *lookup)?;
                }
                _ => return None,
            }
        }
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn source_class(&self, offset: u32) -> Option<&OriginalSourceClassDeclaration> {
        self.class_declarations.get(&offset)?.as_deref()
    }
    pub(crate) fn source_class_for_advice(
        &self,
        advice: &OriginalSourceCommandTransitionAdvice,
    ) -> Option<&OriginalSourceClassDeclaration> {
        advice.logical_source_input()?;
        if advice
            .obligations
            .contains(&SourceCommandTransitionObligation::RegisteredFactoryApplicability)
        {
            return None;
        }
        let mut factory = advice.clone();
        factory
            .obligations
            .push(SourceCommandTransitionObligation::RegisteredFactoryApplicability);
        let mut matching = self
            .class_declaration_facets
            .get(&advice.site.offset)?
            .iter()
            .filter(|facet| facet.factory.as_ref() == &factory);
        let first = matching.next()?.class.as_deref()?;
        matching
            .all(|other| other.class.as_deref() == Some(first))
            .then_some(first)
    }

    pub(super) fn retain_class_declaration_facet(&mut self, facet: SourceClassDeclarationFacet) {
        let facets = self
            .class_declaration_facets
            .entry(facet.factory.site.offset)
            .or_default();
        if let Some(previous) = facets.iter_mut().find(|held| held.factory == facet.factory) {
            if previous.class != facet.class {
                previous.class = None;
            }
        } else {
            facets.push(facet);
        }
    }

    pub(crate) fn constructor_call(&self, offset: u32) -> Option<&OriginalSourceConstructorCall> {
        self.constructor_calls.get(&offset)?.as_ref()
    }
    pub(super) fn extend_class_inventory(
        &mut self,
        declarations: BTreeMap<u32, Option<Arc<OriginalSourceClassDeclaration>>>,
        calls: BTreeMap<u32, Option<OriginalSourceConstructorCall>>,
    ) {
        for (offset, receipt) in declarations {
            merge_receipt(&mut self.class_declarations, offset, receipt);
        }
        for (offset, receipt) in calls {
            merge_receipt(&mut self.constructor_calls, offset, receipt);
        }
    }
}
pub(super) fn merge_receipt<K: Ord, T: PartialEq>(
    entries: &mut BTreeMap<K, Option<T>>,
    offset: K,
    receipt: Option<T>,
) {
    entries
        .entry(offset)
        .and_modify(|previous| {
            if previous != &receipt {
                *previous = None;
            }
        })
        .or_insert(receipt);
}
impl AdviceInvocationContext<'_> {
    pub(super) fn source_class_declaration(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<Arc<OriginalSourceClassDeclaration>> {
        self.source_class_declaration_attempt(graph, invocation, schema, None)?
            .class
    }

    pub(super) fn retain_source_class_declaration(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<Arc<OriginalSourceClassDeclaration>> {
        Self::retain_class_declaration_attempt(
            tape,
            self.source_class_declaration_attempt(graph, invocation, schema, None),
        )
    }

    fn retain_class_declaration_attempt(
        tape: &mut OriginalSourceTransitionAdviceTape,
        attempt: Option<SourceClassDeclarationFacet>,
    ) -> Option<Arc<OriginalSourceClassDeclaration>> {
        let attempt = attempt?;
        let class = attempt.class.clone();
        tape.retain_class_declaration_facet(attempt);
        class
    }

    fn source_class_declaration_attempt(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        owner: Option<(
            &Arc<crate::registry_invocation::OriginalSourceScriptBody>,
            SourceCommandTransitionObligation,
            Option<&Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>>,
        )>,
    ) -> Option<SourceClassDeclarationFacet> {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let selected = schema.authored_source_descriptors().command;
        let grammar = selected.definition_body?;
        if schema.facts().arity_accepts_frozen_arguments() != Some(true)
            || invocation.native.iter().any(|word| word.group().expand)
        {
            return None;
        }
        let name_at = if let Some(installation) = grammar.command_installation() {
            usize::from(installation.name_at)
        } else if grammar.family == DefinerFamily::TclOo {
            let selector =
                std::str::from_utf8(invocation.arguments.first()?.value.as_deref()?).ok()?;
            let manufacturer = tcl_registry::CommandRegistry::manufacturer_method_for_descriptor(
                selected, selector,
            )?;
            usize::from(manufacturer.names_instance_at?)
        } else {
            return None;
        };
        let mut factory = self.schema_advice(invocation, graph, schema)?;
        if let Some((body, obligation, scope)) = owner {
            body.matches_source(self.origin.source_image(), self.config)
                .then_some(())?;
            body.matches_context(self.context).then_some(())?;
            invocation
                .native
                .iter()
                .all(|word| {
                    word.span().start() >= body.content_span().start()
                        && word.span().end() <= body.content_span().end()
                })
                .then_some(())?;
            match self.policy {
                AdviceNamingPolicy::Logical(input) => {
                    if let Some(scope) = scope {
                        (scope.matches_input(input)
                            && scope.body_arc() == body
                            && scope.owns_words(invocation.native))
                        .then_some(())?;
                    }
                    factory.logical_body = Some(Arc::clone(body));
                    factory.logical_scope = scope.cloned();
                }
                AdviceNamingPolicy::Native(policy)
                    if matches!(
                        policy.recipe(),
                        tcl_syntax::naming::NativeNameProtocol::C(_)
                    ) && self.dialect.family() == Some(tcl_dialect::model::Family::Tcl)
                        && !graph.native_baseline
                        && graph.c_source_namespace.is_some()
                        && scope.is_none()
                        && obligation
                            == SourceCommandTransitionObligation::ConditionalCBodyApplicability =>
                {
                    factory.c_source_body = Some(Arc::clone(body));
                }
                AdviceNamingPolicy::Native(_) => return None,
            }
            factory.obligations.push(obligation);
        }
        factory
            .obligations
            .push(SourceCommandTransitionObligation::RegisteredFactoryApplicability);
        let factory = Arc::new(factory);
        let class = (|| {
            let name = invocation.arguments.get(name_at)?.input.as_ref()?.clone();
            if name.bytes().is_empty() || name.bytes().contains(&0) {
                return None;
            }
            let slot = if grammar.family == DefinerFamily::TclOo {
                match self.policy {
                    AdviceNamingPolicy::Native(policy) => {
                        // Source namespace geometry supplies no entered Native context.
                        let context = if !graph.native_baseline
                            && graph.procedure_namespace.is_none()
                            && let Some(namespace) = graph.c_source_namespace.as_ref()
                        {
                            tcl_syntax::naming::NativeNameContext::new(namespace)
                        } else {
                            graph.native_source_context()?
                        };
                        policy
                            .recipe()
                            .oo_object_publication_slot(context, name.bytes())
                            .ok()?
                    }
                    AdviceNamingPolicy::Logical(_) => {
                        graph.publication_key(name.bytes(), AdvicePublicationPurpose::Define)?
                    }
                }
            } else {
                graph.publication_key(name.bytes(), AdvicePublicationPurpose::Define)?
            };
            if !graph.namespaces.contains(&slot.namespace)
                || graph
                    .cells
                    .get(&slot)
                    .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
            {
                return None;
            }
            Some(Arc::new(OriginalSourceClassDeclaration {
                factory: Arc::clone(&factory),
                name,
                slot,
                lineage: Vec::new(),
            }))
        })();
        Some(SourceClassDeclarationFacet { factory, class })
    }
    pub(super) fn install_source_class(
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        class: Option<Arc<OriginalSourceClassDeclaration>>,
    ) {
        if let Some(class) = class {
            graph.class_declarations.push(Arc::clone(&class));
            merge_receipt(
                &mut tape.class_declarations,
                class.factory.site().offset,
                Some(Arc::clone(&class)),
            );
            graph
                .cells
                .insert(class.slot.clone(), AdviceCell::SourceClass(class));
        }
    }
    /// Capture an actual whole single-command substitution before its parent
    /// operand effects. This selects conditional source syntax, never its value.
    pub(super) fn retain_original_operand_constructor_calls(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        native: &[NativeWord],
        body: Option<(
            &Arc<crate::registry_invocation::OriginalSourceScriptBody>,
            &SourceCommandTransitionObligation,
        )>,
    ) {
        for operand in native {
            let Some(substitution) = super::original_single_command_substitution_words(operand)
            else {
                continue;
            };
            let child = &substitution.command().words;
            let Some(written) = super::original_words(child, self.policy) else {
                continue;
            };
            if self.retain_source_constructor_call(tape, graph, child, &written)
                && let Some((body, obligation)) = body
            {
                Self::retain_constructor_body_owner(
                    tape,
                    child[0].span().start(),
                    body,
                    obligation.clone(),
                );
            }
        }
    }
    pub(super) fn retain_original_body_constructor_calls(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        native: &[NativeWord],
        owner: (
            &Arc<crate::registry_invocation::OriginalSourceScriptBody>,
            SourceCommandTransitionObligation,
        ),
    ) -> bool {
        self.retain_original_operand_constructor_calls(
            tape,
            graph,
            native,
            Some((owner.0, &owner.1)),
        );
        let Some(written) = super::original_words(native, self.policy) else {
            return false;
        };
        if self.retain_class_instance_invocation(tape, graph, native, &written) {
            Self::retain_class_instance_body_owner(
                tape,
                native[0].span().start(),
                owner.0,
                owner.1,
            );
            graph.widen(native);
            return true;
        }
        if !self.retain_source_constructor_call(tape, graph, native, &written) {
            return false;
        }
        Self::retain_constructor_body_owner(tape, native[0].span().start(), owner.0, owner.1);
        let instance = tape
            .constructor_call(native[0].span().start())
            .and_then(|call| self.class_instance_from_call(call, graph));
        graph.widen(native);
        Self::install_source_class_instance(graph, instance);
        true
    }
    pub(super) fn source_class_declaration_in_body(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        owner: (
            &Arc<crate::registry_invocation::OriginalSourceScriptBody>,
            SourceCommandTransitionObligation,
            Option<&Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>>,
        ),
    ) -> Option<Arc<OriginalSourceClassDeclaration>> {
        Self::retain_class_declaration_attempt(
            tape,
            self.source_class_declaration_attempt(graph, invocation, schema, Some(owner)),
        )
    }
    pub(super) fn retain_constructor_body_owner(
        tape: &mut OriginalSourceTransitionAdviceTape,
        offset: u32,
        body: &Arc<crate::registry_invocation::OriginalSourceScriptBody>,
        obligation: SourceCommandTransitionObligation,
    ) {
        if let Some(Some(call)) = tape.constructor_calls.get_mut(&offset) {
            if call
                .source_body
                .as_ref()
                .is_some_and(|previous| previous != body)
            {
                tape.constructor_calls.insert(offset, None);
                return;
            }
            call.source_body = Some(Arc::clone(body));
            if !call.obligations.contains(&obligation) {
                call.obligations.push(obligation);
            }
        }
    }
    pub(super) fn retain_source_constructor_call(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        native: &[NativeWord],
        written: &[SourceAdviceWord],
    ) -> bool {
        let Some(head) = written.first().and_then(|word| word.input.as_ref()) else {
            return false;
        };
        let Some((class, mut arguments)) = graph.resolve_source_class(head) else {
            return false;
        };
        for (ordinal, word) in arguments.iter_mut().enumerate() {
            word.origin = crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.iter().skip(1).cloned());
        let mut obligations = class.factory.obligations.clone();
        if graph.procedure_body.is_some() {
            obligations.push(SourceCommandTransitionObligation::OriginalProcedureBodyApplicability);
        }
        if graph.future_procedure_source {
            obligations.push(
                SourceCommandTransitionObligation::FutureOriginalProcedureSourceApplicability,
            );
        }
        let current_callbacks = native.iter().any(|word| {
            word.executable_parts()
                .all_parts()
                .any(|part| !matches!(part.part, tcl_lexer::ExecutablePart::Text(_)))
        });
        if (current_callbacks || !graph.uncertain_operations.is_empty())
            && !obligations.contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
        {
            obligations.push(SourceCommandTransitionObligation::UnknownEarlierMutation);
        }
        let receipt = OriginalSourceConstructorCall {
            site: super::super::CommandAllocationSite {
                source: Arc::clone(self.origin),
                offset: native[0].span().start(),
            },
            original: Arc::from(native),
            arguments,
            class,
            source_body: graph.procedure_body.clone(),
            obligations,
        };
        merge_receipt(
            &mut tape.constructor_calls,
            receipt.site.offset,
            Some(receipt),
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::source_constructor_call_at;

    #[test]
    fn c_source_class_query_preserves_independently_completed_world() {
        use crate::registry_invocation::source_structure::source_class_declaration_at;
        let source = "namespace eval outer {namespace eval inner {oo::class create Widget {method m {} {}}}}";
        let offset = u32::try_from(source.find("oo::class").unwrap()).unwrap();
        for dialect in ["tcl8.6", "tcl9.0"] {
            let analysis = Analyser::new().analyse(source, dialect);
            assert!(!analysis.allows_lexical_declaration_advice());
            let completed = analysis
                .original_completed_command_world()
                .cloned()
                .expect("independently retained closed Normal source root before the query");
            let declaration = source_class_declaration_at(source, &analysis, offset)
                .expect("genuine original namespace-body class declaration");
            assert_eq!(
                declaration
                    .source_class(&analysis)
                    .unwrap()
                    .metadata()
                    .qualified_name,
                "::outer::inner::Widget"
            );
            assert!(declaration.factory.c_source_body.is_some());
            assert!(declaration.factory.logical_source_input().is_none());
            assert_eq!(
                analysis.original_completed_command_world(),
                Some(&completed)
            );
        }
    }

    #[test]
    fn c_source_namespace_class_declarations_retain_original_body_and_conditional_scope() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        use super::super::SourceCommandTransitionObligation;
        use crate::registry_invocation::source_structure::source_class_declaration_at;
        let source = "namespace eval outer {namespace eval inner {oo::class create Widget {method m {} {}}}}";
        let offset = u32::try_from(source.find("oo::class").unwrap()).unwrap();
        for dialect in ["tcl8.6", "tcl9.0"] {
            let analysis = Analyser::new().analyse(source, dialect);
            assert!(!analysis.allows_lexical_declaration_advice());
            assert!(analysis.original_completed_command_world().is_none());
            let declaration = source_class_declaration_at(source, &analysis, offset)
                .expect("original namespace-body class declaration");
            let class = declaration
                .source_class(&analysis)
                .expect("canonical source class card");
            assert_eq!(class.metadata().qualified_name, "::outer::inner::Widget");
            assert!(class.metadata().methods.contains_key("m"));
            assert!(declaration.factory.c_source_body.is_some());
            assert!(declaration.factory.logical_source_input().is_none());
            for obligation in [
                SourceCommandTransitionObligation::ConditionalCNamespaceApplicability,
                SourceCommandTransitionObligation::ConditionalCBodyApplicability,
                SourceCommandTransitionObligation::RegisteredFactoryApplicability,
            ] {
                assert!(declaration.factory.obligations().contains(&obligation));
            }
            assert!(
                source_class_declaration_at(&source.replace("Widget", "Gadget"), &analysis, offset)
                    .is_none()
            );
            let mut foreign = analysis.clone();
            foreign.resolved_input = Analyser::new()
                .analyse("puts foreign", "tcl9.1")
                .resolved_input;
            assert!(source_class_declaration_at(source, &foreign, offset).is_none());
        }
        for source in [
            "namespace eval $unknown {oo::class create Widget {}}",
            "namespace eval outer {proc ::oo::class args {}; oo::class create Widget {}}",
        ] {
            let offset = u32::try_from(source.rfind("oo::class create").unwrap()).unwrap();
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            assert!(
                source_class_declaration_at(source, &analysis, offset).is_none(),
                "{source}"
            );
        }
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let analysis = Analyser::new()
            .with_source_analysis_entry(std::sync::Arc::new(
                crate::command_binding::SourceAnalysisEntry {
                    native_entry: Some(std::sync::Arc::new(entry)),
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            ))
            .analyse(source, profile.name);
        assert!(
            source_class_declaration_at(source, &analysis, offset).is_none(),
            "closed Native baseline cannot borrow conditional source-body ownership"
        );
    }

    #[test]
    fn logical_class_facet_retains_a_same_purpose_occupied_target_refusal() {
        use super::*;
        use crate::registry_invocation::source_structure::{
            OriginalRegistrySource, source_registry_words_at,
        };
        let source = "oo::class create C {}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let words = source_registry_words_at(source, &analysis, 0).unwrap();
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("actual Logical schema");
        };
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let policy = AdviceNamingPolicy::Logical(input.clone());
        let owner = AdviceInvocationContext {
            origin: &advice.site.source,
            context: &context,
            config: advice.config,
            dialect: advice.dialect,
            policy: &policy,
            query: None,
        };
        let mut graph = AdviceGraph::new(&context, &policy, advice.dialect).unwrap();
        let mut tape = OriginalSourceTransitionAdviceTape::default();
        let invocation = AdviceInvocation {
            native: advice.original_words(),
            head: &advice.head,
            arguments: &advice.arguments,
            lineage: &advice.lineage,
        };
        words
            .with_source_schema(&context, |schema| {
                let first = owner
                    .retain_source_class_declaration(&mut tape, &graph, invocation, schema)
                    .unwrap();
                let slot = first.source_slot().clone();
                assert_eq!(tape.source_class_for_advice(advice), Some(first.as_ref()));
                AdviceInvocationContext::install_source_class(&mut tape, &mut graph, Some(first));
                assert!(
                    owner
                        .retain_source_class_declaration(&mut tape, &graph, invocation, schema)
                        .is_none()
                );
                assert!(
                    tape.source_class_for_advice(advice).is_none(),
                    "the same real facet cannot erase occupied-target refusal"
                );
                graph.cells.remove(&slot);
                assert!(
                    owner
                        .retain_source_class_declaration(&mut tape, &graph, invocation, schema)
                        .is_some()
                );
                assert!(
                    tape.source_class_for_advice(advice).is_none(),
                    "same-facet refusal remains terminal"
                );
                Some(())
            })
            .unwrap();
    }

    #[test]
    fn logical_class_declarations_reborrow_the_exact_source_purpose_facet() {
        use super::*;
        use crate::registry_invocation::source_structure::{
            OriginalRegistrySource, source_class_declaration_at, source_registry_words_at,
        };
        let source = "namespace eval left {oo::class create Widget {}}\nnamespace eval right {oo::class create Widget {}}\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert_eq!(analysis.all_classes.len(), 2);
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let realm = analysis.retained_command_realm().unwrap();
        for (offset, _) in source.match_indices("oo::class create") {
            let offset = u32::try_from(offset).unwrap();
            let words = source_registry_words_at(source, &analysis, offset).unwrap();
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("the authentic original Logical source schema");
            };
            let declaration = source_class_declaration_at(source, &analysis, offset)
                .expect("the actual source class issuer facet");
            assert!(declaration.logical_source_class(&analysis).is_some());
            assert!(declaration.source_class(&analysis).is_none());
            let mut tape = realm
                .source_bindings_ref()
                .original_source_transition_advice_tape(&context)
                .unwrap();
            assert_eq!(tape.source_class_for_advice(advice), Some(&declaration));
            // A generic purpose conflict cannot manufacture or erase the
            // independently retained exact source facet.
            tape.class_declarations.insert(offset, None);
            assert!(tape.source_class(offset).is_none());
            assert_eq!(tape.source_class_for_advice(advice), Some(&declaration));
            let mut changed = advice.as_ref().clone();
            changed
                .uncertain_operations
                .push(Arc::from(advice.original_words()));
            assert!(tape.source_class_for_advice(&changed).is_none());
            let mut conflicting = declaration.clone();
            conflicting.slot.simple = tcl_core_types::NameBytes::from(b"Other".as_slice());
            tape.retain_class_declaration_facet(SourceClassDeclarationFacet {
                factory: Arc::clone(&declaration.factory),
                class: Some(Arc::new(conflicting)),
            });
            assert!(tape.source_class_for_advice(advice).is_none());
            let mut tape = realm
                .source_bindings_ref()
                .original_source_transition_advice_tape(&context)
                .unwrap();
            tape.retain_class_declaration_facet(SourceClassDeclarationFacet {
                factory: Arc::clone(&declaration.factory),
                class: None,
            });
            assert!(tape.source_class_for_advice(advice).is_none());
            tape.class_declaration_facets.clear();
            assert!(tape.source_class_for_advice(advice).is_none());
            assert!(
                source_class_declaration_at(&source.replace("Widget", "Other"), &analysis, offset)
                    .is_none()
            );
        }
    }

    #[test]
    fn original_abstract_constructor_defaults_and_exports_keep_selected_native_source_grammar() {
        // naming.tcloo.original-abstract-manufacturer-public-visibility
        // docs/design/analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md
        // A Native source-name model is not a entered Native constructor receipt.
        for dialect in ["tcl9.0", "tcl9.1"] {
            let environment =
                tcl_registry::model::ingress::resolve_known_environment(dialect).unwrap();
            let profile = environment.analyser_profile();
            let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
            config.strict_quoting = false;
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                environment.default_context_registry(),
                config,
            );
            for (source, marker, visible) in [
                ("oo::abstract create Base {}; Base new", "Base new", false),
                (
                    "oo::abstract create Base {}; Base create object",
                    "Base create object",
                    false,
                ),
                (
                    "oo::abstract create Base {}; Base createWithNamespace object ::ns",
                    "Base createWithNamespace",
                    false,
                ),
                (
                    "oo::abstract create Base {}; oo::objdefine Base export new; Base new",
                    "Base new",
                    true,
                ),
                (
                    "oo::abstract create Base {}; oo::class create Concrete {superclass Base}; Concrete new",
                    "Concrete new",
                    true,
                ),
                (
                    "rename oo::abstract Factory; Factory create Base {}; Base new",
                    "Base new",
                    false,
                ),
                (
                    "interp alias {} Factory {} oo::abstract create; Factory Base {}; Base new",
                    "Base new",
                    false,
                ),
                (
                    "oo::abstract create Base {}; rename Base Moved; Moved new",
                    "Moved new",
                    false,
                ),
            ] {
                let analysis = Analyser::new()
                    .with_resolved_input(input.clone())
                    .analyse(source, dialect);
                let offset = u32::try_from(source.rfind(marker).unwrap()).unwrap();
                let call = source_constructor_call_at(source, &analysis, offset).expect(source);
                assert_eq!(
                    call.constructor_shape(&analysis).is_some(),
                    visible,
                    "{dialect}: {source}"
                );
                assert!(call.logical_constructor_shape(&analysis).is_none());
                let mut changed = analysis.clone();
                let mut changed_config = config;
                changed_config.strict_quoting = true;
                changed.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
                    profile,
                    profile,
                    input.context_registry(),
                    changed_config,
                ));
                assert!(call.constructor_shape(&changed).is_none());
                let mut missing = analysis.clone();
                missing.resolved_input = None;
                assert!(call.constructor_shape(&missing).is_none());
                assert!(
                    source_constructor_call_at(
                        &source.replace("Base", "Foreign"),
                        &analysis,
                        offset
                    )
                    .is_none()
                );
            }
        }
    }

    #[test]
    fn original_abstract_constructor_logical_advice_keeps_defaults_without_native_identity() {
        // naming.tcloo.original-abstract-manufacturer-public-visibility
        // docs/design/analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
                .unwrap()
                .default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        for (source, visible) in [
            ("oo::abstract create Base {}; Base new", false),
            (
                "oo::abstract create Base {}; oo::objdefine Base export new; Base new",
                true,
            ),
            (
                "oo::abstract create Base {}; oo::class create Concrete {superclass Base}; Concrete new",
                true,
            ),
            (
                "rename oo::abstract Factory; Factory create Base {}; Base new",
                false,
            ),
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, profile.name);
            let offset = u32::try_from(source.rfind(';').unwrap() + 2).unwrap();
            let call = source_constructor_call_at(source, &analysis, offset).expect(source);
            assert_eq!(
                call.logical_constructor_shape(&analysis).is_some(),
                visible,
                "{source}"
            );
            assert!(call.constructor_shape(&analysis).is_none());
            assert!(call.class_declaration().source_class(&analysis).is_none());
            assert!(
                call.class_declaration()
                    .logical_source_class(&analysis)
                    .is_some()
            );
            let mut missing = analysis.clone();
            missing.resolved_input = None;
            assert!(call.logical_constructor_shape(&missing).is_none());
        }
    }

    #[test]
    fn original_logical_constructor_shape_keeps_canonical_factory_without_native_authority() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        for (source, expected) in [
            ("oo::class create C {method ping {} {}}; [C new] ping", true),
            (
                "oo::class create C {method ping {} {}}; interp alias {} make {} C new; [make] ping",
                true,
            ),
            (
                "oo::class create C {self method new {} {}}; [C new] ping",
                false,
            ),
            (
                "oo::class create C {}; oo::objdefine C unexport new; [C new] ping",
                false,
            ),
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "tcl");
            let offset = u32::try_from(source.rfind('[').unwrap() + 1).unwrap();
            let call = source_constructor_call_at(source, &analysis, offset).expect(source);
            assert_eq!(
                call.logical_constructor_shape(&analysis).is_some(),
                expected,
                "{source}"
            );
            assert!(call.constructor_shape(&analysis).is_none());
            assert!(call.class_declaration().source_class(&analysis).is_none());
            assert!(
                call.class_declaration()
                    .logical_source_class(&analysis)
                    .is_some()
            );
            let mut missing = analysis.clone();
            missing.resolved_input = None;
            assert!(call.logical_constructor_shape(&missing).is_none());
            let mut unavailable = analysis.clone();
            unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
                environment: "tcl8.6".into(),
                overlay: 999,
            });
            assert!(call.logical_constructor_shape(&unavailable).is_none());
        }
    }

    #[test]
    fn original_class_constructor_call_keeps_alias_prefix_move_and_canonical_declaration() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let source = "oo::class create C {constructor {x} {}}; rename C Held; interp alias {} make {} Held create; make c value";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("make c value").unwrap()).unwrap();
        let call =
            source_constructor_call_at(source, &analysis, offset).expect("original class call");
        assert_eq!(call.original_words().len(), 3);
        assert_eq!(
            call.arguments()[0].literal_bytes(),
            Some(b"create".as_slice())
        );
        assert_eq!(call.written_argument(0), None);
        assert_eq!(
            call.argument_input(0).unwrap().original_word(),
            call.argument_word(0)
        );
        assert_eq!(call.argument_input(1).unwrap().bytes(), b"c");
        assert_eq!(call.written_argument(1), Some(0));
        assert_eq!(call.class_declaration().lineage().len(), 2);
        assert_eq!(call.class_declaration().name_input().bytes(), b"C");
        assert_eq!(
            call.class_declaration().source_slot().simple.as_bytes(),
            b"Held"
        );
        let class = call.class_declaration().source_class(&analysis).unwrap();
        assert_eq!(class.name_input().bytes(), b"C");
        assert_eq!(
            call.constructor_shape(&analysis)
                .unwrap()
                .constructor_args_from(),
            2
        );
        assert!(
            source_constructor_call_at(&source.replace("value", "other"), &analysis, offset)
                .is_none()
        );
    }

    #[test]
    fn original_constructor_substitution_keeps_whole_child_words_and_selected_body() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        for source in [
            "oo::class create C {constructor {x} {}}; set o [C new value]",
            "oo::class create C {constructor {x} {}}; interp alias {} make {} C new; set o [make value]",
            "proc p {} {set o [C new value]}; oo::class create C {constructor {x} {}}; p",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(
                source
                    .find("[C new")
                    .or_else(|| source.find("[make value"))
                    .unwrap()
                    + 1,
            )
            .unwrap();
            let call = source_constructor_call_at(source, &analysis, offset).expect(source);
            assert_eq!(call.arguments()[0].literal_bytes(), Some(b"new".as_slice()));
            assert_eq!(
                call.arguments()[1].literal_bytes(),
                Some(b"value".as_slice())
            );
            assert_eq!(
                call.constructor_shape(&analysis)
                    .unwrap()
                    .constructor_args_from(),
                1
            );
            assert_eq!(
                call.class_declaration()
                    .source_class(&analysis)
                    .unwrap()
                    .name_input()
                    .bytes(),
                b"C"
            );
            if source.starts_with("proc") {
                assert!(call.original_source_body().is_some());
                assert!(call.obligations().contains(
                    &super::SourceCommandTransitionObligation::OriginalProcedureBodyApplicability
                ));
            }
            assert!(
                source_constructor_call_at(&source.replace("value", "other"), &analysis, offset)
                    .is_none()
            );
        }
    }

    #[test]
    fn original_constructor_substitution_refuses_compound_words_and_multiple_commands() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        for source in [
            "oo::class create C {}; set o prefix[C new value]",
            "oo::class create C {}; set o [C new value; puts other]",
            "oo::class create C {}; rename C {}; set o [C new value]",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find("[C new").unwrap() + 1).unwrap();
            assert!(
                source_constructor_call_at(source, &analysis, offset).is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_logical_constructor_calls_keep_real_deferred_and_immediate_parent_bodies() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        for (source, obligation) in [
            (
                "oo::class create C {}; proc p {} {set o [C new value]}",
                super::SourceCommandTransitionObligation::DeferredLogicalBodyApplicability,
            ),
            (
                "oo::class create C {}; if {1} {set o [C new value]}",
                super::SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
            ),
            (
                "proc p {} {oo::class create C {}; set o [C new value]}",
                super::SourceCommandTransitionObligation::DeferredLogicalBodyApplicability,
            ),
        ] {
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::context_for_profile(profile),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            let analysis = Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            let offset = u32::try_from(source.find("[C new").unwrap() + 1).unwrap();
            let call = source_constructor_call_at(source, &analysis, offset).expect(source);
            assert!(call.obligations().contains(&obligation));
            let body = call
                .original_source_body()
                .expect("genuine selected enclosing body");
            assert!(body.content_span().start() <= offset);
            assert!(
                call.original_words()
                    .iter()
                    .all(|word| word.span().end() <= body.content_span().end())
            );
            assert!(
                call.class_declaration()
                    .name_input()
                    .native_input()
                    .is_none()
            );
            assert!(call.constructor_shape(&analysis).is_none());
        }
    }

    #[test]
    fn original_source_class_keeps_conditional_factory_after_unknown_call_but_known_replacement_blocks()
     {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let source = "oo::class create C {constructor {x} { }}; C new one; C new two";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        for (ordinal, marker) in ["C new one", "C new two"].into_iter().enumerate() {
            let offset = u32::try_from(source.find(marker).unwrap()).unwrap();
            let call = source_constructor_call_at(source, &analysis, offset).expect(marker);
            assert_eq!(
                call.constructor_shape(&analysis)
                    .unwrap()
                    .constructor_args_from(),
                1
            );
            if ordinal == 1 {
                assert!(
                    call.obligations().contains(
                        &super::SourceCommandTransitionObligation::UnknownEarlierMutation
                    )
                );
            }
        }
        for source in [
            "oo::class create C {}; C new one; proc C args {}; C new two",
            "oo::class create C {}; C new one; rename C {}; C new two",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find("C new two").unwrap()).unwrap();
            assert!(
                source_constructor_call_at(source, &analysis, offset).is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_constructor_manufacturer_visibility_keeps_call_source_horizon() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        for (source, selector, visible) in [
            (
                "oo::class create C {}; C createWithNamespace obj ::ns value; oo::objdefine C export createWithNamespace",
                "C createWithNamespace",
                false,
            ),
            (
                "oo::class create C {}; C new value; oo::objdefine C unexport new",
                "C new",
                true,
            ),
            (
                "oo::class create C {}; oo::objdefine C unexport new; C new value; oo::objdefine C export new",
                "C new",
                false,
            ),
            (
                "oo::class create C {}; C new value; oo::objdefine C method new args {return replacement}",
                "C new",
                true,
            ),
            (
                "oo::class create C {}; oo::objdefine C method new args {return replacement}; C new value",
                "C new",
                false,
            ),
            (
                "oo::class create C {}; proc p {} {C createWithNamespace obj ::ns value}; oo::objdefine C export createWithNamespace",
                "C createWithNamespace",
                true,
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find(selector).unwrap()).unwrap();
            let call = source_constructor_call_at(source, &analysis, offset).expect(source);
            assert_eq!(
                call.constructor_shape(&analysis).is_some(),
                visible,
                "{source}"
            );
        }
    }

    #[test]
    fn original_class_constructor_call_refuses_known_factory_or_class_replacement_and_cycles() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        for source in [
            "proc oo::class args {}; oo::class create C {}; C new value",
            "oo::class create C {}; proc C args {}; C new value",
            "oo::class create C {}; rename C {}; C new value",
            "oo::class create C {}; interp alias {} again {} again; again new value",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind(';').unwrap() + 2).unwrap();
            assert!(
                source_constructor_call_at(source, &analysis, offset).is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_class_constructor_call_preserves_expansion_and_refuses_private_or_overridden_makers()
     {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let source = "oo::class create C {constructor args {}}; C new {*}$values";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("C new").unwrap()).unwrap();
        let call = source_constructor_call_at(source, &analysis, offset).unwrap();
        assert_eq!(
            call.arguments()[1],
            crate::registry_invocation::EffectiveInvocationWord::Expanded
        );
        assert_eq!(
            call.constructor_shape(&analysis)
                .unwrap()
                .constructor_args_from(),
            1
        );
        for source in [
            "oo::class create C {}; C createWithNamespace c ::NS value",
            "oo::class create C {self method create args {}}; C create c value",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind(';').unwrap() + 2).unwrap();
            let call = source_constructor_call_at(source, &analysis, offset).unwrap();
            assert!(call.constructor_shape(&analysis).is_none(), "{source}");
        }
    }

    #[test]
    fn original_source_class_family_uses_selected_definer_not_metaclass_reporting_label() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let source = "rename oo::class Maker; Maker create C {}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let record = analysis.original_class_declarations().next().unwrap();
        let receipt =
            super::OriginalSourceClassDeclaration::from_class(source, &analysis, record).unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert_eq!(
            receipt.grammar(&context).unwrap().family,
            tcl_registry::definer::DefinerFamily::TclOo
        );
        assert_eq!(receipt.factory().command(), "oo::class");
        assert_eq!(receipt.factory().lineage().len(), 1);
    }

    #[test]
    fn original_snit_constructor_call_retains_explicit_and_bare_provider_layouts() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        for (invocation, from) in [("T create t value", 2), ("T t value", 1)] {
            let source = format!(
                "package require snit; snit::type T {{constructor {{x}} {{}}}}; {invocation}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl8.6");
            let offset = u32::try_from(source.find(invocation).unwrap()).unwrap();
            let call = source_constructor_call_at(&source, &analysis, offset).expect(invocation);
            assert_eq!(
                call.constructor_shape(&analysis)
                    .unwrap()
                    .constructor_args_from(),
                from
            );
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            assert_eq!(
                call.class_declaration().grammar(&context).unwrap().family,
                tcl_registry::definer::DefinerFamily::Snit
            );
        }
    }
    #[test]
    fn original_native_class_names_keep_distinct_source_units_without_ascii_report_gate() {
        // docs/design/analysis/name-resolution-proofs/corner-command-distinct-unicode.md

        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        // Native Unicode name observation is separate: naming.corner.command-distinct-unicode.
        // These assertions check source producer/publication geometry, not execution.
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = "oo::class create é {}; oo::class create e\u{301} {}; é new; e\u{301} new";
            let analysis = Analyser::new().analyse(source, dialect);
            let calls: Vec<_> = ["é new", "e\u{301} new"]
                .into_iter()
                .map(|marker| {
                    source_constructor_call_at(
                        source,
                        &analysis,
                        u32::try_from(source.find(marker).unwrap()).unwrap(),
                    )
                    .unwrap()
                })
                .collect();
            assert_ne!(
                calls[0].class_declaration().source_slot(),
                calls[1].class_declaration().source_slot()
            );
            assert_eq!(
                calls[0].class_declaration().name_input().bytes(),
                "é".as_bytes()
            );
            assert_eq!(
                calls[1].class_declaration().name_input().bytes(),
                "e\u{301}".as_bytes()
            );
            for call in calls {
                assert!(
                    call.class_declaration()
                        .name_input()
                        .native_input()
                        .is_some()
                );
                assert!(call.class_declaration().source_class(&analysis).is_some());
            }
        }
        let source = "class {Base α} {}; class Derived {{Base α}} {}";
        let analysis = Analyser::new().analyse(source, "jim");
        let class = crate::registry_invocation::source_structure::source_class_declaration_at(
            source, &analysis, 0,
        )
        .unwrap();
        assert_eq!(class.name_input().bytes(), "Base α".as_bytes());
        assert!(class.source_class(&analysis).is_some());
    }
    #[test]
    fn original_factory_attempt_keeps_source_incarnation_without_constructor_visibility() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        for (tail, selector, name) in [
            ("Base new", "new", b"Base".as_slice()),
            ("Base create object", "create", b"Base".as_slice()),
            (
                "Base createWithNamespace object ::ns",
                "createWithNamespace",
                b"Base".as_slice(),
            ),
            ("set result [Base new]", "new", b"Base".as_slice()),
            (
                "interp alias {} make {} Base new; make",
                "new",
                b"Base".as_slice(),
            ),
            ("rename Base Moved; Moved new", "new", b"Moved".as_slice()),
        ] {
            let source = format!("oo::abstract create Base {{}}; {tail}");
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(&source, "tcl9.0");
            let marker = if tail == "set result [Base new]" {
                "Base new"
            } else if tail.starts_with("interp alias") {
                "make"
            } else if tail.starts_with("rename") {
                "Moved new"
            } else {
                tail
            };
            let offset = u32::try_from(source.rfind(marker).unwrap()).unwrap();
            let call = source_constructor_call_at(&source, &analysis, offset).expect(tail);
            let attempt = call.source_factory_attempt(&analysis).expect(tail);
            assert_eq!(attempt.operation().keyword, selector);
            assert_eq!(
                attempt.class_declaration().source_slot().simple.as_bytes(),
                name
            );
            assert!(
                attempt
                    .factory_traits()
                    .contains(tcl_registry::Traits::ABSTRACT_CLASS_FACTORY)
            );
            assert!(
                call.constructor_shape(&analysis).is_none(),
                "source attempt grants no Native constructor"
            );
            let mut missing = analysis.clone();
            missing.resolved_input = None;
            assert!(call.source_factory_attempt(&missing).is_none());
            let mut changed = analysis.clone();
            let mut config = input.lexer_config();
            config.strict_quoting ^= true;
            changed.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                input.context_registry(),
                config,
            ));
            assert!(call.source_factory_attempt(&changed).is_none());
        }
        for mutation in [
            "proc Base args {return replacement}",
            "oo::objdefine Base method new args {return custom}",
            "oo::objdefine Base export new",
            "unknownMutation",
        ] {
            let source = format!("oo::abstract create Base {{}}; {mutation}; Base new");
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(&source, "tcl9.0");
            let offset = u32::try_from(source.rfind("Base new").unwrap()).unwrap();
            let call = source_constructor_call_at(&source, &analysis, offset);
            if mutation == "unknownMutation" {
                assert!(
                    call.is_some(),
                    "the original conditional call remains retained"
                );
            }
            assert!(
                call.as_ref()
                    .and_then(|call| call.source_factory_attempt(&analysis))
                    .is_none(),
                "{mutation}"
            );
        }
    }
}

impl OriginalSourceClassDeclaration {
    /// Shared conditional display calculation after independent original class
    /// and caller-word correspondence. No receiver, execution or allocation.
    pub(crate) fn display_constructor_shape(
        &self,
        class: &ClassDef,
        selector: &SourceAdviceNameInput,
        context: &ContextRegistry,
        before: Option<&super::super::CommandAllocationSite>,
    ) -> Option<OriginalSourceConstructorShape> {
        let bytes = selector.bytes();
        let word = std::str::from_utf8(bytes).ok()?;
        if word.contains('\0') {
            return None;
        }
        let grammar = self.grammar(context)?;
        if self.name_input().logical_input().is_some() {
            selector.logical_input()?;
            if class.class_methods.contains_key(word) {
                return None;
            }
            if let Some(method) = grammar.manufacturer(word) {
                let visible = !class.class_unexports.contains(word)
                    && (method.visibility == MemberVisibility::Exported
                        || class.class_exports.contains(word));
                return visible.then_some(OriginalSourceConstructorShape::Method(method));
            }
            let name_at = grammar.conditional_construction_name_at(
                word,
                class.class_methods.keys().map(String::as_str),
            )?;
            return (name_at == 0).then_some(OriginalSourceConstructorShape::BareWord {
                constructor_args_from: 1,
            });
        }
        if !bytes.is_ascii() {
            return None;
        }
        let input = selector.native_input()?;
        (input.policy() == self.name_input().native_input()?.policy()).then_some(())?;
        let members = &class.original_members;
        let methods = match before {
            Some(call) => members
                .methods_before_source_call(crate::analyser::MemberSide::ClassObject, call)?,
            None => members.methods(crate::analyser::MemberSide::ClassObject)?,
        };
        let names = methods
            .iter()
            .map(|method| method.original_name_input().bytes())
            .collect::<Vec<_>>();
        if names.iter().any(|name| *name == bytes) {
            return None;
        }
        if let Some(method) = grammar.manufacturer(word) {
            let exported = method.visibility == MemberVisibility::Exported;
            let visible = match before {
                Some(call) => members.inherited_visibility_before_source_call(
                    crate::analyser::MemberSide::ClassObject,
                    input,
                    exported,
                    call,
                )?,
                None => members.inherited_visibility_for_input(
                    crate::analyser::MemberSide::ClassObject,
                    input,
                    exported,
                )?,
            };
            return visible.then_some(OriginalSourceConstructorShape::Method(method));
        }
        let type_methods = names
            .iter()
            .map(|name| std::str::from_utf8(name))
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        let name_at =
            grammar.conditional_construction_name_at(word, type_methods.iter().copied())?;
        (name_at == 0).then_some(OriginalSourceConstructorShape::BareWord {
            constructor_args_from: 1,
        })
    }
}
