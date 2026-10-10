// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Borrow the actual availability generation without reconstructing its label.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use tcl_registry::CommandRegistry;
use tcl_registry::model::{ContextRegistry, ResolvedContext, semantic::SemanticContext};

/// Retained metadata availability, independent of source naming and execution.
/// Missing supplied ownership stays unavailable; standalone compatibility is
/// an explicit variant shared by source binding and CFG consumers.
#[derive(Debug, Clone, Default)]
pub enum OwnedInvocationMetadataContext {
    /// Explicit standalone catalogue compatibility.
    #[default]
    Standalone,
    /// Actual availability without a complete source-input owner.
    Supplied(Arc<ContextRegistry>),
    /// Complete retained source/profile/configuration and availability.
    SuppliedSource(Box<crate::analyser::ResolvedAnalysisInput>),
    /// Missing or withdrawn supplied metadata; never a standalone fallback.
    Unavailable,
}

impl PartialEq for OwnedInvocationMetadataContext {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Standalone, Self::Standalone) | (Self::Unavailable, Self::Unavailable) => true,
            (Self::Supplied(left), Self::Supplied(right)) => Arc::ptr_eq(left, right),
            (Self::SuppliedSource(left), Self::SuppliedSource(right)) => left == right,
            _ => false,
        }
    }
}
impl Eq for OwnedInvocationMetadataContext {}

impl Hash for OwnedInvocationMetadataContext {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Supplied(context) => Arc::as_ptr(context).hash(state),
            Self::SuppliedSource(input) => input.hash(state),
            Self::Standalone | Self::Unavailable => {}
        }
    }
}

impl OwnedInvocationMetadataContext {
    /// Retain the exact supplied input, including terminal missing ownership.
    #[must_use]
    pub fn for_source_input(input: Option<&crate::analyser::ResolvedAnalysisInput>) -> Self {
        input.map_or(Self::Unavailable, |input| {
            Self::SuppliedSource(Box::new(input.clone()))
        })
    }

    /// Reborrow the same actual owner. The explicit standalone variant alone
    /// selects scalar compatibility; supplied refusal cannot reopen that path.
    pub(crate) fn metadata_context<'a>(
        &'a self,
        registry: &CommandRegistry,
    ) -> Option<Option<InvocationMetadataContext<'a>>> {
        let context = match self {
            Self::Supplied(context) => Some(context.as_ref().into()),
            Self::SuppliedSource(input) => Some(InvocationMetadataContext::for_analysis_input(
                registry, input,
            )?),
            Self::Standalone => registry
                .profile()
                .map(SemanticContext::for_profile)
                .map(InvocationMetadataContext::from),
            Self::Unavailable => return None,
        };
        context
            .is_none_or(|context| context.matches_registry(registry))
            .then_some(context)
    }

    /// Reborrow metadata only for the retained source policy. An independently
    /// supplied availability-only context does not manufacture a complete input.
    pub(crate) fn metadata_context_for_source<'a>(
        &'a self,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        profile: Option<&tcl_dialect::DialectProfile>,
    ) -> Option<InvocationMetadataContext<'a>> {
        match self {
            Self::SuppliedSource(input) => {
                InvocationMetadataContext::for_source_input(registry, input, config, profile)
            }
            Self::Standalone | Self::Supplied(_) => self.metadata_context(registry)?,
            Self::Unavailable => None,
        }
    }

    /// Complete actual source input, never a profile reconstruction.
    #[must_use]
    pub fn source_analysis_input(&self) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        match self {
            Self::SuppliedSource(input) => Some(input),
            _ => None,
        }
    }

    /// Explicit compatibility mode; supplied missing input does not satisfy it.
    #[must_use]
    pub const fn is_standalone(&self) -> bool {
        matches!(self, Self::Standalone)
    }
}

