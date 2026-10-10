// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly original Registry source structure shared by editor and host consumers.

use crate::analyser::AnalysisResult;
use tcl_registry::{CommandRegistry, InvocationArguments};

mod procedure;
pub use procedure::{OriginalProcedureSourceDescriptor, original_procedure_source_descriptor};

fn retained_document_image(
    source: &str,
    realm: &crate::realm::CommandBindingRealm,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_lexer::SourceImage> {
    let image = matching_document_image(source, realm.original_source_image()?)?;
    realm
        .matches_original_source_image(&image, config)
        .then_some(image)
}

fn matching_document_image(
    source: &str,
    image: &tcl_lexer::SourceImage,
) -> Option<tcl_lexer::SourceImage> {
    // naming.source.original-immutable-image-geometry
    // docs/design/analysis/name-resolution-proofs/original-immutable-image-geometry.md
    (image.channel() == tcl_lexer::SourceChannel::Document && image.bytes() == source.as_bytes())
        .then(|| image.clone())
}

/// Original document/workspace assistance at the unchanged complete call.
/// Known replacements, unsupported lookup scope and missing full source
/// correspondence decline. This never inherits Registry implementation facts.
#[must_use]
pub fn source_declared_command_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<crate::command_binding::OriginalDeclaredCommandWords> {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, config)?;
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    if !realm.matches_resolved_analysis_input(input) {
        return None;
    }
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let offset = segment.argv.first()?.span.start();
    let original = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    let binding = tokens.source_binding.as_ref()?;
    let selected = binding
        .original_declared_source_selection(&tokens, input, &original)
        .or_else(|| realm.original_authored_declared_source_selection(input, &tokens, &original))?;
    crate::command_binding::OriginalDeclaredCommandWords::from_selected(selected, input)
}

/// Fresh readonly source grammar with its own complete editing input and
/// original source interpretation. Conditional applicability cannot supply
/// execution, complete reference coverage, native entry or rename permission.
/// Consumers of an existing analysis use `source_registry_words` instead so
/// its retained native entry, hosted ancestry and sealed input remain owners.
#[derive(Debug)]
pub struct OriginalSourceRegistryContext {
    image: tcl_lexer::SourceImage,
    input: crate::analyser::ResolvedAnalysisInput,
    realm: crate::realm::CommandBindingRealm,
}
impl OriginalSourceRegistryContext {
    /// Capture an independently selected fresh source context. This never
    /// analyses a detached body as a fresh source or invokes diagnostics.
    #[must_use]
    pub fn capture(source: &str, input: crate::analyser::ResolvedAnalysisInput) -> Self {
        let (realm, input) =
            crate::analyser::Analyser::readonly_source_registry_realm(source, input);
        Self {
            image: tcl_lexer::SourceImage::document(source),
            input,
            realm,
        }
    }

    pub(crate) fn retained_source_realm(&self) -> &crate::realm::CommandBindingRealm {
        &self.realm
    }

    /// Exact immutable availability and full source grammar selected at entry.
    #[must_use]
    pub fn editing_input(&self) -> &crate::analyser::ResolvedAnalysisInput {
        &self.input
    }

    /// Actual complete source channel; no source substring is its own entry.
    #[must_use]
    pub fn source_image(&self) -> &tcl_lexer::SourceImage {
        &self.image
    }

    /// Conditional source roles at the unchanged complete original vector.
    /// Missing positioned lookup retains its own source/future obligations;
    /// known shadowing or absence cannot fall through to a nominal descriptor.
    #[must_use]
    pub fn words(
        &self,
        segment: &crate::segmenter::SegmentedCommand,
    ) -> Option<OriginalRegistryWords> {
        self.realm
            .matches_resolved_analysis_input(&self.input)
            .then_some(())?;
        let source = self.image.try_text().ok()?;
        let config = self.input.lexer_config();
        let context = self.input.context_registry();
        // Hosted source children require their independently retained body
        // occurrence owner; a fresh Native grammar cannot supply that issuer.
        if self.input.has_hosted_source_name_context() {
            return None;
        }
        // This facade owns a fresh source interpretation only. A genuine
        // positioned selection may project Registry syntax, but cannot issue
        // lookup-preserving rewrites or execution authority to this consumer.
        if tcl_registry::InvocationDialect::of_profile(self.input.unit_profile())
            .authored_name_policy()
            .is_some()
            && let Some(mut selected) =
                selected_registry_words_with_context(source, config, &self.realm, segment, &context)
        {
            selected.operands_preserve_source_lookup = false;
            return Some(selected);
        }
        native_source_registry_words(source, config, &self.realm, &context, segment, None)
    }
}

/// One effective operand's original source geometry. Captured prefix operands
/// lack a written location; expansion children cannot acquire a whole word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalOperandSource {
    span: tcl_lexer::Span,
    input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    word: Option<tcl_lexer::NativeWord>,
}
impl OriginalOperandSource {
    /// Exact written operand or independently retained readonly child extent.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Genuine optional naming input, never reconstructed from a label.
    #[must_use]
    pub fn input(&self) -> Option<&crate::signature_scan::scope::SignatureSourceNameInput> {
        self.input.as_ref()
    }
    /// Complete original word, absent for a readonly list expansion child.
    #[must_use]
    pub fn word(&self) -> Option<&tcl_lexer::NativeWord> {
        self.word.as_ref()
    }
}

/// Selection purpose retained independently of the canonical schema label.
/// Conditional applicability remains explicit and cannot supply execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalRegistrySource {
    /// Unanimous original Registry argv/role assistance.
    Selected,
    /// Exact scoped command vocabulary from a retained original body issuer.
    Scoped(std::sync::Arc<super::source_scoped_body::OriginalSourceScopedCommandSchema>),
    /// Authored source tape retains its naming purpose, original operands,
    /// source order and any alias/move lineage with conditional premises.
    SourceTransitions(
        std::sync::Arc<crate::command_binding::OriginalSourceCommandTransitionAdvice>,
    ),
    /// Future source argv keeps its actual deferred parent and original builder.
    /// This is neither an evaluated list object nor a reached target invocation.
    ProducedPrefix(std::sync::Arc<crate::command_binding::OriginalSourceProducedCommandPrefix>),
    /// Native source schema with unknown and alternative applicability obligations.
    Conditional(std::sync::Arc<crate::registry_invocation::OriginalConditionalRegistryMetadata>),
    /// Hosted source schema with its independent policy and cell barriers.
    Vendor(std::sync::Arc<crate::registry_invocation::OriginalConditionalVendorRegistryMetadata>),
}

