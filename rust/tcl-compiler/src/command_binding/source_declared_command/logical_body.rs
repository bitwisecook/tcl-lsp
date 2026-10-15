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
    /// An anonymous lambda retains its own source call, formals and namespace.
    Lambda,
    /// A genuine namespace body retains its independently selected name input.
    Namespace,
    /// A genuine original selected-frame body requests the Logical root.
    Global,
}

/// Original Logical body/scope ancestry for conditional declared roles.
/// Constructed namespace components are source geometry only; no native
/// namespace, entered receiver/frame, implementation or Normal follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeclaredLogicalBodyContext {
    scope: Arc<OriginalSourceScriptBody>,
    body: Arc<OriginalSourceScriptBody>,
    origin: OriginalLogicalBodyOrigin,
    namespace: ByteNamespacePath,
    frame: OriginalDeclaredSourceBodyFrame,
    input: ResolvedAnalysisInput,
    parent: Option<Arc<Self>>,
    selected_frame: Option<OriginalLogicalSourceFrameSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OriginalLogicalBodyOrigin {
    Named(OriginalLogicalSourceNameInput),
    Lambda(Arc<super::OriginalLogicalLambdaBody>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalLogicalSourceFrameSelection {
    body: Arc<OriginalSourceScriptBody>,
    level: OriginalLogicalSourceNameInput,
    ordinal: usize,
}
impl OriginalLogicalSourceFrameSelection {
    fn capture(body: Arc<OriginalSourceScriptBody>, input: &ResolvedAnalysisInput) -> Option<Self> {
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = body.source_words().source()
        else {
            return None;
        };
        if advice.logical_source_input() != Some(input) {
            return None;
        }
        let ordinal = body.source_words().with_source_schema(
            input.borrowed_context_registry(),
            |schema| {
                let frame = schema.semantics.frame_effect?;
                if frame.layout != tcl_registry::FrameArgLayout::ScriptInSelectedFrame
                    || schema.facts().arity_accepts_frozen_arguments() != Some(true)
                {
                    return None;
                }
                let tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                    level_word_len: 1,
                    level,
                } = schema.authored_logical_source_frame_argument_layout()?
                else {
                    return None;
                };
                level
                    .is_global_frame()
                    .then_some(schema.semantics.argument_offset)
            },
        )??;
        let level = advice
            .arguments()
            .get(ordinal)?
            .input
            .as_ref()?
            .logical_input()?
            .clone();
        if !body.matches_source(level.original_word().image(), input.lexer_config())
            || !body.matches_context(input.borrowed_context_registry())
        {
            return None;
        }
        Some(Self {
            body,
            level,
            ordinal,
        })
    }

    fn matches(&self, input: &ResolvedAnalysisInput) -> bool {
        Self::capture(Arc::clone(&self.body), input).as_ref() == Some(self)
    }
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
                        OriginalDeclaredSourceBodyFrame::Global
                        | OriginalDeclaredSourceBodyFrame::Lambda => false,
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
            origin: OriginalLogicalBodyOrigin::Named(scope_name),
            namespace,
            frame,
            input: input.clone(),
            parent,
            selected_frame: None,
        };
        result.matches_input(input).then_some(result)
    }

    pub(crate) fn capture_lambda(
        lambda: Arc<super::OriginalLogicalLambdaBody>,
        input: &ResolvedAnalysisInput,
        parent: Option<Arc<Self>>,
    ) -> Option<Self> {
        let (_, _, content, namespace) = lambda.frame_source(input)?;
        let crate::signature_scan::scope::SignatureNamespaceScope::C(namespace) = namespace else {
            return None;
        };
        if parent.as_ref().is_some_and(|parent| {
            !parent.matches_input(input)
                || !parent.owns_words(lambda.original_words())
                || content.start() < parent.body().content_span().start()
                || content.end() > parent.body().content_span().end()
        }) {
            return None;
        }
        let result = Self {
            scope: Arc::clone(lambda.body()),
            body: Arc::clone(lambda.body()),
            origin: OriginalLogicalBodyOrigin::Lambda(lambda),
            namespace,
            frame: OriginalDeclaredSourceBodyFrame::Lambda,
            input: input.clone(),
            parent,
            selected_frame: None,
        };
        result.matches_input(input).then_some(result)
    }

    pub(crate) fn with_global_body(&self, body: Arc<OriginalSourceScriptBody>) -> Option<Self> {
        let selection =
            OriginalLogicalSourceFrameSelection::capture(Arc::clone(&body), &self.input)?;
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = body.source_words().source()
        else {
            return None;
        };
        if !self.owns_words(advice.original_words()) {
            return None;
        }
        let mut result = self.with_body(body)?;
        result.namespace = ByteNamespacePath::root();
        result.frame = OriginalDeclaredSourceBodyFrame::Global;
        result.selected_frame = Some(selection);
        result.matches_input(&self.input).then_some(result)
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
            && match &self.origin {
                OriginalLogicalBodyOrigin::Named(_) => self.frame != OriginalDeclaredSourceBodyFrame::Lambda,
                OriginalLogicalBodyOrigin::Lambda(lambda) => lambda.matches_source(self.scope.original_container().image(), input)
                    && matches!(self.frame, OriginalDeclaredSourceBodyFrame::Lambda | OriginalDeclaredSourceBodyFrame::Global)
                    && Arc::ptr_eq(lambda.body(), &self.scope)
                    && (lambda.namespace() == &crate::signature_scan::scope::SignatureNamespaceScope::C(self.namespace.clone())
                        || self.selected_frame.as_ref().is_some_and(|selected| selected.matches(input))),
            }
            && self.selected_frame.as_ref().is_none_or(|frame| {
                let selected = frame.body.content_span();
                let current = self.body.content_span();
                frame.matches(input)
                    && current.start() >= selected.start()
                    && current.end() <= selected.end()
            })
            && (self.frame != OriginalDeclaredSourceBodyFrame::Global
                || (self.namespace == ByteNamespacePath::root() && self.selected_frame.is_some()))
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

    /// Original declaration body retained as source ancestry.
    #[must_use]
    pub fn scope_body(&self) -> &OriginalSourceScriptBody {
        &self.scope
    }

    /// Exact nearest original script container of the declared call.
    #[must_use]
    pub fn body(&self) -> &OriginalSourceScriptBody {
        &self.body
    }

    /// Genuine written declaration name. Anonymous lambda scopes have none.
    #[must_use]
    pub const fn scope_name_input(&self) -> Option<&OriginalLogicalSourceNameInput> {
        match &self.origin {
            OriginalLogicalBodyOrigin::Named(name) => Some(name),
            OriginalLogicalBodyOrigin::Lambda(_) => None,
        }
    }

    pub(crate) fn lambda_source(&self) -> Option<&Arc<super::OriginalLogicalLambdaBody>> {
        match &self.origin {
            OriginalLogicalBodyOrigin::Lambda(lambda) => Some(lambda),
            OriginalLogicalBodyOrigin::Named(_) => None,
        }
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

#[cfg(test)]
mod tests {
    use super::OriginalLogicalSourceFrameSelection;
    use crate::registry_invocation::source_structure::source_registry_words;
    use std::sync::Arc;

    #[test]
    fn logical_root_body_selector_requires_the_original_global_frame_request() {
        for (source, expected) in [
            ("proc p {} {uplevel #0 {set x ROOT}}", true),
            ("proc p {} {uplevel 0 {set x CURRENT}}", false),
            ("proc p {} {uplevel 1 {set x CALLER}}", false),
            ("proc p {} {uplevel $level {set x UNKNOWN}}", false),
            ("proc p {} {uplevel #-1 {set x INVALID}}", false),
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
            let input = analysis.resolved_input.as_ref().unwrap();
            let start = source.find("uplevel").unwrap();
            let close = source.rfind('}').unwrap();
            let fragment = &source[start..close];
            let segment = crate::segmenter::segment_commands_with_offset_and_config(
                fragment,
                u32::try_from(start).unwrap(),
                input.lexer_config(),
            )
            .remove(0);
            let bodies = source_registry_words(source, &analysis, &segment)
                .map(|words| words.source_script_bodies(input.borrowed_context_registry()))
                .unwrap_or_default();
            let selections = bodies
                .into_iter()
                .filter_map(|body| {
                    OriginalLogicalSourceFrameSelection::capture(Arc::new(body), input)
                })
                .collect::<Vec<_>>();
            assert_eq!(!selections.is_empty(), expected, "{source}");
            for selected in selections {
                assert!(selected.matches(input));
                assert_eq!(selected.level.bytes(), b"#0");
                assert!(
                    selected.level.original_word().image()
                        == &tcl_lexer::SourceImage::document(source)
                );
                let foreign = crate::analyser::ResolvedAnalysisInput::new(
                    input.analyser_profile(),
                    input.unit_profile(),
                    Arc::new(
                        input.context_registry().with_command_store(
                            input
                                .context_registry()
                                .commands()
                                .snapshot()
                                .shared_registry(),
                        ),
                    ),
                    input.lexer_config(),
                );
                assert!(!selected.matches(&foreign));
            }
        }
    }
}
