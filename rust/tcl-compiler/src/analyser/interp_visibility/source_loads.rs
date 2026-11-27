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

//! The `source` edges retained under a child interpreter's declaration.

use super::super::{Analyser, ResolvedAnalysisInput};
use super::SourceInterpreterVisibilitySnapshot;
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage, Token};
use tcl_names::namespace::SignatureNamespaceScope;
use tcl_registry::StateTransition;

impl Analyser {
    pub(super) fn retain_original_interp_source_loads(
        &mut self,
        argv: &[Token],
        scope_path: &[usize],
    ) {
        let candidate = (|| {
            let snapshot = self.safe_interp_stack.last()?.clone();
            if !snapshot.matches(
                &SourceImage::document(&self.source),
                &self.resolved_analysis_input(),
            ) {
                return None;
            }
            let native = self.original_visibility_words(argv)?;
            let (namespace, declared_receiver, source_receiver_declaration) =
                if let Some(namespace) = self.declaration_namespace_scope(scope_path) {
                    (namespace, None, None)
                } else {
                    let declared = self.original_interp_declared_receiver_body(&native, scope_path);
                    let source_declaration =
                        self.original_interp_source_receiver_declaration(&native, scope_path);
                    if declared.is_none() && source_declaration.is_none() {
                        return None;
                    }
                    (
                        SignatureNamespaceScope::C(tcl_core_types::ByteNamespacePath::root()),
                        declared,
                        source_declaration,
                    )
                };
            let schema = snapshot.source_load_schema(&native, &namespace)?;
            let mut paths = Vec::new();
            for fact in schema.transitions.facts() {
                if let StateTransition::Package(
                    tcl_registry::model::binding::PackageTransition::SourceLoad { path },
                ) = &fact.transition
                {
                    let word = native.get(path.argument_index()?.checked_add(1)?)?.clone();
                    let input = snapshot.original_input(&word);
                    let literal = input
                        .as_ref()
                        .and_then(|input| std::str::from_utf8(input.bytes()).ok())
                        .map(str::to_owned);
                    let raw = literal.clone().or_else(|| {
                        self.source
                            .get(word.content_span().ok()?.as_range())
                            .map(str::to_owned)
                    })?;
                    paths.push((
                        raw,
                        literal.is_some(),
                        OriginalInterpreterSourceLoad {
                            snapshot: snapshot.clone(),
                            invocation: native.clone(),
                            path: word,
                            declared_receiver: declared_receiver.clone(),
                            source_receiver_declaration: source_receiver_declaration.clone(),
                        },
                    ));
                }
            }
            Some(paths)
        })();
        for (raw_path, is_literal, original) in candidate.unwrap_or_default() {
            let range = argv
                .iter()
                .find(|token| {
                    tcl_lexer::word_span_at(&*self.source, token.span) == original.path.span()
                })
                .map_or(original.path.span(), |token| token.span);
            if self
                .result
                .source_targets
                .iter()
                .any(|source| source.range == range)
            {
                continue;
            }
            self.result
                .source_targets
                .push(crate::signature_scan::types::SignatureSource {
                    raw_path,
                    range,
                    is_literal,
                    site_namespace: self.command_resolution_namespace(scope_path),
                    original_project_source_load: None,
                    original_interpreter_source_load: Some(Arc::new(original)),
                });
        }
    }
}

/// A possible file-source edge under a retained child source declaration.
/// The original path and full visibility lineage remain owned; file existence,
/// actual dispatch, file execution and Normal completion are not established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalInterpreterSourceLoad {
    pub(super) snapshot: SourceInterpreterVisibilitySnapshot,
    invocation: Vec<NativeWord>,
    path: NativeWord,
    pub(super) declared_receiver:
        Option<Arc<crate::command_binding::SourceDeclaredReceiverBodyEntry>>,
    pub(super) source_receiver_declaration:
        Option<Arc<super::super::types::OriginalSourceReceiverBodyDeclaration>>,
}

impl OriginalInterpreterSourceLoad {
    /// Independent declared receiver source scope when its runtime namespace
    /// is unknown. This is a possible global-source hypothesis only.
    #[must_use]
    pub fn declared_receiver_body(
        &self,
    ) -> Option<&crate::command_binding::SourceDeclaredReceiverBodyEntry> {
        self.declared_receiver.as_deref()
    }
    /// Selected original method-body declaration without a native class or
    /// receiver allocation. This source scope supplies no executing namespace.
    #[must_use]
    pub fn source_receiver_body_declaration(
        &self,
    ) -> Option<&super::super::types::OriginalSourceReceiverBodyDeclaration> {
        self.source_receiver_declaration.as_deref()
    }
    /// Original path operand, independently of its reporting text.
    #[must_use]
    pub const fn path_word(&self) -> &NativeWord {
        &self.path
    }
    /// Complete original invocation vector, including the real source head.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.invocation
    }
    /// Same retained analysis and full input, without rebuilding source facts.
    #[must_use]
    pub fn matches_analysis(&self, analysis: &super::super::AnalysisResult) -> bool {
        analysis.resolved_input.as_ref().is_some_and(|input| {
            self.matches_source(self.path.image(), input)
                && analysis.matches_original_source_image(self.path.image(), input.lexer_config())
        })
    }

    /// Whole original source/channel and complete immutable input join.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        self.snapshot.matches(image, input)
            && self
                .source_receiver_declaration
                .as_ref()
                .is_none_or(|body| {
                    body.matches_source(image, input)
                        && body.owns_invocation(&self.invocation, input)
                })
            && self.declared_receiver.as_ref().is_none_or(|body| {
                body.source().origin.source_image() == image
                    && matches!(
                        body.source().mapping,
                        tcl_names::origin::ExecutedScriptMapping::Contiguous { .. }
                    )
            })
            && self.path.image() == image
            && self.path.config() == input.lexer_config()
            && self.invocation.contains(&self.path)
            && self
                .invocation
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
    }
}