/// Sealed effective source argv and authored Registry roles. This supplies
/// source structure only: no runtime dispatch, entered frame, cell, compiler,
/// Normal, reflection equivalence or edit permission follows from this view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalRegistryWords {
    command: String,
    dialect: Option<tcl_registry::InvocationDialect>,
    arguments: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    operands: Vec<Option<OriginalOperandSource>>,
    origins: Vec<crate::registry_invocation::InvocationWordOrigin>,
    head: Option<OriginalOperandSource>,
    roles: Option<Vec<(usize, tcl_registry::ArgRole)>>,
    source: OriginalRegistrySource,
    captured_values:
        Option<std::sync::Arc<crate::command_binding::OriginalSourceCommandTransitionAdvice>>,
    operands_preserve_source_lookup: bool,
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    registry: tcl_registry::RegistrySemanticKey,
    context: Option<tcl_registry::model::ResolvedContext>,
}
fn selected_source_package_reference(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::source_navigation::SourcePackageReference> {
    schema.authored_source_package_reference()
}

impl OriginalRegistryWords {
    /// Canonical authored Registry descriptor; this is not a source spelling.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    /// Independently retained invocation dialect, when actual selection owns it.
    #[must_use]
    pub const fn dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.dialect
    }
    /// Effective post-head operands, retaining dynamic/opaque/expanded states.
    #[must_use]
    pub fn arguments(&self) -> &[crate::registry_invocation::EffectiveInvocationWord] {
        &self.arguments
    }
    /// Effective post-head source anchors. No captured operand borrows a span.
    #[must_use]
    pub fn operands(&self) -> &[Option<OriginalOperandSource>] {
        &self.operands
    }
    /// Genuine readonly original value input at an effective operand ordinal.
    /// A captured alias prefix retains its separate declaration producer and
    /// conditional source obligations. It gains no call-site word extent,
    /// runtime `AliasPrefix` object, entered frame or invocation authority.
    #[must_use]
    pub fn original_argument_value_input(
        &self,
        argument: usize,
    ) -> Option<&crate::signature_scan::scope::SignatureSourceNameInput> {
        if let Some(input) = self
            .operands
            .get(argument)?
            .as_ref()
            .and_then(OriginalOperandSource::input)
        {
            return Some(input);
        }
        let advice = match &self.source {
            OriginalRegistrySource::SourceTransitions(advice) => advice,
            _ => self.captured_values.as_ref()?,
        };
        advice
            .arguments()
            .get(argument)?
            .input
            .as_ref()?
            .native_input()
    }

    /// Independently retained conditional declaration producers of captured
    /// operands. Selection, written geometry and execution remain separate.
    #[must_use]
    pub fn captured_argument_values(
        &self,
    ) -> Option<&crate::command_binding::OriginalSourceCommandTransitionAdvice> {
        self.captured_values.as_deref()
    }

    /// Actual written source head, independent of an alias's effective target.
    /// Hosted words retain their own source word without borrowing Native input.
    #[must_use]
    pub fn head_source(&self) -> Option<&OriginalOperandSource> {
        self.head.as_ref()
    }
    /// Original effective argv origins, including the head at ordinal zero.
    #[must_use]
    pub fn origins(&self) -> &[crate::registry_invocation::InvocationWordOrigin] {
        &self.origins
    }
    /// Complete unanimous roles at effective post-head ordinals, or unknown.
    #[must_use]
    pub fn roles(&self) -> Option<&[(usize, tcl_registry::ArgRole)]> {
        self.roles.as_deref()
    }
    /// Possible authored roles projected onto genuine whole written arguments.
    /// Captured prefix operands and expanded children provide no whole-word
    /// position; the head is excluded. This supplies no definite role.
    #[must_use]
    pub fn written_argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        let mut roles = Vec::new();
        for &(argument, role) in self.roles().unwrap_or(&[]) {
            let Some(crate::registry_invocation::InvocationWordOrigin::Written(written)) = argument
                .checked_add(1)
                .and_then(|ordinal| self.origins.get(ordinal))
            else {
                continue;
            };
            if let Some(argument) = written.checked_sub(1)
                && !roles.contains(&(argument, role))
            {
                roles.push((argument, role));
            }
        }
        roles.sort_by_key(|(ordinal, _)| *ordinal);
        roles
    }

    /// Original selection/conditional applicability without a runtime grant.
    #[must_use]
    pub const fn source(&self) -> &OriginalRegistrySource {
        &self.source
    }
    /// Independent original operand-effect check for the selected source
    /// lookup. Conditional schema applicability cannot supply this facet.
    /// This is not an execution, compiler, Normal or general edit receipt.
    #[must_use]
    pub const fn operands_preserve_source_lookup(&self) -> bool {
        self.operands_preserve_source_lookup
    }
    /// Exact complete source/channel and full lexer config correspondence.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.image == *image && self.config == config
    }
    /// Actual complete availability context when the source analysis owns it.
    /// A realm-only geometry projection does not manufacture this context.
    #[must_use]
    pub fn context(&self) -> Option<&tcl_registry::model::ResolvedContext> {
        self.context.as_ref()
    }
    /// Borrow the same exact argv's Registry-authored source schema under its
    /// actual retained context. The projection is source advice only; fields
    /// describing possible handlers/effects never establish execution here.
    #[must_use]
    pub fn with_source_schema<'r, T>(
        &'r self,
        context: &'r tcl_registry::model::ContextRegistry,
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, '_>) -> T,
    ) -> Option<T> {
        let arguments = self
            .arguments
            .iter()
            .map(|word| source_schema_word(word.as_registry_word()))
            .collect::<Vec<_>>();
        self.with_source_arguments(context, &arguments, project)
    }

    /// Query the same selected source descriptor with separately supplied
    /// advisory values on ordinary original written operands. Captured words,
    /// expansion cardinality and the original argv stay unchanged. This does
    /// not establish the values, a Native argv, dispatch or completion.
    pub(crate) fn with_source_value_projection<'r, T>(
        &'r self,
        context: &'r tcl_registry::model::ContextRegistry,
        values: &[(usize, &str)],
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, '_>) -> T,
    ) -> Option<T> {
        let mut arguments = self
            .arguments
            .iter()
            .map(|word| source_schema_word(word.as_registry_word()))
            .collect::<Vec<_>>();
        let mut seen = std::collections::HashSet::new();
        for &(ordinal, value) in values {
            if !seen.insert(ordinal)
                || !matches!(self.origins.get(ordinal.checked_add(1)?),
                    Some(crate::registry_invocation::InvocationWordOrigin::Written(written)) if *written > 0)
            {
                return None;
            }
            let word = self.operands.get(ordinal)?.as_ref()?.word()?;
            if word.group().expand {
                return None;
            }
            *arguments.get_mut(ordinal)? = tcl_registry::InvocationWord::Literal(value);
        }
        self.with_source_arguments(context, &arguments, project)
    }

    fn with_source_arguments<'r, T>(
        &'r self,
        context: &'r tcl_registry::model::ContextRegistry,
        arguments: &[tcl_registry::InvocationWord<'_>],
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, '_>) -> T,
    ) -> Option<T> {
        if !self.matches_registry(context.commands())
            || self.context.as_ref() != Some(context.context())
            || matches!(&self.source, OriginalRegistrySource::SourceTransitions(advice) if !advice.matches_context(context))
            || matches!(&self.source, OriginalRegistrySource::ProducedPrefix(prefix) if !prefix.matches_source_context(&self.image, self.config, context))
        {
            return None;
        }
        let invocation = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(&self.command),
            arguments,
        );
        let invocation = self
            .dialect
            .map_or(invocation, |dialect| invocation.with_dialect(dialect));
        if let OriginalRegistrySource::Scoped(scoped) = &self.source {
            if scoped.body().context() != context.context()
                || !scoped.body().matches_source(&self.image, self.config)
                || !scoped.body().matches_registry(context.commands())
            {
                return None;
            }
            let selected = tcl_registry::registry::resolve_source_descriptor_invocation(
                scoped.descriptor(),
                invocation,
                context.commands().source_descriptor_availability(
                    scoped.body().parent_descriptor(),
                    Some(context.context().authoring_query()),
                ),
                self.dialect.map(|dialect| dialect.numbers),
            );
            return Some(project(&selected));
        }
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                invocation,
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let selected = resolution.resolved()?;
        Some(project(&selected))
    }

    /// Original script regions from the shared source body/role owner.
    /// Complete context and applicability stay retained; no runtime body entry,
    /// scoped vocabulary, current cell, Normal or writable authority follows.
    #[must_use]
    pub fn source_script_bodies(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Vec<super::source_scoped_body::OriginalSourceScriptBody> {
        self.source_script_bodies_for(context, super::OriginalSourceScriptPurpose::Syntax)
    }

    /// The same source inventory under an explicit syntax/evaluation purpose.
    /// Reference-only syntax remains available independently of potential entry.
    #[must_use]
    pub fn source_script_bodies_for(
        &self,
        context: &tcl_registry::model::ContextRegistry,
        purpose: super::OriginalSourceScriptPurpose,
    ) -> Vec<super::source_scoped_body::OriginalSourceScriptBody> {
        super::source_scoped_body::source_script_bodies_for(self, context, purpose)
    }

    pub(crate) fn source_script_bodies_for_mutation_coverage(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<Vec<super::source_scoped_body::OriginalSourceScriptBody>> {
        super::source_scoped_body::source_script_bodies_for_mutation_coverage(self, context)
    }

    /// Original expression command regions at the retained AST's actual
    /// operand-content base. Unique genuine Expr geometry is required;
    /// unrelated or cooked operands cannot donate a child source span.
    #[must_use]
    pub fn source_expression_script_bodies_at(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        expression_base: u32,
    ) -> Option<Vec<super::OriginalSourceScriptBody>> {
        super::source_scoped_body::source_expression_script_bodies_at(self, input, expression_base)
    }

    /// Registry contents used by the issuer, independent of reporting names.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
    /// Original expression command regions under the caller's retained complete
    /// input. Multiple/cooked operands and unknown expression grammar decline.
    #[must_use]
    pub fn source_expression_script_bodies(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<super::OriginalSourceScriptBody>> {
        super::source_scoped_body::source_expression_script_bodies(self, input)
    }
}

/// Borrow exact text for Registry source controls without changing the retained
/// counted value. Opaque units, raw zero, substitution and expansion retain
/// their independent byte/cardinality facets; no executable argv is issued.
pub(crate) fn source_schema_word(
    word: tcl_registry::InvocationWord<'_>,
) -> tcl_registry::InvocationWord<'_> {
    let tcl_registry::InvocationWord::KnownBytes(bytes) = word else {
        return word;
    };
    if bytes.contains(&0) {
        return word;
    }
    std::str::from_utf8(bytes).map_or(word, tcl_registry::InvocationWord::Literal)
}

/// Unanimous Registry advice retains effective argument values and their own
/// source origins. Captured arguments occupy slots without source anchors;
/// expanded children use only independently retained readonly extents.
pub fn selected_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
) -> Option<OriginalRegistryWords> {
    let config = analysis.body_lexer_config?;
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return None;
    }
    let actual_registry = analysis.resolved_registry()?;
    if actual_registry.snapshot().semantic_key() != registry.snapshot().semantic_key() {
        return None;
    }
    let context = analysis.resolved_input.as_ref()?.context_registry();
    selected_registry_words_with_context(
        source,
        config,
        analysis.retained_command_realm()?,
        segment,
        &context,
    )
}

fn selected_registry_words_with_context(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    segment: &crate::segmenter::SegmentedCommand,
    context: &tcl_registry::model::ContextRegistry,
) -> Option<OriginalRegistryWords> {
    let mut selected =
        selected_registry_words_in_realm(source, config, realm, segment, context.commands())?;
    // Implementation contract: naming.source.authored-registry-role-projection
    // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
    selected.context = Some(context.context().clone());
    selected.roles = selected
        .with_source_schema(context, |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            conditional_roles(&roles, schema.semantics.argument_offset, complete)
        })
        .flatten();
    selected.captured_values =
        selected_captured_argument_values(source, config, realm, segment, context, &selected);
    Some(selected)
}

/// A selected call does not inherit the source tape's command selection.
/// Matching its entire effective vector retains only independently authored
/// captured value producers and their conditional transition obligations.
fn selected_captured_argument_values(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    segment: &crate::segmenter::SegmentedCommand,
    context: &tcl_registry::model::ContextRegistry,
    selected: &OriginalRegistryWords,
) -> Option<std::sync::Arc<crate::command_binding::OriginalSourceCommandTransitionAdvice>> {
    use crate::registry_invocation::InvocationWordOrigin;
    if !selected
        .origins
        .iter()
        .any(|origin| matches!(origin, InvocationWordOrigin::BindingPrefix(_)))
    {
        return None;
    }
    let image = retained_document_image(source, realm, config)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let advice = realm.original_source_transition_value_advice(context, &tokens)?;
    let values = source_transition_words_with_segment(source, config, context, segment, advice)?;
    if values.command != selected.command
        || values.dialect != selected.dialect
        || values.origins.get(1..) != selected.origins.get(1..)
        || values.arguments.len() != selected.arguments.len()
        || !values
            .arguments
            .iter()
            .zip(&selected.arguments)
            .all(
                |(source, actual)| match (source.literal_bytes(), actual.literal_bytes()) {
                    (Some(source), Some(actual)) => source == actual,
                    _ => source == actual,
                },
            )
    {
        return None;
    }
    let OriginalRegistrySource::SourceTransitions(advice) = values.source else {
        return None;
    };
    Some(advice)
}

/// Same selected original argv/role owner for a consumer with an independently
/// retained realm, complete source and full config. No nominal-head fallback.
pub fn selected_registry_words_in_realm(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    segment: &crate::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
) -> Option<OriginalRegistryWords> {
    let image = retained_document_image(source, realm, config)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let assistance = crate::registry_invocation::original_registry_invocation_assistance(
        registry, None, &tokens,
    )
    .or_else(|| {
        crate::registry_invocation::original_incomplete_body_registry_invocation_assistance(
            registry, realm, &tokens,
        )
    })?;
    let selected = assistance.unanimous_command_words()?;
    let roles = assistance.candidates.first().and_then(|first| {
        let roles = first
            .roles
            .iter()
            .map(|&(index, role)| {
                first
                    .argument_offset
                    .checked_add(usize::from(index))
                    .map(|index| (index, role))
            })
            .collect::<Option<Vec<_>>>()?;
        assistance
            .candidates
            .iter()
            .all(|candidate| {
                candidate.roles_complete
                    && candidate
                        .roles
                        .iter()
                        .map(|&(index, role)| {
                            candidate
                                .argument_offset
                                .checked_add(usize::from(index))
                                .map(|index| (index, role))
                        })
                        .collect::<Option<Vec<_>>>()
                        .as_ref()
                        == Some(&roles)
            })
            .then_some(roles)
    });
    let effective = selected.effective();
    let binding = tokens.source_binding.as_ref()?;
    let native = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        segment.argv.first()?.span.start(),
        config,
    );
    let dialect = binding.original_source_word_dialect_for_tokens(&tokens)?;
    let protocol = dialect.native_source_string_protocol()?;
    let arguments =
        original_source_argument_words(&tokens, effective, native.as_deref()?, protocol)?;
    let words: Vec<_> = arguments
        .iter()
        .map(|word| word.as_registry_word())
        .collect();
    InvocationArguments::structured(&words).exact_argv_len()?;
    let operands =
        effective_operand_sources(segment, &tokens, effective, &arguments, native.as_deref())?;
    let head = native.as_deref().and_then(|words| {
        Some(OriginalOperandSource {
            span: words.first()?.span(),
            input: binding.original_written_name_input(&tokens, 0),
            word: words.first().cloned(),
        })
    });
    Some(OriginalRegistryWords {
        command: selected.command().to_owned(),
        dialect: Some(dialect),
        arguments,
        operands,
        origins: effective.origins.clone(),
        head,
        roles,
        source: OriginalRegistrySource::Selected,
        captured_values: None,
        operands_preserve_source_lookup: binding
            .original_source_operands_preserve_lookup(&tokens, registry),
        image,
        config,
        registry: registry.snapshot().semantic_key(),
        context: None,
    })
}

/// Original lexical values have their own capture purpose. Positioned frozen
/// runtime values cannot replace static source escapes or promote a written
/// substitution to a literal. Captured argv and proved expansion children keep
/// their independent exact bytes and effective cardinality.
fn original_source_argument_words(
    tokens: &crate::ir::CommandTokens,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    native: &[tcl_lexer::NativeWord],
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Option<Vec<crate::registry_invocation::EffectiveInvocationWord>> {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let captured =
        tcl_registry::native_compiler_words::NativeCompilerWords::capture(native, protocol).ok()?;
    let frozen = crate::registry_invocation::frozen_argument_words(tokens, effective);
    effective
        .origins
        .iter()
        .skip(1)
        .zip(frozen)
        .map(|(origin, frozen)| {
            let InvocationWordOrigin::Written(ordinal) = origin else {
                return Some(frozen);
            };
            let word = native.get(*ordinal)?;
            Some(if word.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                captured
                    .literal(*ordinal)
                    .map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                        EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(bytes))
                    })
            })
        })
        .collect()
}

