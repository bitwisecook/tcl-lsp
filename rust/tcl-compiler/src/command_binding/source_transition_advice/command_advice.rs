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

//! What the transition advice of one command answers: sources, contexts and catalogue bodies.

use super::{
    OriginalSourceCatalogueBody, OriginalSourceCommandTransition,
    OriginalSourceCommandTransitionAdvice, SourceAdviceNameInput, SourceAdviceWord,
    SourceCommandTransitionObligation,
};
use std::sync::Arc;
use tcl_core_types::ByteNamespacePath;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_native_model::InvocationDialect;
use tcl_registry::model::{ContextRegistry, ResolvedContext};

#[cfg(debug_assertions)]
fn trace_jim_member_earlier_operation(
    contributor: &str,
    contributor_site: u32,
    index: usize,
    total: usize,
    operation: &[NativeWord],
) {
    let stage = "jim.own_member.earlier_operation";
    if !crate::registry_invocation::source_structure::jim_source_trace_row(stage) {
        return;
    }
    let Some(head) = operation.first() else {
        eprintln!(
            "JIM_SOURCE_OWNER stage={stage} contributor={contributor} contributor_site={contributor_site} index={index} total={total} words=0 state=NO_HEAD"
        );
        return;
    };
    let source = head.image().bytes();
    let written = head.written_bytes();
    eprintln!(
        "JIM_SOURCE_OWNER stage={stage} contributor={contributor} contributor_site={contributor_site} index={index} total={total} words={} source_data={:p} source_len={} source_complete={} source_prefix={:?} start={} end={} head_len={} head_complete={} head_prefix={:?}",
        operation.len(),
        source.as_ptr(),
        source.len(),
        source.len() <= 64,
        &source[..source.len().min(64)],
        head.span().start(),
        head.span().end(),
        written.len(),
        written.len() <= 32,
        &written[..written.len().min(32)],
    );
}

impl OriginalSourceCommandTransitionAdvice {
    /// Failure-only held fields; no lookup, analysis or applicability grant.
    #[cfg(debug_assertions)]
    pub(in super::super) fn trace_jim_member_earlier(&self, contributor: &str) {
        let stage = "jim.own_member.earlier_summary";
        if !crate::registry_invocation::source_structure::jim_source_trace_row(stage) {
            return;
        }
        let debt = self.obligations.iter().any(|obligation| {
            matches!(
                obligation,
                SourceCommandTransitionObligation::UnknownEarlierMutation
            )
        });
        let count = self.uncertain_operations.len();
        eprintln!(
            "JIM_SOURCE_OWNER stage={stage} contributor={contributor} contributor_source={:p} contributor_site={} earlier_debt_present={debt} earlier_count={count} shown={} complete={}",
            Arc::as_ptr(&self.site.source),
            self.site.offset,
            count.min(4),
            count <= 4,
        );
        for (index, operation) in self.uncertain_operations.iter().take(4).enumerate() {
            trace_jim_member_earlier_operation(
                contributor,
                self.site.offset,
                index,
                count,
                operation,
            );
        }
    }

