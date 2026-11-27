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

//! The caches an `AnalysisResult` carries: the class hierarchy and the memoised source words.

use super::AnalysisResult;

/// Lazily-built, equality-transparent cache of the [`ClassHierarchy`]
/// derived from [`AnalysisResult::all_classes`].
///
/// The hierarchy is a pure function of `all_classes`, so it is excluded
/// from equality (two results with equal classes are equal regardless of
/// cache state) and reset on clone (rebuilt on demand).  This lets the LSP
/// providers (hover / completion / definition / rename / type-hierarchy)
/// share one MRO/method-provider computation per analysis instead of
/// rebuilding it on every request.
#[derive(Debug, Default)]
pub struct HierarchyCache(
    pub(super) std::sync::OnceLock<super::super::class_hierarchy::ClassHierarchy>,
);

impl Clone for HierarchyCache {
    fn clone(&self) -> Self {
        // Fresh cache; the clone rebuilds identically on first access.
        Self(std::sync::OnceLock::new())
    }
}

impl PartialEq for HierarchyCache {
    fn eq(&self, _: &Self) -> bool {
        // A derived cache never affects analysis-result equality.
        true
    }
}

/// The whole-result inputs a parentless `source_registry_words` answer reads.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SourceWordsStamp {
    realm: usize,
    pub(super) input: Option<super::super::ResolvedAnalysisInput>,
    pub(super) config: Option<tcl_lexer::LexerConfig>,
    context_unavailable: bool,
}

/// The per-command inventory rows the same answer reads: the conditional
/// metadata at the command head and the scoped bodies covering it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SourceWordsRowInputs {
    pub(super) metadata: HeadMetadataRow,
    scoped_bodies: usize,
}

/// The conditional-metadata row at a command head, or its absence.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum HeadMetadataRow {
    Absent,
    Recorded(Option<Box<crate::registry_invocation::OriginalConditionalRegistryMetadata>>),
}

pub(super) type SourceWordsRows = Vec<(
    tcl_source::segment::SegmentedCommand,
    SourceWordsRowInputs,
    Option<crate::registry_invocation::source_structure::OriginalRegistryWords>,
)>;

pub(super) type SourceWordsTable = rustc_hash::FxHashMap<(u32, u32), SourceWordsRows>;

/// Equality-transparent memo of parentless original registry words.
///
/// The analyser asks for the same command's words up to ~15 times per walk
/// (1.9k calls over 230 commands for a 650-line file). A row is reused only
/// while the realm, input and configuration are unchanged and the command's
/// own head metadata and covering scoped bodies are as they were. Vendor-name
/// documents are not memoised. In-place inventory updates clear the memo, and
/// it is emptied when an analysis completes, so a retained result holds no
/// rows until a later provider query asks again.
#[derive(Debug, Default)]
pub struct SourceWordsMemo(std::sync::Mutex<Option<(SourceWordsStamp, SourceWordsTable)>>);

/// A memoised answer, which may itself be "no words".
pub(crate) struct MemoisedSourceWords(
    pub(crate) Option<crate::registry_invocation::source_structure::OriginalRegistryWords>,
);

impl Clone for SourceWordsMemo {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for SourceWordsMemo {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl SourceWordsMemo {
    pub(super) const LIMIT: usize = 65_536;

    pub(crate) fn clear(&mut self) {
        *self
            .0
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
}

impl AnalysisResult {
    fn source_words_stamp(&self) -> Option<SourceWordsStamp> {
        self.original_vendor_source_names
            .is_empty()
            .then(|| SourceWordsStamp {
                realm: self
                    .command_realm
                    .as_ref()
                    .map_or(0, |realm| std::sync::Arc::as_ptr(realm) as usize),
                input: self.resolved_input.clone(),
                config: self.body_lexer_config,
                context_unavailable: self.analysis_context_unavailable.is_some(),
            })
    }

    fn source_words_row_inputs(
        &self,
        segment: &tcl_source::segment::SegmentedCommand,
    ) -> SourceWordsRowInputs {
        let head = segment.argv.first().map(|word| word.span.start());
        SourceWordsRowInputs {
            metadata: head
                .and_then(|head| self.original_conditional_registry_metadata.get(&head))
                .map_or(HeadMetadataRow::Absent, |row| {
                    HeadMetadataRow::Recorded(row.clone().map(Box::new))
                }),
            scoped_bodies: head.map_or(0, |head| {
                self.original_scoped_bodies
                    .iter()
                    .filter(|body| {
                        let span = body.content_span();
                        span.start() <= head && head < span.end()
                    })
                    .count()
            }),
        }
    }

    /// The memoised words of `segment` under the current inputs, if any.
    pub(crate) fn memoised_source_words(
        &self,
        segment: &tcl_source::segment::SegmentedCommand,
    ) -> Option<MemoisedSourceWords> {
        let stamp = self.source_words_stamp()?;
        let memo = self
            .source_words_memo
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (retained, rows) = memo.as_ref()?;
        if *retained != stamp {
            return None;
        }
        let candidates = rows.get(&(segment.span.start(), segment.span.end()))?;
        let inputs = self.source_words_row_inputs(segment);
        candidates
            .iter()
            .find(|(command, row, _)| command == segment && *row == inputs)
            .map(|(_, _, words)| MemoisedSourceWords(words.clone()))
    }

    pub(crate) fn memoise_source_words(
        &self,
        segment: &tcl_source::segment::SegmentedCommand,
        words: Option<&crate::registry_invocation::source_structure::OriginalRegistryWords>,
    ) {
        let Some(stamp) = self.source_words_stamp() else {
            return;
        };
        let inputs = self.source_words_row_inputs(segment);
        let mut memo = self
            .source_words_memo
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (retained, rows) =
            memo.get_or_insert_with(|| (stamp.clone(), SourceWordsTable::default()));
        if *retained != stamp || rows.len() >= SourceWordsMemo::LIMIT {
            *retained = stamp;
            rows.clear();
        }
        let candidates = rows
            .entry((segment.span.start(), segment.span.end()))
            .or_default();
        candidates.retain(|(command, _, _)| command != segment);
        candidates.push((segment.clone(), inputs, words.cloned()));
    }
}
