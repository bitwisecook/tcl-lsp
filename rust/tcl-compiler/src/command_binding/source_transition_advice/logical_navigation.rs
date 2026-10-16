// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional Logical workspace candidates retain their actual source occurrence.

use super::{AdviceGraph, AdviceNamingPolicy, OriginalSourceTransitionAdviceTape};
use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::OriginalDeclaredLogicalBodyContext;
use std::sync::Arc;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::NativeWord;

#[derive(Debug, Clone, PartialEq, Eq)]
enum LogicalNavigationScope {
    Declared(Arc<OriginalDeclaredLogicalBodyContext>),
    Document(Arc<crate::command_binding::SourceOriginId>),
}
impl LogicalNavigationScope {
    fn owns_words(&self, original: &[NativeWord], input: &ResolvedAnalysisInput) -> bool {
        match self {
            Self::Declared(scope) => scope.matches_input(input) && scope.owns_words(original),
            Self::Document(origin) => {
                !original.is_empty()
                    && original.iter().all(|word| {
                        word.image() == origin.source_image()
                            && word.config() == input.lexer_config()
                    })
            }
        }
    }
}

/// Original variable geometry retains its source frame independently of head lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalLogicalVariableWords {
    input: ResolvedAnalysisInput,
    original: Arc<[NativeWord]>,
    scope: LogicalNavigationScope,
    earlier_unknown_mutations: bool,
}
impl OriginalLogicalVariableWords {
    fn capture(
        graph: &AdviceGraph,
        original: &[NativeWord],
        scope: LogicalNavigationScope,
    ) -> Option<Self> {
        let AdviceNamingPolicy::Logical(input) = &graph.policy else {
            return None;
        };
        let root = tcl_core_types::ByteNamespacePath::root();
        let namespace = match &scope {
            LogicalNavigationScope::Document(_) => &root,
            LogicalNavigationScope::Declared(body) => body.namespace_recipe(),
        };
        if original.is_empty()
            || !scope.owns_words(original, input)
            || graph.logical_namespace.as_ref() != Some(namespace)
        {
            return None;
        }
        Some(Self {
            input: input.clone(),
            original: Arc::from(original),
            scope,
            earlier_unknown_mutations: !graph.uncertain_operations.is_empty(),
        })
    }

    fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        &self.input == input
            && input.has_logical_source_name_context()
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
            && self.scope.owns_words(&self.original, input)
    }

    pub(super) fn variable_source_scope(
        &self,
        span: tcl_lexer::Span,
        name: &crate::signature_scan::scope::SignatureSourceNameInput,
        procedure: Option<Arc<super::OriginalLogicalProcedureBody>>,
    ) -> Option<crate::command_binding::OriginalLogicalVariableScope> {
        match &self.scope {
            LogicalNavigationScope::Document(origin) => {
                crate::command_binding::OriginalLogicalVariableScope::document(
                    &self.input,
                    Arc::clone(origin),
                    Arc::clone(&self.original),
                    self.earlier_unknown_mutations,
                    span,
                    name,
                )
            }
            LogicalNavigationScope::Declared(scope) => {
                crate::command_binding::OriginalLogicalVariableScope::declared(
                    &self.input,
                    Arc::clone(scope),
                    procedure,
                    Arc::clone(&self.original),
                    self.earlier_unknown_mutations,
                    span,
                    name,
                )
            }
        }
    }
}

/// Ordered Logical source coordinates at one authentic original call occurrence.
/// Unavailable actual lookup and unmodelled mutations remain applicability
/// obligations; this is neither an installation nor a resolved call edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalCommandCandidates {
    input: ResolvedAnalysisInput,
    original: Arc<[NativeWord]>,
    scope: LogicalNavigationScope,
    candidates: Vec<ByteCommandSlot>,
    earlier_unknown_mutations: bool,
    external_header_candidates: Option<Vec<ByteCommandSlot>>,
}

impl OriginalLogicalCommandCandidates {
    pub(super) fn is_original_document_scope(&self) -> bool {
        matches!(self.scope, LogicalNavigationScope::Document(_))
    }

    fn capture(
        graph: &AdviceGraph,
        original: &[NativeWord],
        scope: &Arc<OriginalDeclaredLogicalBodyContext>,
    ) -> Option<Self> {
        let AdviceNamingPolicy::Logical(input) = &graph.policy else {
            return None;
        };
        if !scope.matches_input(input)
            || !scope.owns_words(original)
            || graph.logical_namespace.as_ref() != Some(scope.namespace_recipe())
        {
            return None;
        }
        let head = original.first()?;
        if head.group().expand {
            return None;
        }
        let written = tcl_syntax::word_rules::original_static_word_unicode_value(head)?;
        let namespace = scope.namespace_recipe();
        let candidates = logical_coordinates(graph, namespace, &written)?;
        Some(Self {
            input: input.clone(),
            original: Arc::from(original),
            scope: LogicalNavigationScope::Declared(Arc::clone(scope)),
            external_header_candidates: external_header_candidates(graph, &written, &candidates),
            candidates,
            earlier_unknown_mutations: !graph.uncertain_operations.is_empty(),
        })
    }

    fn capture_document(
        graph: &AdviceGraph,
        original: &[NativeWord],
        origin: &Arc<crate::command_binding::SourceOriginId>,
    ) -> Option<Self> {
        let AdviceNamingPolicy::Logical(input) = &graph.policy else {
            return None;
        };
        let root = tcl_core_types::ByteNamespacePath::root();
        let scope = LogicalNavigationScope::Document(Arc::clone(origin));
        if graph.logical_namespace.as_ref() != Some(&root) || !scope.owns_words(original, input) {
            return None;
        }
        let head = original.first()?;
        if head.group().expand {
            return None;
        }
        let written = tcl_syntax::word_rules::original_static_word_unicode_value(head)?;
        let candidates = logical_coordinates(graph, &root, &written)?;
        Some(Self {
            input: input.clone(),
            original: Arc::from(original),
            scope,
            external_header_candidates: external_header_candidates(graph, &written, &candidates),
            candidates,
            earlier_unknown_mutations: !graph.uncertain_operations.is_empty(),
        })
    }

    /// Whole original head, independently of reporting labels and diagnostics.
    #[must_use]
    pub fn original_head(&self) -> Option<&NativeWord> {
        self.original.first()
    }

    /// Complete original syntax, without evaluated argv or invocation authority.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }

    /// Original syntactic argv cardinality. Expansion keeps the count unknown;
    /// no successful argument evaluation or runtime argv follows.
    #[must_use]
    pub fn argument_count(&self) -> Option<usize> {
        if self.original.iter().any(|word| word.group().expand) {
            return None;
        }
        self.original.len().checked_sub(1)
    }

    /// Ordered conditional coordinates, without existence or dispatch authority.
    #[must_use]
    pub fn coordinates(&self) -> &[ByteCommandSlot] {
        &self.candidates
    }

    /// Conditional external header coordinates admitted by the source graph.
    /// Selected source occupants are terminal and deleted alternatives cannot
    /// regain a header. Failed lookup representation grants no advice or absence.
    #[must_use]
    pub fn external_header_coordinates(&self) -> Option<&[ByteCommandSlot]> {
        self.external_header_candidates.as_deref()
    }

    /// Whether an earlier unmodelled source operation adds an applicability premise.
    /// Even false grants no workspace loading or actual command lookup.
    #[must_use]
    pub const fn has_earlier_unknown_mutations(&self) -> bool {
        self.earlier_unknown_mutations
    }

    /// Current original argv contains executable or opaque operand effects
    /// before command lookup. This is source applicability, not evaluated data.
    #[must_use]
    pub fn has_lookup_operand_effects(&self) -> bool {
        !super::body_effects::original_operand_effects(&self.original).is_empty()
    }

    /// Same complete Logical source/input and genuine Document or body recipe.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        &self.input == input
            && input.has_logical_source_name_context()
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
            && self.scope.owns_words(&self.original, input)
    }

    fn matches_source_recipe(&self, other: &Self) -> bool {
        self.input == other.input
            && self.original == other.original
            && self.scope == other.scope
            && self.candidates == other.candidates
            && self.earlier_unknown_mutations == other.earlier_unknown_mutations
    }

    fn intersect_external_headers(&mut self, other: &Self) {
        match (
            &mut self.external_header_candidates,
            &other.external_header_candidates,
        ) {
            (Some(previous), Some(current)) => {
                previous.retain(|slot| current.contains(slot));
            }
            (previous, None) => *previous = None,
            (None, Some(_)) => {}
        }
    }
}

pub(super) fn external_header_candidates(
    graph: &AdviceGraph,
    written: &[u8],
    candidates: &[ByteCommandSlot],
) -> Option<Vec<ByteCommandSlot>> {
    external_header_candidates_at_selected(graph, graph.lookup_head_key(written)?, candidates)
}

fn external_header_candidates_at_selected(
    graph: &AdviceGraph,
    selected: ByteCommandSlot,
    candidates: &[ByteCommandSlot],
) -> Option<Vec<ByteCommandSlot>> {
    if graph.cells.contains_key(&selected) {
        return None;
    }
    Some(
        candidates
            .iter()
            .filter(|slot| !graph.cells.contains_key(*slot))
            .cloned()
            .collect(),
    )
}

/// One literal absolute source command coordinate in an authentic original
/// script/member body. No caller namespace is supplied or inferred. This is
/// conditional navigation advice, never installed lookup or method-frame proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalAbsoluteCommandCandidate {
    input: ResolvedAnalysisInput,
    original: Arc<[NativeWord]>,
    body: super::OriginalSourceCatalogueBody,
    coordinates: [ByteCommandSlot; 1],
    earlier_unknown_mutations: bool,
}
impl OriginalLogicalAbsoluteCommandCandidate {
    fn capture(
        graph: &AdviceGraph,
        original: &[NativeWord],
        body: super::OriginalSourceCatalogueBody,
    ) -> Option<Self> {
        let AdviceNamingPolicy::Logical(input) = &graph.policy else {
            return None;
        };
        let head = original.first()?;
        if head.group().expand
            || !body.matches_source(head.image(), input.lexer_config())
            || !body.matches_context(input.borrowed_context_registry())
            || original
                .iter()
                .any(|word| word.image() != head.image() || word.config() != input.lexer_config())
            || head.span().start() < body.content_span().start()
            || original.last()?.span().end() > body.content_span().end()
        {
            return None;
        }
        let written = tcl_syntax::word_rules::original_static_word_unicode_value(head)?;
        if !written.starts_with(b"::") {
            return None;
        }
        let coordinate = AdviceNamingPolicy::logical_slot(&written)?;
        Some(Self {
            input: input.clone(),
            original: Arc::from(original),
            body,
            coordinates: [coordinate],
            earlier_unknown_mutations: !graph.uncertain_operations.is_empty(),
        })
    }
    /// The root-independent coordinate of this authentic absolute source head.
    #[must_use]
    pub const fn coordinates(&self) -> &[ByteCommandSlot; 1] {
        &self.coordinates
    }
    /// Earlier unmodelled source effects remain an applicability obligation.
    #[must_use]
    pub const fn has_earlier_unknown_mutations(&self) -> bool {
        self.earlier_unknown_mutations
    }
    /// Current original argv contains executable or opaque operand effects
    /// before command lookup. This is source applicability, not evaluated data.
    #[must_use]
    pub fn has_lookup_operand_effects(&self) -> bool {
        !super::body_effects::original_operand_effects(&self.original).is_empty()
    }

    /// Exact complete input/image/grammar and retained original body purpose.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        &self.input == input
            && input.has_logical_source_name_context()
            && self.body.matches_source(image, input.lexer_config())
            && self.body.matches_context(input.borrowed_context_registry())
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
    }
}

