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

//! Source-instance services that reach binding: the shared lexical projection
//! memo of one source instance and the activation names derived from it.

use super::{
    AllocationMemoTable, DeferredSourceBody, SourceCommandTarget, original_source_projection,
};
use std::sync::{Arc, Mutex, OnceLock};
use tcl_names::origin::{SourceOriginId, SourceOriginKind};

/// Lexical projections of one immutable source instance, shared by every walk
/// of that instance.
pub trait SourceOriginLexical {
    /// Reuse only derived lexical geometry of this immutable source instance.
    /// Complete config, offset and original vector are matched independently
    /// of every caller's current lookup, input, metadata and Native admission.
    #[doc(hidden)]
    fn original_lexical_command(
        &self,
        words: &[tcl_source::words::WordExpr],
        offset: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Arc<crate::registry_invocation::OriginalSourceCommandProjection>>;

    /// Checked original words of `region` in this source's own image. Clean
    /// nested command-body verdicts are shared with every other walk of this
    /// immutable source instance; the plan equals the unshared capture.
    ///
    /// # Errors
    /// Returns the lexer's unavailable-words reason.
    fn original_script_words_in(
        &self,
        region: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        purpose: tcl_lexer::SourceSegmentationPurpose,
    ) -> Result<tcl_lexer::NativeScriptWordsPlan, tcl_lexer::NativeScriptWordsUnavailable>;
}

impl SourceOriginLexical for SourceOriginId {
    fn original_lexical_command(
        &self,
        words: &[tcl_source::words::WordExpr],
        offset: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Arc<crate::registry_invocation::OriginalSourceCommandProjection>> {
        self.original_lexical_projection()
            .capture(self.source_image(), words, offset, config)
    }

    fn original_script_words_in(
        &self,
        region: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        purpose: tcl_lexer::SourceSegmentationPurpose,
    ) -> Result<tcl_lexer::NativeScriptWordsPlan, tcl_lexer::NativeScriptWordsUnavailable> {
        let projection = self.original_lexical_projection();
        tcl_lexer::native_script_words_in_with_purpose_and_memo(
            self.source_image().clone(),
            region,
            config,
            purpose,
            projection.nested_cuts(),
        )
    }
}

/// The lexical projection memo shared by every walk of one source instance.
pub(super) trait SourceOriginProjection {
    fn original_lexical_projection(
        &self,
    ) -> Arc<original_source_projection::OriginalSourceProjectionMemo>;
}

impl SourceOriginProjection for SourceOriginId {
    fn original_lexical_projection(
        &self,
    ) -> Arc<original_source_projection::OriginalSourceProjectionMemo> {
        original_lexical_projection(self)
    }
}

fn original_lexical_projection(
    source: &SourceOriginId,
) -> Arc<original_source_projection::OriginalSourceProjectionMemo> {
    static MEMOS: OnceLock<
        Mutex<
            AllocationMemoTable<
                SourceOriginKind,
                original_source_projection::OriginalSourceProjectionMemo,
            >,
        >,
    > = OnceLock::new();
    MEMOS
        .get_or_init(|| Mutex::new(AllocationMemoTable::default()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_or_init(source.shared_kind())
}

pub(super) fn source_activation_name(
    origin: Option<&Arc<SourceOriginId>>,
    label: &str,
    offset: u32,
) -> String {
    origin.map_or_else(
        || format!("{label}/site:{offset}"),
        |origin| {
            format!(
                "{label}/source:{}/site:{offset}",
                SourceOriginId::activation_key(origin)
            )
        },
    )
}

fn source_procedure_activation_name(
    origin: Option<&Arc<SourceOriginId>>,
    target: &SourceCommandTarget,
    site: u32,
) -> String {
    let mut label = format!("{}@{}", target.command, target.implementation_generation);
    if let Some(allocation) = &target.implementation_allocation {
        use std::fmt::Write;
        write!(
            label,
            "/implementation:{}:{}:{:?}",
            SourceOriginId::activation_key(&allocation.site.source),
            allocation.site.offset,
            allocation.incarnation,
        )
        .expect("formatting an activation identity into a String");
    }
    source_activation_name(origin, &label, site)
}

pub(super) fn source_called_body_activation_name(
    origin: Option<&Arc<SourceOriginId>>,
    target: &SourceCommandTarget,
    body: &DeferredSourceBody,
    site: u32,
) -> String {
    let parent = source_procedure_activation_name(origin, target, site);
    if body.receiver_method
        && let Some((declaration, _, _)) = &body.executed_script
    {
        return format!(
            "{parent}/body:{}:{}",
            SourceOriginId::activation_key(&declaration.source),
            declaration.offset,
        );
    }
    parent
}