    /// Actual lexical source occurrence; not a fabricated dispatch point.
    #[must_use]
    pub const fn site(&self) -> &tcl_names::command::CommandAllocationSite {
        &self.site
    }
    /// Current call's complete unchanged original vector.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Whether two advice values retain the same issued original vector.
    #[expect(
        dead_code,
        reason = "only used by the unwired `SourceReadCatalogue::owns_original_advice`"
    )]
    pub(in super::super) fn shares_original_record(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.original, &other.original)
    }
    /// Genuine current written command head, independent of its target schema.
    #[must_use]
    pub const fn original_head(&self) -> &SourceAdviceNameInput {
        &self.head
    }
    /// Validated immutable full input for explicit Logical source advice.
    /// Native advice never supplies this carrier, and this cannot issue a
    /// Native naming policy, execution state or source-lookup authority.
    #[must_use]
    pub const fn logical_source_input(&self) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        self.logical_input.as_ref()
    }
    /// Independently selected authored target schema.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    /// Actual full source availability context.
    #[must_use]
    pub const fn context(&self) -> &ResolvedContext {
        &self.context
    }
    /// Same independently retained invocation dialect.
    #[must_use]
    pub const fn dialect(&self) -> InvocationDialect {
        self.dialect
    }
    /// Source transition lineage, retaining aliases, moves and operand producers.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.lineage
    }
    /// Every applicability limitation remains separate from source syntax.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Complete prior source vectors that left transition applicability unknown.
    /// Their source syntax is retained without asserting that any handler ran.
    #[must_use]
    pub fn uncertain_operations(&self) -> &[Arc<[NativeWord]>] {
        &self.uncertain_operations
    }
    /// Complete original source channel, image and lexer configuration agree.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.site.source.source_image() == image
            && self.config == config
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == config)
            && self.logical_scope.as_ref().is_none_or(|scope| {
                self.logical_input
                    .as_ref()
                    .is_some_and(|input| scope.matches_input(input))
                    && scope.owns_words(&self.original)
                    && scope.body().matches_source(image, config)
            })
            && self.logical_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config)
                    && self.original.iter().all(|word| {
                        word.span().start() >= body.content_span().start()
                            && word.span().end() <= body.content_span().end()
                    })
            })
            && self.procedure_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config)
                    && self.original.iter().all(|word| {
                        word.span().start() >= body.content_span().start()
                            && word.span().end() <= body.content_span().end()
                    })
            })
            && self.c_source_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config)
                    && self.original.iter().all(|word| {
                        word.span().start() >= body.content_span().start()
                            && word.span().end() <= body.content_span().end()
                    })
            })
            && self.catalogue_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config)
                    && self.original.iter().all(|word| {
                        word.span().start() >= body.content_span().start()
                            && word.span().end() <= body.content_span().end()
                    })
            })
            && self.member_catalogue_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config)
                    && self.original.iter().all(|word| {
                        word.span().start() >= body.content_span().start()
                            && word.span().end() <= body.content_span().end()
                    })
            })
            && self.declaration.as_ref().is_none_or(|rows| {
                rows.iter().all(|row| {
                    row.config == config
                        && row.entry.owns_source(&self.site.source, self.site.offset)
                        && crate::registry_invocation::original_native_compiler_words(
                            image,
                            &row.words,
                            self.site.offset,
                            config,
                        )
                        .is_some_and(|words| words.as_slice() == self.original.as_ref())
                })
            })
    }
    /// Actual full availability context and Registry semantic contents agree.
    #[must_use]
    pub fn matches_context(&self, context: &ContextRegistry) -> bool {
        self.context == *context.context()
            && self.registry == context.commands().snapshot().semantic_key()
            && self
                .logical_body
                .as_ref()
                .is_none_or(|body| body.matches_context(context))
            && self
                .procedure_body
                .as_ref()
                .is_none_or(|body| body.matches_context(context))
            && self
                .c_source_body
                .as_ref()
                .is_none_or(|body| body.matches_context(context))
            && self
                .catalogue_body
                .as_ref()
                .is_none_or(|body| body.matches_context(context))
            && self
                .member_catalogue_body
                .as_ref()
                .is_none_or(|body| body.matches_context(context))
            && self
                .logical_input
                .as_ref()
                .is_none_or(|input| std::ptr::eq(Arc::as_ptr(&input.context_registry()), context))
    }
    /// A retained source descriptor excludes a declared-body dependency only
    /// within its named Logical model, without fallback, alias or mutation debt.
    pub(crate) fn has_named_only_logical_summary_descriptor(&self) -> bool {
        self.logical_input.is_some()
            && self.logical_scope.is_some()
            && self.declaration.is_none()
            && self.lineage.is_empty()
            && self.uncertain_operations.is_empty()
            && !self.obligations.iter().any(|obligation| {
                matches!(
                    obligation,
                    SourceCommandTransitionObligation::UnknownEarlierMutation
                        | SourceCommandTransitionObligation::UnacceptedSourceArity
                        | SourceCommandTransitionObligation::ConditionalDeclarationApplicability
                )
            })
    }

    pub(crate) fn logical_source_scope(
        &self,
    ) -> Option<&Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>> {
        self.logical_scope.as_ref()
    }
    /// Authentic parent Logical source body for this conditional header.
    /// Its geometry supplies no runtime procedure/frame or Native input.
    #[must_use]
    pub fn logical_source_body(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceScriptBody> {
        self.logical_body.as_deref()
    }
    /// Authentic original procedure parent body, without entered execution.
    #[must_use]
    pub fn original_procedure_source_body(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceScriptBody> {
        self.procedure_body.as_deref()
    }
    /// Exact retained callable member-body lineage for readonly source callers.
    /// This does not establish an entered receiver, namespace or lookup frame.
    #[must_use]
    pub fn original_definition_member_source_body(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody> {
        let catalogue = match self.catalogue_body.as_ref() {
            Some(OriginalSourceCatalogueBody::Member(body)) => Some(body.as_ref()),
            _ => None,
        };
        match (self.member_catalogue_body.as_deref(), catalogue) {
            (Some(direct), Some(catalogue)) => (direct == catalogue).then_some(direct),
            (direct, catalogue) => direct.or(catalogue),
        }
    }

    pub(crate) fn original_source_load_association_body(
        &self,
    ) -> Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>> {
        let catalogue = match self.catalogue_body.as_ref() {
            Some(OriginalSourceCatalogueBody::Script(body)) => Some(body),
            _ => None,
        };
        let mut bodies = [
            self.procedure_body.as_ref(),
            self.c_source_body.as_ref(),
            self.logical_body.as_ref(),
            catalogue,
        ]
        .into_iter()
        .flatten();
        let first = bodies.next()?;
        bodies.all(|body| body == first).then(|| Arc::clone(first))
    }

    pub(crate) fn original_catalogue_source_body(&self) -> Option<&OriginalSourceCatalogueBody> {
        self.catalogue_body.as_ref()
    }

    /// Conditional C source coordinates, without a native namespace token.
    #[must_use]
    pub fn conditional_c_source_namespace(&self) -> Option<&ByteNamespacePath> {
        self.c_source_namespace.as_ref()
    }
    /// Ordered source path geometry. Unretained providers remain obligations;
    /// an unavailable path stays None rather than becoming an empty path.
    #[must_use]
    pub fn conditional_c_source_path(&self) -> Option<&[ByteNamespacePath]> {
        self.c_source_path.as_deref()
    }
    pub(crate) fn arguments(&self) -> &[SourceAdviceWord] {
        &self.arguments
    }
    pub(crate) fn roles(&self) -> Option<&[(usize, tcl_registry::ArgRole)]> {
        self.roles.as_deref()
    }
}