/// Borrowed metadata ingress for source interpretation. A supplied missing
/// input has its own tag and cannot become standalone compatibility.
#[derive(Debug, Clone, Copy, Default)]
pub enum InvocationMetadataInput<'a> {
    /// Explicit legacy catalogue compatibility.
    #[default]
    Standalone,
    /// Actual availability owner without manufacturing complete source input.
    Supplied(&'a Arc<ContextRegistry>),
    /// Complete supplied input, or an explicitly missing supplied owner.
    SuppliedSource(Option<&'a crate::analyser::ResolvedAnalysisInput>),
    /// Reborrow an already retained shared owner without changing its purpose.
    Retained(&'a OwnedInvocationMetadataContext),
}
impl PartialEq for InvocationMetadataInput<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Standalone, Self::Standalone) => true,
            (Self::Supplied(left), Self::Supplied(right)) => Arc::ptr_eq(left, right),
            (Self::SuppliedSource(left), Self::SuppliedSource(right)) => left == right,
            (Self::Retained(left), Self::Retained(right)) => left == right,
            _ => false,
        }
    }
}
impl Eq for InvocationMetadataInput<'_> {}

impl InvocationMetadataInput<'_> {
    /// Keep the original ingress mode while moving into an owned carrier.
    #[must_use]
    pub fn retain(self) -> OwnedInvocationMetadataContext {
        match self {
            Self::Standalone => OwnedInvocationMetadataContext::Standalone,
            Self::Supplied(context) => {
                OwnedInvocationMetadataContext::Supplied(Arc::clone(context))
            }
            Self::SuppliedSource(input) => OwnedInvocationMetadataContext::for_source_input(input),
            Self::Retained(owner) => owner.clone(),
        }
    }
}

/// Availability selection for source metadata, independent of execution.
/// Actual analysis input retains its command-generation identity; explicitly
/// static callers keep the interned default generation they supplied.
#[derive(Clone, Copy)]
pub struct InvocationMetadataContext<'a> {
    context: &'a ResolvedContext,
    actual_commands: Option<&'a CommandRegistry>,
    actual_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
}
impl<'a> InvocationMetadataContext<'a> {
    /// Select the supplied generation only for its actual command store.
    #[must_use]
    pub fn for_analysis_input(
        registry: &CommandRegistry,
        input: &'a crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        let context = Self {
            actual_input: Some(input),
            ..Self::from(input.borrowed_context_registry())
        };
        context.matches_registry(registry).then_some(context)
    }

    /// Select metadata only when the source producer retains the input's exact
    /// unit profile and lexer policy. Catalogue availability remains independent.
    #[must_use]
    pub fn for_source_input(
        registry: &CommandRegistry,
        input: &'a crate::analyser::ResolvedAnalysisInput,
        config: tcl_lexer::LexerConfig,
        profile: Option<&tcl_dialect::DialectProfile>,
    ) -> Option<Self> {
        if config.normalized() != input.lexer_config().normalized()
            || !profile
                .is_some_and(|profile| profile.cache_key() == input.unit_profile().cache_key())
        {
            return None;
        }
        Self::for_analysis_input(registry, input)
    }

    /// Borrow a Module's retained input only for the actual source producer.
    /// Missing input, changed grammar/profile and foreign command stores refuse.
    #[must_use]
    pub fn for_module(registry: &CommandRegistry, module: &'a crate::ir::Module) -> Option<Self> {
        Self::for_source_input(
            registry,
            module.source_metadata_input.as_ref()?,
            module.lexer_config,
            module.dialect_profile,
        )
    }

    /// Complete supplied availability, including packages and authoring scope.
    /// This does not supply native implementation or invocation presence.
    #[must_use]
    pub const fn context(self) -> &'a ResolvedContext {
        self.context
    }
    /// Exact retained source grammar/profile, independently of invocation entry.
    pub(crate) const fn source_analysis_input(
        self,
    ) -> Option<&'a crate::analyser::ResolvedAnalysisInput> {
        self.actual_input
    }

    /// Positive Logical source naming applicability, independent of metadata
    /// availability and of every Native execution or contents-presence grant.
    #[must_use]
    pub(crate) fn permits_logical_source_names(self) -> bool {
        self.actual_input
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context)
    }

    pub(super) const fn is_actual(self) -> bool {
        self.actual_commands.is_some()
    }
    /// Whether this carrier was issued from the explicit static semantic
    /// context adapter. Supplied availability without source input retains
    /// its actual command owner and cannot become standalone compatibility.
    #[must_use]
    pub(crate) const fn is_standalone(self) -> bool {
        self.actual_commands.is_none() && self.actual_input.is_none()
    }
    pub(crate) fn matches_registry(self, commands: &CommandRegistry) -> bool {
        self.actual_commands.is_none_or(|actual| {
            actual.snapshot().semantic_key() == commands.snapshot().semantic_key()
        })
    }
}
impl From<SemanticContext> for InvocationMetadataContext<'_> {
    fn from(context: SemanticContext) -> Self {
        Self {
            context: context.context(),
            actual_commands: None,
            actual_input: None,
        }
    }
}
impl<'a> From<&'a ContextRegistry> for InvocationMetadataContext<'a> {
    fn from(context: &'a ContextRegistry) -> Self {
        Self {
            context: context.context(),
            actual_commands: Some(context.commands()),
            actual_input: None,
        }
    }
}

