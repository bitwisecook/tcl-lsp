// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional Jim class headers and base relations from the original source tape.
//! No class allocation, Native argument/frame, receiver, or runtime publication is issued.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    AdvicePublicationPurpose, OriginalSourceCommandTransition,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    source_class::merge_receipt,
};
use crate::analyser::{AnalysisResult, ClassDef, ResolvedAnalysisInput};
use std::sync::Arc;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Token};
use tcl_registry::model::ContextRegistry;
use tcl_syntax::{
    naming::{NamePolicyAuthority, NamePolicyProtocol, NativeNameProtocol},
    word_rules::OriginalSourceListElement,
};

/// These source purposes retain independent issuers; neither grants Native authority.
#[derive(Debug, Clone, PartialEq, Eq)]
enum JimClassSourcePurpose {
    Authored(NamePolicyProtocol),
    Logical(ResolvedAnalysisInput),
}

impl JimClassSourcePurpose {
    fn list_fields(&self, word: &NativeWord) -> Option<Vec<OriginalSourceListElement>> {
        match self {
            Self::Authored(policy) => jim_list_fields(word, *policy),
            Self::Logical(input) => {
                (input.has_logical_source_name_context() && word.config() == input.lexer_config())
                    .then_some(())?;
                tcl_syntax::word_rules::original_static_word_list_elements(word)
            }
        }
    }
    fn matches(&self, config: LexerConfig, context: &ContextRegistry) -> bool {
        match self {
            Self::Authored(policy) => {
                policy.authority() == NamePolicyAuthority::AuthoredSimulation
                    && policy.recipe() == NativeNameProtocol::Jim084
            }
            Self::Logical(input) => {
                input.has_logical_source_name_context()
                    && input.lexer_config() == config
                    && std::ptr::eq(input.borrowed_context_registry(), context)
            }
        }
    }
}

/// A source list child and its independently selected conditional source provider.
/// Missing providers remain unavailable; a reporting class name is never queried.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthoredJimBase {
    original: OriginalSourceListElement,
    provider: Option<Arc<OriginalAuthoredJimClass>>,
}

/// Original conditional Jim class syntax, independently of Native class identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalAuthoredJimClass {
    factory: Arc<OriginalSourceCommandTransitionAdvice>,
    purpose: JimClassSourcePurpose,
    slot: ByteCommandSlot,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
    parent_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    variables_at: usize,
    variables: Option<Vec<OriginalSourceListElement>>,
    // None is a computed/unavailable base list; Some(empty) is genuine no bases.
    bases: Option<Vec<AuthoredJimBase>>,
}

impl OriginalAuthoredJimClass {
    /// Genuine selected source schema and original complete vector, with all debts.
    #[must_use]
    pub fn factory(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.factory
    }

    /// Authentic containing source body, without an entered body/frame grant.
    #[must_use]
    pub fn parent_body(&self) -> Option<&crate::registry_invocation::OriginalSourceScriptBody> {
        self.parent_body.as_deref()
    }

    /// Original source dictionary fields; unavailable never becomes an empty proof.
    #[must_use]
    pub fn variable_fields(&self) -> Option<&[OriginalSourceListElement]> {
        self.variables.as_deref()
    }

    /// Exact authored base names, separately from provider/class existence.
    #[must_use]
    pub fn base_fields(&self) -> Option<Vec<&OriginalSourceListElement>> {
        Some(
            self.bases
                .as_ref()?
                .iter()
                .map(|base| &base.original)
                .collect(),
        )
    }

    /// Conditional source provider at this original base-list ordinal.
    /// This historical source relation supplies no Native class or receiver.
    #[must_use]
    pub fn base_provider(&self, ordinal: usize) -> Option<&Self> {
        self.bases.as_ref()?.get(ordinal)?.provider.as_deref()
    }

    /// A computed or unsupported original base-list value remains unknown.
    #[must_use]
    pub const fn bases_unknown(&self) -> bool {
        self.bases.is_none()
    }

