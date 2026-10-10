// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Borrow the actual availability generation without reconstructing its label.

use tcl_registry::CommandRegistry;
use tcl_registry::model::{ContextRegistry, ResolvedContext, semantic::SemanticContext};

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
