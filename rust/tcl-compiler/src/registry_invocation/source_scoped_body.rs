// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original source bodies retain their selected scoped candidate schema.

use std::sync::Arc;

use super::{OriginalConditionalRegistryMetadata, OriginalConditionalVendorRegistryMetadata};
use crate::ir::CommandTokens;
use crate::signature_scan::original_name::SourceOriginalNameOccurrence;
use crate::signature_scan::scope::SignatureNamespaceScope;
use crate::signature_scan::vendor_name::VendorSourceNameOccurrence;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span, TokenType};
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_registry::{ArgRole, RegistrySemanticKey};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScopedBodySource {
    Selected(Arc<super::RegistryInvocationAssistance>),
    Conditional(Arc<OriginalConditionalRegistryMetadata>),
    Vendor(Arc<OriginalConditionalVendorRegistryMetadata>),
}

/// Source-only command candidates within one genuine original braced body.
/// The complete original word, actual context and source applicability remain
/// retained. This grants no entered frame, live command table, Normal or edit.
#[derive(Debug, Clone)]
pub struct OriginalSourceScopedBody {
    body: NativeWord,
    content: Span,
    context: ResolvedContext,
    registry: RegistrySemanticKey,
    source: ScopedBodySource,
    parent: &'static tcl_registry::CommandSpec,
    environment: &'static tcl_registry::scoped::ScopedCommandEnv,
}
impl PartialEq for OriginalSourceScopedBody {
    fn eq(&self, other: &Self) -> bool {
        self.body == other.body
            && self.content == other.content
            && self.context == other.context
            && self.registry == other.registry
            && self.source == other.source
            && std::ptr::eq(self.parent, other.parent)
            && std::ptr::eq(self.environment, other.environment)
    }
}
impl OriginalSourceScopedBody {
    /// Complete original body word; no reconstructed script or naming key.
    #[must_use]
    pub const fn original_body(&self) -> &NativeWord {
        &self.body
    }
    /// Exact body content extent in the retained full image.
    #[must_use]
    pub const fn content_span(&self) -> Span {
        self.content
    }
    /// Independently selected owning descriptor, including inherited availability.
    /// This metadata does not establish a live provider implementation.
    #[must_use]
    pub const fn parent_descriptor(&self) -> &'static tcl_registry::CommandSpec {
        self.parent
    }
    /// Registry-authored candidate vocabulary, independently of live presence.
    #[must_use]
    pub const fn environment(&self) -> &'static tcl_registry::scoped::ScopedCommandEnv {
        self.environment
    }
    /// Actual complete source availability context used by this schema.
    #[must_use]
    pub const fn context(&self) -> &ResolvedContext {
        &self.context
    }
    /// Conditional Native applicability remains separately visible.
    #[must_use]
    pub fn conditional_metadata(&self) -> Option<&OriginalConditionalRegistryMetadata> {
        match &self.source {
            ScopedBodySource::Conditional(metadata) => Some(metadata),
            _ => None,
        }
    }
    /// Hosted applicability keeps its independent policy and obligations.
    #[must_use]
    pub fn vendor_metadata(&self) -> Option<&OriginalConditionalVendorRegistryMetadata> {
        match &self.source {
            ScopedBodySource::Vendor(metadata) => Some(metadata),
            _ => None,
        }
    }
    /// Whole source/channel and full config correspondence only.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.body.image() == image && self.body.executable_parts().config() == config
    }
    /// Exact Registry semantic contents used by the retained source schema.
    #[must_use]
    pub fn matches_registry(&self, registry: &tcl_registry::CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}

/// One scoped source command selected from a sealed original body issuer.
/// Descriptor metadata is owned independently of global Registry name lookup.
#[derive(Debug, Clone)]
pub struct OriginalSourceScopedCommandSchema {
    body: OriginalSourceScopedBody,
    command: &'static tcl_registry::scoped::ScopedCommand,
    descriptor: tcl_registry::CommandSpec,
}
impl PartialEq for OriginalSourceScopedCommandSchema {
    fn eq(&self, other: &Self) -> bool {
        self.body == other.body && std::ptr::eq(self.command, other.command)
    }
}
impl Eq for OriginalSourceScopedCommandSchema {}
impl OriginalSourceScopedCommandSchema {
    pub(crate) fn select(body: &OriginalSourceScopedBody, name: &str) -> Option<Self> {
        let command = body.environment().command(name)?;
        Some(Self {
            body: body.clone(),
            command,
            descriptor: command.source_descriptor(body.parent_descriptor()),
        })
    }
    /// Genuine body, actual context and source applicability of this selection.
    #[must_use]
    pub const fn body(&self) -> &OriginalSourceScopedBody {
        &self.body
    }
    /// Registry-authored scoped vocabulary entry; no global command identity.
    #[must_use]
    pub const fn command(&self) -> &'static tcl_registry::scoped::ScopedCommand {
        self.command
    }
    pub(crate) const fn descriptor(&self) -> &tcl_registry::CommandSpec {
        &self.descriptor
    }
}

fn retain_body(
    context: &ContextRegistry,
    body: &NativeWord,
    source: ScopedBodySource,
    environment: &'static tcl_registry::scoped::ScopedCommandEnv,
    parent: &'static tcl_registry::CommandSpec,
) -> Option<OriginalSourceScopedBody> {
    if body.group().expand || body.tokens().len() != 1 || body.tokens()[0].kind != TokenType::Str {
        return None;
    }
    Some(OriginalSourceScopedBody {
        body: body.clone(),
        content: body.content_span().ok()?,
        context: context.context().clone(),
        registry: context.commands().snapshot().semantic_key(),
        source,
        parent,
        environment,
    })
}

/// Issue candidate-only body descriptors through shared original lookup and
/// role owners. The hosted source path never selects a C/Jim naming recipe.
pub(crate) fn original_source_scoped_bodies(
    context: &ContextRegistry,
    tokens: &CommandTokens,
    head: Option<&SourceOriginalNameOccurrence>,
    namespace: Option<&SignatureNamespaceScope>,
    vendor: Option<&VendorSourceNameOccurrence>,
) -> Vec<OriginalSourceScopedBody> {
    if let Some(vendor) = vendor {
        let Some(metadata) =
            super::original_vendor_occurrence_registry_metadata(context, tokens, vendor)
        else {
            return Vec::new();
        };
        let metadata = Arc::new(metadata);
        let shape = metadata.shape();
        let Some(parent) = context
            .context()
            .resolve_spec(context.commands(), shape.command())
        else {
            return Vec::new();
        };
        let Some(environment) = parent.body_scope else {
            return Vec::new();
        };
        if !shape.roles_complete() {
            return Vec::new();
        }
        return shape
            .roles()
            .iter()
            .filter(|(_, role)| *role == ArgRole::Body)
            .filter_map(|(ordinal, _)| {
                let index = shape
                    .argument_offset()
                    .checked_add(usize::from(*ordinal))?
                    .checked_add(1)?;
                retain_body(
                    context,
                    shape.original_words().get(index)?,
                    ScopedBodySource::Vendor(metadata.clone()),
                    environment,
                    parent,
                )
            })
            .collect();
    }
    if let Some(selected) = selected_bodies(context, tokens) {
        return selected;
    }
    let Some(metadata) = head.and_then(|head| {
        super::original_conditional_registry_metadata(context, tokens, head, namespace)
    }) else {
        return Vec::new();
    };
    let metadata = Arc::new(metadata);
    let Some(parent) = context
        .context()
        .resolve_spec(context.commands(), metadata.command())
    else {
        return Vec::new();
    };
    let Some(environment) = parent.body_scope else {
        return Vec::new();
    };
    if !metadata.roles_complete() {
        return Vec::new();
    }
    metadata
        .roles()
        .iter()
        .filter(|(_, role)| *role == ArgRole::Body)
        .filter_map(|(ordinal, _)| {
            let index = metadata
                .argument_offset()
                .checked_add(usize::from(*ordinal))?
                .checked_add(1)?;
            retain_body(
                context,
                metadata.original_words().get(index)?,
                ScopedBodySource::Conditional(metadata.clone()),
                environment,
                parent,
            )
        })
        .collect()
}

fn selected_bodies(
    context: &ContextRegistry,
    tokens: &CommandTokens,
) -> Option<Vec<OriginalSourceScopedBody>> {
    let registry = context.commands();
    let assistance = Arc::new(super::original_registry_invocation_assistance(
        registry, None, tokens,
    )?);
    let selected = assistance.unanimous_command_words()?;
    let Some(parent) = context.context().resolve_spec(registry, selected.command()) else {
        return Some(Vec::new());
    };
    let Some(environment) = parent.body_scope else {
        return Some(Vec::new());
    };
    let effective = selected.effective();
    let values = super::frozen_argument_words(tokens, effective);
    let args = values
        .iter()
        .map(super::EffectiveInvocationWord::as_registry_word)
        .collect::<Vec<_>>();
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            registry,
            Some(context.context()),
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(selected.command()),
                &args,
            ),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
    let resolved = resolution.resolved()?;
    let (roles, complete) = resolved.authored_source_argument_roles();
    if !complete {
        return None;
    }
    let config = tokens
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(tokens)?;
    let image = tokens
        .source_binding
        .as_ref()?
        .invocation_site()?
        .source
        .source_image();
    let offset = tokens.words().first()?.source().span.start();
    let original = super::original_native_compiler_words(image, tokens.words(), offset, config)?;
    let offset = resolved.facts().argument_offset;
    Some(
        roles
            .iter()
            .filter(|(_, role)| *role == ArgRole::Body)
            .filter_map(|(ordinal, _)| {
                let index = effective
                    .written_argument(offset.checked_add(usize::from(*ordinal))?)?
                    .checked_add(1)?;
                retain_body(
                    context,
                    original.get(index)?,
                    ScopedBodySource::Selected(assistance.clone()),
                    environment,
                    parent,
                )
            })
            .collect(),
    )
}

impl crate::analyser::AnalysisResult {
    /// Select the innermost independently retained source scoped-body schema.
    /// Reporting regions and stale/ambiguous source owners never donate it.
    #[must_use]
    pub fn original_scoped_body_in_source(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        offset: u32,
    ) -> Option<&OriginalSourceScopedBody> {
        if !self.matches_original_source_image(image, config) {
            return None;
        }
        let registry = self.resolved_registry()?;
        let context = self.resolved_input.as_ref()?.context_registry();
        let mut selected: Option<&OriginalSourceScopedBody> = None;
        for body in &self.original_scoped_bodies {
            let span = body.content_span();
            if !body.matches_source(image, config)
                || !body.matches_registry(registry)
                || body.context() != context.context()
                || offset < span.start()
                || offset >= span.end()
            {
                continue;
            }
            if let Some(previous) = selected {
                let prior = previous.content_span();
                if prior == span && previous != body {
                    return None;
                }
                if prior.end() - prior.start() <= span.end() - span.start() {
                    continue;
                }
            }
            selected = Some(body);
        }
        selected
    }
}

/// Purpose of an original script region inventory, independent of body entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalSourceScriptPurpose {
    /// Source syntax/navigation includes reference-only body operands.
    Syntax,
    /// Potential evaluation excludes bodies the schema only references.
    /// This still supplies no reached execution, frame, effects or completion.
    PotentialEvaluation,
}

/// One original script region from the guarded authored source-role schema.
/// Its whole source container, original argv and applicability remain retained;
/// a list body keeps its real parent word instead of forging a child `NativeWord`.
/// This grants no scoped vocabulary, entered frame, live cell, Normal or edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceScriptBody {
    source: std::sync::Arc<super::source_structure::OriginalRegistryWords>,
    container: NativeWord,
    extent: Span,
    container_extent: Span,
    content: Span,
}