    /// Original argument text/tokens join, including dynamic words without evaluating them.
    pub(crate) fn matches_arguments(&self, args: &[String], tokens: &[Token]) -> bool {
        let originals = self.factory.arguments();
        originals.len() == args.len()
            && tokens.len() == args.len()
            && originals
                .iter()
                .zip(args.iter().zip(tokens))
                .all(|(original, (text, token))| {
                    original_argument_matches(&original.original, text, *token)
                })
    }

    pub(crate) fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.purpose.matches(config, context)
            && self.factory.matches_source(image, config)
            && self.factory.matches_context(context)
            && self.parent_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config)
                    && body.matches_context(context)
                    && self.factory.original_words().iter().all(|word| {
                        word.span().start() >= body.content_span().start()
                            && word.span().end() <= body.content_span().end()
                    })
            })
            && self.lineage.iter().all(|step| {
                step.original_words()
                    .iter()
                    .all(|word| word.image() == image && word.config() == config)
            })
    }

    /// Conditional inherited metadata from the source providers retained at this parent.
    /// Original metadata correspondence does not prove execution or a class object.
    #[must_use]
    pub fn inherited_variables(&self, analysis: &AnalysisResult) -> Vec<String> {
        let mut variables = Vec::new();
        for base in self.bases.iter().flatten() {
            let Some(class) = base
                .provider
                .as_ref()
                .and_then(|owner| owner.metadata(analysis))
            else {
                continue;
            };
            for variable in &class.variables {
                if !variables.contains(variable) {
                    variables.push(variable.clone());
                }
            }
        }
        variables
    }

    fn metadata<'a>(&self, analysis: &'a AnalysisResult) -> Option<&'a ClassDef> {
        let input = analysis.resolved_input.as_ref()?;
        let image = self.factory.original_words().first()?.image();
        if !self.matches_source_context(image, input.lexer_config(), &input.context_registry())
            || !analysis.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let retained = analysis
            .retained_command_realm()?
            .original_authored_jim_class(input, self.factory.original_words())?;
        (retained.factory == self.factory && retained.purpose == self.purpose).then_some(())?;
        let name_span = self
            .factory
            .arguments()
            .first()?
            .original
            .tokens()
            .first()?
            .span;
        let body_span = self
            .factory
            .arguments()
            .get(self.variables_at)?
            .original
            .tokens()
            .first()?
            .span;
        let mut matches = analysis.all_classes.values().filter(|class| {
            class.metaclass_provenance == crate::analyser::MetaclassProvenance::Observed
                && class.name_span == name_span
                && class.body_span == body_span
        });
        let class = matches.next()?;
        matches.next().is_none().then_some(class)
    }

    fn statically_complete(&self) -> bool {
        self.variables
            .as_ref()
            .is_some_and(|fields| fields.len().is_multiple_of(2))
            && self
                .bases
                .as_ref()
                .is_some_and(|bases| bases.iter().all(|base| base.provider.is_some()))
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

fn original_argument_matches(word: &NativeWord, text: &str, token: Token) -> bool {
    let Ok(source) = word.try_text() else {
        return false;
    };
    let mut projection = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        word.word_span().start(),
        word.config().nested(),
    );
    if projection.len() != 1 {
        return false;
    }
    let original = projection.pop().expect("one original source word");
    original.argv == [token]
        && original.texts == [text]
        && original.all_tokens == word.tokens()
        && !word.group().expand
}

fn jim_list_fields(
    word: &NativeWord,
    policy: NamePolicyProtocol,
) -> Option<Vec<OriginalSourceListElement>> {
    let fields = tcl_syntax::word_rules::original_static_word_list_elements(word)?;
    let words = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        std::slice::from_ref(word),
        policy.string_protocol(),
    )
    .ok()?;
    let native =
        tcl_syntax::list::split_native_list_bytes(words.literal(0)?, policy.string_protocol())
            .ok()?;
    (fields.len() == native.len()
        && fields
            .iter()
            .zip(native)
            .all(|(source, value)| source.value().as_bytes() == value.as_ref()))
    .then_some(fields)
}

impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn authored_jim_class(&self, offset: u32) -> Option<&Arc<OriginalAuthoredJimClass>> {
        self.authored_jim_classes.get(&offset)?.as_ref()
    }
}

impl AdviceInvocationContext<'_> {
    fn selected_jim_source_purpose(
        &self,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<(
        JimClassSourcePurpose,
        tcl_registry::resolved_invocation::AuthoredSourceJimClassLists,
    )> {
        use tcl_registry::source_name_ownership::SourceRolePurpose;
        let purpose = match self.policy {
            AdviceNamingPolicy::Native(policy) => {
                let layout = schema.authored_source_jim_class_lists(SourceRolePurpose::Original)?;
                (layout.policy == Some(*policy)).then_some(())?;
                JimClassSourcePurpose::Authored(*policy)
            }
            AdviceNamingPolicy::Logical(input) => {
                input.has_logical_source_name_context().then_some(())?;
                (input.lexer_config() == self.config
                    && std::ptr::eq(input.borrowed_context_registry(), self.context))
                .then_some(())?;
                schema.authored_source_jim_class_lists(SourceRolePurpose::Logical)?;
                JimClassSourcePurpose::Logical(input.clone())
            }
        };
        let role = match &purpose {
            JimClassSourcePurpose::Authored(_) => SourceRolePurpose::Original,
            JimClassSourcePurpose::Logical(_) => SourceRolePurpose::Logical,
        };
        let layout = schema.authored_source_jim_class_lists(role)?;
        Some((purpose, layout))
    }

    pub(super) fn authored_jim_class_source(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        parent: Option<&Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    ) -> Option<Arc<OriginalAuthoredJimClass>> {
        let (purpose, layout) = self.selected_jim_source_purpose(schema)?;
        if graph.native_baseline || invocation.native.iter().any(|word| word.group().expand) {
            return None;
        }
        let parent_body = parent.or(graph.procedure_body.as_ref());
        if let Some(body) = parent_body
            && (!body.matches_source(self.origin.source_image(), self.config)
                || !body.matches_context(self.context)
                || invocation.native.iter().any(|word| {
                    word.span().start() < body.content_span().start()
                        || word.span().end() > body.content_span().end()
                }))
        {
            return None;
        }
        let mut factory = self.schema_advice(invocation, graph, schema)?;
        if parent_body.is_some() {
            factory.obligations.push(super::SourceCommandTransitionObligation::ConditionalOriginalBodyCatalogueApplicability);
        }
        let factory = Arc::new(factory);
        let name = factory.arguments().first()?.value.as_deref()?;
        if name.is_empty() || name.contains(&0) {
            return None;
        }
        let slot = graph.publication_key(name, AdvicePublicationPurpose::Define)?;
        let variables = purpose.list_fields(&factory.arguments().get(layout.variables)?.original);
        let bases = layout.bases.map_or_else(
            || Some(Vec::new()),
            |ordinal| {
                let fields = purpose.list_fields(&factory.arguments().get(ordinal)?.original)?;
                Some(
                    fields
                        .into_iter()
                        .map(|original| {
                            let provider = graph
                                .selected_head_key(original.value().as_bytes())
                                .and_then(|key| match graph.cells.get(&key) {
                                    Some(AdviceCell::AuthoredJimClass(class))
                                        if class.purpose == purpose
                                            && class.parent_body.as_ref() == parent_body =>
                                    {
                                        Some(Arc::clone(class))
                                    }
                                    _ => None,
                                });
                            AuthoredJimBase { original, provider }
                        })
                        .collect(),
                )
            },
        );
        Some(Arc::new(OriginalAuthoredJimClass {
            factory,
            purpose,
            slot,
            lineage: Vec::new(),
            parent_body: parent_body.cloned(),
            variables_at: layout.variables,
            variables,
            bases,
        }))
    }

    pub(super) fn install_authored_jim_class(
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        class: Option<Arc<OriginalAuthoredJimClass>>,
    ) {
        let Some(class) = class else {
            return;
        };
        merge_receipt(
            &mut tape.authored_jim_classes,
            class.factory.site().offset,
            Some(Arc::clone(&class)),
        );
        let cell = if class.statically_complete() {
            AdviceCell::AuthoredJimClass(Arc::clone(&class))
        } else {
            // A new original attempt withdraws an earlier source provider even
            // when its replacement dictionary or bases are unavailable.
            AdviceCell::Shadowed
        };
        graph.cells.insert(class.slot.clone(), cell);
    }
}