fn effective_operand_sources(
    segment: &crate::segmenter::SegmentedCommand,
    tokens: &crate::ir::CommandTokens,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    arguments: &[crate::registry_invocation::EffectiveInvocationWord],
    native: Option<&[tcl_lexer::NativeWord]>,
) -> Option<Vec<Option<OriginalOperandSource>>> {
    use crate::registry_invocation::InvocationWordOrigin;
    if effective.origins.len() != arguments.len().checked_add(1)?
        || effective.words.len() != effective.origins.len()
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    Some(
        effective
            .origins
            .iter()
            .skip(1)
            .zip(arguments)
            .map(|(origin, value)| match origin {
                InvocationWordOrigin::Written(written) => Some(OriginalOperandSource {
                    span: segment.argv.get(*written)?.span,
                    input: binding.original_written_name_input(tokens, *written),
                    word: native.and_then(|words| words.get(*written)).cloned(),
                }),
                InvocationWordOrigin::ExpandedElement { written, element } => {
                    let parent = binding.original_written_name_input(tokens, *written)?;
                    let mut children = parent.original_list_elements_with_source_spans()?;
                    let (input, span) = children.get_mut(*element)?;
                    (value.literal_bytes()? == input.bytes()).then_some(())?;
                    let span = (*span)?;
                    let parent = native?.get(*written)?.span();
                    if span.start() < parent.start() || span.end() > parent.end() {
                        return None;
                    }
                    Some(OriginalOperandSource {
                        span,
                        input: Some(input.clone()),
                        word: None,
                    })
                }
                InvocationWordOrigin::BindingPrefix(_) | InvocationWordOrigin::ResolvedHead => None,
            })
            .collect(),
    )
}

/// Readonly source argument topology of the actual retained procedure
/// implementation. Formal bindings describe argv ordinals only; they do not
/// install a frame, read a cell, prepare a native call or permit an edit.
pub struct OriginalProcedureArguments<'a> {
    declaration: &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
        crate::analyser::ProcDef,
    >,
    formals: crate::signature_scan::formal_parameters::SignatureSourceFormalParameters,
    bindings: Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>,
    arguments: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    operands: Vec<Option<OriginalOperandSource>>,
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    registry: tcl_registry::RegistrySemanticKey,
}

impl<'a> OriginalProcedureArguments<'a> {
    /// Canonical immutable declaration selected by the implementation allocation.
    #[must_use]
    pub const fn declaration(
        &self,
    ) -> &'a crate::signature_scan::original_name::SourceDeclarationMetadata<crate::analyser::ProcDef>
    {
        self.declaration
    }
    /// Actual original `ParamList` and its independently selected source grammar.
    #[must_use]
    pub const fn formals(
        &self,
    ) -> &crate::signature_scan::formal_parameters::SignatureSourceFormalParameters {
        &self.formals
    }
    /// Required/default/rest/caller-link syntax at this exact effective argc.
    #[must_use]
    pub fn bindings(&self) -> &[tcl_syntax::formal_params::FormalByteArgumentBinding] {
        &self.bindings
    }
    /// Effective post-head values, with captured prefix slots preserved.
    #[must_use]
    pub fn arguments(&self) -> &[crate::registry_invocation::EffectiveInvocationWord] {
        &self.arguments
    }
    /// Source anchors of those exact effective values; captured operands have none.
    #[must_use]
    pub fn operands(&self) -> &[Option<OriginalOperandSource>] {
        &self.operands
    }
    /// Exact immutable complete source/channel and lexer configuration.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.image == *image && self.config == config && self.formals.matches_source(image, config)
    }
    /// Registry generation used for the independently selected formal grammar.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}

/// Match an actual original procedure reference to its same-document immutable
/// declaration and exact effective argv. Alias prefixes and expansion children
/// retain their own origins; unknown lookup, foreign formals, ambiguous records,
/// unavailable child extents and changed source cannot acquire another anchor.
#[must_use]
pub fn original_procedure_arguments<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalProcedureArguments<'a>> {
    // naming.source.original-procedure-argument-topology
    // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
    let descriptor = original_procedure_source_descriptor(source, analysis, segment)?;
    let words = descriptor
        .arguments
        .iter()
        .map(|word| word.as_registry_word())
        .collect::<Vec<_>>();
    let count = InvocationArguments::structured(&words).exact_argv_len()?;
    let bindings = descriptor.formals.bindings(count).ok()?;
    Some(OriginalProcedureArguments {
        declaration: descriptor.declaration,
        formals: descriptor.formals,
        bindings,
        arguments: descriptor.arguments,
        operands: descriptor.operands,
        image: descriptor.image,
        config: descriptor.config,
        registry: descriptor.registry,
    })
}

/// Source structure from the selected original argv, or the independently
/// retained conditional schema. The latter keeps its applicability obligations
/// and grants no runtime dispatch, entered frame, compiler or edit permission.
pub fn source_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    let config = analysis.body_lexer_config?;
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, config)?;
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let registry = analysis.resolved_registry()?;
    let input = analysis.resolved_input.as_ref()?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    if realm.original_source_registry_barrier(&input.context_registry(), &tokens) {
        return None;
    }
    if let Some(scoped) = original_scoped_registry_words(source, analysis, segment) {
        return Some(scoped);
    }
    if realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        == Some(input)
    {
        return logical_source_registry_words(source, realm, input, segment);
    }
    if !analysis.has_original_vendor_source_names()
        && let Some(selected) = selected_registry_words(source, analysis, segment, registry)
    {
        return Some(selected);
    }
    let offset = segment.argv.first()?.span.start();
    if analysis.has_original_vendor_source_names() {
        let (metadata, _) = selected_vendor_registry_words_at(source, analysis, offset)?;
        let context = analysis.resolved_input.as_ref()?.context_registry();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&image),
            config,
            segment,
        );
        analysis
            .retained_command_realm()?
            .stamp_original_tokens(&mut tokens);
        // The shared builder authenticates the complete original native
        // vector. Resegmentation's comment trivia is not word identity.
        return original_vendor_registry_words(&context, &tokens, metadata);
    }
    let metadata = analysis
        .original_conditional_registry_metadata_in_source(&image, config, offset)
        .cloned();
    let context = analysis.resolved_input.as_ref()?.context_registry();
    native_source_registry_words(
        source,
        config,
        analysis.retained_command_realm()?,
        &context,
        segment,
        metadata,
    )
}

fn original_scoped_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    let config = analysis.body_lexer_config?;
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, config)?;
    let offset = segment.argv.first()?.span.start();
    let body = analysis.original_scoped_body_in_source(&image, config, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let original = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    let first = original.first()?;
    let last = original.last()?;
    let extent = body.content_span();
    if first.span().start() < extent.start() || last.span().end() > extent.end() {
        return None;
    }
    let name = tcl_syntax::word_rules::original_static_word_ascii_presentation(first)?;
    let name = std::str::from_utf8(&name).ok()?;
    let schema = super::source_scoped_body::OriginalSourceScopedCommandSchema::select(body, name)?;
    let arguments = original
        .iter()
        .skip(1)
        .map(|word| {
            if word.group().expand {
                crate::registry_invocation::EffectiveInvocationWord::Expanded
            } else {
                tcl_syntax::word_rules::original_static_word_ascii_presentation(word).map_or(
                    crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                    |value| {
                        crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(
                            std::sync::Arc::from(value),
                        )
                    },
                )
            }
        })
        .collect::<Vec<_>>();
    let operands = original
        .iter()
        .skip(1)
        .map(|word| {
            Some(OriginalOperandSource {
                span: word.span(),
                input: None,
                word: Some(word.clone()),
            })
        })
        .collect();
    let head = Some(OriginalOperandSource {
        span: first.span(),
        input: None,
        word: Some(first.clone()),
    });
    let mut words = OriginalRegistryWords {
        command: schema.command().name.to_owned(),
        dialect: tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_source_word_dialect_for_tokens(&tokens)),
        arguments,
        operands,
        origins: (0..original.len())
            .map(crate::registry_invocation::InvocationWordOrigin::Written)
            .collect(),
        head,
        roles: None,
        source: OriginalRegistrySource::Scoped(std::sync::Arc::new(schema)),
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
    };
    words.roles = words
        .with_source_schema(&context, |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            conditional_roles(&roles, schema.semantics.argument_offset, complete)
        })
        .flatten();
    Some(words)
}

/// Source schema at one original invocation offset under the same retained
/// analysis. A stored complete vector is preferred; conditional future/body
/// source advice retains its own full-image issuer through the shared builder.
/// No reporting command string selects the schema or supplies execution.
#[must_use]
pub fn source_registry_words_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<OriginalRegistryWords> {
    // naming.diagnostic.original-package-source-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let realm = analysis.retained_command_realm()?;
    let binding = realm.invocation_at_source("", offset);
    let segment = if let Some((segment, _)) = binding.original_recorded_command() {
        segment
    } else {
        let tail = source.get(usize::try_from(offset).ok()?..)?;
        crate::segmenter::segment_commands_with_offset_and_config(tail, offset, config)
            .into_iter()
            .next()?
    };
    (segment.argv.first()?.span.start() == offset).then_some(())?;
    source_registry_words(source, analysis, &segment)
}

/// Original whole words under a positively retained Logical source input.
/// Missing Native or hosted producers cannot enter this compatibility domain.
/// The vector is readonly geometry, without a Native name or runtime grant.
#[must_use]
pub fn original_logical_source_words_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<Vec<tcl_lexer::NativeWord>> {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let realm = analysis.retained_command_realm()?;
    let input = analysis.resolved_input.as_ref()?;
    (realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        == Some(input))
    .then_some(())?;
    let binding = realm.invocation_at_source("", offset);
    let (_, tokens) = binding.original_recorded_command()?;
    super::original_native_compiler_words(&image, tokens.words(), offset, config)
}

/// Original Registry-created instance syntax at its actual source occurrence.
/// The full existing Analysis owns both factory and invocation; a source slice
/// is never recaptured as a fresh entry, nor is a reporting class name looked up.
#[must_use]
pub fn source_registered_instance_words_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceRegisteredInstanceWords> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_registered_instance_words(input, &native)
}

/// Original whole handle setter and its selected constructor substitution.
/// This is authored source ownership, without cell, allocation or value proof.
#[must_use]
pub fn source_registered_handle_binding_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceRegisteredHandleBinding> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_registered_handle_binding(input, &native)
}

/// Original future command naming layout, retaining source applicability only.
#[must_use]
pub fn source_command_publication_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<super::OriginalSourceCommandPublication> {
    let words = source_registry_words_at(source, analysis, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    super::source_publication::OriginalSourceCommandPublication::capture(words, &context)
}

/// Genuine setter operands and complete single-command construction syntax.
/// No value, frame, allocation or successful store is issued.
#[must_use]
pub fn source_handle_construction_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<super::OriginalSourceHandleConstruction> {
    let words = source_registry_words_at(source, analysis, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    super::source_publication::OriginalSourceHandleConstruction::capture(words, &context)
}

/// Original setter and exact factory substitution for source handle-class advice.
/// This supplies no stored handle, variable cell or successful allocation.
#[must_use]
pub fn source_handle_class_advice_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<super::OriginalSourceHandleClassAdvice> {
    let words = source_registry_words_at(source, analysis, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    super::source_publication::OriginalSourceHandleClassAdvice::capture(
        source, analysis, words, &context,
    )
}

/// Original named Registry factory source declaration. This retains actual
/// factory/name words and conditional applicability, without successful creation.
#[must_use]
pub fn source_registered_factory_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceRegisteredInstance> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_registered_factory(input, &native)
}

/// Actual whole source constructor call and its original selected class definer.
/// This supplies source advice only, without a class or constructor execution.
#[must_use]
pub fn source_constructor_call_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceConstructorCall> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_constructor_call(input, &native)
}

/// Conditional named or symbolic source-class receiver with whole original argv.
/// This supplies no allocation, stored value, Native dispatch or lifetime grant.
#[must_use]
pub fn source_class_instance_words_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceClassInstanceWords> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    let receipt = realm.original_source_class_instance_words(input, &native)?;
    receipt
        .instance()
        .constructor()
        .constructor_shape(analysis)?;
    Some(receipt)
}

/// Final conditional procedure source slots and authentic declaration ancestry.
/// The represented source graph honours known moves, deletions and replacement;
/// unknown mutations remain obligations. This grants no completed runtime world.
#[must_use]
pub fn source_procedure_publications(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<crate::command_binding::OriginalSourceProcedurePublications> {
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, input.lexer_config())?;
    analysis
        .matches_original_source_image(&image, input.lexer_config())
        .then_some(())?;
    realm.original_source_procedure_publications(input)
}