/// Conditional source-only body candidates for consumers that inspect syntax
/// in every possible case arm. The selected source invocation and its complete
/// projection remain attached even when option selection is unknown; candidates
/// do not establish a chosen body, execution, effects, or Normal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalConditionalSourceCaseBodies {
    source: std::sync::Arc<super::source_structure::OriginalRegistryWords>,
    projection: tcl_registry::case_bodies::CaseBodyOperands,
    bodies: Vec<OriginalSourceScriptBody>,
}

impl OriginalConditionalSourceCaseBodies {
    /// Whether dynamic subject/option interpretation leaves another selection.
    pub(crate) const fn selection_unknown(&self) -> bool {
        self.projection.selection_unknown
    }

    /// Original possible body regions under the retained source projection.
    pub(crate) fn bodies(&self) -> &[OriginalSourceScriptBody] {
        &self.bodies
    }

    /// Complete source image/config and every retained original body match.
    pub(crate) fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.source.matches_source(image, config)
            && self
                .bodies
                .iter()
                .all(|body| body.matches_source(image, config))
    }
}

impl OriginalSourceScriptBody {
    /// Whole original word owning this body or exact source list child.
    #[must_use]
    pub const fn original_container(&self) -> &NativeWord {
        &self.container
    }

    /// Source fold extent; the original closing delimiter stays outside it.
    #[must_use]
    pub const fn extent_span(&self) -> Span {
        self.extent
    }

    /// Original enclosing fold geometry. Case-list bodies retain their whole
    /// list container separately from the script child's own extent.
    #[must_use]
    pub const fn container_extent_span(&self) -> Span {
        self.container_extent
    }

    /// Exact original script content, excluding its genuine delimiters.
    #[must_use]
    pub const fn content_span(&self) -> Span {
        self.content
    }

    /// Same sealed original argv/schema and its explicit applicability.
    #[must_use]
    pub fn source_words(&self) -> &super::source_structure::OriginalRegistryWords {
        &self.source
    }

    /// Complete source/channel and full original parser configuration.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.source.matches_source(image, config)
            && self.container.image() == image
            && self.container.config() == config
    }

    /// Exact contiguous original content inside this retained body. This is
    /// readonly source geometry, not evaluated-script or entered-frame proof.
    pub(crate) fn matches_original_content(
        &self,
        source: &str,
        base: u32,
        config: LexerConfig,
    ) -> bool {
        let Ok(length) = u32::try_from(source.len()) else {
            return false;
        };
        let Some(end) = base.checked_add(length) else {
            return false;
        };
        self.matches_source(self.container.image(), config)
            && self.content.start() <= base
            && end <= self.content.end()
            && self
                .container
                .image()
                .bytes()
                .get(base as usize..end as usize)
                == Some(source.as_bytes())
    }

    /// Complete lexical command vectors inside this sealed original body.
    /// The full container image, channel, configuration and selected context
    /// stay retained. A fatal tail withdraws the whole inventory. These words
    /// grant no child lookup point, runtime body entry, frame or Normal result;
    /// supplied consumers must independently join their current request owner
    /// and each child must retain its own original source sidecar.
    pub(crate) fn original_commands(
        &self,
        context: &ContextRegistry,
    ) -> Option<Vec<tcl_lexer::NativeScriptCommandWords>> {
        if !self.matches_context(context)
            || !self.matches_source(self.container.image(), self.container.config())
        {
            return None;
        }
        let plan = tcl_lexer::native_script_words_in(
            self.container.image().clone(),
            self.content,
            self.container.config(),
        )
        .ok()?;
        plan.fatal_tail.is_none().then_some(plan.commands)
    }

    /// Actual full availability context and Registry semantic generation.
    #[must_use]
    pub fn matches_context(&self, context: &ContextRegistry) -> bool {
        self.source.context() == Some(context.context())
            && self.source.matches_registry(context.commands())
    }

    /// This original script role preserves immediate caller source metadata.
    /// It supplies no physical frame, runtime entry or completed invocation.
    #[must_use]
    pub fn inherits_immediate_caller_source_context(&self, context: &ContextRegistry) -> bool {
        if !self.matches_context(context) {
            return false;
        }
        self.source.with_source_schema(context, |schema| {
            let roles = self.source.roles().unwrap_or_default();
            if roles
                .iter()
                .any(|(_, role)| *role == ArgRole::LambdaLiteral)
            {
                return false;
            }
            let mut bodies = roles
                .iter()
                .filter(|(_, role)| *role == ArgRole::Body)
                .peekable();
            schema.facts().body_kind == tcl_registry::BodyKind::Plain
                && schema.facts().body_interpreter == tcl_registry::BodyInterpreter::Current
                && bodies.peek().is_some()
                && bodies.all(|(argument, _)| {
                    schema.authored_source_script_timing_at(*argument)
                        == Some(tcl_registry::ScriptTiming::SameInvocation)
                })
        }) == Some(true)
    }

    /// Source definition vocabulary for this genuine body, independently of
    /// entered frames. A new authored definition body selects itself; ordinary
    /// bodies inherit only explicit immediate caller-context source metadata.
    #[must_use]
    pub fn definition_parent_for(
        &self,
        context: &ContextRegistry,
        previous: Option<&Self>,
    ) -> Option<Self> {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        if !self.matches_context(context) {
            return None;
        }
        let opens = self.source.with_source_schema(context, |schema| {
            schema.authored_source_definition_body_grammar().is_some()
        })?;
        let inherits = self.inherits_immediate_caller_source_context(context);
        if opens {
            return Some(self.clone());
        }
        let previous = previous.filter(|previous| {
            inherits
                && previous.matches_context(context)
                && previous.matches_source(self.container.image(), self.container.config())
                && previous.content.start() <= self.content.start()
                && self.content.end() <= previous.content.end()
        })?;
        Some(previous.clone())
    }
}

/// Original member script selected within an authentic definition body.
/// The genuine parent source schema and complete original member argv stay
/// retained. Member vocabulary is source syntax, never global command lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceDefinitionMemberScriptBody {
    parent: OriginalSourceScriptBody,
    original: Vec<NativeWord>,
    argument: usize,
    container: NativeWord,
    content: Span,
    retains_definition_grammar: bool,
    presentation: tcl_registry::definer::SourceDefinitionMemberBodyPresentation,
    member_row: Option<tcl_registry::definer::MemberRow>,
}
/// Two genuine source-body issuers joined for readonly member correspondence.
/// Both applicability receipts remain intact; this is no body/frame entry or
/// conversion from conditional source data to a selected Native invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalSourceDefinitionMemberBodyCorrespondence {
    catalogue: OriginalSourceDefinitionMemberScriptBody,
    declaration: OriginalSourceDefinitionMemberScriptBody,
    heads: super::source_structure::OriginalSourceHeadCorrespondence,
}

impl OriginalSourceDefinitionMemberBodyCorrespondence {
    pub(crate) const fn catalogue_body(&self) -> &OriginalSourceDefinitionMemberScriptBody {
        &self.catalogue
    }
    pub(crate) const fn declaration_body(&self) -> &OriginalSourceDefinitionMemberScriptBody {
        &self.declaration
    }
    #[cfg(test)]
    pub(crate) const fn original_heads(
        &self,
    ) -> &super::source_structure::OriginalSourceHeadCorrespondence {
        &self.heads
    }
}

impl OriginalSourceDefinitionMemberScriptBody {
    /// Actual enclosing declaration body, retained solely as source lineage.
    /// Callable bodies do not acquire the parent's definition vocabulary,
    /// receiver namespace, command lookup or entered context from this borrow.
    pub(crate) const fn original_parent_body(&self) -> &OriginalSourceScriptBody {
        &self.parent
    }

    /// Complete original member argv, including its genuine source keyword.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Exact original whole word owning this script content.
    #[must_use]
    pub const fn original_container(&self) -> &NativeWord {
        &self.container
    }
    /// Actual member effect/receiver and body operand under the retained grammar.
    /// This is source applicability, not member installation or frame entry.
    #[must_use]
    pub fn member_row(&self) -> Option<&tcl_registry::definer::MemberRow> {
        self.member_row.as_ref()
    }
    /// Receiver vocabulary for this actual selected callable body. The
    /// parent source schema owns the grammar; this supplies no entered receiver.
    #[must_use]
    pub fn receiver_grammar(
        &self,
        context: &ContextRegistry,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        use tcl_registry::definer::{CallableRole, MemberEffect, MemberReceiver};
        self.matches_context(context).then_some(())?;
        let row = self.member_row()?;
        let MemberEffect::Callable { role, .. } = row.effect else {
            return None;
        };
        if row.receiver != MemberReceiver::Instance || role == CallableRole::Procedure {
            return None;
        }
        self.parent
            .source
            .with_source_schema(context, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten()
    }
    /// Whole original member/body source and parser configuration agree.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.parent.matches_source(image, config)
            && self.container.image() == image
            && self.container.config() == config
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == config)
    }
    /// The member issuer retains its actual full context and Registry generation.
    #[must_use]
    pub fn matches_context(&self, context: &ContextRegistry) -> bool {
        self.parent.matches_context(context)
    }
    /// Join the actual original declaration's parent without equating its
    /// applicability receipt with a separate conditional catalogue issuer.
    /// Complete source/configuration, parent argv/origins/values and selected
    /// descriptor/definition grammar must agree; both sealed bodies survive.
    pub(crate) fn original_source_correspondence_in_parent(
        &self,
        context: &ContextRegistry,
        parent: &OriginalSourceScriptBody,
    ) -> Option<OriginalSourceDefinitionMemberBodyCorrespondence> {
        // naming.editor.original-method-candidates
        // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
        let original_parent = self.original_parent_body();
        let image = self.original_container().image();
        let config = self.original_container().config();
        if !self.matches_source(image, config)
            || !self.matches_context(context)
            || !parent.matches_source(image, config)
            || !parent.matches_context(context)
            || original_parent.original_container() != parent.original_container()
            || original_parent.extent_span() != parent.extent_span()
            || original_parent.container_extent_span() != parent.container_extent_span()
            || original_parent.content_span() != parent.content_span()
        {
            return None;
        }
        let left = original_parent.source_words();
        let right = parent.source_words();
        let heads = crate::registry_invocation::source_structure::OriginalRegistryWords::original_source_head_correspondence(left, right, context)?;
        if left.origins().get(1..) != right.origins().get(1..)
            || left.arguments().len() != right.arguments().len()
            || !left
                .arguments()
                .iter()
                .zip(right.arguments())
                .all(
                    |(left, right)| match (left.literal_bytes(), right.literal_bytes()) {
                        (Some(left), Some(right)) => left == right,
                        (None, None) => left == right,
                        _ => false,
                    },
                )
            || !(0..left.arguments().len()).all(|index| {
                left.original_argument_word(index).is_some()
                    && left.original_argument_word(index) == right.original_argument_word(index)
            })
        {
            return None;
        }
        let same_schema = left.with_source_schema(context, |left| {
            right.with_source_schema(context, |right| {
                std::ptr::eq(
                    left.authored_source_descriptors().command,
                    right.authored_source_descriptors().command,
                ) && left.semantics.argument_offset == right.semantics.argument_offset
                    && left
                        .authored_source_definition_body_grammar()
                        .zip(right.authored_source_definition_body_grammar())
                        .is_some_and(|(left, right)| std::ptr::eq(left, right))
            })
        })??;
        same_schema.then_some(())?;
        let region = parent.definition_member_region(context, parent.content_span())?;
        let original =
            region.original_command_words_at(self.original_words().first()?.span().start())?;
        (original == self.original_words()).then_some(())?;
        let mut bodies = region.script_bodies(original)?.into_iter().filter(|body| {
            body.original == self.original
                && body.argument == self.argument
                && body.container == self.container
                && body.content == self.content
                && body.retains_definition_grammar == self.retains_definition_grammar
                && body.presentation == self.presentation
                && body.member_row == self.member_row
        });
        let declaration = bodies.next()?;
        bodies
            .next()
            .is_none()
            .then_some(OriginalSourceDefinitionMemberBodyCorrespondence {
                catalogue: self.clone(),
                declaration,
                heads,
            })
    }

    /// Complete original member-body command geometry. Counted source must
    /// exactly retain the authored content; a partial or cooked body refuses.
    /// This supplies no entered member, effect closure or normal completion.
    pub(crate) fn complete_original_commands(&self) -> Option<Vec<Vec<NativeWord>>> {
        let value = tcl_syntax::word_rules::original_static_word_unicode_value(&self.container)?;
        (self
            .container
            .image()
            .bytes()
            .get(self.content.as_range())?
            == value.as_slice())
        .then_some(())?;
        let plan = tcl_lexer::native_script_words_in(
            self.container.image().clone(),
            self.content,
            self.container.config(),
        )
        .ok()?;
        plan.fatal_tail.is_none().then_some(())?;
        Some(
            plan.commands
                .into_iter()
                .map(|command| command.words)
                .collect(),
        )
    }

    /// Original post-keyword body ordinal, preserving wrapper/option offsets.
    #[must_use]
    pub const fn argument(&self) -> usize {
        self.argument
    }
    /// Presentation selected by the genuine member effect and body slot.
    /// This describes source layout, without entered member or runtime authority.
    #[must_use]
    pub const fn presentation(
        &self,
    ) -> tcl_registry::definer::SourceDefinitionMemberBodyPresentation {
        self.presentation
    }
    /// Exact original script bytes, excluding word delimiters.
    #[must_use]
    pub const fn content_span(&self) -> Span {
        self.content
    }
    /// Original source fold extent, excluding the authentic closing delimiter.
    #[must_use]
    pub fn extent_span(&self) -> Span {
        Span::new(self.container.group().span.start(), self.content.end())
    }
    /// Independently retained source vocabulary applies only to wrapper blocks.
    #[must_use]
    pub const fn definition_parent(&self) -> Option<&OriginalSourceScriptBody> {
        if self.retains_definition_grammar {
            Some(&self.parent)
        } else {
            None
        }
    }
}
/// One readonly member-reference operand from a genuine definition region.
/// Original value production, parent vocabulary and complete source membership
/// remain retained; this grants no member table, entered worker or edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceDefinitionMemberReference {
    original: NativeWord,
    kind: tcl_registry::definer::MemberRefKind,
    value: Vec<u8>,
    protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
}
impl OriginalSourceDefinitionMemberReference {
    /// Whole original operand, independently of its displayed name.
    #[must_use]
    pub const fn original_word(&self) -> &NativeWord {
        &self.original
    }
    /// Registry-selected referenced entity kind.
    #[must_use]
    pub const fn kind(&self) -> tcl_registry::definer::MemberRefKind {
        self.kind
    }
    /// Source-produced counted units, never a runtime member or allocation.
    #[must_use]
    pub fn value(&self) -> &[u8] {
        &self.value
    }
    /// Readonly source-name comparison under the independently selected recipe.
    /// A reporting label cannot supply native table or editable-name evidence.
    #[must_use]
    pub fn matches_reported_name(&self, name: &str) -> bool {
        self.protocol.map_or_else(
            || name.is_ascii() && self.value.as_slice() == name.as_bytes(),
            |protocol| {
                tcl_syntax::backslash::native_source_literal_bytes(
                    name.as_bytes(),
                    self.original.image().channel(),
                    protocol,
                )
                .is_ok_and(|value| value.as_ref() == self.value.as_slice())
            },
        )
    }
}