/// Retain complete supplied source availability only for its actual command store.
/// Missing and foreign inputs remain unavailable, independently of profile labels.
pub(crate) fn retained_source_metadata_context(
    registry: &CommandRegistry,
    input: Option<&crate::analyser::ResolvedAnalysisInput>,
) -> Option<std::sync::Arc<ContextRegistry>> {
    let context = input?.context_registry();
    InvocationMetadataContext::from(context.as_ref())
        .matches_registry(registry)
        .then_some(context)
}

/// Retain metadata only for this Module's actual source producer. The complete
/// input remains owned by the Module even when this query refuses its use.
pub(crate) fn retained_module_metadata_context(
    registry: &CommandRegistry,
    module: &crate::ir::Module,
) -> Option<std::sync::Arc<ContextRegistry>> {
    Some(
        InvocationMetadataContext::for_module(registry, module)?
            .source_analysis_input()?
            .context_registry(),
    )
}

#[cfg(test)]
mod owned_metadata_tests {
    use super::*;

    #[test]
    fn retained_metadata_ingress_keeps_actual_availability_and_terminal_missing_tags() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let owner = InvocationMetadataInput::SuppliedSource(Some(&input)).retain();
        assert_eq!(owner.source_analysis_input(), Some(&input));
        let metadata = owner
            .metadata_context_for_source(context.commands(), config, Some(profile))
            .unwrap();
        assert!(
            metadata
                .context()
                .resolve_spec_in_realm(
                    context.commands(),
                    "throw",
                    tcl_dialect::model::InvocationRealm::InterpreterRuntime
                )
                .is_some()
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        let older_input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, older, config);
        let older_owner = OwnedInvocationMetadataContext::for_source_input(Some(&older_input));
        let older_metadata = older_owner
            .metadata_context_for_source(context.commands(), config, Some(profile))
            .unwrap();
        assert!(
            older_metadata
                .context()
                .resolve_spec_in_realm(
                    context.commands(),
                    "throw",
                    tcl_dialect::model::InvocationRealm::InterpreterRuntime
                )
                .is_none()
        );
        let missing = InvocationMetadataInput::SuppliedSource(None).retain();
        assert!(!missing.is_standalone());
        assert!(missing.metadata_context(context.commands()).is_none());
        assert!(
            missing
                .metadata_context_for_source(context.commands(), config, Some(profile))
                .is_none()
        );
        let mut stale = config;
        stale.strict_quoting = !stale.strict_quoting;
        assert!(
            owner
                .metadata_context_for_source(context.commands(), stale, Some(profile))
                .is_none()
        );
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        assert!(
            owner
                .metadata_context_for_source(foreign.commands(), config, Some(profile))
                .is_none()
        );
        let standalone = InvocationMetadataInput::Standalone.retain();
        assert!(standalone.is_standalone());
        assert!(standalone.metadata_context(context.commands()).is_some());
        assert_eq!(
            InvocationMetadataInput::Retained(&missing).retain(),
            missing
        );
    }
}