/// Final conditional class source slots and their authentic factory ancestry.
/// Known moves, deletion and replacement withdraw the old source spelling;
/// unknown operations retain applicability obligations, never Native identity.
#[must_use]
pub fn source_class_publications(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<crate::command_binding::OriginalSourceClassPublications> {
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, input.lexer_config())?;
    analysis
        .matches_original_source_image(&image, input.lexer_config())
        .then_some(())?;
    realm.original_source_class_publications(input)
}

/// Conditional callback procedure target from the original registration horizon.
/// Captured alias operands and local canonical declaration remain authentic;
/// callback reach, future table, argv, frame and dispatch stay unresolved.
#[must_use]
pub fn source_callback_procedure_target_at(
    source: &str,
    analysis: &AnalysisResult,
    registration_offset: u32,
    prefix: &crate::command_binding::OriginalCallbackPrefix,
) -> Option<crate::command_binding::OriginalSourceCallbackProcedureLookup> {
    let (input, realm, native) =
        registered_original_words_at(source, analysis, registration_offset)?;
    realm.original_source_callback_procedure_target(input, &native, prefix)
}

/// Attach the genuine selected whole source installer to a readonly prefix.
/// The actual consumer site is checked against the source graph's registration;
/// captured operand producer spans cannot replace it. This issues no Native
/// lookup, installed callback, future frame or successful factory capability.
#[must_use]
pub fn source_callback_prefix_at(
    source: &str,
    analysis: &AnalysisResult,
    registration_offset: u32,
    prefix: &crate::command_binding::OriginalCallbackPrefix,
) -> Option<crate::command_binding::OriginalCallbackPrefix> {
    let original =
        source_callback_procedure_target_at(source, analysis, registration_offset, prefix)?;
    prefix.with_source_registration(&original)
}

/// Original readonly class reference selected at its genuine source consumer.
/// Original list children and any earlier procedure producer retain independent
/// source ownership. This supplies no native class token or runtime inheritance.
#[must_use]
pub fn source_class_reference_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
    name: &crate::signature_scan::scope::SignatureSourceNameInput,
) -> Option<crate::command_binding::OriginalSourceClassReference> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_class_reference(input, &native, name)
}

/// Whole original class configuration target and its pretransition source
/// declaration. Logical and Native inputs remain separate; this provides no
/// object identity, entered definition worker, method-table closure or edit.
#[must_use]
pub fn source_configured_class_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceConfiguredClassReference> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_configured_class(input, &native)
}

/// Exact original selected class declaration at its authentic source occurrence.
#[must_use]
pub fn source_class_declaration_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceClassDeclaration> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_class_declaration(input, &native)
}

fn registered_original_words_at<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    offset: u32,
) -> Option<(
    &'a crate::analyser::ResolvedAnalysisInput,
    &'a crate::realm::CommandBindingRealm,
    Vec<tcl_lexer::NativeWord>,
)> {
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm()?;
    realm.matches_resolved_analysis_input(input).then_some(())?;
    let binding = realm.invocation_at_source("", offset);
    let native = if let Some((_, tokens)) = binding.original_recorded_command() {
        super::original_native_compiler_words(&image, tokens.words(), offset, config)?
    } else {
        // The original tape remains the body/source-role authority below.
        // Parsing a suffix only supplies unchanged word geometry for that join.
        let tail = source.get(usize::try_from(offset).ok()?..)?;
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(tail, offset, config)
                .into_iter()
                .next()?;
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        super::original_native_compiler_words(&image, tokens.words(), offset, config)?
    };
    (native.first()?.span().start() == offset).then_some((input, realm, native))
}

/// Current root insertion geometry shared by missing-package diagnostics and
/// reviewed package suggestions. Only selected leading root requirements are
/// header commands; nested scripts, data and prefix-like text cannot supply
/// this position. The full original input and availability remain owners.
#[must_use]
pub fn original_package_require_insert_offset(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<u32> {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    use tcl_registry::source_navigation::SourcePackageReferenceKind as Kind;
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let mut end =
        if config.leading_bom == tcl_lexer::LeadingBom::Skip && source.starts_with('\u{feff}') {
            3
        } else {
            0
        };
    if source.get(end..)?.starts_with("#!") {
        end = source
            .get(end..)?
            .find('\n')
            .map_or(source.len(), |newline| end + newline + 1);
    }
    for segment in crate::segmenter::segment_commands_with_offset_and_config(source, 0, config) {
        let Some(words) = source_registry_words(source, analysis, &segment) else {
            break;
        };
        let head = words.head_source()?.word()?;
        let start = usize::try_from(head.span().start()).ok()?;
        if start < end {
            continue;
        }
        if !source.get(end..start)?.trim().is_empty()
            || words
                .package_reference(&context)
                .is_none_or(|reference| reference.kind() != Kind::Require)
        {
            break;
        }
        let last = words
            .operands()
            .iter()
            .flatten()
            .filter_map(OriginalOperandSource::word)
            .map(|word| word.span().end())
            .max()?;
        let mut after = usize::try_from(last).ok()?;
        while after < source.len() && source.as_bytes()[after] != b'\n' {
            after += 1;
        }
        if after < source.len() {
            after += 1;
        }
        end = after;
    }
    u32::try_from(end).ok()
}

/// Genuine source package-reference operand under its selected source schema.
/// Native package purposes retain the original value recipe. Hosted and
/// Logical source operands keep exact ASCII metadata matching independently;
/// neither path claims a runtime package table, loader or successful call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourcePackageReference {
    words: std::sync::Arc<OriginalRegistryWords>,
    reference: tcl_registry::source_navigation::SourcePackageReference,
    literal: Vec<u8>,
    native_key: Option<tcl_registry::native_package::NativePackageNameKey>,
}
impl OriginalSourcePackageReference {
    /// Same selected whole invocation, including its conditional obligations.
    #[must_use]
    pub fn words(&self) -> &OriginalRegistryWords {
        &self.words
    }
    /// Requirement or actual version-supplying source provision.
    #[must_use]
    pub const fn kind(&self) -> tcl_registry::source_navigation::SourcePackageReferenceKind {
        self.reference.kind
    }
    /// Original effective name ordinal; captured arguments gain no call anchor.
    #[must_use]
    pub const fn argument(&self) -> usize {
        self.reference.argument
    }
    /// Match fixed ASCII descriptor metadata under the original package purpose.
    #[must_use]
    pub fn matches_ascii(&self, metadata: &str) -> bool {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        self.native_key.as_ref().map_or_else(
            || metadata.is_ascii() && self.literal == metadata.as_bytes(),
            |key| key.matches_ascii(metadata),
        )
    }
}
impl OriginalRegistryWords {
    /// Project a package source reference using the same complete actual
    /// availability context, selected grammar and original effective operand.
    #[must_use]
    pub fn package_reference(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<OriginalSourcePackageReference> {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        let reference = self.with_source_schema(context, selected_source_package_reference)??;
        let literal = self
            .arguments
            .get(reference.argument)?
            .literal_bytes()?
            .to_vec();
        let native_key = self
            .original_argument_value_input(reference.argument)
            .map(|input| {
                tcl_registry::native_package::NativePackageNameKey::from_native_units(
                    input.bytes(),
                    input.policy(),
                )
            });
        Some(OriginalSourcePackageReference {
            words: std::sync::Arc::new(self.clone()),
            reference,
            literal,
            native_key,
        })
    }
}

/// Source schema for one retained original complete invocation vector.
/// The current Analysis remains the owner; no fresh source analysis is made.
pub(crate) fn original_segment_for_tokens(
    source: &str,
    analysis: &AnalysisResult,
    tokens: &crate::ir::CommandTokens,
) -> Option<(
    crate::segmenter::SegmentedCommand,
    Vec<tcl_lexer::NativeWord>,
)> {
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let native = super::original_native_compiler_words(
        &image,
        tokens.words(),
        tokens.argv.first()?.start(),
        config,
    )?;
    let start = native.first()?.span().start();
    let end = native.last()?.span().end();
    let text = source.get(usize::try_from(start).ok()?..usize::try_from(end).ok()?)?;
    let mut segments =
        crate::segmenter::segment_commands_with_offset_and_config(text, start, config);
    if segments.len() != 1 {
        return None;
    }
    Some((segments.pop()?, native))
}

/// Selected Registry metadata at the unchanged original complete invocation.
/// This preserves the retained analysis rather than analysing detached text.
pub(crate) fn original_registry_words_for_tokens(
    source: &str,
    analysis: &AnalysisResult,
    tokens: &crate::ir::CommandTokens,
) -> Option<OriginalRegistryWords> {
    let (segment, native) = original_segment_for_tokens(source, analysis, tokens)?;
    let words = source_registry_words(source, analysis, &segment)?;
    (words.head_source()?.word() == native.first()).then_some(words)
}

/// Authored declaration roles at the unchanged complete invocation, with
/// their original declaration owner and no inherited Registry implementation.
pub(crate) fn original_declared_words_for_tokens(
    source: &str,
    analysis: &AnalysisResult,
    tokens: &crate::ir::CommandTokens,
) -> Option<crate::command_binding::OriginalDeclaredCommandWords> {
    let (segment, native) = original_segment_for_tokens(source, analysis, tokens)?;
    let words = source_declared_command_words(source, analysis, &segment)?;
    (words.original_words() == native).then_some(words)
}

/// Positioned Logical source schema from the retained immutable Realm and
/// the exact original command vector. Dynamic values remain unknown; this
/// supplies no Native naming input, dispatch, Normal or lookup preservation.
#[must_use]
pub fn original_logical_registry_words_for_command(
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    command: &tcl_lexer::NativeScriptCommandWords,
) -> Option<OriginalRegistryWords> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let image = command.words.first()?.image();
    let source = image.try_text().ok()?;
    let config = input.lexer_config();
    if command
        .words
        .iter()
        .any(|word| word.image() != image || word.config() != config)
    {
        return None;
    }
    let text = source.get(command.span.as_range())?;
    let mut segments = crate::segmenter::segment_commands_with_offset_and_config(
        text,
        command.span.start(),
        config,
    );
    if segments.len() != 1 {
        return None;
    }
    let segment = segments.pop()?;
    let words = logical_source_registry_words(source, realm, input, &segment)?;
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    (advice.original_words() == command.words.as_slice()).then_some(words)
}

fn logical_source_registry_words(
    source: &str,
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    if realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        != Some(input)
        || !realm.matches_resolved_analysis_input(input)
        || !realm.matches_original_source_image(
            &tcl_lexer::SourceImage::document(source),
            input.lexer_config(),
        )
    {
        return None;
    }
    let context = input.context_registry();
    let words =
        source_transition_registry_words(source, input.lexer_config(), realm, &context, segment)?;
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    (advice.original_head().logical_input().is_some()
        && advice.logical_source_input() == Some(input))
    .then_some(words)
}

fn native_source_registry_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    retained_metadata: Option<crate::registry_invocation::OriginalConditionalRegistryMetadata>,
) -> Option<OriginalRegistryWords> {
    let image = retained_document_image(source, realm, config)?;
    realm
        .matches_original_source_image(&image, config)
        .then_some(())?;
    if let Some(input) = realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
    {
        // This domain is selected by the validated full Logical input, never
        // by a missing Native operand or a Registry/profile display label.
        if config != input.lexer_config()
            || !std::ptr::eq(std::sync::Arc::as_ptr(&input.context_registry()), context)
        {
            return None;
        }
        return logical_source_registry_words(source, realm, input, segment);
    }
    let Some(metadata) =
        native_source_metadata(source, config, realm, context, segment, retained_metadata)
    else {
        return source_transition_registry_words(source, config, realm, context, segment);
    };
    conditional_source_registry_words(source, config, realm, context, segment, metadata)
}

