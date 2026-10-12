// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional Logical declaration scopes retain their real source body.

use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::OriginalLogicalSourceNameInput;
use crate::registry_invocation::OriginalSourceScriptBody;
use std::sync::Arc;
use tcl_core_types::ByteNamespacePath;
use tcl_lexer::NativeWord;

/// Registry-selected source frame recipe; never an entered variable frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalDeclaredSourceBodyFrame {
    /// A genuine deferred procedure body retains its declaration namespace.
    Procedure,
    /// A genuine namespace body retains its independently selected name input.
    Namespace,
}

/// Original Logical body/scope ancestry for conditional declared roles.
/// Constructed namespace components are source geometry only; no native
/// namespace, entered receiver/frame, implementation or Normal follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeclaredLogicalBodyContext {
    scope: Arc<OriginalSourceScriptBody>,
    body: Arc<OriginalSourceScriptBody>,
    scope_name: OriginalLogicalSourceNameInput,
    namespace: ByteNamespacePath,
    frame: OriginalDeclaredSourceBodyFrame,
    input: ResolvedAnalysisInput,
    parent: Option<Arc<Self>>,
}
impl OriginalDeclaredLogicalBodyContext {
    pub(crate) fn capture(
        scope: Arc<OriginalSourceScriptBody>,
        scope_name: OriginalLogicalSourceNameInput,
        namespace: ByteNamespacePath,
        frame: OriginalDeclaredSourceBodyFrame,
        input: &ResolvedAnalysisInput,
        parent: Option<Arc<Self>>,
    ) -> Option<Self> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        if !input.has_logical_source_name_context()
            || scope_name.original_word().image() != scope.original_container().image()
            || scope_name.original_word().config() != input.lexer_config()
            || parent.as_ref().is_some_and(|parent| {
                !parent.matches_input(input)
                    || !parent.owns_words(&[scope_name.original_word().clone()])
                    || scope.content_span().start() < parent.body().content_span().start()
                    || scope.content_span().end() > parent.body().content_span().end()
            })
        {
            return None;
        }
        let context = input.context_registry();
        let selected = scope
            .source_words()
            .with_source_schema(&context, |schema| {
                schema.facts().arity_accepts_frozen_arguments() == Some(true)
                    && match frame {
                        OriginalDeclaredSourceBodyFrame::Procedure => {
                            schema.semantics.body_kind == tcl_registry::BodyKind::Structural
                                && schema.semantics.traits.contains(
                                    tcl_registry::Traits::DEFINES_PROCEDURE
                                        | tcl_registry::Traits::DEFERS_BODY,
                                )
                        }
                        OriginalDeclaredSourceBodyFrame::Namespace => {
                            schema.semantics.body_kind == tcl_registry::BodyKind::Structural
                                && schema
                                    .semantics
                                    .traits
                                    .contains(tcl_registry::Traits::DECLARES_NAMESPACE)
                        }
                    }
            })?;
        if !selected {
            return None;
        }
        let result = Self {
            body: Arc::clone(&scope),
            scope,
            scope_name,
            namespace,
            frame,
            input: input.clone(),
            parent,
        };
        result.matches_input(input).then_some(result)
    }

    pub(crate) fn with_body(&self, body: Arc<OriginalSourceScriptBody>) -> Option<Self> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let scope = self.scope.content_span();
        let child = body.content_span();
        if child.start() < scope.start() || child.end() > scope.end() {
            return None;
        }
        let mut result = self.clone();
        result.body = body;
        result.matches_input(&self.input).then_some(result)
    }

    pub(crate) fn matches_input(&self, input: &ResolvedAnalysisInput) -> bool {
        let context = input.context_registry();
        self.input == *input
            && input.has_logical_source_name_context()
            && [&self.scope, &self.body].into_iter().all(|body| {
                body.matches_source(self.scope.original_container().image(), input.lexer_config())
                    && body.matches_context(&context)
                    && matches!(body.source_words().source(),
                        crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice)
                        if advice.logical_source_input() == Some(input))
            })
    }

    pub(crate) fn owns_words(&self, words: &[NativeWord]) -> bool {
        let span = self.body.content_span();
        !words.is_empty()
            && words.iter().all(|word| {
                word.image() == self.body.original_container().image()
                    && word.config() == self.input.lexer_config()
                    && word.span().start() >= span.start()
                    && word.span().end() <= span.end()
            })
    }

    pub(crate) fn body_arc(&self) -> &Arc<OriginalSourceScriptBody> {
        &self.body
    }

    /// Real original body that selected this source namespace/frame recipe.
    #[must_use]
    pub fn scope_body(&self) -> &OriginalSourceScriptBody {
        &self.scope
    }

    /// Exact nearest original script container of the declared call.
    #[must_use]
    pub fn body(&self) -> &OriginalSourceScriptBody {
        &self.body
    }

    /// Genuine original name input selected by the owning source worker.
    #[must_use]
    pub const fn scope_name_input(&self) -> &OriginalLogicalSourceNameInput {
        &self.scope_name
    }

    /// Constructed Logical source components; no native namespace identity.
    #[must_use]
    pub const fn namespace_recipe(&self) -> &ByteNamespacePath {
        &self.namespace
    }

    /// Source-only frame recipe selected by the retained Registry body schema.
    #[must_use]
    pub const fn frame_recipe(&self) -> OriginalDeclaredSourceBodyFrame {
        self.frame
    }
}
