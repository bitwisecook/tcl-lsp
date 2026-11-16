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

//! The source a coarse semantic-token answer is issued from, and its transaction.

use super::super::{Backend, DocumentSourceResult};
use super::{
    CoarseSemanticTokenSource, FreshSourceAnalysisPurpose, ReadonlyTransaction,
    WorkspaceSymbolAnalysisKind, WorkspaceSymbolSourceAnalysis,
};
use tower_lsp_server::ls_types::Uri;

impl Backend {
    /// Issue only from the real current open source and configured fresh worker.
    pub(in super::super) async fn capture_coarse_semantic_source(
        &self,
        uri: &Uri,
        snapshot: &super::super::DocumentState,
    ) -> DocumentSourceResult<Option<CoarseSemanticTokenSource>> {
        if snapshot.dialect_resolution_generation != snapshot.resolved_dialect_generation {
            // A current buffer with unresolved resource dialect is not a
            // current configured analysis input, even when bytes are unchanged.
            return Ok(None);
        }
        let Some(analysis) = self
            .capture_fresh_source_analysis(
                uri,
                snapshot.clone().normalised_for_analysis(),
                FreshSourceAnalysisPurpose::SemanticStructure,
            )
            .await?
        else {
            return Ok(None);
        };

        let Some(source) = CoarseSemanticTokenSource::from_fresh_analysis(analysis) else {
            return Ok(None);
        };
        Ok(Some(source))
    }

    /// Same bounded readonly transaction, including original provider lease,
    /// encloses both coarse token cache mutation and response projection.
    #[cfg(test)]
    pub(in super::super) async fn with_current_coarse_semantic_source<T>(
        &self,
        source: &CoarseSemanticTokenSource,
        publish: impl FnMut() -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        self.coarse_semantic_source_transaction(source, publish)
            .await
            .map(ReadonlyTransaction::published)
    }

    pub(in super::super) async fn coarse_semantic_source_transaction<T>(
        &self,
        source: &CoarseSemanticTokenSource,
        mut publish: impl FnMut() -> Option<T>,
    ) -> DocumentSourceResult<ReadonlyTransaction<T>> {
        let admissions = [source.analysis.admission()];
        self.readonly_source_transaction(
            source.inputs_epoch,
            source.factory_generation,
            &admissions,
            None,
            |state| {
                WorkspaceSymbolSourceAnalysis::matches_current_state(
                    &source.analysis,
                    state,
                    source.inputs_epoch,
                    source.factory_generation,
                )
                .then_some(())
            },
            |_, ()| publish(),
        )
        .await
    }

    /// Retain a fresh source only after resolving its current dialect generation.
    /// Old tokens are discarded; the replacement analysis and final publication
    /// must authenticate the newly captured generations independently.
    pub(in super::super) async fn coarse_source_recompute_snapshot(
        &self,
        source: &CoarseSemanticTokenSource,
    ) -> DocumentSourceResult<Option<super::super::DocumentState>> {
        let WorkspaceSymbolAnalysisKind::Fresh(document) = &source.analysis.kind else {
            return Ok(None);
        };
        self.resolve_dirty_open_document_dialect(&document.uri, "coarse_recompute_resolve")
            .await?;
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let admissions = [source.analysis.admission()];
        self.with_current_readonly_source_state(
            epoch,
            generation,
            &admissions,
            None,
            |state| {
                let current = state.open.get(&document.uri)?;
                if epoch == source.inputs_epoch
                    && generation == source.factory_generation
                    && document.open_snapshot_matches(current)
                {
                    return None;
                }
                (document.was_open
                    && document.recompute_snapshot_matches(current)
                    && document.uri_generation_matches(state)
                    && document.matches_input_state(
                        state,
                        source.inputs_epoch,
                        source.factory_generation,
                    ))
                .then(|| current.clone())
            },
            |_, current| Some(current.clone()),
        )
        .await
    }
}