fn native_source_metadata(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    retained_metadata: Option<crate::registry_invocation::OriginalConditionalRegistryMetadata>,
) -> Option<crate::registry_invocation::OriginalConditionalRegistryMetadata> {
    let registry = context.commands();
    let image = retained_document_image(source, realm, config)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    if realm.original_source_registry_barrier(context, &tokens) {
        return None;
    }

    let metadata = if let Some(metadata) = retained_metadata {
        metadata
    } else {
        let candidate = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| {
                binding.original_catalogue_source_candidate_from_tokens(&tokens, registry)
            })
            .or_else(|| {
                realm
                    .source_bindings_ref()
                    .original_future_source_catalogue_candidate(&image, config, &tokens, registry)
            });
        candidate.and_then(|candidate| {
            super::conditional_metadata::original_catalogue_registry_metadata(context, candidate)
        })?
    };
    Some(metadata)
}

fn conditional_source_values(
    metadata: &crate::registry_invocation::OriginalConditionalRegistryMetadata,
) -> Option<(
    Vec<tcl_lexer::NativeWord>,
    Vec<crate::registry_invocation::EffectiveInvocationWord>,
)> {
    use crate::registry_invocation::EffectiveInvocationWord;
    let native = metadata.original_words().to_vec();
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        &native,
        metadata
            .original_head()
            .name_input()
            .policy()
            .string_protocol(),
    )
    .ok()?;
    let values = native
        .iter()
        .enumerate()
        .skip(1)
        .map(|(ordinal, word)| {
            if word.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                captured
                    .literal(ordinal)
                    .map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                        EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(bytes))
                    })
            }
        })
        .collect::<Vec<_>>();
    Some((native, values))
}

fn conditional_source_operands(
    native: &[tcl_lexer::NativeWord],
    segment: &crate::segmenter::SegmentedCommand,
    origin: &OriginalRegistrySource,
) -> Vec<Option<OriginalOperandSource>> {
    native
        .iter()
        .enumerate()
        .skip(1)
        .map(|(ordinal, word)| {
            Some(OriginalOperandSource {
                span: segment.argv.get(ordinal)?.span,
                input: match origin {
                    OriginalRegistrySource::Conditional(metadata) => {
                        let head = metadata.original_head().name_input();
                        crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                            word, head.word_value_rules(), head.policy())
                            .map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord)
                    },
                    OriginalRegistrySource::Selected | OriginalRegistrySource::Scoped(_) | OriginalRegistrySource::Vendor(_) | OriginalRegistrySource::SourceTransitions(_) | OriginalRegistrySource::ProducedPrefix(_) => None,
                },
                word: Some(word.clone()),
            })
        })
        .collect()
}

fn conditional_source_registry_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    metadata: crate::registry_invocation::OriginalConditionalRegistryMetadata,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::InvocationWordOrigin;
    let registry = context.commands();
    let image = retained_document_image(source, realm, config)?;
    let offset = segment.argv.first()?.span.start();
    let (command, native, values, roles, origin) = {
        let (native, values) = conditional_source_values(&metadata)?;
        let roles = conditional_roles(
            metadata.roles(),
            metadata.argument_offset(),
            metadata.roles_complete(),
        );
        (
            metadata.command().to_owned(),
            native,
            values,
            roles,
            OriginalRegistrySource::Conditional(std::sync::Arc::new(metadata)),
        )
    };
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let dialect = match &origin {
        OriginalRegistrySource::Scoped(_)
        | OriginalRegistrySource::Vendor(_)
        | OriginalRegistrySource::SourceTransitions(_)
        | OriginalRegistrySource::ProducedPrefix(_) => None,
        OriginalRegistrySource::Conditional(metadata) => metadata.original_source_dialect(),
        OriginalRegistrySource::Selected => tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_source_word_dialect_for_tokens(&tokens)),
    };
    let original = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    if original != native {
        return None;
    }
    let operands = conditional_source_operands(&native, segment, &origin);
    let head = native.first().map(|word| OriginalOperandSource {
        span: word.span(),
        input: match &origin {
            OriginalRegistrySource::Conditional(metadata) => Some(
                crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                    metadata.original_head().name_input().clone(),
                ),
            ),
            OriginalRegistrySource::Selected
            | OriginalRegistrySource::Scoped(_)
            | OriginalRegistrySource::Vendor(_)
            | OriginalRegistrySource::SourceTransitions(_)
            | OriginalRegistrySource::ProducedPrefix(_) => None,
        },
        word: Some(word.clone()),
    });
    Some(OriginalRegistryWords {
        command,
        dialect,
        arguments: values,
        operands,
        origins: (0..native.len())
            .map(InvocationWordOrigin::Written)
            .collect(),
        roles,
        head,
        source: origin,
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: registry.snapshot().semantic_key(),
        context: Some(context.context().clone()),
    })
}

/// Conditional root-script aliases and moves share one immutable source tape.
/// Known targets and definite absence do not fall through; an unbounded
/// snapshot retains conditional source applicability without a dispatch grant.
fn source_transition_registry_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    let image = retained_document_image(source, realm, config)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let advice = realm.original_source_transition_advice(context, &tokens)?;
    source_transition_words_with_segment(source, config, context, segment, advice)
}

/// Project a sealed original source receipt through the same role/body owner.
/// The original vector is recaptured in its whole-image address space; a
/// detached script or reconstructed argv cannot supply this correspondence.
pub(crate) fn source_transition_words_from_advice(
    source: &str,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    advice: crate::command_binding::OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalRegistryWords> {
    let original = advice.original_words();
    let span = tcl_lexer::Span::new(
        original.first()?.span().start(),
        original.last()?.span().end(),
    );
    let mut segments = crate::segmenter::segment_commands_with_offset_and_config(
        source.get(span.as_range())?,
        span.start(),
        config,
    );
    if segments.len() != 1 {
        return None;
    }
    source_transition_words_with_segment(source, config, context, &segments.pop()?, advice)
}

fn source_transition_words_with_segment(
    source: &str,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    advice: crate::command_binding::OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let image = matching_document_image(source, advice.original_words().first()?.image())?;
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    if crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        segment.argv.first()?.span.start(),
        config,
    )?
    .as_slice()
        != advice.original_words()
    {
        return None;
    }
    if !advice.matches_context(context) || !advice.matches_source(&image, config) {
        return None;
    }
    let values = advice
        .arguments()
        .iter()
        .map(|argument| {
            if argument.original.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                argument
                    .value
                    .as_ref()
                    .map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                        EffectiveInvocationWord::ByteLiteral(std::sync::Arc::clone(bytes))
                    })
            }
        })
        .collect();
    let operands = advice
        .arguments()
        .iter()
        .map(|argument| {
            let InvocationWordOrigin::Written(ordinal) = argument.origin else {
                return None;
            };
            let token = segment.argv.get(ordinal)?;
            // The realm authenticated the exact complete native vector.
            // Segmented tokens and grouped original words retain different
            // genuine delimiter extents; both are preserved independently.
            Some(OriginalOperandSource {
                span: token.span,
                input: argument
                    .input
                    .as_ref()
                    .and_then(|input| input.native_input())
                    .cloned(),
                word: Some(argument.original.clone()),
            })
        })
        .collect();
    let mut origins = vec![InvocationWordOrigin::Written(0)];
    origins.extend(advice.arguments().iter().map(|argument| argument.origin));
    let head = advice
        .original_words()
        .first()
        .map(|word| OriginalOperandSource {
            span: word.span(),
            input: advice.original_head().native_input().cloned(),
            word: Some(word.clone()),
        });
    Some(OriginalRegistryWords {
        command: advice.command().to_owned(),
        dialect: Some(advice.dialect()),
        arguments: values,
        operands,
        origins,
        roles: advice.roles().map(<[_]>::to_vec),
        head,
        source: OriginalRegistrySource::SourceTransitions(std::sync::Arc::new(advice)),
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
    })
}

/// Conditional future source argv built by a selected original list invocation
/// in a retained deferred operand. Every written target operand keeps its own
/// builder source word; captured target-prefix values acquire no source anchor.
/// This receipt cannot supply execution, native values or syntax compaction.
#[must_use]
pub fn source_produced_command_prefix_words(
    source: &str,
    analysis: &AnalysisResult,
    producer: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    let input = analysis.resolved_input.as_ref()?;
    analysis
        .matches_original_source_image(
            &tcl_lexer::SourceImage::document(source),
            input.lexer_config(),
        )
        .then_some(())?;
    source_produced_command_prefix_words_in(
        source,
        input,
        analysis.retained_command_realm()?,
        producer,
    )
}

/// The same source producer while an analyser retains the complete input and
/// realm independently of its final result. No fresh interpretation, default
/// context or presentation head replaces those owners.
pub(crate) fn source_produced_command_prefix_words_in(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    realm: &crate::realm::CommandBindingRealm,
    producer: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    let context = input.context_registry();
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    (realm.matches_resolved_analysis_input(input)
        && realm.matches_original_source_image(&image, config))
    .then_some(())?;
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        producer,
    );
    let original = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        producer.argv.first()?.span.start(),
        config,
    )?;
    let prefix = realm.original_produced_command_prefix(input, &original)?;
    // The genuine deferred parent owns this builder's conditional schema.
    // A standalone nested lookup would require an unrelated entered frame.
    let builder =
        source_transition_words_from_advice(source, config, &context, prefix.producer().clone())?;
    if !builder.with_source_schema(&context, |schema| {
        schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
            && schema.semantics.native_result
                == Some(
                    tcl_registry::native_result::NativeResultContract::ListArguments { from: 0 },
                )
    })? {
        return None;
    }
    if builder.command() != prefix.producer().command()
        || builder.origins()
            != (0..original.len())
                .map(crate::registry_invocation::InvocationWordOrigin::Written)
                .collect::<Vec<_>>()
    {
        return None;
    }
    produced_prefix_words(source, config, &context, producer, prefix)
}

fn produced_prefix_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    prefix: crate::command_binding::OriginalSourceProducedCommandPrefix,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let image = tcl_lexer::SourceImage::document(source);
    prefix
        .matches_source_context(&image, config, context)
        .then_some(())?;
    let mut origins = vec![InvocationWordOrigin::Written(1)];
    origins.extend(
        prefix
            .arguments()
            .iter()
            .map(|argument| match argument.origin {
                InvocationWordOrigin::Written(ordinal) => {
                    ordinal.checked_add(1).map(InvocationWordOrigin::Written)
                }
                other => Some(other),
            })
            .collect::<Option<Vec<_>>>()?,
    );
    let operands = prefix
        .arguments()
        .iter()
        .map(|argument| {
            let InvocationWordOrigin::Written(_) = argument.origin else {
                return None;
            };
            let word = &argument.original;
            let token = segment.argv.iter().find(|token| {
                word.tokens()
                    .first()
                    .is_some_and(|first| first.span == token.span)
            })?;
            Some(OriginalOperandSource {
                span: token.span,
                input: argument
                    .input
                    .as_ref()
                    .and_then(|input| input.native_input())
                    .cloned(),
                word: Some(word.clone()),
            })
        })
        .collect();
    let arguments = prefix
        .arguments()
        .iter()
        .map(|argument| {
            argument
                .value
                .as_ref()
                .map_or(EffectiveInvocationWord::Dynamic, |value| {
                    EffectiveInvocationWord::ByteLiteral(std::sync::Arc::clone(value))
                })
        })
        .collect();
    let head = prefix.original_head().original_word()?;
    Some(OriginalRegistryWords {
        command: prefix.command().to_owned(),
        dialect: Some(prefix.producer().dialect()),
        arguments,
        operands,
        origins,
        roles: prefix.roles().map(<[_]>::to_vec),
        head: Some(OriginalOperandSource {
            span: head.span(),
            input: prefix.original_head().native_input().cloned(),
            word: Some(head.clone()),
        }),
        source: OriginalRegistrySource::ProducedPrefix(std::sync::Arc::new(prefix)),
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
    })
}