/// Sealed original member-command inventory under a genuine definition body.
/// One complete lexical plan owns argv membership for the current source region.
/// Selected vocabulary remains conditional source syntax, never native dispatch.
#[derive(Debug, Clone)]
pub struct OriginalSourceDefinitionMemberRegion {
    parent: OriginalSourceScriptBody,
    context: ResolvedContext,
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    plan: tcl_lexer::NativeScriptWordsPlan,
    region: Span,
}

impl OriginalSourceScriptBody {
    /// Capture a complete current script region inside this genuine body.
    /// The same original image/configuration and actual context are mandatory.
    /// Callers retain this issuer for every member in the region instead of
    /// re-parsing a class body or manufacturing a nominal grammar owner.
    #[must_use]
    pub fn definition_member_region(
        &self,
        context: &ContextRegistry,
        script_region: Span,
    ) -> Option<OriginalSourceDefinitionMemberRegion> {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        if !self.matches_context(context)
            || script_region.start() < self.content.start()
            || script_region.end() > self.content.end()
            || script_region.start() > script_region.end()
        {
            return None;
        }
        let grammar = self
            .source
            .with_source_schema(context, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten()?;
        let plan = tcl_lexer::native_script_words_in(
            self.container.image().clone(),
            script_region,
            self.container.config(),
        )
        .ok()?;
        Some(OriginalSourceDefinitionMemberRegion {
            parent: self.clone(),
            context: context.context().clone(),
            grammar,
            plan,
            region: script_region,
        })
    }
}
impl OriginalSourceDefinitionMemberRegion {
    /// Complete original vectors only when the retained lexical plan covers
    /// the entire genuine parent definition body without a fatal tail.
    /// An empty complete vector is source coverage, never lifecycle absence.
    #[must_use]
    pub fn complete_original_commands(&self) -> Option<Vec<Vec<NativeWord>>> {
        // The complete-plan caller must retain the whole parent extent; a
        // subregion cannot certify the rest of a class definition.
        let complete = self.region == self.parent.content_span();
        (complete && self.plan.fatal_tail.is_none()).then_some(())?;
        Some(
            self.plan
                .commands
                .iter()
                .map(|command| command.words.clone())
                .collect(),
        )
    }
    /// Complete original argv at its genuine command-head offset in this
    /// retained source region. No fresh segmentation or displayed head is used.
    #[must_use]
    pub fn original_command_words_at(&self, start: u32) -> Option<&[NativeWord]> {
        let index = self
            .plan
            .commands
            .binary_search_by_key(&start, |command| command.span.start())
            .ok()?;
        Some(&self.plan.commands.get(index)?.words)
    }

    /// Complete original command vectors from the retained lexical plan.
    /// Returning a source command grants no reached execution or worker entry.
    pub fn original_commands(&self) -> impl Iterator<Item = &[NativeWord]> {
        self.plan
            .commands
            .iter()
            .map(|command| command.words.as_slice())
    }

    /// Whole original variable-name operands under the selected member role.
    /// These words supply source display geometry, not accepted member names,
    /// variable cells or an entered definition frame.
    #[must_use]
    pub fn variable_argument_words(&self, original: &[NativeWord]) -> Option<Vec<&NativeWord>> {
        self.argument_words_for_role(original, ArgRole::VarWrite)
    }

    /// Whole original operands carrying one selected source member role.
    /// Names and list containers retain their own word geometry; this does
    /// not manufacture fields, member identity or runtime invocation authority.
    #[must_use]
    pub fn argument_words_for_role(
        &self,
        original: &[NativeWord],
        role: ArgRole,
    ) -> Option<Vec<&NativeWord>> {
        let values = self.member_values(original)?;
        let keyword = values.first()?.as_deref()?;
        let arguments = original
            .iter()
            .zip(&values)
            .skip(1)
            .map(|(word, value)| {
                if word.group().expand {
                    tcl_registry::InvocationWord::Expanded
                } else {
                    value.as_deref().map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::KnownBytes,
                    )
                }
            })
            .collect::<Vec<_>>();
        let indices = self.grammar.source_member_arg_indices_for_role_in(
            keyword,
            tcl_registry::InvocationArguments::structured(&arguments),
            Some(self.context.authoring_query()),
            role,
        )?;
        let retained = self.original_command_words_at(original.first()?.span().start())?;
        indices
            .into_iter()
            .map(|index| retained.get(index.checked_add(1)?))
            .collect()
    }

    /// Readonly reference operands under this genuine parent vocabulary.
    /// Complete command membership and actual availability remain mandatory;
    /// dynamic operands retain no static reference-name or writable geometry.
    #[must_use]
    pub fn references(
        &self,
        original: &[NativeWord],
    ) -> Option<Vec<OriginalSourceDefinitionMemberReference>> {
        self.project_references(original, false)
    }

    /// Complete selected reference-role projection. Missing dynamic names
    /// remain unavailable; known members without reference roles yield empty.
    #[must_use]
    pub fn checked_references(
        &self,
        original: &[NativeWord],
    ) -> Option<Vec<OriginalSourceDefinitionMemberReference>> {
        self.project_references(original, true)
    }

