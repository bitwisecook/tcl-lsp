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

type SourceClassDeclarationOwner<'a> = (
    &'a Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    SourceCommandTransitionObligation,
    Option<&'a Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>>,
);

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
/// Whole original `Class::proc` call whose qualifier names an exact original
/// [incr Tcl] source class (user contract D: a registry-dispatched definer
/// publishes its class procs as `::Class::proc` commands). Itcl installs each
/// class-scoped `proc` as an ordinary command in the class's namespace, so the
/// qualified spelling dispatches it; the two-word `Class name` form is
/// instance creation instead. This joins source syntax to the declaration
/// only: whether the class body declares that member, and whether the call
/// executes, remain the consumer's and the runtime's facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassProcCall {
    site: super::super::CommandAllocationSite,
    original: Arc<[NativeWord]>,
    class: Arc<OriginalSourceClassDeclaration>,
    member: Box<[u8]>,
    member_span: tcl_lexer::Span,
}
impl OriginalSourceClassProcCall {
    /// Exact written call including its head.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Original occurrence, without a fabricated runtime lookup point.
    #[must_use]
    pub const fn site(&self) -> &super::super::CommandAllocationSite {
        &self.site
    }
    /// Canonical original class definer and naming operand of the qualifier.
    #[must_use]
    pub fn class_declaration(&self) -> &OriginalSourceClassDeclaration {
        &self.class
    }
    /// The written member name: the head's final `::` segment.
    #[must_use]
    pub fn member_name(&self) -> &[u8] {
        &self.member
    }
    /// Source span of [`Self::member_name`] inside the written head.
    #[must_use]
    pub const fn member_span(&self) -> tcl_lexer::Span {
        self.member_span
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
            && self.class.factory.matches_source(image, config)
            && self.class.factory.matches_context(context)
    }
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
    #[cfg(test)]
    fn trace_source_factory_quiet(&self, trace_enabled: bool, trace: &impl Fn(&str, bool)) {
        if trace_enabled {
            trace(
                "attempt.quiet",
                !self
                    .obligations
                    .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
                    && self.class.factory.uncertain_operations().is_empty(),
            );
        }
    }

    fn source_factory_native_methods(
        &self,
        members: &crate::analyser::types::OriginalSourceMemberLedger,
    ) -> Option<Vec<crate::analyser::types::OriginalSourceMethodMetadata>> {
        let deferred = self
            .obligations
            .contains(&SourceCommandTransitionObligation::OriginalProcedureBodyApplicability)
            || self
                .obligations
                .contains(&SourceCommandTransitionObligation::DeferredLogicalBodyApplicability);
        if deferred {
            members.methods(crate::analyser::MemberSide::ClassObject)
        } else {
            members
                .methods_before_source_call(crate::analyser::MemberSide::ClassObject, self.site())
        }
    }