/// Same sealed hosted source argv for pre-dispatch lexical inventory and
/// current Analysis consumers. Metadata, complete original words, context and
/// the authentic token carrier must agree; no handler or body entry follows.
pub(crate) fn original_vendor_registry_words(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &crate::ir::CommandTokens,
    metadata: crate::registry_invocation::OriginalConditionalVendorRegistryMetadata,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let shape = metadata.shape();
    let image = shape.original_head().source_image().clone();
    let config = shape.original_head().lexer_config();
    if shape.context() != context.context()
        || shape.registry() != &context.commands().snapshot().semantic_key()
        || !metadata.matches_source(&image, config)
        || !metadata.catalogue().matches_invocation(tokens)
    {
        return None;
    }
    let offset = tokens.argv.first()?.start();
    let native = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    if native.as_slice() != shape.original_words() || native.len() != tokens.argv.len() {
        return None;
    }
    let arguments = native
        .iter()
        .skip(1)
        .map(|word| {
            if word.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                tcl_syntax::naming::vendor_source_literal_units(
                    shape.original_head().policy(),
                    word,
                    tcl_syntax::naming::VendorSourceNamePurpose::SourceName,
                )
                .map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                    EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(bytes))
                })
            }
        })
        .collect();
    let operands = native
        .iter()
        .enumerate()
        .skip(1)
        .map(|(ordinal, word)| {
            Some(OriginalOperandSource {
                span: *tokens.argv.get(ordinal)?,
                input: None,
                word: Some(word.clone()),
            })
        })
        .collect();
    let head = native.first().map(|word| OriginalOperandSource {
        span: word.span(),
        input: None,
        word: Some(word.clone()),
    });
    Some(OriginalRegistryWords {
        command: shape.command().to_owned(),
        dialect: None,
        arguments,
        operands,
        origins: (0..native.len())
            .map(InvocationWordOrigin::Written)
            .collect(),
        roles: conditional_roles(
            shape.roles(),
            shape.argument_offset(),
            shape.roles_complete(),
        ),
        head,
        source: OriginalRegistrySource::Vendor(std::sync::Arc::new(metadata)),
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
    })
}

fn conditional_roles(
    roles: &[(u8, tcl_registry::ArgRole)],
    offset: usize,
    complete: bool,
) -> Option<Vec<(usize, tcl_registry::ArgRole)>> {
    if !complete {
        return None;
    }
    roles
        .iter()
        .map(|&(ordinal, role)| {
            offset
                .checked_add(usize::from(ordinal))
                .map(|index| (index, role))
        })
        .collect()
}