    fn project_references(
        &self,
        original: &[NativeWord],
        complete: bool,
    ) -> Option<Vec<OriginalSourceDefinitionMemberReference>> {
        // naming.core.original-member-reference-hazard
        // docs/design/analysis/name-resolution-proofs/core-original-member-reference-hazard.md
        let values = self.member_values(original)?;
        let keyword = values.first()?.as_deref()?;
        let arguments = original
            .iter()
            .zip(&values)
            .skip(1)
            .map(|(word, value)| {
                if word.group().expand {
                    tcl_registry::InvocationWord::Expanded
                } else {
                    value.as_deref().map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::KnownBytes,
                    )
                }
            })
            .collect::<Vec<_>>();
        let selected = self.grammar.checked_source_member_ref_indices_in(
            keyword,
            tcl_registry::InvocationArguments::structured(&arguments),
            Some(self.context.authoring_query()),
        )?;
        let Some((kind, indices)) = selected else {
            return complete.then(Vec::new);
        };
        let protocol = self
            .parent
            .source
            .dialect()
            .and_then(tcl_registry::InvocationDialect::authored_name_policy)
            .map(|policy| policy.string_protocol());
        let references = indices
            .into_iter()
            .map(|index| {
                let ordinal = index.checked_add(1)?;
                Some(OriginalSourceDefinitionMemberReference {
                    original: original.get(ordinal)?.clone(),
                    kind,
                    value: values.get(ordinal)?.as_ref()?.clone(),
                    protocol,
                })
            })
            .collect::<Vec<_>>();
        if complete {
            references.into_iter().collect()
        } else {
            Some(references.into_iter().flatten().collect())
        }
    }

    /// Source member scripts through genuine parent vocabulary and whole words.
    /// Every supplied word must match one complete command in the retained
    /// lexical plan. Dynamic selectors, expansions and cooked body geometry
    /// cannot be repaired through a nominal spelling or token prefix.
    #[must_use]
    pub fn script_bodies(
        &self,
        original: &[NativeWord],
    ) -> Option<Vec<OriginalSourceDefinitionMemberScriptBody>> {
        self.project_script_bodies(original, false)
    }

    /// Complete selected member-body geometry through the same source grammar.
    /// Missing or cooked bodies cannot certify full source coverage.
    #[must_use]
    pub fn checked_script_bodies(
        &self,
        original: &[NativeWord],
    ) -> Option<Vec<OriginalSourceDefinitionMemberScriptBody>> {
        self.project_script_bodies(original, true)
    }

    fn project_script_bodies(
        &self,
        original: &[NativeWord],
        complete: bool,
    ) -> Option<Vec<OriginalSourceDefinitionMemberScriptBody>> {
        let values = self.member_values(original)?;
        let keyword = values.first()?.as_deref()?;
        self.grammar.member(std::str::from_utf8(keyword).ok()?)?;
        let arguments: Vec<_> = original
            .iter()
            .zip(&values)
            .skip(1)
            .map(|(word, value)| {
                if word.group().expand {
                    tcl_registry::InvocationWord::Expanded
                } else {
                    value.as_deref().map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::KnownBytes,
                    )
                }
            })
            .collect();
        let selected = self.grammar.source_member_script_arguments_in(
            keyword,
            tcl_registry::InvocationArguments::structured(&arguments),
            Some(self.context.authoring_query()),
        )?;
        // Row selectors require an independent original Document literal,
        // not a text projection of the retained counted value alone.
        let source_literals = original
            .iter()
            .map(tcl_syntax::word_rules::original_static_word_ascii_presentation)
            .collect::<Vec<_>>();
        let member_words = original
            .iter()
            .zip(&values)
            .zip(&source_literals)
            .map(|((word, value), literal)| {
                if word.group().expand {
                    return tcl_registry::InvocationWord::Expanded;
                }
                let counted = value.as_deref().map_or(
                    tcl_registry::InvocationWord::Dynamic,
                    tcl_registry::InvocationWord::KnownBytes,
                );
                match (value.as_deref(), literal.as_deref()) {
                    (Some(value), Some(literal)) if value == literal && !literal.contains(&0) => {
                        super::source_structure::source_schema_word(counted)
                    }
                    _ => counted,
                }
            })
            .collect::<Vec<_>>();
        let member_row = self.grammar.member_row(
            0,
            tcl_registry::InvocationArguments::structured(&member_words),
            Some(self.context.authoring_query()),
        );
        let bodies = selected
            .into_iter()
            .map(|selected| {
                let word = original.get(selected.argument().checked_add(1)?)?;
                let geometry = original_word_script_body_geometry(word)?;
                Some(OriginalSourceDefinitionMemberScriptBody {
                    parent: self.parent.clone(),
                    original: original.to_vec(),
                    argument: selected.argument(),
                    container: geometry.container,
                    content: geometry.content,
                    retains_definition_grammar: selected.retains_definition_grammar(),
                    presentation: selected.presentation(),
                    member_row: member_row.clone().filter(|row| {
                        selected
                            .argument()
                            .checked_add(1)
                            .is_some_and(|ordinal| row.body.is_some_and(|body| body.0 == ordinal))
                    }),
                })
            })
            .collect::<Vec<_>>();
        if complete {
            bodies.into_iter().collect()
        } else {
            Some(bodies.into_iter().flatten().collect())
        }
    }

    /// Source keyword selected from complete original argv in this genuine
    /// parent region. This supplies vocabulary only, no global command identity.
    #[must_use]
    pub fn member_keyword(
        &self,
        original: &[NativeWord],
    ) -> Option<&'static tcl_registry::definer::MemberSpec> {
        let values = self.member_values(original)?;
        self.grammar.source_member_in(
            values.first()?.as_deref()?,
            Some(self.context.authoring_query()),
        )
    }

    fn member_values(&self, original: &[NativeWord]) -> Option<Vec<Option<Vec<u8>>>> {
        let first = original.first()?;
        if first.group().expand {
            return None;
        }
        (self.original_command_words_at(first.span().start())? == original).then_some(())?;
        if let super::source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            self.parent.source.source()
            && let Some(input) = advice.logical_source_input()
        {
            let context = input.context_registry();
            (input.has_logical_source_name_context()
                && self.context == *context.context()
                && self.parent.matches_context(&context)
                && self
                    .parent
                    .matches_source(first.image(), input.lexer_config())
                && advice.matches_source(first.image(), input.lexer_config())
                && advice.matches_context(&context)
                && advice.site().source.source_image() == first.image()
                && original.iter().all(|word| {
                    word.image() == first.image() && word.config() == input.lexer_config()
                }))
            .then_some(())?;
            return original
                .iter()
                .map(|word| {
                    tcl_syntax::word_rules::original_static_word_unicode_value(word).map(Some)
                })
                .collect();
        }
        let captured = self
            .parent
            .source
            .dialect()
            .and_then(tcl_registry::InvocationDialect::authored_name_policy)
            .map(|policy| {
                tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                    original,
                    policy.string_protocol(),
                )
            })
            .transpose()
            .ok()?;
        let values: Vec<_> = original
            .iter()
            .enumerate()
            .map(|(index, word)| {
                captured.as_ref().map_or_else(
                    || tcl_syntax::word_rules::original_static_word_ascii_presentation(word),
                    |captured| captured.literal(index).map(Vec::from),
                )
            })
            .collect();
        Some(values)
    }
}

/// Source syntax body selected solely by a genuine authored declaration.
/// The declaration, complete original argv, full input and whole original body
/// remain retained. A Body role without timing grants no potential evaluation,
/// builtin traits, Native lookup, entered frame, completion or edit permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeclaredSourceScriptBody {
    source: Arc<crate::command_binding::OriginalDeclaredCommandWords>,
    container: NativeWord,
    extent: Span,
    content: Span,
}
impl OriginalDeclaredSourceScriptBody {
    /// Genuine complete source body word, never a cooked or expansion child.
    #[must_use]
    pub const fn original_container(&self) -> &NativeWord {
        &self.container
    }
    /// Original fold extent, excluding its closing delimiter.
    #[must_use]
    pub const fn extent_span(&self) -> Span {
        self.extent
    }
    /// Exact original script content without genuine word delimiters.
    #[must_use]
    pub const fn content_span(&self) -> Span {
        self.content
    }
    /// Authentic complete declaration and its conditional source obligations.
    #[must_use]
    pub fn source_words(&self) -> &crate::command_binding::OriginalDeclaredCommandWords {
        &self.source
    }
    /// Exact immutable source image and complete parser configuration.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.source.site().source.source_image() == image
            && self.source.resolved_input().lexer_config() == config
            && self.container.image() == image
            && self.container.config() == config
    }
    /// Complete retained availability and immutable Registry generation.
    #[must_use]
    pub fn matches_context(&self, context: &ContextRegistry) -> bool {
        let input = self.source.resolved_input().context_registry();
        input.context() == context.context()
            && input.commands().snapshot().semantic_key()
                == context.commands().snapshot().semantic_key()
    }
}

pub(crate) fn declared_source_script_bodies_for(
    words: &crate::command_binding::OriginalDeclaredCommandWords,
    purpose: OriginalSourceScriptPurpose,
) -> Vec<OriginalDeclaredSourceScriptBody> {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    // Authored declarations specify source Body roles, but do not specify
    // script timing. Role syntax cannot establish potential evaluation.
    if purpose != OriginalSourceScriptPurpose::Syntax {
        return Vec::new();
    }
    let source = Arc::new(words.clone());
    let image = words.site().source.source_image();
    let config = words.resolved_input().lexer_config();
    words
        .supplied_argument_roles()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(argument, role)| {
            if role != ArgRole::Body {
                return None;
            }
            let word = words.argument_word(argument)?;
            if word.image() != image || word.config() != config {
                return None;
            }
            let geometry = original_word_script_body_geometry(word)?;
            Some(OriginalDeclaredSourceScriptBody {
                source: Arc::clone(&source),
                container: geometry.container,
                extent: geometry.extent,
                content: geometry.content,
            })
        })
        .collect()
}

/// Sealed readonly ancestry of a script region selected by an original hosted
/// role or case grammar. Parent lookup evidence never becomes a child lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalSourceScriptBodyOrigin {
    container: NativeWord,
    content: Span,
    parent: super::VendorRegistryInvocationShape,
    regions: Vec<Span>,
    barriers: super::VendorSourceCatalogueBarriers,
    obligations: Vec<crate::command_binding::VendorCatalogueSourceObligation>,
}
impl std::hash::Hash for OriginalSourceScriptBodyOrigin {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.container, state);
        std::hash::Hash::hash(&self.content, state);
        std::hash::Hash::hash(self.parent.registry(), state);
    }
}
impl OriginalSourceScriptBodyOrigin {
    pub(crate) fn matches_child(&self, context: &ContextRegistry, original: &[NativeWord]) -> bool {
        self.parent.context() == context.context()
            && self.parent.registry() == &context.commands().snapshot().semantic_key()
            && self.barriers.matches_context(context)
            && !original.is_empty()
            && original.iter().all(|word| {
                word.image() == self.container.image()
                    && word.config() == self.container.config()
                    && word.span().start() >= self.content.start()
                    && word.span().end() <= self.content.end()
            })
    }
    pub(crate) fn merge(&self, other: &Self) -> Option<Self> {
        if self.container != other.container
            || self.content != other.content
            || self.parent != other.parent
            || self.regions != other.regions
            || self.barriers != other.barriers
        {
            return None;
        }
        let mut merged = self.clone();
        for obligation in &other.obligations {
            if !merged.obligations.contains(obligation) {
                merged.obligations.push(obligation.clone());
            }
        }
        Some(merged)
    }
    pub(crate) fn source_regions(&self) -> &[Span] {
        &self.regions
    }
    pub(crate) fn obligations(&self) -> &[crate::command_binding::VendorCatalogueSourceObligation] {
        &self.obligations
    }
}
impl OriginalSourceScriptBody {
    pub(crate) fn vendor_origin(&self) -> Option<std::sync::Arc<OriginalSourceScriptBodyOrigin>> {
        let super::source_structure::OriginalRegistrySource::Vendor(metadata) =
            self.source.source()
        else {
            return None;
        };
        let mut regions = metadata
            .catalogue()
            .script_body_origin()
            .map(|origin| origin.source_regions().to_vec())
            .unwrap_or_default();
        regions.push(self.content);
        Some(std::sync::Arc::new(OriginalSourceScriptBodyOrigin {
            regions,
            container: self.container.clone(),
            content: self.content,
            parent: metadata.shape().clone(),
            barriers: metadata.authored_barriers().clone(),
            obligations: metadata.obligations().to_vec(),
        }))
    }
}

#[derive(Clone)]
struct ScriptBodyGeometry {
    container: NativeWord,
    extent: Span,
    container_extent: Span,
    content: Span,
}

fn original_word_script_body_geometry(word: &NativeWord) -> Option<ScriptBodyGeometry> {
    let unchanged = match word.group().kind {
        tcl_lexer::WordKind::Braced => true,
        tcl_lexer::WordKind::Bare | tcl_lexer::WordKind::Quoted => {
            word.executable_parts().all_parts().all(|part| {
                matches!(
                    part.part,
                    tcl_lexer::ExecutablePart::Text(tcl_lexer::ExecutableText::Original)
                )
            })
        }
    };
    if word.group().expand || !unchanged {
        return None;
    }
    let content = word.content_span().ok()?;
    let extent = Span::new(word.group().span.start(), content.end());
    Some(ScriptBodyGeometry {
        container: word.clone(),
        extent,
        container_extent: extent,
        content,
    })
}

/// Original script content geometry; selected body roles and entry remain separate.
pub(crate) fn original_written_script_body_span(word: &NativeWord) -> Option<Span> {
    Some(original_word_script_body_geometry(word)?.content)
}

/// Lexical zero-command shape of one genuine written script word.
/// Selected body roles, timing, effects and execution remain independent.
pub(crate) fn original_written_script_body_is_empty(word: &NativeWord) -> Option<bool> {
    let geometry = original_word_script_body_geometry(word)?;
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        word.image().clone(),
        geometry.content,
        word.config(),
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    Some(plan.commands.is_empty() && plan.fatal_tail.is_none())
}

fn source_operand_body(
    operand: &super::source_structure::OriginalOperandSource,
) -> Option<ScriptBodyGeometry> {
    if let Some(word) = operand.word() {
        return original_word_script_body_geometry(word);
    }
    // A frozen expansion child supplies only its independently retained static
    // source extent and parent. It cannot become a fabricated whole body word.
    let input = operand.input()?;
    let container = input.original_static_list_container()?;
    let parent = container.parent_word();
    if parent.image().bytes().get(operand.span().as_range()) != Some(input.bytes()) {
        return None;
    }
    Some(ScriptBodyGeometry {
        container: parent.clone(),
        extent: operand.span(),
        container_extent: operand.span(),
        content: operand.span(),
    })
}

