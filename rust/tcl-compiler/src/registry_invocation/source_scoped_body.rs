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

//! Original source bodies retain their selected scoped candidate schema.

use std::sync::Arc;

use super::{OriginalConditionalRegistryMetadata, OriginalConditionalVendorRegistryMetadata};
use crate::ir::CommandTokens;
use crate::signature_scan::original_name::SourceOriginalNameOccurrence;
use crate::signature_scan::vendor_name::VendorSourceNameOccurrence;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span, TokenType};
use tcl_names::namespace::SignatureNamespaceScope;
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_registry::{ArgRole, RegistrySemanticKey};

mod definition_members;
mod operand_script_bodies;

pub(crate) use definition_members::OriginalSourceDefinitionMemberBodyCorrespondence;
pub use definition_members::{
    OpaqueScriptAuthoringRegion, OriginalSourceDefinitionMemberDisposition,
    OriginalSourceDefinitionMemberReference, OriginalSourceDefinitionMemberRegion,
    OriginalSourceDefinitionMemberScriptBody,
};
pub use operand_script_bodies::OriginalDeclaredSourceScriptBody;
#[cfg(test)]
use operand_script_bodies::original_word_script_body_geometry;
pub(crate) use operand_script_bodies::{
    OriginalSourceScriptBodyOrigin, conditional_source_case_bodies_for,
    declared_source_script_bodies_for, original_written_script_body_is_empty,
    original_written_script_body_span, source_expression_script_bodies_at,
};
pub(super) use operand_script_bodies::{
    source_expression_script_bodies, source_script_bodies_for, source_script_bodies_for_coverage,
    source_script_bodies_for_mutation_coverage,
};

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

/// Genuine case bodies for the conditional normal-source question only.
/// The original source/input owner and its unknown-selection residual remain.
pub(crate) struct OriginalNormalSourceCaseBodies {
    normal: tcl_registry::normal_source_case::NormalSourceCaseLayout,
    possible: OriginalConditionalSourceCaseBodies,
}

impl OriginalNormalSourceCaseBodies {
    pub(crate) fn bodies(&self) -> Option<&[OriginalSourceScriptBody]> {
        (self.normal.possible_bodies() == &self.possible.projection)
            .then_some(self.possible.bodies())
    }
}

pub(super) fn normal_source_case_bodies_for(
    words: &super::source_structure::OriginalRegistryWords,
    context: &ContextRegistry,
) -> Option<OriginalNormalSourceCaseBodies> {
    if words.context() != Some(context.context()) || !words.matches_registry(context.commands()) {
        return None;
    }
    if words.origins().iter().enumerate().any(|(ordinal, origin)| {
        *origin != crate::registry_invocation::InvocationWordOrigin::Written(ordinal)
    }) {
        return None;
    }
    let config = words.head_source()?.word()?.config();
    let logical_scope = match words.source() {
        super::source_structure::OriginalRegistrySource::SourceTransitions(advice) => advice
            .logical_source_input()
            .zip(advice.logical_source_scope())
            .is_some_and(|(input, scope)| {
                input.has_logical_source_name_context()
                    && scope.matches_input(input)
                    && scope.owns_words(advice.original_words())
                    && std::ptr::eq(input.borrowed_context_registry(), context)
            }),
        _ => false,
    };
    let normal = words.with_source_schema(context, |schema| {
        let values = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        tcl_registry::normal_source_case::case_layout_on_normal_source_continuation(schema, values)
            .or_else(|| {
                logical_scope.then(|| {
                    tcl_registry::normal_source_case::case_layout_on_logical_source_continuation(schema, values)
                }).flatten()
            })
    })??;
    let possible = conditional_source_case_bodies_for(words, context)?;
    if normal.possible_bodies() != &possible.projection
        || possible.bodies.len() != possible.projection.bodies.len()
        || !possible.matches_source(words.head_source()?.word()?.image(), config)
    {
        return None;
    }
    Some(OriginalNormalSourceCaseBodies { normal, possible })
}

/// Original Logical body and its descriptor-selected candidate vocabulary.
/// This retains full source/input/schema ownership, without a runtime scope,
/// entered frame, complete command presence or editing permission.
#[derive(Debug, Clone)]
pub struct OriginalLogicalScopedEnvironment {
    body: OriginalSourceScriptBody,
    environment: &'static tcl_registry::scoped::ScopedCommandEnv,
}

impl OriginalLogicalScopedEnvironment {
    /// Actual original parent body, including whole argv and source premises.
    #[must_use]
    pub const fn body(&self) -> &OriginalSourceScriptBody {
        &self.body
    }

    /// Reborrow candidate vocabulary only under the actual retained context.
    #[must_use]
    pub fn environment(
        &self,
        context: &ContextRegistry,
    ) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
        let selected = self.body.logical_scoped_environment(context)?;
        std::ptr::eq(selected.environment, self.environment).then_some(self.environment)
    }
}

impl OriginalSourceScriptBody {
    /// Retain scoped candidate vocabulary from this exact genuine Logical
    /// Body operand and its selected parent descriptor. A script/Expr child
    /// or captured prefix cannot borrow a whole written Body's environment.
    #[must_use]
    pub fn logical_scoped_environment(
        &self,
        context: &ContextRegistry,
    ) -> Option<OriginalLogicalScopedEnvironment> {
        self.matches_context(context).then_some(())?;
        let super::source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            self.source.source()
        else {
            return None;
        };
        let input = advice.logical_source_input()?;
        (input.has_logical_source_name_context()
            && advice.matches_context(context)
            && self.matches_source(self.container.image(), input.lexer_config())
            && self.source.roles()?.iter().any(|&(argument, role)| {
                role == ArgRole::Body
                    && self
                        .source
                        .operands()
                        .get(argument)
                        .and_then(Option::as_ref)
                        .and_then(super::source_structure::OriginalOperandSource::word)
                        == Some(&self.container)
            }))
        .then_some(())?;
        let environment = self.source.with_source_schema(context, |schema| {
            schema.authored_source_descriptors().command.body_scope
        })??;
        Some(OriginalLogicalScopedEnvironment {
            body: self.clone(),
            environment,
        })
    }

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
        self.runs_during_invocation(context)
            && self.source.with_source_schema(context, |schema| {
                schema.facts().body_kind == tcl_registry::BodyKind::Plain
            }) == Some(true)
    }

    /// Every script operand of this original role runs during the command's
    /// own invocation in the current interpreter, whatever frame or namespace
    /// it enters, so its commands take their source order at the command.
    #[must_use]
    pub fn runs_during_invocation(&self, context: &ContextRegistry) -> bool {
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
            schema.facts().body_interpreter == tcl_registry::BodyInterpreter::Current
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

#[cfg(test)]
mod script_purpose_tests;

#[cfg(test)]
#[path = "source_scoped_body/class_source_join_tests.rs"]
mod class_source_join_tests;