/// Hosted source candidates use their own context/policy and complete retained
/// native vector. This returns conditional Registry syntax only, without a
/// C/Jim name recipe or actual handler/table grant.
pub fn selected_vendor_registry_words_at(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<(
    crate::registry_invocation::OriginalConditionalVendorRegistryMetadata,
    crate::segmenter::SegmentedCommand,
)> {
    let config = analysis.body_lexer_config?;
    analysis
        .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        .then_some(())?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let realm = analysis.retained_command_realm()?;
    let mut selected = None;
    let mut selected_width = u32::MAX;
    for occurrence in analysis.original_vendor_source_names() {
        let words = occurrence.original_words();
        if words.first()? != occurrence.name_input().original_word() {
            continue;
        }
        let start = words.first()?.span().start();
        let end = words.last()?.span().end();
        let immediate_next = cursor > end
            && source
                .get(end as usize..cursor as usize)
                .is_some_and(|text| {
                    !text.is_empty() && text.chars().all(|c| c == ' ' || c == '\t')
                });
        if cursor < start || (cursor > end && !immediate_next) {
            continue;
        }
        let width = end.checked_sub(start)?;
        if width < selected_width {
            selected_width = width;
            selected = Some(occurrence);
        } else if width == selected_width && selected != Some(occurrence) {
            return None;
        }
    }
    // Choose the actual innermost original command before querying metadata.
    // Missing/blocked inner metadata cannot borrow an outer script's roles.
    let occurrence = selected?;
    let words = occurrence.original_words();
    let start = words.first()?.span().start();
    let end = words.last()?.span().end();
    let text = source.get(start as usize..end as usize)?;
    let mut commands =
        crate::segmenter::segment_commands_with_offset_and_config(text, start, config);
    if commands.len() != 1 {
        return None;
    }
    let command = commands.pop()?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &command,
    );
    realm.stamp_original_tokens(&mut tokens);
    let metadata = crate::registry_invocation::original_vendor_occurrence_registry_metadata(
        &context, &tokens, occurrence,
    )?;
    Some((metadata, command))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected_subcommand_diagnostic(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<tcl_registry::AuthoredSourceSubcommandDiagnostic> {
        schema.authored_source_subcommand_diagnostic()
    }
    fn selected_argument_roles(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> (Vec<(u8, tcl_registry::ArgRole)>, bool) {
        schema.authored_source_argument_roles()
    }
    fn selected_rule_procedure_operand(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<usize> {
        schema.authored_source_rule_procedure_operand()
    }

    #[test]
    fn selected_captured_values_keep_the_original_declaration_producer() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let source = "interp alias {} invoke {} apply {{a b} {return ok}}\ninvoke 1";
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = crate::analyser::Analyser::new().analyse(source, profile);
            let config = analysis.body_lexer_config.unwrap();
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            let words =
                source_registry_words(source, &analysis, commands.last().unwrap()).expect(profile);
            assert!(matches!(
                words.origins().get(1),
                Some(crate::registry_invocation::InvocationWordOrigin::BindingPrefix(0))
            ));
            assert!(
                words.operands()[0].is_none(),
                "captured value has no written call operand"
            );
            let input = words.original_argument_value_input(0).unwrap_or_else(|| {
                panic!(
                    "missing original captured producer for {profile}; source purpose {:?}",
                    std::mem::discriminant(words.source())
                )
            });
            assert_eq!(input.bytes(), b"{a b} {return ok}");
            let parent = input
                .original_word_key()
                .expect("original alias-definition word");
            assert_eq!(&source[parent.span().as_range()], "{{a b} {return ok}}");
            assert!(parent.span().end() < words.head_source().unwrap().span().start());
            let producer = words
                .captured_argument_values()
                .expect("separate conditional source-value purpose");
            assert!(!producer.lineage().is_empty());
            assert!(!producer.obligations().is_empty());
            assert!(
                source_registry_words(
                    &(source.to_owned() + " "),
                    &analysis,
                    commands.last().unwrap()
                )
                .is_none()
            );
        }
        for source in [
            "interp alias {} invoke {} apply $lambda\ninvoke 1",
            "interp alias {} invoke {} apply {{a b} {return ok}}\nproc invoke {x} {}\ninvoke 1",
            "interp alias {} invoke {} apply {{a b} {return ok}}\nrename invoke {}\ninvoke 1",
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            let commands = crate::segmenter::segment_commands_with_offset_and_config(
                source,
                0,
                analysis.body_lexer_config.unwrap(),
            );
            assert!(
                source_registry_words(source, &analysis, commands.last().unwrap())
                    .is_none_or(|words| words.original_argument_value_input(0).is_none())
            );
        }
    }

    #[test]
    fn original_scoped_words_keep_body_issuer_and_selected_subcommand_schema() {
        // naming.source.original-scoped-command-schema
        // docs/design/analysis/name-resolution-proofs/original-scoped-command-schema.md
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        // Source advice only: no report package installation or safe-interpreter
        // aliases are asserted by this candidate schema test.
        let source = "::report::defstyle st {} {\n    top {bogus}\n}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let start = source.find("top").unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            &source[start..start + 11],
            u32::try_from(start).unwrap(),
            config,
        )
        .remove(0);
        let words =
            source_registry_words(source, &analysis, &segment).expect("sealed scoped words");
        let OriginalRegistrySource::Scoped(scoped) = words.source() else {
            panic!("scoped issuer required");
        };
        assert_eq!(scoped.command().name, "top");
        assert_eq!(
            scoped.body().original_body().image(),
            &tcl_lexer::SourceImage::document(source)
        );
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert!(matches!(
            words
                .with_source_schema(&context, selected_subcommand_diagnostic)
                .flatten(),
            Some(tcl_registry::AuthoredSourceSubcommandDiagnostic::Unknown { .. })
        ));
        assert_eq!(
            words.operands()[0]
                .as_ref()
                .unwrap()
                .word()
                .unwrap()
                .bytes(),
            b"{bogus}"
        );
        assert!(
            source_registry_words(&source.replace("bogus", "other"), &analysis, &segment).is_none()
        );
        assert!(analysis.diagnostics.iter().any(|diagnostic| diagnostic.code
            == tcl_core_types::DiagCode::W001
            && diagnostic.registry_source().is_some()));
    }

    #[test]
    fn original_source_static_values_keep_escapes_separate_from_frozen_runtime_values() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        // This verifies shared source-value capture; it asserts no executed
        // conversion, runtime handler identity or successful native invocation.
        // naming.source.original-static-escape-values
        // docs/design/analysis/name-resolution-proofs/original-source-static-escape-values.md
        // SourceEscapes173 independently caught bare/quoted/braced values on
        // all five C releases and Jim. Ordinary ASCII hex projections establish
        // these values; this test claims no native getter or NUL representation.
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let hex_boundary = if matches!(dialect, "tcl8.4" | "tcl8.5") {
                b"[".as_slice()
            } else {
                b"%b".as_slice()
            };
            for (source, expected) in [
                (r"format \x25b 5", hex_boundary),
                (r#"format "\x25b" 5"#, hex_boundary),
                (r"format {\x25b} 5", br"\x25b".as_slice()),
                (r"format \x25\u0062 5", b"%b".as_slice()),
                (r#"format "\x25\u0062" 5"#, b"%b".as_slice()),
                (r"format {\x25\u0062} 5", br"\x25\u0062".as_slice()),
            ] {
                let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
                let config = analysis.body_lexer_config.unwrap();
                let segment =
                    crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                        .remove(0);
                let words = source_registry_words(source, &analysis, &segment)
                    .unwrap_or_else(|| panic!("{dialect}: {source}"));
                assert_eq!(
                    words.arguments()[0].literal_bytes(),
                    Some(expected),
                    "{dialect}: {source}"
                );
                assert_eq!(
                    words.operands()[0]
                        .as_ref()
                        .unwrap()
                        .word()
                        .unwrap()
                        .bytes(),
                    source.as_bytes().get(7..source.len() - 2).unwrap()
                );
            }
            let source = "set template %b; format $template 5";
            let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
            let config = analysis.body_lexer_config.unwrap();
            let segment =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .pop()
                    .unwrap();
            let words = source_registry_words(source, &analysis, &segment)
                .unwrap_or_else(|| panic!("{dialect}: dynamic template"));
            assert!(
                words.arguments()[0].literal_bytes().is_none(),
                "{dialect}: source substitution cannot borrow a prior cell value"
            );
        }
    }

    #[test]
    fn original_logical_source_schema_selects_its_positive_input_before_native_advice() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "expr $left + $right\n";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let words = source_registry_words(source, &analysis, &segment).unwrap();
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("Logical input must select its own source advice domain");
        };
        assert_eq!(advice.logical_source_input(), Some(&input));
        assert!(advice.original_head().logical_input().is_some());
        assert!(words.head_source().unwrap().input().is_none());
        assert!(!words.operands_preserve_source_lookup());
        assert!(
            words
                .roles()
                .unwrap()
                .iter()
                .all(|(_, role)| *role == tcl_registry::ArgRole::Expr)
        );
        let command = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap()
        .commands
        .remove(0);
        let realm = analysis.retained_command_realm().unwrap();
        assert_eq!(
            original_logical_registry_words_for_command(realm, &input, &command),
            Some(words)
        );
        let foreign = std::sync::Arc::new(
            context.with_command_store(context.commands().snapshot().shared_registry()),
        );
        let foreign_input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, foreign, config);
        assert!(
            original_logical_registry_words_for_command(realm, &foreign_input, &command).is_none()
        );
        let mut changed_config = config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        let changed =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, changed_config);
        assert!(original_logical_registry_words_for_command(realm, &changed, &command).is_none());
        let native = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            original_logical_registry_words_for_command(
                native.retained_command_realm().unwrap(),
                native.resolved_input.as_ref().unwrap(),
                &command,
            )
            .is_none()
        );
    }

    #[test]
    fn original_vendor_source_schema_keeps_comment_trivia_out_of_word_identity() {
        // naming.consumer.original-diagnostic-source-actions
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
        let source = "# proc html_encode is only a comment\nwhen HTTP_REQUEST {HTTP::respond 200 content ok}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "f5-irules");
        let config = analysis.body_lexer_config.unwrap();
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        assert!(command.preceding_comment.is_some());
        let offset = command.argv[0].span.start();
        let (metadata, projected) =
            selected_vendor_registry_words_at(source, &analysis, offset).unwrap();
        assert!(projected.preceding_comment.is_none());
        assert_ne!(command, projected);
        let words = source_registry_words(source, &analysis, &command).unwrap();
        assert!(matches!(words.source(), OriginalRegistrySource::Vendor(_)));
        assert_eq!(
            words.head_source().unwrap().word().unwrap(),
            metadata.shape().original_words().first().unwrap()
        );
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let bodies = words.source_script_bodies(&context);
        assert_eq!(bodies.len(), 1);
        assert!(bodies[0].matches_source(&tcl_lexer::SourceImage::document(source), config));
        assert_eq!(
            bodies[0].original_container(),
            words.operands()[1].as_ref().unwrap().word().unwrap()
        );

        let mut changed_trivia = command.clone();
        changed_trivia.preceding_comment = Some("proc report_only is presentation".to_owned());
        assert_eq!(
            source_registry_words(source, &analysis, &changed_trivia).unwrap(),
            words
        );
        let mut cut = command;
        let _ = cut.argv.pop();
        let _ = cut.word_fragments.pop();
        let _ = cut.texts.pop();
        let _ = cut.single_token_word.pop();
        assert!(
            source_registry_words(source, &analysis, &cut).is_none(),
            "an incomplete native vector cannot borrow the authentic complete invocation"
        );
    }

    #[test]
    fn original_standalone_source_context_keeps_conditional_bodies_and_alias_order() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        // Implementation contract: naming.source.authored-command-transition-advice
        // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = "pick subject {default {puts before}}; interp alias {} pick {} switch; pick subject {default {puts after}}; interp alias {} pick {}; pick subject {default {puts deleted}}";
            let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
                .analyser_profile();
            let context = tcl_registry::model::ingress::context_for_profile(profile);
            let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                config,
            );
            let syntax = OriginalSourceRegistryContext::capture(source, input);
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            assert!(syntax.words(&commands[0]).is_none());
            let words = syntax
                .words(&commands[2])
                .expect("actual authored alias must retain conditional source grammar");
            assert!(matches!(
                words.source(),
                OriginalRegistrySource::SourceTransitions(_)
            ));
            assert!(!words.operands_preserve_source_lookup());
            assert!(
                words
                    .source_script_bodies(&context)
                    .iter()
                    .any(|body| source.get(body.content_span().as_range()) == Some("puts after"))
            );
            assert!(syntax.words(&commands[4]).is_none());
        }
    }

    #[test]
    fn original_standalone_selected_alias_roles_keep_source_purpose() {
        // Implementation contract: naming.source.authored-registry-role-projection
        // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = "namespace eval ::A {}; interp alias {} ::pick {} ::switch; interp alias {} ::A::pick {} switch; ::pick subject {default {puts first}}; ::A::pick subject {default {puts second}}";
            let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
                .analyser_profile();
            let context = tcl_registry::model::ingress::context_for_profile(profile);
            let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                config,
            );
            let syntax = OriginalSourceRegistryContext::capture(source, input);
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            for (ordinal, body) in [(3, "puts first"), (4, "puts second")] {
                let words = syntax
                    .words(&commands[ordinal])
                    .expect("genuine selected alias grammar");
                assert!(matches!(words.source(), OriginalRegistrySource::Selected));
                assert!(!words.operands_preserve_source_lookup());
                assert_eq!(
                    words.head_source().unwrap().word().unwrap().span(),
                    commands[ordinal].argv[0].span
                );
                assert!(
                    words
                        .source_script_bodies(&context)
                        .iter()
                        .any(|region| source.get(region.content_span().as_range()) == Some(body))
                );
            }
            for changed in [
                "interp alias {} pick {} switch; interp alias {} pick {}; pick subject {default {puts stale}}",
                "interp alias {} pick {} switch; proc pick args {return other}; pick subject {default {puts stale}}",
            ] {
                let syntax =
                    OriginalSourceRegistryContext::capture(changed, syntax.editing_input().clone());
                let commands =
                    crate::segmenter::segment_commands_with_offset_and_config(changed, 0, config);
                assert!(syntax.words(commands.last().unwrap()).is_none());
            }
        }
    }

    #[test]
    fn original_standalone_source_context_keeps_nested_unknown_selector_source_only() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = "set result [switch $kind {alpha {# physical comment\nputs child}}]";
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let syntax = OriginalSourceRegistryContext::capture(source, input);
        let start = source.find("switch").unwrap();
        let end = source.rfind(']').unwrap();
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            &source[start..end],
            u32::try_from(start).unwrap(),
            config,
        )
        .remove(0);
        let words = syntax
            .words(&command)
            .expect("physical case grammar survives an unknown selector value");
        assert!(matches!(
            words.arguments().first(),
            Some(crate::registry_invocation::EffectiveInvocationWord::Dynamic)
        ));
        let original_selector = words
            .operands()
            .first()
            .unwrap()
            .as_ref()
            .unwrap()
            .word()
            .unwrap();
        assert_eq!(
            original_selector.image(),
            &tcl_lexer::SourceImage::document(source)
        );
        assert_eq!(original_selector.config(), config);
        assert!(!words.operands_preserve_source_lookup());
        let bodies = words.source_script_bodies(&context);
        assert_eq!(bodies.len(), 1);
        assert_eq!(
            source.get(bodies[0].content_span().as_range()),
            Some("# physical comment\nputs child")
        );
        assert_eq!(
            syntax.source_image(),
            &tcl_lexer::SourceImage::document(source)
        );
        assert_eq!(syntax.editing_input().lexer_config(), config);
    }

    #[test]
    fn original_deferred_source_schema_keeps_future_entry_and_current_lookup_distinct() {
        // naming.package.original-file-candidate-provenance
        // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
        let source = "package ifneeded P 1.0 {source actual.tcl}";
        for version in tcl_dialect::TclVersion::ALL {
            let analysis = crate::analyser::Analyser::new().analyse(source, version.dialect_name());
            let config = analysis.body_lexer_config.unwrap();
            let offset = u32::try_from(source.find("source actual.tcl").unwrap()).unwrap();
            let command = crate::segmenter::segment_commands_with_offset_and_config(
                "source actual.tcl",
                offset,
                config,
            )
            .remove(0);
            let registry = analysis.resolved_registry().unwrap();
            assert!(selected_registry_words(source, &analysis, &command, registry).is_none());
            let words = source_registry_words(source, &analysis, &command)
                .unwrap_or_else(|| panic!("{}", version.dialect_name()));
            let OriginalRegistrySource::Conditional(metadata) = words.source() else {
                panic!("future syntax cannot become selected document lookup");
            };
            assert!(metadata.obligations().contains(
                &crate::command_binding::OriginalCatalogueSourceObligation::UnknownFutureEntry,
            ));
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            assert_eq!(
                words.with_source_schema(&context, |schema| schema.semantics.analyser_hook),
                Some(Some(tcl_registry::hooks::AnalyserHookId::Source))
            );
            assert_eq!(
                words.operands()[0]
                    .as_ref()
                    .unwrap()
                    .input()
                    .unwrap()
                    .bytes(),
                b"actual.tcl"
            );
            assert_eq!(
                words.head_source().unwrap().word().unwrap().image(),
                &tcl_lexer::SourceImage::document(source)
            );
            assert!(
                source_registry_words(&source.replace("actual", "other"), &analysis, &command)
                    .is_none()
            );
            let mut changed = analysis.clone();
            let changed_config = changed.body_lexer_config.as_mut().unwrap();
            changed_config.strict_quoting = !changed_config.strict_quoting;
            assert!(source_registry_words(source, &changed, &command).is_none());
            let mut foreign = analysis.clone();
            let original = foreign.resolved_input.as_ref().unwrap();
            let profile = original.analyser_profile();
            let generation = original.context_registry();
            foreign.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
                profile,
                original.unit_profile(),
                std::sync::Arc::new(
                    generation
                        .with_command_store(generation.commands().snapshot().shared_registry()),
                ),
                original.lexer_config(),
            ));
            assert!(source_registry_words(source, &foreign, &command).is_none());
        }
    }

    #[test]
    fn original_deferred_source_schema_keeps_shadow_and_produced_script_boundaries() {
        // naming.package.original-file-candidate-provenance
        // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
        for prefix in ["proc source args {return custom}\n", "rename source {}\n"] {
            let source = format!("{prefix}package ifneeded P 1.0 {{source actual.tcl}}");
            let analysis = crate::analyser::Analyser::new().analyse(&source, "tcl8.6");
            let config = analysis.body_lexer_config.unwrap();
            let offset = u32::try_from(source.find("source actual.tcl").unwrap()).unwrap();
            let command = crate::segmenter::segment_commands_with_offset_and_config(
                "source actual.tcl",
                offset,
                config,
            )
            .remove(0);
            assert!(source_registry_words(&source, &analysis, &command).is_none());
        }
        let source = "package ifneeded P 1.0 [list source [file join $dir produced.tcl]]";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let words = source_registry_words(source, &analysis, &command).unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert!(words.source_script_bodies(&context).is_empty());
    }

    #[test]
    fn selected_source_roles_retain_procedure_grammar_under_the_actual_context() {
        // naming.source.authored-registry-role-projection
        // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "jim"] {
            let source = "proc p {a b} {return}";
            let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
            let config = analysis.body_lexer_config.unwrap();
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let registry = analysis.resolved_registry().unwrap();
            let words =
                selected_registry_words(source, &analysis, &command, registry).expect(dialect);
            assert_eq!(
                words.roles(),
                Some(
                    [
                        (0, tcl_registry::ArgRole::Name),
                        (1, tcl_registry::ArgRole::ParamList),
                        (2, tcl_registry::ArgRole::Body),
                    ]
                    .as_slice()
                ),
                "{dialect}"
            );
            let parameters = words.operands()[1].as_ref().unwrap();
            let tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                config,
                &command,
            );
            let native = crate::registry_invocation::original_native_compiler_words(
                &tcl_lexer::SourceImage::document(source),
                tokens.words(),
                0,
                config,
            )
            .unwrap();
            assert_eq!(parameters.span(), command.argv[2].span);
            assert_eq!(parameters.word().unwrap(), &native[2]);
            assert!(
                words.operands()[1].as_ref().unwrap().input().is_some(),
                "{dialect}"
            );
            assert!(
                selected_registry_words("proc q {a b} {return}", &analysis, &command, registry)
                    .is_none()
            );
            let foreign = tcl_registry::model::context_for_profile(
                tcl_dialect::DialectProfile::find("tcl8.5").unwrap(),
            );
            assert!(words.with_source_schema(&foreign, |_| ()).is_none());
        }
    }

    #[test]
    fn original_source_structure_is_shared_and_owner_checked() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "set first 1\nset second 2\n";
        let mut analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        analysis.command_invocations.clear();
        analysis.global_scope.variables.clear();
        let config = analysis.body_lexer_config.unwrap();
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let words = source_registry_words(source, &analysis, &commands[1]).unwrap();
        let head = words.head_source().unwrap();
        assert_eq!(head.input().unwrap().bytes(), b"set");
        assert_eq!(head.word().unwrap().span(), commands[1].argv[0].span);
        assert_eq!(words.arguments().len(), 2);
        assert!(
            words
                .roles()
                .unwrap()
                .contains(&(0, tcl_registry::ArgRole::VarWrite))
        );
        assert_eq!(
            words.operands()[0].as_ref().unwrap().span(),
            commands[1].argv[1].span
        );
        assert!(words.matches_source(&tcl_lexer::SourceImage::document(source), config));
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let roles = words
            .with_source_schema(&context, selected_argument_roles)
            .unwrap();
        assert!(roles.1);
        assert!(roles.0.contains(&(0, tcl_registry::ArgRole::VarWrite)));
        assert!(!words.matches_source(
            &tcl_lexer::SourceImage::document(&source.replace("second", "absent")),
            config
        ));
        assert!(!words.matches_source(
            &tcl_lexer::SourceImage::document(source),
            tcl_lexer::LexerConfig {
                expand_syntax: !config.expand_syntax,
                ..config
            }
        ));
        let foreign = tcl_registry::model::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
        );
        assert!(words.with_source_schema(&foreign, |_| ()).is_none());
    }
    #[test]
    fn original_hosted_source_schema_keeps_controls_and_unknown_single_slots() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "call -debug Lib::helper $payload\npool $target\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "f5-irules");
        let config = analysis.body_lexer_config.unwrap();
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let call = source_registry_words(source, &analysis, &commands[0]).unwrap();
        assert!(matches!(
            call.arguments()[0],
            crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(_)
        ));
        assert!(matches!(
            call.arguments()[2],
            crate::registry_invocation::EffectiveInvocationWord::Dynamic
        ));
        assert_eq!(
            call.with_source_schema(&context, selected_rule_procedure_operand),
            Some(Some(1))
        );
        assert!(matches!(
            call.arguments()[0],
            crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(_)
        ));
        let pool = source_registry_words(source, &analysis, &commands[1]).unwrap();
        assert!(matches!(
            pool.arguments()[0],
            crate::registry_invocation::EffectiveInvocationWord::Dynamic
        ));
        assert_eq!(
            pool.with_source_schema(&context, |schema| schema.words.arguments().exact_argv_len()),
            Some(Some(1))
        );
        assert!(pool.operands()[0].as_ref().unwrap().word().is_some());
    }
    #[test]
    fn original_conditional_operand_keeps_its_static_word_and_list_lineage() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "unavailable\n::foreach {a b} {1 2} {}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        );
        let words = source_registry_words(source, &analysis, &commands[1]).unwrap();
        assert!(matches!(
            words.source(),
            OriginalRegistrySource::Conditional(_)
        ));
        let operand = words.operands()[0].as_ref().unwrap();
        let input = operand.input().unwrap();
        assert_eq!(input.bytes(), b"a b");
        assert_eq!(
            input.original_word_key().unwrap().original_word(),
            operand.word().unwrap()
        );
        let children = input.original_list_elements_with_source_spans().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].0.bytes(), b"a");
        assert!(
            children
                .iter()
                .all(|(child, span)| child.original_word_key().is_none() && span.is_some())
        );
    }
}