fn source_lambda_body(
    words: &super::source_structure::OriginalRegistryWords,
    context: &ContextRegistry,
) -> Option<ScriptBodyGeometry> {
    let selected =
        words.with_source_schema(context, |schema| schema.authored_source_lambda_call())??;
    let word = words
        .operands()
        .get(selected.lambda_argument)?
        .as_ref()?
        .word()?;
    let parent = original_word_script_body_geometry(word)?;
    let value = tcl_syntax::word_rules::original_static_word_source_bytes(word)?;
    if word.image().bytes().get(parent.content.as_range())? != value.as_slice() {
        return None;
    }
    let fields = tcl_syntax::word_rules::original_static_word_list_elements(word)?;
    if !(2..=3).contains(&fields.len()) {
        return None;
    }
    let body = fields.get(1)?;
    let content = body.source_span()?;
    if content.start() < parent.content.start()
        || content.end() > parent.content.end()
        || word.image().bytes().get(content.as_range())? != body.value().as_bytes()
    {
        return None;
    }
    Some(ScriptBodyGeometry {
        container: word.clone(),
        extent: content,
        container_extent: parent.extent,
        content,
    })
}

impl super::source_structure::OriginalRegistryWords {
    /// Exact unchanged lambda-list body geometry under its selected source schema.
    /// This separate projection supplies no caller frame, namespace, parameter
    /// binding, body entry or generic script-inventory coverage.
    #[must_use]
    pub fn original_lambda_body_geometry(
        &self,
        context: &ContextRegistry,
    ) -> Option<OriginalSourceScriptBody> {
        let body = source_lambda_body(self, context)?;
        Some(OriginalSourceScriptBody {
            source: Arc::new(self.clone()),
            container: body.container,
            extent: body.extent,
            container_extent: body.container_extent,
            content: body.content,
        })
    }
}

fn source_case_invocation(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<(
    tcl_registry::CaseListSpec,
    tcl_registry::spec::CaseInvocation,
)> {
    schema.authored_source_case_invocation()
}

/// Complete original body geometry for a mutation-coverage consumer. The
/// ordinary source inventory may expose valid partial regions; it cannot
/// establish that every possible executable body was inspected.
pub(super) fn source_script_bodies_for_mutation_coverage(
    words: &super::source_structure::OriginalRegistryWords,
    context: &ContextRegistry,
) -> Option<Vec<OriginalSourceScriptBody>> {
    source_script_bodies_for_coverage(
        words,
        context,
        OriginalSourceScriptPurpose::PotentialEvaluation,
    )
}

/// Shared complete body geometry for independently selected syntax/evaluation
/// purposes. Unrepresented lambda containers and malformed bodies stay unknown.
pub(super) fn source_script_bodies_for_coverage(
    words: &super::source_structure::OriginalRegistryWords,
    context: &ContextRegistry,
    purpose: OriginalSourceScriptPurpose,
) -> Option<Vec<OriginalSourceScriptBody>> {
    if matches!(
        words.source(),
        super::source_structure::OriginalRegistrySource::DefinitionMember(_)
    ) && purpose != OriginalSourceScriptPurpose::Syntax
    {
        return None;
    }
    let roles = words.roles()?;
    let timing_at = |argument| {
        words
            .with_source_schema(context, |schema| {
                schema.authored_source_script_timing_at(argument)
            })
            .flatten()
    };
    // This Body inventory does not represent lambda-container bodies.
    // Missing representation cannot certify complete evaluated coverage.
    for &(argument, role) in roles {
        if role == ArgRole::LambdaLiteral
            && (purpose == OriginalSourceScriptPurpose::Syntax
                || timing_at(argument)? != tcl_registry::ScriptTiming::ReferenceOnly)
        {
            return None;
        }
    }
    let case = words.with_source_schema(context, source_case_invocation)?;
    let list_argument = case.and_then(|(_, layout)| layout.clause_list_index);
    let mut expected = Vec::new();
    if let Some((case, layout)) = case
        && let Some(argument) = layout.clause_list_index
        && (purpose == OriginalSourceScriptPurpose::Syntax
            || timing_at(argument)? != tcl_registry::ScriptTiming::ReferenceOnly)
    {
        let operand = words.operands().get(argument)?.as_ref()?;
        let body = source_operand_body(operand)?;
        let list = body
            .container
            .image()
            .try_text()
            .ok()?
            .get(body.content.as_range())?;
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: case.clause_flags,
            clause_value_flags: case.clause_value_flags,
        };
        for clause in tcl_syntax::case_list::split_case_list_with_syntax(
            list,
            &shape,
            body.container.config().list_parse,
        ) {
            if !clause.valid || clause.pattern.is_none() {
                return None;
            }
            let element = clause.body?;
            if !element.braced {
                return None;
            }
            let range = element.content_range();
            let start = body
                .content
                .start()
                .checked_add(u32::try_from(range.start).ok()?)?;
            let end = body
                .content
                .start()
                .checked_add(u32::try_from(range.end).ok()?)?;
            expected.push(Span::new(start, end));
        }
    }
    for &(argument, role) in roles {
        if role == ArgRole::Body
            && Some(argument) != list_argument
            && (purpose == OriginalSourceScriptPurpose::Syntax
                || timing_at(argument)? != tcl_registry::ScriptTiming::ReferenceOnly)
        {
            expected.push(source_operand_body(words.operands().get(argument)?.as_ref()?)?.content);
        }
    }
    expected.sort_by_key(|span| (span.start(), span.end()));
    expected.dedup();
    let bodies = source_script_bodies_for(words, context, purpose);
    (bodies
        .iter()
        .map(OriginalSourceScriptBody::content_span)
        .collect::<Vec<_>>()
        == expected)
        .then_some(bodies)
}

/// Possible case-body source geometry for a syntax consumer that must visit
/// every candidate arm. Unlike the strong inventory below, this carries the
/// Registry's explicit unknown-selection residual and never chooses a branch.
pub(crate) fn conditional_source_case_bodies_for(
    words: &super::source_structure::OriginalRegistryWords,
    context: &ContextRegistry,
) -> Option<OriginalConditionalSourceCaseBodies> {
    if words.context() != Some(context.context()) || !words.matches_registry(context.commands()) {
        return None;
    }
    let config = words
        .operands()
        .iter()
        .flatten()
        .find_map(|operand| operand.word().map(NativeWord::config))?;
    let value_rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
    let (case, projection) = words
        .with_source_schema(context, |schema| {
            let case = *schema.semantics.options.case_list?;
            let projection = schema.authored_source_case_clause_projection(value_rules)?;
            Some((case, projection))
        })
        .flatten()?;
    let source = std::sync::Arc::new(words.clone());
    let mut bodies = Vec::new();
    for candidate in &projection.bodies {
        let operand = words.operands().get(candidate.argument)?.as_ref()?;
        let geometry = source_operand_body(operand)?;
        let mut retain = |geometry: ScriptBodyGeometry| {
            if words.matches_source(geometry.container.image(), geometry.container.config())
                && geometry.content.start() >= geometry.container.span().start()
                && geometry.content.end() <= geometry.container.span().end()
            {
                bodies.push(OriginalSourceScriptBody {
                    source: std::sync::Arc::clone(&source),
                    container: geometry.container,
                    extent: geometry.extent,
                    container_extent: geometry.container_extent,
                    content: geometry.content,
                });
            }
        };
        if let Some(list_element) = candidate.list_element {
            let text = geometry.container.image().try_text().ok()?;
            let list = text.get(geometry.content.as_range())?;
            let shape = tcl_syntax::case_list::CaseListShape {
                clause_flags: case.clause_flags,
                clause_value_flags: case.clause_value_flags,
            };
            let clause = tcl_syntax::case_list::split_case_list_with_syntax(
                list,
                &shape,
                geometry.container.config().list_parse,
            )
            .into_iter()
            .find(|clause| clause.valid && clause.body_index == Some(list_element))?;
            let element = clause.body.filter(|body| body.braced)?;
            let range = element.content_range();
            let start = geometry
                .content
                .start()
                .checked_add(u32::try_from(range.start).ok()?)?;
            let end = geometry
                .content
                .start()
                .checked_add(u32::try_from(range.end).ok()?)?;
            let span = Span::new(start, end);
            retain(ScriptBodyGeometry {
                container: geometry.container,
                extent: span,
                container_extent: geometry.extent,
                content: span,
            });
        } else {
            retain(geometry);
        }
    }
    bodies.sort_unstable_by_key(|body| (body.content.start(), body.content.end()));
    bodies.dedup_by_key(|body| body.content);
    Some(OriginalConditionalSourceCaseBodies {
        source,
        projection,
        bodies,
    })
}

/// Project original script geometry through the shared authored role/case
/// grammar. Unknown source applicability stays attached to every region;
/// malformed lists and cooked values cannot donate original child positions.
pub(super) fn source_script_bodies_for(
    words: &super::source_structure::OriginalRegistryWords,
    context: &ContextRegistry,
    purpose: OriginalSourceScriptPurpose,
) -> Vec<OriginalSourceScriptBody> {
    if matches!(
        words.source(),
        super::source_structure::OriginalRegistrySource::DefinitionMember(_)
    ) && purpose != OriginalSourceScriptPurpose::Syntax
    {
        return Vec::new();
    }
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    if words.context() != Some(context.context()) || !words.matches_registry(context.commands()) {
        return Vec::new();
    }
    let admits = |argument| {
        purpose == OriginalSourceScriptPurpose::Syntax
            || matches!(
                words.with_source_schema(context, |schema| schema.authored_source_script_timing_at(argument)),
                Some(Some(timing)) if timing != tcl_registry::ScriptTiming::ReferenceOnly
            )
    };
    let case = words
        .with_source_schema(context, source_case_invocation)
        .flatten();
    let list_argument = case.and_then(|(_, layout)| layout.clause_list_index);
    let source = std::sync::Arc::new(words.clone());
    let mut out = Vec::new();
    let mut retain = |geometry: ScriptBodyGeometry| {
        if !words.matches_source(geometry.container.image(), geometry.container.config())
            || geometry.content.start() < geometry.container.span().start()
            || geometry.content.end() > geometry.container.span().end()
        {
            return;
        }
        out.push(OriginalSourceScriptBody {
            source: std::sync::Arc::clone(&source),
            container: geometry.container,
            extent: geometry.extent,
            container_extent: geometry.container_extent,
            content: geometry.content,
        });
    };
    if case.is_some_and(|(_, layout)| layout.clause_list_index.is_none_or(admits)) {
        retain_source_case_bodies(words, case.as_ref(), &mut retain);
    }
    let concatenates = words
        .with_source_schema(context, |schema| {
            schema
                .facts()
                .traits
                .contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
        })
        .unwrap_or(false);
    let body_count = words
        .roles()
        .unwrap_or_default()
        .iter()
        .filter(|(_, role)| *role == ArgRole::Body)
        .count();
    for &(argument, role) in words.roles().unwrap_or_default() {
        // A concatenating schema marks the first fragment as Body; the
        // remaining actual argv still belongs to that one constructed script.
        // Only one complete original word can supply a contiguous body here.
        if concatenates
            && (body_count != 1
                || argument.checked_add(1) != Some(words.arguments().len())
                || words
                    .operands()
                    .get(argument)
                    .and_then(Option::as_ref)
                    .and_then(super::source_structure::OriginalOperandSource::word)
                    .is_none())
        {
            continue;
        }
        if role != ArgRole::Body || list_argument == Some(argument) || !admits(argument) {
            continue;
        }
        if let Some(body) = words
            .operands()
            .get(argument)
            .and_then(Option::as_ref)
            .and_then(source_operand_body)
        {
            retain(body);
        }
    }
    out.sort_by_key(|body| (body.content.start(), body.content.end()));
    out.dedup_by_key(|body| body.content);
    out
}