impl OriginalSourceTransitionAdviceTape {
    pub(super) fn retain_logical_absolute_navigation(
        &mut self,
        graph: &AdviceGraph,
        original: &[NativeWord],
        body: super::OriginalSourceCatalogueBody,
    ) {
        let Some(head) = original.first() else {
            return;
        };
        super::source_class::merge_receipt(
            &mut self.logical_absolute_navigation,
            head.span().start(),
            OriginalLogicalAbsoluteCommandCandidate::capture(graph, original, body),
        );
    }
    pub(crate) fn logical_absolute_navigation_at(
        &self,
        input: &ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> Option<OriginalLogicalAbsoluteCommandCandidate> {
        let candidate = self.logical_absolute_navigation.get(&offset)?.as_ref()?;
        (candidate.original.first()?.span().start() == offset
            && candidate.matches_source(image, input))
        .then(|| candidate.clone())
    }
}

pub(super) fn logical_coordinates(
    graph: &AdviceGraph,
    namespace: &tcl_core_types::ByteNamespacePath,
    written: &[u8],
) -> Option<Vec<ByteCommandSlot>> {
    if !written.starts_with(b"::") && graph.unknown_paths.contains(namespace) {
        return None;
    }
    let mut candidates = tcl_syntax::naming::authored_source_command_candidates(
        namespace,
        graph
            .namespace_paths
            .get(namespace)
            .map_or(&[], Vec::as_slice),
        written,
    )?;
    for candidate in &mut candidates {
        let mut seen = Vec::new();
        while let Some(target) = graph.namespace_imports.get(candidate) {
            if seen.contains(candidate) {
                return None;
            }
            seen.push(candidate.clone());
            *candidate = target.clone();
        }
    }
    Some(candidates)
}

/// Conditional callback candidates from an actual selected Registry prefix role.
/// The whole installer, original list field and source frame recipe remain held;
/// these coordinates prove neither callback reach nor an installed procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalCallbackCandidates {
    registration: super::OriginalSourceCommandTransitionAdvice,
    scope: Option<Arc<OriginalDeclaredLogicalBodyContext>>,
    head: tcl_syntax::word_rules::OriginalSourceListElement,
    candidates: Vec<ByteCommandSlot>,
    external_header_candidates: Option<Vec<ByteCommandSlot>>,
    appended: tcl_registry::AppendedArity,
    baked: usize,
}

impl OriginalLogicalCallbackCandidates {
    /// Independent original installer schema and all applicability obligations.
    #[must_use]
    pub const fn registration(&self) -> &super::OriginalSourceCommandTransitionAdvice {
        &self.registration
    }
    /// Exact source list field; cooked values never manufacture a word or range.
    #[must_use]
    pub const fn head(&self) -> &tcl_syntax::word_rules::OriginalSourceListElement {
        &self.head
    }
    /// Ordered conditional source coordinates, without actual lookup authority.
    #[must_use]
    pub fn coordinates(&self) -> &[ByteCommandSlot] {
        &self.candidates
    }
    /// External source headers admitted by the actual callback-scope graph.
    /// Selected source occupants and deleted slots cannot borrow project headers.
    /// This remains conditional registration advice, without callback entry.
    #[must_use]
    pub fn external_header_coordinates(&self) -> Option<&[ByteCommandSlot]> {
        self.external_header_candidates.as_deref()
    }
    /// Original installer operands can affect lookup before registration.
    #[must_use]
    pub fn has_lookup_operand_effects(&self) -> bool {
        !super::body_effects::original_operand_effects(self.registration.original_words())
            .is_empty()
    }
    /// Arguments the selected callback descriptor appends.
    #[must_use]
    pub const fn appended_arity(&self) -> tcl_registry::AppendedArity {
        self.appended
    }
    /// Original static list elements after the callback head.
    #[must_use]
    pub const fn baked_argument_count(&self) -> usize {
        self.baked
    }
    /// Same complete source/input and, where present, genuine declared ancestry.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        self.registration.logical_source_input() == Some(input)
            && self
                .registration
                .matches_source(image, input.lexer_config())
            && input.has_logical_source_name_context()
            && self.scope.as_ref().is_none_or(|scope| {
                scope.matches_input(input) && scope.owns_words(self.registration.original_words())
            })
    }
}

#[derive(Clone, Copy)]
pub(super) enum LogicalCallbackScope<'a> {
    Document,
    Declared(&'a Arc<OriginalDeclaredLogicalBodyContext>),
    Unavailable,
}

impl super::AdviceInvocationContext<'_> {
    pub(super) fn retain_logical_callback_candidates(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: super::AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        frame: LogicalCallbackScope<'_>,
    ) {
        let AdviceNamingPolicy::Logical(input) = self.policy else {
            return;
        };
        let Some(offset) = invocation.native.first().map(|head| head.span().start()) else {
            return;
        };
        let receipt = (|| {
            if schema.semantics.body_interpreter.resolve_with(|ordinal| {
                std::str::from_utf8(invocation.arguments.get(ordinal)?.value.as_deref()?).ok()
            }) != tcl_registry::InterpreterScope::Current
            {
                return None;
            }
            let prefixes = schema.authored_logical_source_command_prefix_arguments()?;
            let mut registration = self.schema_advice(invocation, graph, schema)?;
            let scope = match frame {
                LogicalCallbackScope::Declared(scope) => {
                    if !scope.matches_input(input)
                        || !scope.owns_words(invocation.native)
                        || graph.logical_namespace.as_ref() != Some(scope.namespace_recipe())
                    {
                        return None;
                    }
                    registration.logical_scope = Some(Arc::clone(scope));
                    Some(Arc::clone(scope))
                }
                LogicalCallbackScope::Document | LogicalCallbackScope::Unavailable => None,
            };
            let mut result = Vec::new();
            for (ordinal, appended) in prefixes {
                let argument = invocation.arguments.get(ordinal)?;
                let crate::registry_invocation::InvocationWordOrigin::Written(written) =
                    argument.origin
                else {
                    continue;
                };
                if invocation.native.get(written) != Some(&argument.original)
                    || argument.original.group().expand
                {
                    continue;
                }
                let root = tcl_core_types::ByteNamespacePath::root();
                let namespace = match schema.facts().script_lookup_scope(ordinal) {
                    Some(tcl_registry::ScriptLookupScope::GlobalFrame) => &root,
                    Some(tcl_registry::ScriptLookupScope::InvokingFrame) => match frame {
                        LogicalCallbackScope::Document => &root,
                        LogicalCallbackScope::Declared(scope) => scope.namespace_recipe(),
                        LogicalCallbackScope::Unavailable => continue,
                    },
                    Some(tcl_registry::ScriptLookupScope::TriggerFrame) | None => continue,
                };
                let Some(elements) =
                    tcl_syntax::word_rules::original_static_word_list_elements(&argument.original)
                else {
                    continue;
                };
                let Some(head) = elements.first() else {
                    continue;
                };
                let Some(candidates) =
                    logical_coordinates(graph, namespace, head.value().as_bytes())
                else {
                    continue;
                };
                result.push(OriginalLogicalCallbackCandidates {
                    registration: registration.clone(),
                    scope: scope.clone(),
                    head: head.clone(),
                    external_header_candidates: graph
                        .lookup_logical_head_key(namespace, head.value().as_bytes())
                        .and_then(|selected| {
                            external_header_candidates_at_selected(graph, selected, &candidates)
                        }),
                    candidates,
                    appended,
                    baked: elements.len().checked_sub(1)?,
                });
            }
            Some((Arc::from(invocation.native), result))
        })();
        super::source_class::merge_receipt(&mut tape.logical_callbacks, offset, receipt);
    }
}

impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn logical_callback_inventory(
        &self,
        input: &ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
    ) -> Vec<OriginalLogicalCallbackCandidates> {
        self.logical_callbacks
            .iter()
            .filter_map(|(offset, receipt)| {
                let (original, candidates) = receipt.as_ref()?;
                (original.first()?.span().start() == *offset).then_some((original, candidates))
            })
            .flat_map(|(original, candidates)| {
                candidates.iter().filter(move |candidate| {
                    candidate.registration.original_words() == original.as_ref()
                        && candidate.matches_source(image, input)
                })
            })
            .cloned()
            .collect()
    }

    pub(crate) fn logical_callbacks_at(
        &self,
        input: &ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> Option<Vec<OriginalLogicalCallbackCandidates>> {
        let (original, candidates) = self.logical_callbacks.get(&offset)?.as_ref()?;
        if original.first()?.span().start() != offset
            || original
                .iter()
                .any(|word| word.image() != image || word.config() != input.lexer_config())
        {
            return None;
        }
        candidates
            .iter()
            .all(|candidate| {
                candidate.registration.original_words() == original.as_ref()
                    && candidate.matches_source(image, input)
            })
            .then(|| candidates.clone())
    }
}

impl OriginalSourceTransitionAdviceTape {
    pub(super) fn merge_logical_navigation(
        &mut self,
        offset: u32,
        receipt: Option<OriginalLogicalCommandCandidates>,
    ) {
        self.logical_navigation
            .entry(offset)
            .and_modify(|previous| match (previous.as_mut(), receipt.as_ref()) {
                (Some(held), Some(current)) if held.matches_source_recipe(current) => {
                    held.intersect_external_headers(current);
                }
                _ => *previous = None,
            })
            .or_insert(receipt);
    }

    pub(super) fn retain_logical_document_navigation(
        &mut self,
        graph: &AdviceGraph,
        original: &[NativeWord],
        origin: &Arc<crate::command_binding::SourceOriginId>,
    ) {
        let Some(head) = original.first() else {
            return;
        };
        super::source_class::merge_receipt(
            &mut self.logical_variable_words,
            head.span().start(),
            OriginalLogicalVariableWords::capture(
                graph,
                original,
                LogicalNavigationScope::Document(Arc::clone(origin)),
            ),
        );
        self.merge_logical_navigation(
            head.span().start(),
            OriginalLogicalCommandCandidates::capture_document(graph, original, origin),
        );
    }

    pub(super) fn retain_logical_navigation(
        &mut self,
        graph: &AdviceGraph,
        original: &[NativeWord],
        scope: &Arc<OriginalDeclaredLogicalBodyContext>,
    ) {
        let Some(head) = original.first() else {
            return;
        };
        super::source_class::merge_receipt(
            &mut self.logical_variable_words,
            head.span().start(),
            OriginalLogicalVariableWords::capture(
                graph,
                original,
                LogicalNavigationScope::Declared(Arc::clone(scope)),
            ),
        );
        self.merge_logical_navigation(
            head.span().start(),
            OriginalLogicalCommandCandidates::capture(graph, original, scope),
        );
    }
}

impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn logical_lambda_formal_occurrences(
        &self,
        input: &ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
    ) -> Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use crate::command_binding::{
            OriginalDeclaredSourceBodyFrame, OriginalLogicalVariableScope,
        };
        use crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;
        let mut result = Vec::new();
        for scope in self
            .logical_variable_scopes
            .values()
            .filter_map(Option::as_ref)
        {
            let Some(lambda) = scope.lambda_source() else {
                continue;
            };
            if scope.frame_recipe() != OriginalDeclaredSourceBodyFrame::Lambda
                || !scope.matches_input(input)
                || !Arc::ptr_eq(scope.body_arc(), lambda.body())
                || !lambda.matches_source(image, input)
            {
                continue;
            }
            let Some(formals) = lambda.formal_parameters() else {
                continue;
            };
            for ordinal in 0..formals.parameters().len() {
                let Some(field) = lambda.formal_name_field(input, ordinal) else {
                    continue;
                };
                let Some(formal_scope) =
                    OriginalLogicalVariableScope::lambda_formal(input, Arc::clone(scope), &field)
                else {
                    continue;
                };
                if let Some(occurrence) =
                    SignatureSourceVariableOccurrence::from_logical_lambda_formal(
                        field,
                        formal_scope,
                    )
                {
                    result.push(occurrence);
                }
            }
        }
        result
    }

    pub(crate) fn logical_variable_scope_at(
        &self,
        input: &ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        name: &crate::signature_scan::scope::SignatureSourceNameInput,
    ) -> Option<crate::command_binding::OriginalLogicalVariableScope> {
        for (&offset, candidate) in self.logical_variable_words.range(..=span.start()).rev() {
            let Some(candidate) = candidate.as_ref() else {
                continue;
            };
            if candidate.original.first()?.span().start() != offset
                || !candidate.matches_source(image, input)
                || !crate::command_binding::OriginalLogicalVariableScope::original_words_match_name(
                    &candidate.original,
                    input,
                    span,
                    name,
                )
            {
                continue;
            }
            let procedure = match &candidate.scope {
                LogicalNavigationScope::Declared(scope)
                    if scope.frame_recipe()
                        == crate::command_binding::OriginalDeclaredSourceBodyFrame::Procedure =>
                {
                    let mut cards = self
                        .logical_procedure_bodies
                        .values()
                        .filter_map(Option::as_ref)
                        .filter(|card| {
                            std::ptr::eq(card.scope().scope_body(), scope.scope_body())
                                && card.scope().scope_name_input().is_some()
                                && card.scope().scope_name_input() == scope.scope_name_input()
                                && card.scope().namespace_recipe() == scope.namespace_recipe()
                                && card.scope().frame_recipe() == scope.frame_recipe()
                                && card.matches_source(image, input)
                        });
                    let card = cards.next()?.clone();
                    if cards.any(|other| other != &card) {
                        return None;
                    }
                    Some(card)
                }
                _ => None,
            };
            return candidate.variable_source_scope(span, name, procedure);
        }
        None
    }

    pub(crate) fn logical_navigation_inventory(
        &self,
        input: &ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
    ) -> Vec<OriginalLogicalCommandCandidates> {
        self.logical_navigation
            .keys()
            .filter_map(|offset| self.logical_navigation_at(input, image, *offset))
            .collect()
    }

    pub(crate) fn logical_navigation_at(
        &self,
        input: &ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> Option<OriginalLogicalCommandCandidates> {
        let candidate = self.logical_navigation.get(&offset)?.as_ref()?;
        (candidate.original.first()?.span().start() == offset
            && candidate.matches_source(image, input))
        .then(|| candidate.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::source_logical_command_candidates_at;

    fn query(
        source: &str,
        analysis: &crate::analyser::AnalysisResult,
        needle: &str,
    ) -> Option<OriginalLogicalCommandCandidates> {
        source_logical_command_candidates_at(
            source,
            analysis,
            u32::try_from(source.find(needle)?).ok()?,
        )
    }

    fn logical_variable_analysis(source: &str) -> crate::analyser::AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    fn logical_variable_tape(
        analysis: &crate::analyser::AnalysisResult,
    ) -> OriginalSourceTransitionAdviceTape {
        analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref()
            .original_source_transition_advice_tape_for(
                analysis
                    .resolved_input
                    .as_ref()
                    .unwrap()
                    .borrowed_context_registry(),
                super::super::OriginalSourceCataloguePurpose::CompletePrefix,
            )
            .unwrap()
    }

    #[test]
    fn logical_variable_occurrences_survive_unavailable_heads_and_paths() {
        for (source, head, local) in [
            ("$dispatch $value", "$dispatch", false),
            ("{*}$dispatch $value", "{*}$dispatch", false),
            ("proc p {value} {$dispatch $value}", "$dispatch", true),
            ("proc p {value} {{*}$dispatch $value}", "{*}$dispatch", true),
            ("namespace path $paths; helper $value", "helper", false),
            (
                "namespace eval N {namespace path $paths; proc p {value} {helper $value}}",
                "helper",
                true,
            ),
        ] {
            let analysis = logical_variable_analysis(source);
            let input = analysis.resolved_input.as_ref().unwrap();
            let image = tcl_lexer::SourceImage::document(source);
            let offset = u32::try_from(source.rfind("$value").unwrap() + 1).unwrap();
            let read = analysis
                .original_variable_symbol_at_offset(offset)
                .unwrap_or_else(|| panic!("original variable independent of head: {source}"));
            assert_eq!(!read.symbol().is_namespace(), local, "{source}");
            assert!(read.has_source_applicability_obligation());
            assert!(!analysis.original_variable_rename_is_complete(read.symbol()));
            assert!(
                analysis
                    .original_variable_symbol_in_source(&image, input.lexer_config(), offset,)
                    .is_some()
            );
            assert!(query(source, &analysis, head).is_none(), "{source}");
            if local {
                let tape = logical_variable_tape(&analysis);
                let mut cards = tape
                    .logical_procedure_bodies
                    .values()
                    .filter_map(Option::as_ref);
                let card = cards.next().expect("genuine retained procedure body");
                assert!(cards.next().is_none(), "one original procedure in {source}");
                assert!(card.matches_source(&image, input));
                let frame = crate::command_binding::SourceOriginalVariableFrame::from_logical_procedure_source(
                    card, input,
                ).expect("original Logical procedure frame");
                let crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::Local {
                    frame: selected,
                    simple,
                } = read.symbol().slot()
                else {
                    panic!("local occurrence in its actual source frame: {source}");
                };
                assert_eq!(selected, &frame, "{source}");
                assert_eq!(simple.as_bytes(), b"value", "{source}");
            }
        }
    }

    #[test]
    fn logical_variable_occurrences_keep_alias_and_local_frame_boundaries() {
        let source = "proc p {} {global shared; $dispatch $shared}; proc q {} {$dispatch $shared}";
        let analysis = logical_variable_analysis(source);
        let offsets: Vec<_> = source
            .match_indices("$shared")
            .map(|(offset, _)| u32::try_from(offset + 1).unwrap())
            .collect();
        let linked = analysis
            .original_variable_symbol_at_offset(offsets[0])
            .unwrap();
        let local = analysis
            .original_variable_symbol_at_offset(offsets[1])
            .unwrap();
        assert_eq!(
            linked.original_local_alias().unwrap().1.as_bytes(),
            b"shared"
        );
        assert!(linked.symbol().is_namespace());
        assert!(local.original_local_alias().is_none());
        assert!(!local.symbol().is_namespace());
        assert_ne!(linked.symbol(), local.symbol());
        for source in [
            "proc p {value value} {$dispatch $value}",
            "namespace eval $unknown {$dispatch $value}",
            "oo::class create C {method m {} {$dispatch $value}}",
            "set inert {$dispatch $value}",
        ] {
            let analysis = logical_variable_analysis(source);
            let offset = u32::try_from(source.rfind("$value").unwrap() + 1).unwrap();
            assert!(
                analysis
                    .original_variable_symbol_at_offset(offset)
                    .is_none(),
                "{source}"
            );
        }
        for source in [
            "proc p {} {global shared; namespace upvar ::other $unknown shared; $dispatch $shared}",
            "proc p {} {global shared; upvar 0 $unknown shared; $dispatch $shared}",
        ] {
            let analysis = logical_variable_analysis(source);
            let image = tcl_lexer::SourceImage::document(source);
            assert!(
                !analysis.original_variable_alias_source_is_complete(
                    &image,
                    analysis.body_lexer_config.unwrap(),
                ),
                "unrepresented dynamic alias target in {source}",
            );
            if let Some(read) = analysis.original_variable_symbol_at_offset(
                u32::try_from(source.rfind("$shared").unwrap() + 1).unwrap(),
            ) {
                assert!(!analysis.original_variable_rename_is_complete(read.symbol()));
            }
        }
    }

    #[test]
    fn logical_variable_scope_receipts_preserve_prior_unknowns_and_withdraw_conflicts() {
        let source = "unknown_command; $dispatch $value";
        let analysis = logical_variable_analysis(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let offset = u32::try_from(source.find("$dispatch").unwrap()).unwrap();
        let read = analysis
            .original_variable_symbol_at_offset(
                u32::try_from(source.find("$value").unwrap() + 1).unwrap(),
            )
            .unwrap();
        let mut tape = logical_variable_tape(&analysis);
        let receipt = tape
            .logical_variable_words
            .get(&offset)
            .unwrap()
            .as_ref()
            .unwrap()
            .clone();
        assert!(receipt.earlier_unknown_mutations);
        let expected = receipt
            .variable_source_scope(read.span(), read.original_name_input(), None)
            .unwrap();
        assert_eq!(
            tape.logical_variable_scope_at(input, &image, read.span(), read.original_name_input(),),
            Some(expected)
        );
        let mut conflicting = receipt;
        conflicting.earlier_unknown_mutations = false;
        let mut other = OriginalSourceTransitionAdviceTape::default();
        other
            .logical_variable_words
            .insert(offset, Some(conflicting));
        tape.extend_inventory(other);
        assert!(
            tape.logical_variable_scope_at(input, &image, read.span(), read.original_name_input(),)
                .is_none()
        );
    }

    #[test]
    fn logical_variable_scope_receipts_require_current_source_input_and_body() {
        let source = "proc p {value} {$dispatch $value}; proc q {value} {$dispatch $value}";
        let analysis = logical_variable_analysis(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let tape = logical_variable_tape(&analysis);
        let read = analysis
            .original_variable_symbol_at_offset(
                u32::try_from(source.find("$value").unwrap() + 1).unwrap(),
            )
            .unwrap();
        assert!(
            tape.logical_variable_scope_at(input, &image, read.span(), read.original_name_input(),)
                .is_some()
        );
        let stale = tcl_lexer::SourceImage::document(&format!("{source} "));
        assert!(
            tape.logical_variable_scope_at(input, &stale, read.span(), read.original_name_input(),)
                .is_none()
        );
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        for changed in [
            ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                input.context_registry(),
                config,
            ),
            ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                tcl_registry::model::ingress::resolve_environment("tcl8.6")
                    .default_context_registry(),
                input.lexer_config(),
            ),
        ] {
            assert!(
                tape.logical_variable_scope_at(
                    &changed,
                    &image,
                    read.span(),
                    read.original_name_input(),
                )
                .is_none()
            );
        }
        let receipts: Vec<_> = tape
            .logical_variable_words
            .values()
            .filter_map(Option::as_ref)
            .filter(|receipt| {
                receipt
                    .original
                    .first()
                    .is_some_and(|word| word.try_text().ok() == Some("$dispatch"))
            })
            .collect();
        assert_eq!(receipts.len(), 2);
        let LogicalNavigationScope::Declared(wrong) = &receipts[1].scope else {
            panic!("genuine original procedure scope");
        };
        let mut graph = AdviceGraph::new(
            input.borrowed_context_registry(),
            &AdviceNamingPolicy::Logical(input.clone()),
            tcl_registry::InvocationDialect::of_profile(input.unit_profile()),
        )
        .unwrap();
        graph.logical_namespace = Some(wrong.namespace_recipe().clone());
        assert!(
            OriginalLogicalVariableWords::capture(
                &graph,
                &receipts[0].original,
                LogicalNavigationScope::Declared(Arc::clone(wrong)),
            )
            .is_none()
        );
        let native = Analyser::new().analyse(source, "tcl9.1");
        let native_input = native.resolved_input.as_ref().unwrap();
        assert!(
            native
                .retained_command_realm()
                .unwrap()
                .original_logical_variable_scope_at(
                    native_input,
                    &image,
                    read.span(),
                    read.original_name_input(),
                )
                .is_none()
        );
    }

    #[test]
    fn logical_occurrence_inventory_keeps_original_scopes_and_partial_coverage() {
        use crate::registry_invocation::source_structure::source_logical_command_candidates;
        let source = "proc caller {} {namespace eval A {helper}; namespace eval B {helper}}; oo::class create C {method m {} {relative}}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let inventory = source_logical_command_candidates(source, &analysis).unwrap();
        for offset in source.match_indices("helper").map(|(offset, _)| offset) {
            let offset = u32::try_from(offset).unwrap();
            let per_site = source_logical_command_candidates_at(source, &analysis, offset).unwrap();
            let held = inventory
                .iter()
                .find(|candidate| {
                    candidate
                        .original_head()
                        .is_some_and(|head| head.span().start() == offset)
                })
                .unwrap();
            assert!(held == &per_site);
        }
        let method = u32::try_from(source.find("relative").unwrap()).unwrap();
        assert!(!inventory.iter().any(|candidate| {
            candidate
                .original_head()
                .is_some_and(|head| head.span().start() == method)
        }));
        assert!(source_logical_command_candidates(&format!("{source} "), &analysis).is_none());
        let config = analysis.body_lexer_config.unwrap();
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(source_logical_command_candidates(source, &analysis).is_none());
        analysis.body_lexer_config = Some(config);
        analysis.resolved_input = None;
        assert!(source_logical_command_candidates(source, &analysis).is_none());
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(source_logical_command_candidates(source, &native).is_none());
    }

    #[test]
    fn logical_external_headers_preserve_selected_source_occupants() {
        for (source, needle, allowed) in [
            ("missing ONE", "missing ONE", true),
            ("puts ONE", "puts ONE", false),
            ("proc helper {} {}; helper ONE", "helper ONE", false),
            (
                "interp alias {} helper {} puts; helper ONE",
                "helper ONE",
                false,
            ),
            ("rename puts {}; puts ONE", "puts ONE", false),
            (
                "namespace eval A {proc helper {} {}; proc caller {} {helper ONE}}",
                "helper ONE",
                false,
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let candidate = query(source, &analysis, needle).unwrap();
            assert_eq!(
                candidate.external_header_coordinates().is_some(),
                allowed,
                "{source}"
            );
        }
    }

    #[test]
    fn logical_external_headers_skip_deleted_local_slots_and_refuse_unknown_paths() {
        let source =
            "namespace eval A {proc helper {} {}; rename helper {}; proc caller {} {helper ONE}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let candidate = query(source, &analysis, "helper ONE").unwrap();
        assert_eq!(candidate.coordinates().len(), 2);
        assert_eq!(
            candidate.external_header_coordinates().unwrap(),
            &[AdviceNamingPolicy::logical_slot(b"::helper").unwrap()]
        );
        assert!(!candidate.has_earlier_unknown_mutations());
        let source = "namespace eval A {namespace path $paths; proc caller {} {helper ONE}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(query(source, &analysis, "helper ONE").is_none());
    }

    #[test]
    fn logical_occurrence_merge_preserves_geometry_and_intersects_external_headers() {
        let source = "namespace eval A {proc caller {} {helper ONE}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let candidate = query(source, &analysis, "helper ONE").unwrap();
        let LogicalNavigationScope::Declared(scope) = &candidate.scope else {
            panic!("original declared source scope");
        };
        let mut graph = AdviceGraph::new(
            candidate.input.borrowed_context_registry(),
            &AdviceNamingPolicy::Logical(candidate.input.clone()),
            tcl_registry::InvocationDialect::of_profile(candidate.input.unit_profile()),
        )
        .unwrap();
        graph.logical_namespace = Some(scope.namespace_recipe().clone());
        let offset = candidate.original_head().unwrap().span().start();
        let image = tcl_lexer::SourceImage::document(source);
        let mut tape = OriginalSourceTransitionAdviceTape::default();
        tape.retain_logical_navigation(&graph, &candidate.original, scope);
        let initial = tape
            .logical_navigation_at(&candidate.input, &image, offset)
            .unwrap();
        assert_eq!(initial.external_header_coordinates().unwrap().len(), 2);

        let local =
            tcl_syntax::naming::authored_source_command_slot(scope.namespace_recipe(), b"helper")
                .unwrap();
        graph.cells.insert(local, super::super::AdviceCell::Deleted);
        let mut alternative = OriginalSourceTransitionAdviceTape::default();
        alternative.retain_logical_navigation(&graph, &candidate.original, scope);
        tape.extend_inventory(alternative);
        let held = tape
            .logical_navigation_at(&candidate.input, &image, offset)
            .unwrap();
        assert_eq!(held.coordinates(), initial.coordinates());
        let root = AdviceNamingPolicy::logical_slot(b"::helper").unwrap();
        assert_eq!(
            held.external_header_coordinates().unwrap(),
            std::slice::from_ref(&root)
        );

        graph.cells.insert(root, super::super::AdviceCell::Deleted);
        tape.retain_logical_navigation(&graph, &candidate.original, scope);
        let held = tape
            .logical_navigation_at(&candidate.input, &image, offset)
            .unwrap();
        assert_eq!(held.coordinates(), initial.coordinates());
        assert!(held.external_header_coordinates().is_none());

        graph.record_uncertainty(&candidate.original);
        tape.retain_logical_navigation(&graph, &candidate.original, scope);
        assert!(
            tape.logical_navigation_at(&candidate.input, &image, offset)
                .is_none()
        );
    }

    #[test]
    fn logical_absolute_method_heads_keep_original_body_purpose_without_a_namespace_recipe() {
        use crate::registry_invocation::source_structure::source_logical_absolute_command_candidate_at;
        let source = "oo::class create C {method m {} {::helper; relative; puts {::inert}}}";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let offset = u32::try_from(source.find("::helper").unwrap()).unwrap();
        let candidate = source_logical_absolute_command_candidate_at(source, &analysis, offset)
            .expect("literal absolute original member-body command");
        assert_eq!(
            candidate.coordinates()[0],
            AdviceNamingPolicy::logical_slot(b"::helper").unwrap()
        );
        assert!(candidate.matches_source(&tcl_lexer::SourceImage::document(source), &input));
        assert!(
            source_logical_command_candidates_at(source, &analysis, offset).is_none(),
            "absolute syntax grants no unknown method namespace"
        );
        for needle in ["relative", "::inert"] {
            assert!(
                source_logical_absolute_command_candidate_at(
                    source,
                    &analysis,
                    u32::try_from(source.find(needle).unwrap()).unwrap()
                )
                .is_none()
            );
        }
        assert!(
            source_logical_absolute_command_candidate_at(source, &analysis, offset + 1).is_none()
        );
        assert!(
            source_logical_absolute_command_candidate_at(&format!("{source} "), &analysis, offset)
                .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(source_logical_absolute_command_candidate_at(source, &missing, offset).is_none());
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(source_logical_absolute_command_candidate_at(source, &native, offset).is_none());
    }

    #[test]
    fn logical_document_candidates_keep_root_source_recipe_and_original_cardinality() {
        let source = "namespace eval N {}; namespace path ::N; helper ONE; other {*}$arguments; set inert {helper TWO}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let helper = query(source, &analysis, "helper ONE").expect("actual root source command");
        let root = tcl_core_types::ByteNamespacePath::root();
        let namespace = tcl_syntax::naming::authored_source_namespace_path(&root, b"N").unwrap();
        assert_eq!(
            helper.coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(&root, &[namespace], b"helper")
                .unwrap()
        );
        assert_eq!(helper.argument_count(), Some(1));
        assert!(!helper.has_lookup_operand_effects());
        for source in ["helper [namespace path ::Changed]", "helper $value"] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let candidate = query(source, &analysis, "helper").unwrap();
            assert!(candidate.has_lookup_operand_effects());
        }
        let inert_source = "helper {[namespace path ::Changed]}";
        let inert_analysis = Analyser::new().analyse(inert_source, "tcl");
        assert!(
            !query(inert_source, &inert_analysis, "helper")
                .unwrap()
                .has_lookup_operand_effects()
        );
        assert_eq!(
            helper.original_head().unwrap().span().start(),
            u32::try_from(source.find("helper ONE").unwrap()).unwrap()
        );
        let expanded_source = "other {*}$arguments";
        let expanded_analysis = Analyser::new().analyse(expanded_source, "tcl");
        let expanded = query(expanded_source, &expanded_analysis, "other")
            .expect("actual expanded source argv before unknown prior effects");
        assert_eq!(expanded.argument_count(), None);
        assert!(query(source, &analysis, "helper TWO").is_none());
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(query(source, &native, "helper ONE").is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(query(source, &missing, "helper ONE").is_none());
        assert!(query(&format!("{source} "), &analysis, "helper ONE").is_none());
    }

    #[test]
    fn logical_candidate_queries_retain_body_bounds_and_require_an_exact_original_head() {
        use crate::registry_invocation::source_structure::source_logical_callback_candidates_at;
        let source = "proc caller {} {namespace eval A {helper}; after 1 {handler ARG}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let helper = u32::try_from(source.find("helper").unwrap()).unwrap();
        let candidate = source_logical_command_candidates_at(source, &analysis, helper)
            .expect("original body command before its closing delimiter");
        let root = tcl_core_types::ByteNamespacePath::root();
        let namespace = tcl_syntax::naming::authored_source_namespace_path(&root, b"A").unwrap();
        assert_eq!(
            candidate.coordinates().first().unwrap().namespace,
            namespace
        );
        assert_eq!(
            candidate.coordinates().first().unwrap().simple.as_bytes(),
            b"helper"
        );
        let installer = u32::try_from(source.find("after").unwrap()).unwrap();
        let callbacks = source_logical_callback_candidates_at(source, &analysis, installer)
            .expect("original installer vector before its enclosing closing delimiter");
        assert_eq!(callbacks.len(), 1);
        assert_eq!(callbacks[0].head().value().as_bytes(), b"handler");
        for offset in [helper + 1, helper + 6, installer + 1] {
            assert!(source_logical_command_candidates_at(source, &analysis, offset).is_none());
            assert!(source_logical_callback_candidates_at(source, &analysis, offset).is_none());
        }
        let mut missing = analysis.clone();
        missing.body_lexer_config = None;
        assert!(source_logical_command_candidates_at(source, &missing, helper).is_none());
        assert!(source_logical_callback_candidates_at(source, &missing, installer).is_none());
        assert!(
            source_logical_command_candidates_at(&format!("{source} "), &analysis, helper)
                .is_none()
        );
        assert!(
            source_logical_callback_candidates_at(&format!("{source} "), &analysis, installer)
                .is_none()
        );
    }

    #[test]
    fn logical_navigation_candidates_keep_each_original_namespace_occurrence_and_current_input() {
        let source = "proc caller {} {namespace eval A {helper}; namespace eval B {helper}}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let first = query(source, &analysis, "helper").unwrap();
        let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
        let second = source_logical_command_candidates_at(source, &analysis, offset).unwrap();
        let root = tcl_core_types::ByteNamespacePath::root();
        let a = tcl_syntax::naming::authored_source_namespace_path(&root, b"A").unwrap();
        let b = tcl_syntax::naming::authored_source_namespace_path(&root, b"B").unwrap();
        assert_eq!(
            first.coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(&a, &[], b"helper").unwrap()
        );
        assert_eq!(
            second.coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(&b, &[], b"helper").unwrap()
        );
        assert_ne!(first.coordinates(), second.coordinates());
        assert!(query(&format!("{source}\n# changed"), &analysis, "helper").is_none());
        let config = analysis.body_lexer_config.unwrap();
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(query(source, &analysis, "helper").is_none());
        analysis.body_lexer_config = Some(config);
        analysis.resolved_input = None;
        assert!(query(source, &analysis, "helper").is_none());
    }

    #[test]
    fn logical_navigation_candidates_use_the_original_path_and_refuse_unowned_heads() {
        let source =
            "namespace eval B {}; namespace eval A {namespace path {::B}; proc caller {} {helper}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let request = query(source, &analysis, "helper").unwrap();
        let root = tcl_core_types::ByteNamespacePath::root();
        let a = tcl_syntax::naming::authored_source_namespace_path(&root, b"A").unwrap();
        let b = tcl_syntax::naming::authored_source_namespace_path(&root, b"B").unwrap();
        assert_eq!(
            request.coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(&a, &[b], b"helper").unwrap()
        );
        for (source, needle) in [
            ("proc caller {} {$head}", "$head"),
            ("proc caller {} {{*}$heads}", "{*}"),
            ("proc caller {} {puts {helper}}", "helper"),
            (
                "namespace eval A {namespace path $paths; proc caller {} {helper}}",
                "helper",
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            assert!(query(source, &analysis, needle).is_none(), "{source}");
        }
        let source = "proc caller {} {helper}";
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(query(source, &native, "helper").is_none());
    }
    #[test]
    fn logical_callback_candidates_retain_selected_prefix_and_source_scope() {
        use crate::registry_invocation::source_structure::source_logical_callback_candidates_at;
        let source = "namespace eval A {proc caller {} {lsort -command {helper baked} {a b}}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let offset = u32::try_from(source.find("lsort").unwrap()).unwrap();
        let callbacks = source_logical_callback_candidates_at(source, &analysis, offset).unwrap();
        assert_eq!(callbacks.len(), 1);
        let callback = &callbacks[0];
        let root = tcl_core_types::ByteNamespacePath::root();
        let a = tcl_syntax::naming::authored_source_namespace_path(&root, b"A").unwrap();
        assert_eq!(
            callback.coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(&a, &[], b"helper").unwrap()
        );
        assert_eq!(callback.head().value(), "helper");
        assert_eq!(callback.baked_argument_count(), 1);
        assert_eq!(
            callback
                .appended_arity()
                .exact_counts()
                .unwrap()
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(
            callback.registration().original_words()[0].span().start(),
            offset
        );
        let extent = callback.head().source_span().unwrap();
        assert_eq!(
            &source[extent.start() as usize..extent.end() as usize],
            "helper"
        );
        assert!(
            source_logical_callback_candidates_at(
                &format!("{source}\n# changed"),
                &analysis,
                offset
            )
            .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(source_logical_callback_candidates_at(source, &missing, offset).is_none());
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(source_logical_callback_candidates_at(source, &native, offset).is_none());
    }

    #[test]
    fn logical_callback_candidates_refuse_computed_expanded_and_unowned_method_frames() {
        use crate::registry_invocation::source_structure::source_logical_callback_candidates_at;
        for source in [
            "proc caller {} {lsort -command $prefix {a b}}",
            "proc caller {} {lsort {*}{-command helper} {a b}}",
            "oo::class create C {method m {} {lsort -command helper {a b}}}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let offset = u32::try_from(source.find("lsort").unwrap()).unwrap();
            assert!(
                source_logical_callback_candidates_at(source, &analysis, offset)
                    .is_none_or(|candidates| candidates.is_empty()),
                "{source}"
            );
        }
        let source = "lsort -command {helper baked} {a b}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let callbacks = source_logical_callback_candidates_at(source, &analysis, 0).unwrap();
        assert_eq!(callbacks.len(), 1);
        assert_eq!(
            callbacks[0].coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(
                &tcl_core_types::ByteNamespacePath::root(),
                &[],
                b"helper"
            )
            .unwrap()
        );
    }
}