    #[cfg(test)]
    fn trace_source_factory_guard(&self, stage: &str, passed: bool) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some() {
            let offset = self.site.offset;
            eprintln!("ORIGINAL_FACTORY_GUARD stage={stage} offset={offset} passed={passed}");
        }
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
        let trace_enabled =
            tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some();
        #[cfg(test)]
        let trace = |stage: &str, passed: bool| self.trace_source_factory_guard(stage, passed);
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
        self.trace_source_factory_quiet(trace_enabled, &trace);
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
            let methods = self.source_factory_native_methods(members);
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
        self.resolve_source_class_bytes(head.bytes())
    }

    fn resolve_source_class_bytes(
        &self,
        head: &[u8],
    ) -> Option<(Arc<OriginalSourceClassDeclaration>, Vec<SourceAdviceWord>)> {
        let mut key = crate::registry_invocation::source_structure::trace_jim_source_option(
            "constructor.selected_head_key",
            self.selected_head_key(head),
        )?;
        let mut seen = Vec::new();
        let mut prefix = Vec::new();
        let mut lineage = Vec::new();
        loop {
            if seen.contains(&key) {
                return None;
            }
            seen.push(key.clone());
            let cell = self.cells.get(&key);
            #[cfg(debug_assertions)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_JIM_SOURCE_OWNER").is_some() {
                let kind = cell.map(|cell| match cell {
                    AdviceCell::SourceClass(_) => "SourceClass",
                    AdviceCell::AuthoredJimClass(_) => "AuthoredJimClass",
                    AdviceCell::Alias { .. } => "Alias",
                    _ => "Other",
                });
                eprintln!(
                    "JIM_SOURCE_OWNER stage=constructor.graph_cell head={head:?} kind={kind:?}"
                );
            }
            match cell? {
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
    pub(crate) fn class_proc_call(&self, offset: u32) -> Option<&OriginalSourceClassProcCall> {
        self.class_proc_calls.get(&offset)?.as_ref()
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
    pub(super) fn extend_class_proc_calls(
        &mut self,
        calls: BTreeMap<u32, Option<OriginalSourceClassProcCall>>,
    ) {
        for (offset, receipt) in calls {
            merge_receipt(&mut self.class_proc_calls, offset, receipt);
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

    fn source_class_slot(
        &self,
        graph: &AdviceGraph,
        family: DefinerFamily,
        name: &[u8],
    ) -> Option<ByteCommandSlot> {
        let slot = if family == DefinerFamily::TclOo {
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
                        .oo_object_publication_slot(context, name)
                        .ok()?
                }
                AdviceNamingPolicy::Logical(_) => {
                    graph.publication_key(name, AdvicePublicationPurpose::Define)?
                }
            }
        } else {
            graph.publication_key(name, AdvicePublicationPurpose::Define)?
        };
        // A `TclOO` object creates its missing qualifier namespaces; every
        // other definer publishes into an existing one.
        let creates_namespace =
            family == DefinerFamily::TclOo && matches!(self.policy, AdviceNamingPolicy::Native(_));
        if (!creates_namespace && !graph.namespaces.contains(&slot.namespace))
            || graph
                .cells
                .get(&slot)
                .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
        {
            return None;
        }
        Some(slot)
    }

    fn source_class_declaration_attempt(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        owner: Option<SourceClassDeclarationOwner<'_>>,
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
            let slot = self.source_class_slot(graph, grammar.family, name.bytes())?;
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
            let mut namespace = Some(class.slot.namespace.clone());
            while let Some(path) = namespace {
                namespace = path.parent();
                graph.namespaces.insert(path);
            }
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
            if body.is_some()
                && graph
                    .selected_head_key(
                        written
                            .first()
                            .and_then(|word| word.input.as_ref())
                            .map_or(&[], |head| head.bytes()),
                    )
                    .and_then(|key| graph.cells.get(&key))
                    .is_some_and(|cell| matches!(cell, AdviceCell::AuthoredJimClass(_)))
            {
                continue;
            }
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
        if graph
            .selected_head_key(
                written
                    .first()
                    .and_then(|word| word.input.as_ref())
                    .map_or(&[], |head| head.bytes()),
            )
            .and_then(|key| graph.cells.get(&key))
            .is_some_and(|cell| matches!(cell, AdviceCell::AuthoredJimClass(_)))
        {
            return false;
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
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_JIM_SOURCE_OWNER").is_some() {
            eprintln!(
                "JIM_SOURCE_OWNER stage=constructor.retain_attempt offset={:?} words={}",
                native.first().map(|word| word.span().start()),
                native.len()
            );
        }
        let Some(head) = crate::registry_invocation::source_structure::trace_jim_source_option(
            "constructor.written_head",
            written.first().and_then(|word| word.input.as_ref()),
        ) else {
            return false;
        };
        if self.retain_authored_jim_call(tape, graph, native, written) {
            return true;
        }
        let Some((class, mut arguments)) =
            crate::registry_invocation::source_structure::trace_jim_source_option(
                "constructor.source_class",
                graph.resolve_source_class(head),
            )
        else {
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

    /// Join a static `Qualifier::member` head to the [incr Tcl] source class
    /// its qualifier names. The qualifier resolves as an ordinary command
    /// name, exactly as Tcl resolves the namespace part of a qualified command
    /// (current namespace, then global); an alias prefix or a non-Itcl class
    /// supplies no class-proc dispatch.
    pub(super) fn retain_source_class_proc_call(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        native: &[NativeWord],
        written: &[SourceAdviceWord],
    ) -> bool {
        let Some(head) = written.first().and_then(|word| word.input.as_ref()) else {
            return false;
        };
        let Some(word) = native.first() else {
            return false;
        };
        let bytes = head.bytes();
        // The member span is a byte suffix of the head only when the written
        // word is its own unescaped, unquoted value.
        if word.group().expand
            || usize::try_from(word.span().end() - word.span().start()).ok() != Some(bytes.len())
        {
            return false;
        }
        let Some(separator) = tcl_syntax::naming::written_separator_extent(bytes) else {
            return false;
        };
        let member = &bytes[separator.end..];
        let qualifier = &bytes[..separator.start];
        if member.is_empty() || member.contains(&b':') || qualifier.is_empty() {
            return false;
        }
        // A command already occupying (or possibly occupying) the qualified
        // name itself wins: `proc Factory::make` written in the class's
        // namespace replaces the class proc's command there.
        if graph.lookup_head_key(bytes).is_some_and(|key| {
            graph.possible_occupants.contains(&key)
                || graph
                    .cells
                    .get(&key)
                    .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
        }) {
            return false;
        }
        let Some((class, prefix)) = graph.resolve_source_class_bytes(qualifier) else {
            return false;
        };
        if !prefix.is_empty()
            || class
                .grammar(self.context)
                .is_none_or(|grammar| grammar.family != DefinerFamily::Itcl)
        {
            return false;
        }
        let Ok(member_len) = u32::try_from(member.len()) else {
            return false;
        };
        let receipt = OriginalSourceClassProcCall {
            site: super::super::CommandAllocationSite {
                source: Arc::clone(self.origin),
                offset: word.span().start(),
            },
            original: Arc::from(native),
            class,
            member: member.into(),
            member_span: tcl_lexer::Span::new(word.span().end() - member_len, word.span().end()),
        };
        merge_receipt(
            &mut tape.class_proc_calls,
            receipt.site.offset,
            Some(receipt),
        );
        true
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
            // Itcl class-scoped procs dispatch only as `Class::proc`; the
            // two-word form constructs even when the word names one
            // (`DefinitionBodyGrammar::conditional_construction_name_at`).
            if grammar.family != DefinerFamily::Itcl && class.class_methods.contains_key(word) {
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
        if grammar.family != DefinerFamily::Itcl && names.contains(&bytes) {
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

#[cfg(test)]
mod tests;