/// Actual selected conditional Jim class call; no callable class or allocation grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalAuthoredJimClassCall {
    original: Arc<[NativeWord]>,
    class: Arc<OriginalAuthoredJimClass>,
    obligations: Vec<super::SourceCommandTransitionObligation>,
}
impl OriginalAuthoredJimClassCall {
    /// Complete original call words, retaining their image, grouping and grammar.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Authentic conditional source class selected by this original graph lookup.
    #[must_use]
    pub fn class(&self) -> &OriginalAuthoredJimClass {
        &self.class
    }
    /// Whether earlier unknown effects remain an explicit applicability debt.
    #[must_use]
    pub fn has_earlier_unknown_mutations(&self) -> bool {
        self.obligations
            .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
    }
    /// Unresolved source and execution premises; no successful dispatch is implied.
    #[must_use]
    pub fn obligations(&self) -> &[super::SourceCommandTransitionObligation] {
        &self.obligations
    }
    pub(crate) fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.original
            .iter()
            .all(|word| word.image() == image && word.config() == config)
            && self.class.matches_source_context(image, config, context)
    }
    pub(crate) fn metadata<'a>(&self, analysis: &'a AnalysisResult) -> Option<&'a ClassDef> {
        self.class.metadata(analysis)
    }
}
impl OriginalSourceTransitionAdviceTape {
    /// Genuine conditional source calls; iteration supplies no runtime lookup.
    pub(crate) fn jim_wrapper_calls(&self) -> impl Iterator<Item = &OriginalAuthoredJimClassCall> {
        self.authored_jim_calls.values().filter_map(Option::as_ref)
    }

    pub(crate) fn authored_jim_call(&self, offset: u32) -> Option<&OriginalAuthoredJimClassCall> {
        self.authored_jim_calls.get(&offset)?.as_ref()
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn retain_authored_jim_call(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        native: &[NativeWord],
        written: &[super::SourceAdviceWord],
    ) -> bool {
        // Deferred owner attachment is intentionally unsupported here. It must not
        // borrow the distinct SourceClass call's body-owner receipt.
        if graph.procedure_body.is_some() || graph.future_procedure_source {
            return false;
        }
        let Some(head) = written.first().and_then(|word| word.input.as_ref()) else {
            return false;
        };
        let Some(key) = graph.selected_head_key(head.bytes()) else {
            return false;
        };
        let Some(AdviceCell::AuthoredJimClass(class)) = graph.cells.get(&key) else {
            return false;
        };
        if !class.matches_source_context(self.origin.source_image(), self.config, self.context)
            || native.iter().any(|word| {
                word.group().expand
                    || word
                        .executable_parts()
                        .all_parts()
                        .any(|part| !matches!(part.part, tcl_lexer::ExecutablePart::Text(_)))
            })
        {
            return false;
        }
        let Some(first) = native.first() else {
            return false;
        };
        let mut obligations = class.factory.obligations.clone();
        if !graph.uncertain_operations.is_empty()
            && !obligations
                .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
        {
            obligations.push(super::SourceCommandTransitionObligation::UnknownEarlierMutation);
        }
        let call = OriginalAuthoredJimClassCall {
            original: Arc::from(native),
            class: Arc::clone(class),
            obligations,
        };
        merge_receipt(
            &mut tape.authored_jim_calls,
            first.span().start(),
            Some(call),
        );
        true
    }
}