/// Original command regions inside one whole source expression operand.
/// The actual complete input selects expression grammar; no reconstructed
/// argv fragments or cooked expression can donate child source offsets.
pub(super) fn source_expression_script_bodies(
    words: &super::source_structure::OriginalRegistryWords,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<Vec<OriginalSourceScriptBody>> {
    let context = input.context_registry();
    if words.context() != Some(context.context()) || !words.matches_registry(context.commands()) {
        return None;
    }
    let whole_source = words.with_source_schema(&context, |schema| {
        !schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
            || schema
                .words
                .arguments()
                .exact_argv_len()
                .zip(schema.semantics.argument_offset.checked_add(1))
                .is_some_and(|(count, expected)| count == expected)
    })?;
    if !whole_source {
        return None;
    }
    let roles = words.roles()?;
    let mut expressions = roles.iter().filter(|(_, role)| *role == ArgRole::Expr);
    let Some(&(argument, _)) = expressions.next() else {
        return Some(Vec::new());
    };
    if expressions.next().is_some() {
        return None;
    }
    source_expression_script_bodies_for_argument(words, input, argument)
}

/// Expression substitution regions at the genuine content base retained by
/// the expression AST. Multiple Expr operands remain separate source owners.
pub(crate) fn source_expression_script_bodies_at(
    words: &super::source_structure::OriginalRegistryWords,
    input: &crate::analyser::ResolvedAnalysisInput,
    expression_base: u32,
) -> Option<Vec<OriginalSourceScriptBody>> {
    let context = input.context_registry();
    if words.context() != Some(context.context()) || !words.matches_registry(context.commands()) {
        return None;
    }
    let arguments = words
        .roles()?
        .iter()
        .filter_map(|&(argument, role)| {
            if role != ArgRole::Expr {
                return None;
            }
            let geometry = source_operand_body(words.operands().get(argument)?.as_ref()?)?;
            (geometry.content.start() == expression_base).then_some(argument)
        })
        .collect::<Vec<_>>();
    let [argument] = arguments.as_slice() else {
        return None;
    };
    source_expression_script_bodies_for_argument(words, input, *argument)
}

fn source_expression_script_bodies_for_argument(
    words: &super::source_structure::OriginalRegistryWords,
    input: &crate::analyser::ResolvedAnalysisInput,
    argument: usize,
) -> Option<Vec<OriginalSourceScriptBody>> {
    let geometry = source_operand_body(words.operands().get(argument)?.as_ref()?)?;
    let config = input.lexer_config();
    if !words.matches_source(geometry.container.image(), config)
        || geometry.container.config() != config
    {
        return None;
    }
    let expression = geometry
        .container
        .image()
        .bytes()
        .get(geometry.content.as_range())?;
    let profile = input.analyser_profile();
    let grammar = config.grammar_over(profile.grammar);
    let (tokens, unknown) = tcl_lexer::tokenise_expr_bytes_checked_with_expression_grammar(
        expression,
        &grammar,
        profile.expr_grammar_base,
        profile.f5_core_expr_grammar(),
    );
    if unknown {
        return None;
    }
    let terms = tcl_lexer::expression_terms(expression, &tokens, config)?;
    let source = Arc::new(words.clone());
    terms
        .into_iter()
        .filter(|term| term.kind == tcl_lexer::ExprTermKind::Command)
        .map(|term| {
            let start = geometry.content.start().checked_add(term.value.start)?;
            let end = geometry.content.start().checked_add(term.value.end)?;
            let command_span = Span::new(start, end);
            (start <= end && end <= geometry.content.end()).then_some(OriginalSourceScriptBody {
                source: Arc::clone(&source),
                container: geometry.container.clone(),
                extent: command_span,
                container_extent: geometry.container_extent,
                content: command_span,
            })
        })
        .collect()
}

fn retain_source_case_bodies(
    words: &super::source_structure::OriginalRegistryWords,
    case: Option<&(
        tcl_registry::CaseListSpec,
        tcl_registry::spec::CaseInvocation,
    )>,
    retain: &mut impl FnMut(ScriptBodyGeometry),
) {
    if let Some((case, layout)) = case
        && let Some(argument) = layout.clause_list_index
        && let Some(operand) = words.operands().get(argument).and_then(Option::as_ref)
        && let Some(body) = source_operand_body(operand)
        && let Ok(text) = body.container.image().try_text()
        && let Some(list) = text.get(body.content.as_range())
    {
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: case.clause_flags,
            clause_value_flags: case.clause_value_flags,
        };
        for clause in tcl_syntax::case_list::split_case_list_with_syntax(
            list,
            &shape,
            body.container.config().list_parse,
        ) {
            let Some(element) = clause.body.filter(|element| clause.valid && element.braced) else {
                continue;
            };
            let range = element.content_range();
            let Some(start) = u32::try_from(range.start)
                .ok()
                .and_then(|offset| body.content.start().checked_add(offset))
            else {
                continue;
            };
            let Some(end) = u32::try_from(range.end)
                .ok()
                .and_then(|offset| body.content.start().checked_add(offset))
            else {
                continue;
            };
            let body_span = Span::new(start, end);
            retain(ScriptBodyGeometry {
                container: body.container.clone(),
                extent: body_span,
                container_extent: body.extent,
                content: body_span,
            });
        }
    }
    if let Some((case, layout)) = case
        && let Some(start) = layout.inline_clause_start
        && let Some(values) = words.arguments().get(start..).and_then(|arguments| {
            arguments
                .iter()
                .map(|argument| {
                    let bytes = argument.literal_bytes()?;
                    (!bytes.contains(&0)).then_some(())?;
                    std::str::from_utf8(bytes).ok()
                })
                .collect::<Option<Vec<_>>>()
        })
        && let Some(clauses) = case.inline_clauses(&values, 0)
    {
        for clause in clauses {
            let Some(argument) = clause.body_index.and_then(|index| start.checked_add(index))
            else {
                continue;
            };
            if let Some(body) = words
                .operands()
                .get(argument)
                .and_then(Option::as_ref)
                .and_then(source_operand_body)
            {
                retain(body);
            }
        }
    }
}

#[cfg(test)]
mod script_purpose_tests {
    use super::*;
    use std::sync::Arc;
    use tcl_registry::{
        ArgRole, Arity, CommandRegistry, CommandSpec, InvocationArguments, ScriptTiming,
    };

    #[test]
    fn original_bare_script_geometry_keeps_actual_text_and_value_boundaries() {
        for (source, expected) in [
            ("if 1 break", Some("break")),
            ("if 1 $", Some("$")),
            ("if 1 \"break\"", Some("break")),
            ("if 1 $body", None),
            ("if 1 prefix[body]", None),
            (r"if 1 escaped\ body", None),
            (r#"if 1 "br\u0065ak""#, None),
            ("if 1 {*}{break}", None),
        ] {
            let config = LexerConfig {
                expand_syntax: true,
                ..LexerConfig::default()
            };
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let word = &plan.commands[0].words[2];
            let geometry = original_word_script_body_geometry(word);
            assert_eq!(
                geometry
                    .as_ref()
                    .map(|body| source.get(body.content.as_range()).unwrap()),
                expected,
                "{source}"
            );
            if let Some(geometry) = geometry {
                assert_eq!(&geometry.container, word);
                assert_eq!(geometry.container.config(), config);
                assert!(
                    word.span().start() <= geometry.content.start()
                        && geometry.content.end() <= word.span().end()
                );
            }
        }
    }

    #[test]
    fn complete_body_coverage_refuses_unrepresented_lambda_containers() {
        for source in ["apply {{} {puts hidden}}", "apply $lambda"] {
            let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
            let context = tcl_registry::model::context_for_profile(profile);
            let config = LexerConfig::for_file_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(&context),
                config,
            );
            let syntax = super::super::source_structure::OriginalSourceRegistryContext::capture(
                source, input,
            );
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let words = syntax.words(&command).unwrap();
            assert!(
                words
                    .roles()
                    .unwrap()
                    .iter()
                    .any(|(_, role)| *role == ArgRole::LambdaLiteral)
            );
            assert!(source_script_bodies_for_mutation_coverage(&words, &context).is_none());
            assert!(words.source_script_bodies(&context).is_empty());
        }
    }

    #[test]
    fn original_lambda_bodies_retain_actual_list_container_and_unchanged_source() {
        for (source, expected) in [
            ("apply {{} {puts visible}}", Some("puts visible")),
            ("apply {{} break}", Some("break")),
            ("apply {{x} {return $x} ::N} value", Some("return $x")),
            (r#"apply {{} "puts visible"}"#, Some("puts visible")),
            (r"apply {{} puts\ visible}", None),
            (r"apply {{} {puts visible}tail}", None),
            ("apply {{} {puts hidden} ::N extra}", None),
            ("apply {{}}", None),
            ("apply $lambda", None),
            ("apply {*}{ {} {puts hidden} }", None),
        ] {
            let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
            let context = tcl_registry::model::context_for_profile(profile);
            let config = LexerConfig::for_file_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(&context),
                config,
            );
            let syntax = super::super::source_structure::OriginalSourceRegistryContext::capture(
                source, input,
            );
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let Some(words) = syntax.words(&command) else {
                assert!(expected.is_none(), "missing original schema: {source}");
                continue;
            };
            let body = words.original_lambda_body_geometry(&context);
            assert!(source_script_bodies_for_mutation_coverage(&words, &context).is_none());
            assert!(words.source_script_bodies(&context).is_empty());
            match expected {
                Some(expected) => {
                    let body = body.expect("explicit unchanged original lambda geometry");
                    assert_eq!(source.get(body.content_span().as_range()), Some(expected));
                    assert_eq!(
                        body.original_container(),
                        words.original_argument_word(0).unwrap()
                    );
                    assert!(body.matches_source(&SourceImage::document(source), config));
                    assert_eq!(body.original_container().config(), config);
                    let foreign = tcl_registry::model::context_for_profile(
                        tcl_dialect::DialectProfile::find("tcl8.4").unwrap(),
                    );
                    assert!(words.original_lambda_body_geometry(&foreign).is_none());
                }
                None => assert!(body.is_none(), "{source}"),
            }
        }
    }

    fn reference_only(_arguments: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
        vec![(0, ScriptTiming::ReferenceOnly)]
    }

    fn original_definition_parent(
        source: &str,
        dialect: &str,
    ) -> (crate::analyser::AnalysisResult, OriginalSourceScriptBody) {
        let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let words =
            super::super::source_structure::source_registry_words(source, &analysis, &command)
                .unwrap();
        let parent = words.source_script_bodies(&context).remove(0);
        (analysis, parent)
    }