#[cfg(test)]
mod original_procedure_argument_tests {
    use super::*;
    use crate::analyser::Analyser;
    use tcl_syntax::formal_params::FormalByteArgumentBinding;

    #[test]
    fn original_procedure_arguments_keep_actual_formals_prefixes_and_child_anchors() {
        // naming.source.original-procedure-argument-topology
        // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
        for source in [
            "proc target {captured written} {}; target {*}{FIXED VALUE}",
            "proc target {captured written} {}; interp alias {} alias {} target FIXED; alias {*}{VALUE}",
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl8.6");
            analysis.all_procs.clear();
            analysis.global_scope.procs.clear();
            analysis.command_invocations.clear();
            let config = analysis.body_lexer_config.unwrap();
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            let call = commands.last().unwrap();
            let prototype = original_procedure_arguments(source, &analysis, call)
                .expect("actual source prototype");
            assert_eq!(prototype.formals().parameters().len(), 2);
            assert_eq!(prototype.arguments().len(), 2);
            assert_eq!(
                prototype
                    .formals()
                    .name_field(1)
                    .unwrap()
                    .original_input()
                    .bytes(),
                b"written"
            );
            assert!(
                prototype
                    .bindings()
                    .contains(&FormalByteArgumentBinding::Value {
                        parameter: 1,
                        argument: 1
                    })
            );
            let operand = prototype.operands()[1]
                .as_ref()
                .expect("actual expansion child");
            assert!(operand.word().is_none());
            assert_eq!(operand.input().unwrap().bytes(), b"VALUE");
            assert_eq!(&source[operand.span().as_range()], "VALUE");
            if source.contains("interp alias") {
                assert!(prototype.operands()[0].is_none());
            }
            assert!(prototype.matches_source(&tcl_lexer::SourceImage::document(source), config));
            assert!(prototype.matches_registry(analysis.resolved_registry().unwrap()));
            assert!(original_procedure_arguments(&format!("#{source}"), &analysis, call).is_none());
        }
        let source = "proc target {first {second DEFAULT}} {}; target ONLY";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        );
        let prototype =
            original_procedure_arguments(source, &analysis, commands.last().unwrap()).unwrap();
        assert!(
            prototype
                .bindings()
                .contains(&FormalByteArgumentBinding::Default { parameter: 1 })
        );
        assert_eq!(prototype.operands().len(), 1);
    }

    #[test]
    fn original_procedure_arguments_keep_missing_original_correspondence_terminal() {
        // naming.source.original-procedure-argument-topology
        // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
        let source = "proc target {value} {}; target ONE";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let call = commands.last().unwrap();
        assert!(original_procedure_arguments(source, &analysis, call).is_some());
        analysis
            .command_invocations
            .iter_mut()
            .find(|row| row.range.start() == call.argv[0].span.start())
            .unwrap()
            .original_lookup = None;
        assert!(original_procedure_arguments(source, &analysis, call).is_none());
        let source = "proc target {value} {}; set values ONE; target {*}$values";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        );
        let prototype =
            original_procedure_arguments(source, &analysis, commands.last().unwrap()).unwrap();
        assert_eq!(
            prototype.arguments()[0].literal_bytes(),
            Some(b"ONE".as_slice())
        );
        assert!(
            prototype.operands()[0].is_none(),
            "a frozen list value cannot borrow its earlier setter's source extent"
        );
    }
}

#[cfg(test)]
mod source_lookup_boundary_tests {
    use super::*;

    #[test]
    fn original_source_lookup_facet_is_independent_of_advisory_body_roles() {
        // Implementation contract: naming.source.closed-syntax-script-regions
        // docs/design/analysis/name-resolution-proofs/closed-syntax-script-regions.md
        for (source, preserved) in [
            ("if {1} {puts quiet}\n", true),
            ("if [rename if saved] {puts data}\n", false),
            ("unavailable\nif {1} {puts advisory}\n", false),
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            let config = analysis.body_lexer_config.unwrap();
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            let words = source_registry_words(source, &analysis, commands.last().unwrap()).unwrap();
            assert_eq!(
                words.roles(),
                Some(
                    [
                        (0, tcl_registry::ArgRole::Expr),
                        (1, tcl_registry::ArgRole::Body)
                    ]
                    .as_slice()
                ),
                "ordinary unknown expression payload preserves source positions: {source}"
            );
            if source.contains("[rename") {
                assert!(matches!(
                    words.arguments()[0],
                    crate::registry_invocation::EffectiveInvocationWord::Dynamic
                ));
            }
            assert_eq!(
                words.operands_preserve_source_lookup(),
                preserved,
                "{source}"
            );
        }
    }
}

#[cfg(test)]
mod source_word_dialect_tests {
    use super::*;

    #[test]
    fn original_source_schema_uses_retained_word_dialect_for_file_grammar() {
        // naming.core.original-document-link-selection
        // docs/design/analysis/name-resolution-proofs/original-document-link-selection.md
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "jim"] {
            let source = "source {a $b [c].tcl}";
            let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
            let config = analysis.body_lexer_config.unwrap();
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let words = source_registry_words(source, &analysis, &command).expect(dialect);
            let grammar = words
                .dialect
                .and_then(tcl_registry::InvocationDialect::source_file_grammar)
                .expect(dialect);
            assert!(
                matches!(
                    grammar.select_original(
                        words.arguments.len(),
                        words
                            .arguments
                            .first()
                            .and_then(|word| word.literal_bytes())
                    ),
                    tcl_registry::source_file::SourceFileSelection::Selected(_)
                ),
                "{dialect}"
            );
            assert_eq!(
                words.arguments[0].literal_bytes(),
                Some(b"a $b [c].tcl".as_slice())
            );
            assert!(
                words.operands[0].as_ref().unwrap().input().is_some(),
                "{dialect}"
            );
            assert!(source_registry_words(&format!("#{source}"), &analysis, &command).is_none());
        }
    }
    #[test]
    fn original_package_source_point_keeps_whole_image_context_and_alias_grammar() {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        let source = "interp alias {} need {} package require -exact csv\nneed 1";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("need 1").unwrap()).unwrap();
        let words = super::source_registry_words_at(source, &analysis, offset).unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let reference = words.package_reference(&context).unwrap();
        assert_eq!(
            reference.kind(),
            tcl_registry::source_navigation::SourcePackageReferenceKind::Require
        );
        assert!(reference.matches_ascii("csv"));
        assert!(!reference.matches_ascii("other"));
        assert!(
            super::source_registry_words_at(&source.replace("need 1", "need 2"), &analysis, offset)
                .is_none()
        );
        let foreign = tcl_registry::model::ingress::static_context_for("tcl8.5");
        assert!(words.package_reference(foreign).is_none());
        let mut stale = analysis.clone();
        let original = stale.resolved_input.as_ref().unwrap();
        let mut config = original.lexer_config();
        config.leading_bom = match config.leading_bom {
            tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
            tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
        };
        stale.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            original.analyser_profile(),
            original.unit_profile(),
            original.context_registry(),
            config,
        ));
        assert!(super::source_registry_words_at(source, &stale, offset).is_none());
    }
    #[test]
    fn original_package_insertion_uses_root_commands_split_lines_and_full_input() {
        // naming.core.original-package-source-action-context
        // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
        let sources = [
            (
                r"#!/usr/bin/env tclsh
package \
 require \
 csv
set x 1
",
                0usize,
            ),
            ("proc later {} {\n package require csv\n}\n", 0usize),
            ("set text {package require csv}\n", 0usize),
            ("package provide csv\n", 0usize),
            ("package require csv", "package require csv".len()),
        ];
        for (index, (source, fallback)) in sources.into_iter().enumerate() {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            let expected = if index == 0 {
                source.find("set x").unwrap()
            } else {
                fallback
            };
            assert_eq!(
                super::original_package_require_insert_offset(source, &analysis),
                Some(u32::try_from(expected).unwrap()),
                "{source:?}"
            );
            assert!(
                super::original_package_require_insert_offset(
                    &format!("# foreign\n{source}"),
                    &analysis
                )
                .is_none()
            );
        }
    }
}

#[cfg(test)]
mod retained_image_geometry_tests {
    use super::{matching_document_image, retained_document_image, source_registry_words};

    #[test]
    fn source_schema_geometry_reuses_the_whole_retained_image_and_keeps_currency() {
        // naming.source.original-immutable-image-geometry
        // docs/design/analysis/name-resolution-proofs/original-immutable-image-geometry.md
        let source = "set café 1\r\nset λ 2";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            config,
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let realm = analysis.retained_command_realm().unwrap();
        let original = realm.original_source_image().unwrap();
        let reused = retained_document_image(source, realm, config).unwrap();
        assert_eq!(reused.bytes().as_ptr(), original.bytes().as_ptr());
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        assert_eq!(segments.len(), 2);
        for segment in &segments {
            let words = source_registry_words(source, &analysis, segment).unwrap();
            assert!(words.matches_source(original, config));
            let word = words.operands()[0].as_ref().unwrap().word().unwrap();
            assert_eq!(word.image().bytes().as_ptr(), original.bytes().as_ptr());
            assert_eq!(word.config(), config);
        }
        let start = u32::try_from(source.find("set λ").unwrap()).unwrap();
        let position = reused.source_map().position_at(start);
        assert_eq!(position.line, 1);
        assert_eq!(position.character.get(), 0);
        assert!(retained_document_image(&format!("{source} "), realm, config).is_none());
        let changed = tcl_lexer::LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        };
        assert!(retained_document_image(source, realm, changed).is_none());
        assert!(
            matching_document_image(source, &tcl_lexer::SourceImage::native(source.as_bytes()))
                .is_none()
        );
    }
}