    fn definition_child_at(
        source: &str,
        analysis: &crate::analyser::AnalysisResult,
        parent: &OriginalSourceScriptBody,
        head: &str,
    ) -> OriginalSourceScriptBody {
        let body_region = parent.content_span();
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source.get(body_region.as_range()).unwrap(),
            body_region.start(),
            analysis.body_lexer_config.unwrap(),
        )
        .into_iter()
        .find(|command| command.name() == head)
        .unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        super::super::source_structure::source_registry_words(source, analysis, &command)
            .unwrap()
            .source_script_bodies(&context)
            .remove(0)
    }

    #[test]
    fn original_member_roles_lend_whole_names_formals_and_wrapper_selectors() {
        let source =
            "oo::class create C {self self method café {arg {optional 2}} {}; export café}";
        for dialect in ["tcl", "tcl8.6"] {
            let (analysis, parent) = original_definition_parent(source, dialect);
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let region = parent
                .definition_member_region(&context, parent.content_span())
                .unwrap();
            let original = region.original_commands().next().unwrap();
            for (role, indices) in [
                (ArgRole::Name, vec![3]),
                (ArgRole::ParamList, vec![4]),
                (ArgRole::Keyword, vec![1, 2]),
            ] {
                assert_eq!(
                    region.argument_words_for_role(original, role).unwrap(),
                    indices
                        .into_iter()
                        .map(|index| &original[index])
                        .collect::<Vec<_>>()
                );
                assert!(
                    region
                        .argument_words_for_role(&original[..2], role)
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn original_member_variable_words_retain_roles_and_whole_source_geometry() {
        let source = r"oo::class create C {variable a\ b arr($index); self variable wrapped\ name; method run {} {}}";
        for dialect in ["tcl", "tcl8.6"] {
            let (analysis, parent) = original_definition_parent(source, dialect);
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let region = parent
                .definition_member_region(&context, parent.content_span())
                .unwrap();
            let commands = region.original_commands().collect::<Vec<_>>();
            let variables = region.variable_argument_words(commands[0]).unwrap();
            assert_eq!(variables, vec![&commands[0][1], &commands[0][2]]);
            assert_eq!(
                region.variable_argument_words(commands[1]).unwrap(),
                vec![&commands[1][2]]
            );
            assert!(
                region
                    .variable_argument_words(commands[2])
                    .unwrap()
                    .is_empty()
            );
            assert!(region.variable_argument_words(&commands[0][..2]).is_none());
            let other = source.replace("a\\ b", "different");
            let (_, other_parent) = original_definition_parent(&other, dialect);
            let other_region = other_parent
                .definition_member_region(&context, other_parent.content_span())
                .unwrap();
            assert!(
                region
                    .variable_argument_words(other_region.original_commands().next().unwrap())
                    .is_none()
            );
        }
    }

    #[test]
    fn original_member_references_retain_whole_words_parent_vocabulary_and_scope() {
        // naming.core.original-member-reference-hazard
        // docs/design/analysis/name-resolution-proofs/core-original-member-reference-hazard.md
        let source = "oo::class create C {export {m space} m\\0n; self export m; export $unknown}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let (analysis, parent) = original_definition_parent(source, dialect);
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let region = parent
                .definition_member_region(&context, parent.content_span())
                .unwrap();
            let commands = region.original_commands().collect::<Vec<_>>();
            let references = region.references(commands[0]).unwrap();
            assert_eq!(references.len(), 2, "{dialect}");
            assert!(references[0].matches_reported_name("m space"));
            assert!(references[1].matches_reported_name("m\0n"));
            assert_eq!(references[1].value(), b"m\xc0\x80n");
            assert_eq!(references[0].original_word(), &commands[0][1]);
            assert_eq!(
                references[0].kind(),
                tcl_registry::definer::MemberRefKind::Method
            );
            let wrapped = region.references(commands[1]).unwrap();
            assert_eq!(wrapped.len(), 1);
            assert_eq!(wrapped[0].original_word(), &commands[1][2]);
            assert!(region.references(commands[2]).unwrap().is_empty());
            assert!(region.references(&commands[0][..2]).is_none());
            let other = source.replace("m space", "n space");
            let (_, foreign) = original_definition_parent(&other, dialect);
            let foreign_region = foreign
                .definition_member_region(&context, foreign.content_span())
                .unwrap();
            let foreign_command = foreign_region.original_commands().next().unwrap();
            assert!(region.references(foreign_command).is_none());
            let retired = tcl_registry::model::ingress::resolve_environment("tcl8.4")
                .default_context_registry();
            assert!(
                parent
                    .definition_member_region(&retired, parent.content_span())
                    .is_none()
            );
        }
    }
    #[test]
    fn mixed_lambda_roles_cannot_donate_immediate_caller_source_context() {
        for (roles, inherits) in [
            (&[(0, ArgRole::Body)][..], true),
            (
                &[(0, ArgRole::Body), (1, ArgRole::LambdaLiteral)][..],
                false,
            ),
        ] {
            let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
            let mut registry = CommandRegistry::build_default();
            registry.insert(CommandSpec {
                name: "mixed-script",
                arity: Arity::exact(2),
                arg_roles: roles,
                ..CommandSpec::DEFAULT
            });
            let context = Arc::new(
                tcl_registry::model::context_for_profile(profile)
                    .with_command_store(Arc::new(registry)),
            );
            let config = LexerConfig::for_file_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(&context),
                config,
            );
            let source = "mixed-script {puts current} {{} {puts lambda}}";
            let syntax = super::super::source_structure::OriginalSourceRegistryContext::capture(
                source, input,
            );
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let words = syntax.words(&command).unwrap();
            let bodies = words.source_script_bodies(&context);
            assert_eq!(bodies.len(), 1);
            assert_eq!(
                bodies[0].inherits_immediate_caller_source_context(&context),
                inherits
            );
        }
    }

    #[test]
    fn original_definition_parent_uses_actual_plain_current_immediate_body_metadata() {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        fn immediate(_args: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
            vec![(0, ScriptTiming::SameInvocation)]
        }
        fn deferred(_args: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
            vec![(0, ScriptTiming::Deferred)]
        }
        let source = "oo::class create C {source-body {method café {} {puts live}}}";
        for (kind, interpreter, timing, inherits) in [
            (
                tcl_registry::BodyKind::Plain,
                tcl_registry::BodyInterpreter::Current,
                immediate as tcl_registry::ScriptTimingResolver,
                true,
            ),
            (
                tcl_registry::BodyKind::Structural,
                tcl_registry::BodyInterpreter::Current,
                immediate as tcl_registry::ScriptTimingResolver,
                false,
            ),
            (
                tcl_registry::BodyKind::Plain,
                tcl_registry::BodyInterpreter::Argument(0),
                immediate as tcl_registry::ScriptTimingResolver,
                false,
            ),
            (
                tcl_registry::BodyKind::Plain,
                tcl_registry::BodyInterpreter::Current,
                deferred as tcl_registry::ScriptTimingResolver,
                false,
            ),
            (
                tcl_registry::BodyKind::Plain,
                tcl_registry::BodyInterpreter::Current,
                reference_only as tcl_registry::ScriptTimingResolver,
                false,
            ),
        ] {
            let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
            let mut registry = CommandRegistry::build_default();
            registry.insert(CommandSpec {
                name: "source-body",
                arity: Arity::exact(1),
                arg_roles: &[(0, ArgRole::Body)],
                body_kind: kind,
                body_interpreter: interpreter,
                script_timing_resolver: Some(timing),
                ..CommandSpec::DEFAULT
            });
            let context = Arc::new(
                tcl_registry::model::context_for_profile(profile)
                    .with_command_store(Arc::new(registry)),
            );
            let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(&context),
                config,
            );
            let analysis = crate::analyser::Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let parent =
                super::super::source_structure::source_registry_words(source, &analysis, &command)
                    .unwrap()
                    .source_script_bodies(&context)
                    .remove(0);
            assert_eq!(
                parent.definition_parent_for(&context, None),
                Some(parent.clone())
            );
            let child = definition_child_at(source, &analysis, &parent, "source-body");
            assert_eq!(
                child
                    .definition_parent_for(&context, Some(&parent))
                    .is_some(),
                inherits,
                "{kind:?} {interpreter:?}"
            );
            assert!(child.definition_parent_for(&context, None).is_none());
        }
    }

    #[test]
    fn original_definition_parent_declines_structural_foreign_and_uncontained_source() {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        let source =
            "oo::class create C {if 1 {method café {} {puts live}}; proc p {} {method ordinary}}";
        let (analysis, parent) = original_definition_parent(source, "tcl8.6");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let immediate = definition_child_at(source, &analysis, &parent, "if");
        assert_eq!(
            immediate.definition_parent_for(&context, Some(&parent)),
            Some(parent.clone())
        );
        let procedure = definition_child_at(source, &analysis, &parent, "proc");
        assert!(
            procedure
                .definition_parent_for(&context, Some(&parent))
                .is_none()
        );
        let foreign = tcl_registry::model::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
        );
        assert!(
            immediate
                .definition_parent_for(&foreign, Some(&parent))
                .is_none()
        );
        let (_, unrelated) =
            original_definition_parent("oo::class create D {method d {} {puts other}}", "tcl8.6");
        assert!(
            immediate
                .definition_parent_for(&context, Some(&unrelated))
                .is_none()
        );
        let source = "oo::class create C {}; if 1 {method outside}";
        let (analysis, parent) = original_definition_parent(source, "tcl8.6");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        )
        .remove(1);
        let outside =
            super::super::source_structure::source_registry_words(source, &analysis, &command)
                .unwrap()
                .source_script_bodies(&context)
                .remove(0);
        assert!(
            outside
                .definition_parent_for(&context, Some(&parent))
                .is_none()
        );
    }

    #[test]
    fn receiver_grammar_belongs_to_the_actual_instance_callable_body() {
        let source = "oo::class create C {method run {} {}; constructor {} {}; destructor {}; self method build {} {}}";
        for dialect in ["tcl", "tcl8.6"] {
            let (analysis, parent) = original_definition_parent(source, dialect);
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let region = parent
                .definition_member_region(&context, parent.content_span())
                .unwrap();
            let commands = region.original_commands().collect::<Vec<_>>();
            assert_eq!(commands.len(), 4);
            let foreign = tcl_registry::model::ingress::resolve_environment("f5-irules")
                .default_context_registry();
            for (index, command) in commands.into_iter().enumerate() {
                let body = region.script_bodies(command).unwrap().remove(0);
                assert_eq!(
                    body.receiver_grammar(&context)
                        .map(|grammar| grammar.family),
                    (index < 3).then_some(tcl_registry::definer::DefinerFamily::TclOo),
                    "{dialect}: {index}"
                );
                assert!(body.receiver_grammar(&foreign).is_none());
            }
        }
    }

    #[test]
    fn original_callable_member_script_keeps_shifted_row_and_original_context() {
        use tcl_registry::definer::{CallableRole, MemberEffect, MemberReceiver};
        let source = "oo::class create C {method wire {value} {set x $value}; self method build {} {set x 1}}";
        let (analysis, parent) = original_definition_parent(source, "tcl8.6");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let region = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let commands = region.original_commands().collect::<Vec<_>>();
        let instance = region.script_bodies(commands[0]).unwrap().remove(0);
        let class = region.script_bodies(commands[1]).unwrap().remove(0);
        assert_eq!(
            instance.member_row().unwrap().receiver,
            MemberReceiver::Instance
        );
        assert_eq!(
            class.member_row().unwrap().receiver,
            MemberReceiver::TypeObject
        );
        for body in [instance, class] {
            assert!(matches!(
                body.member_row().unwrap().effect,
                MemberEffect::Callable {
                    role: CallableRole::Method,
                    ..
                }
            ));
            assert!(body.definition_parent().is_none());
            assert!(body.matches_context(&context));
            assert!(body.matches_source(
                &tcl_lexer::SourceImage::document(source),
                analysis.body_lexer_config.unwrap()
            ));
            assert!(!body.matches_source(
                &tcl_lexer::SourceImage::native(source.as_bytes()),
                analysis.body_lexer_config.unwrap()
            ));
            let different = tcl_registry::model::ingress::resolve_environment("tcl9.0")
                .default_context_registry();
            assert!(!body.matches_context(&different));
        }
    }

    #[test]
    fn original_member_row_ingress_keeps_unknown_and_counted_values() {
        let source = r"oo::class create C {method $name {} {}; method \uD800 {} {}; method Ω {} {}; method \x00 {} {}; method literal $params {}}";
        let (analysis, parent) = original_definition_parent(source, "tcl8.6");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let region = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let bodies = region
            .original_commands()
            .map(|command| region.script_bodies(command).unwrap().remove(0))
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), 5);
        for body in &bodies[..4] {
            let row = body.member_row().unwrap();
            assert!(row.name.is_none());
            assert!(matches!(
                row.effect,
                tcl_registry::definer::MemberEffect::Callable { .. }
            ));
            assert!(body.matches_source(
                &tcl_lexer::SourceImage::document(source),
                analysis.body_lexer_config.unwrap()
            ));
        }
        let row = bodies[4].member_row().unwrap();
        assert_eq!(row.name.as_deref(), Some("literal"));
        assert!(row.arity.is_none());
    }

    #[test]
    fn original_member_keyword_requires_complete_owned_argv_but_not_body_applicability() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "oo::class create C {method café $params {puts hidden}}";
        let (analysis, parent) = original_definition_parent(source, "tcl8.6");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let members = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let start = u32::try_from(source.find("method").unwrap()).unwrap();
        let words = members.original_command_words_at(start).unwrap();
        assert_eq!(members.member_keyword(words).unwrap().keyword, "method");
        assert!(
            members.script_bodies(words).is_none(),
            "a keyword does not prove its unknown body layout"
        );
        assert!(members.member_keyword(&words[..3]).is_none());
        assert!(members.original_command_words_at(start + 1).is_none());
    }

    #[test]
    fn original_property_member_bodies_skip_option_like_values_and_quoted_options() {
        // naming.tcloo.property-option-value-boundary
        // docs/design/analysis/name-resolution-proofs/tcloo-property-option-value-boundary.md
        // Native observations concern stored Tcl scripts; this control binds
        // their layout to authentic source spans without granting installation.
        for dialect in ["tcl9.0", "tcl9.1"] {
            let source =
                "oo::configurable create C {property p -get {-set} \"-set\" {set café live}}";
            let (analysis, parent) = original_definition_parent(source, dialect);
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let members = parent
                .definition_member_region(&context, parent.content_span())
                .unwrap();
            let start = u32::try_from(source.find("property").unwrap()).unwrap();
            let words = members.original_command_words_at(start).unwrap();
            let bodies = members.script_bodies(words).unwrap();
            assert_eq!(bodies.len(), 2);
            assert_eq!(bodies[0].argument(), 2);
            assert_eq!(bodies[0].original_container(), &words[3]);
            assert_eq!(
                source.get(bodies[0].content_span().as_range()),
                Some("-set")
            );
            assert_eq!(bodies[1].argument(), 4);
            assert_eq!(bodies[1].original_container(), &words[5]);
            assert_eq!(
                source.get(bodies[1].content_span().as_range()),
                Some("set café live")
            );
            assert!(bodies.iter().all(|body| body.definition_parent().is_none()));
            assert!(
                bodies
                    .iter()
                    .all(|body| body.original_container() != &words[4])
            );
        }
        let source = "oo::configurable create C {property p -get {return superseded} \"-get\" {-set} -set {set café live}}";
        let (analysis, parent) = original_definition_parent(source, "tcl9.1");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let members = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let start = u32::try_from(source.find("property").unwrap()).unwrap();
        let words = members.original_command_words_at(start).unwrap();
        let bodies = members.script_bodies(words).unwrap();
        assert_eq!(
            bodies
                .iter()
                .map(OriginalSourceDefinitionMemberScriptBody::argument)
                .collect::<Vec<_>>(),
            vec![4, 6]
        );
        assert_eq!(
            source.get(bodies[0].content_span().as_range()),
            Some("-set")
        );
        assert!(
            bodies
                .iter()
                .all(|body| body.original_container() != &words[3])
        );
    }

    #[test]
    fn original_member_region_owns_complete_argv_and_unicode_body_geometry() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "oo::class create C {\nmethod café {} {puts café}\nself self {method n {} {puts nested}}\n}\n";
        let (analysis, parent) = original_definition_parent(source, "tcl8.6");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let members = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            parent.content_span(),
            analysis.body_lexer_config.unwrap(),
        )
        .unwrap();
        let method = &plan.commands[0].words;
        let bodies = members.script_bodies(method).unwrap();
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].argument(), 2);
        assert_eq!(bodies[0].original_words(), method);
        assert_eq!(bodies[0].original_container(), &method[3]);
        assert_eq!(
            source.get(bodies[0].content_span().as_range()),
            Some("puts café")
        );
        assert!(bodies[0].definition_parent().is_none());
        assert!(
            members.script_bodies(&method[..3]).is_none(),
            "a trimmed argv is not the complete original command"
        );
        let wrapper = members
            .script_bodies(&plan.commands[1].words)
            .unwrap()
            .remove(0);
        assert_eq!(wrapper.argument(), 1);
        let nested = wrapper
            .definition_parent()
            .unwrap()
            .definition_member_region(&context, wrapper.content_span())
            .unwrap();
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            wrapper.content_span(),
            analysis.body_lexer_config.unwrap(),
        )
        .unwrap();
        let body = nested
            .script_bodies(&plan.commands[0].words)
            .unwrap()
            .remove(0);
        assert_eq!(
            source.get(body.content_span().as_range()),
            Some("puts nested")
        );
    }

    #[test]
    fn original_member_region_declines_foreign_context_words_and_cooked_bodies() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "oo::class create C {method café {} {puts café}}";
        let (analysis, parent) = original_definition_parent(source, "tcl8.6");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let config = analysis.body_lexer_config.unwrap();
        let members = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let foreign = tcl_lexer::native_script_words_in(
            SourceImage::document(&source.replace("café", "cafè")),
            parent.content_span(),
            config,
        )
        .unwrap();
        assert!(members.script_bodies(&foreign.commands[0].words).is_none());
        let mut foreign_config = config;
        foreign_config.strict_quoting ^= true;
        let foreign = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            parent.content_span(),
            foreign_config,
        )
        .unwrap();
        assert!(members.script_bodies(&foreign.commands[0].words).is_none());
        let foreign_context = tcl_registry::model::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
        );
        assert!(
            parent
                .definition_member_region(&foreign_context, parent.content_span())
                .is_none()
        );
        assert!(
            parent
                .definition_member_region(
                    &context,
                    Span::new(0, u32::try_from(source.len()).unwrap())
                )
                .is_none()
        );
        for (source, unknown_layout) in [
            ("oo::class create C {$member m {} {puts hidden}}", true),
            ("oo::class create C {method m $params {puts hidden}}", true),
            (
                r#"oo::class create C {method m {} "\u0070uts hidden"}"#,
                false,
            ),
            ("oo::class create C {method m {} body[puts hidden]}", false),
        ] {
            let (analysis, parent) = original_definition_parent(source, "tcl8.6");
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let members = parent
                .definition_member_region(&context, parent.content_span())
                .unwrap();
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                parent.content_span(),
                analysis.body_lexer_config.unwrap(),
            )
            .unwrap();
            let bodies = members.script_bodies(&plan.commands[0].words);
            if unknown_layout {
                assert!(bodies.is_none(), "{source}");
            } else {
                assert!(bodies.unwrap().is_empty(), "{source}");
            }
        }
    }

    #[test]
    fn original_expression_script_regions_use_actual_grammar_and_container() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        for dialect in ["tcl8.6", "tcl9.0"] {
            let source = "expr {[format café]}";
            let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
            let config = analysis.body_lexer_config.unwrap();
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let words =
                super::super::source_structure::source_registry_words(source, &analysis, &command)
                    .unwrap();
            let input = analysis.resolved_input.as_ref().unwrap();
            let bodies = words.source_expression_script_bodies(input).unwrap();
            assert_eq!(bodies.len(), 1);
            assert_eq!(
                source.get(bodies[0].content_span().as_range()),
                Some("format café")
            );
            assert_eq!(
                bodies[0].original_container(),
                words.operands()[0].as_ref().unwrap().word().unwrap()
            );
            let mut changed = config;
            changed.strict_quoting ^= true;
            let stale = crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                input.context_registry(),
                changed,
            );
            assert!(words.source_expression_script_bodies(&stale).is_none());
        }
        for source in [
            "expr {[format hidden]} + 1",
            r#"expr "\u005bformat hidden\u005d""#,
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            let config = analysis.body_lexer_config.unwrap();
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let words =
                super::super::source_structure::source_registry_words(source, &analysis, &command)
                    .unwrap();
            assert!(
                words
                    .source_expression_script_bodies(analysis.resolved_input.as_ref().unwrap())
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_script_inventory_separates_reference_syntax_from_potential_evaluation() {
        // naming.source.original-script-region-purpose
        // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
        let source = "reference-script {format reference}\nrun-script {format potential}";
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "reference-script",
            arity: Arity::exact(1),
            arg_roles: &[(0, ArgRole::Body)],
            script_timing_resolver: Some(reference_only),
            ..CommandSpec::DEFAULT
        });
        registry.insert(CommandSpec {
            name: "run-script",
            arity: Arity::exact(1),
            arg_roles: &[(0, ArgRole::Body)],
            ..CommandSpec::DEFAULT
        });
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let generation = tcl_registry::model::context_for_profile(profile);
        let context = Arc::new(generation.with_command_store(Arc::new(registry)));
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let syntax =
            super::super::source_structure::OriginalSourceRegistryContext::capture(source, input);
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let reference = syntax.words(&commands[0]).unwrap();
        assert_eq!(reference.source_script_bodies(&context).len(), 1);
        assert!(
            reference
                .source_script_bodies_for(
                    &context,
                    OriginalSourceScriptPurpose::PotentialEvaluation
                )
                .is_empty()
        );
        let potential = syntax.words(&commands[1]).unwrap();
        assert_eq!(
            potential
                .source_script_bodies_for(
                    &context,
                    OriginalSourceScriptPurpose::PotentialEvaluation
                )
                .len(),
            1
        );
        assert_eq!(
            potential.with_source_schema(&context, |schema| schema
                .authored_source_script_timing_at(0)),
            Some(Some(ScriptTiming::SameInvocation))
        );
        assert_eq!(
            source_script_bodies_for_mutation_coverage(&reference, &context),
            Some(vec![])
        );
    }

    #[test]
    fn original_definition_coverage_requires_complete_parent_and_member_plans() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            let source = "oo::class create C {constructor {} { }; method ping {} {}}";
            let (analysis, parent) = original_definition_parent(source, dialect);
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let region = parent
                .definition_member_region(&context, parent.content_span())
                .unwrap();
            let commands = region.complete_original_commands().expect(dialect);
            assert_eq!(commands.len(), 2);
            let constructor = region.script_bodies(&commands[0]).unwrap().remove(0);
            assert_eq!(constructor.complete_original_commands(), Some(Vec::new()));
            let prefix = parent
                .definition_member_region(
                    &context,
                    Span::new(
                        parent.content_span().start(),
                        commands[0].last().unwrap().span().end(),
                    ),
                )
                .unwrap();
            assert!(prefix.complete_original_commands().is_none(), "{dialect}");

            let (analysis, parent) = original_definition_parent(
                "oo::class create C {constructor {} { }; \"unterminated}",
                dialect,
            );
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let region = parent
                .definition_member_region(&context, parent.content_span())
                .unwrap();
            assert!(region.complete_original_commands().is_none(), "{dialect}");

            let (analysis, parent) = original_definition_parent(
                "oo::class create C {constructor {} {\"unterminated}}",
                dialect,
            );
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let region = parent
                .definition_member_region(&context, parent.content_span())
                .unwrap();
            let command = region.original_commands().next().unwrap();
            let constructor = region.script_bodies(command).unwrap().remove(0);
            assert!(
                constructor.complete_original_commands().is_none(),
                "{dialect}"
            );
        }
    }
}

#[cfg(test)]
#[path = "source_scoped_body/class_source_join_tests.rs"]
mod class_source_join_tests;
