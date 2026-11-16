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

//! Conditional namespace-import advice, projected under its original input and publication owners.

use super::super::{Backend, DocumentPathAdmission, DocumentSourceResult};
use super::retry_readonly_attempt;
use std::ops::ControlFlow;
use std::sync::Arc;

impl Backend {
    /// Project namespace relationships only under their original input/index
    /// and publication owners. Missing is unavailable, never namespace absence.
    pub(in super::super) async fn finish_conditional_namespace_advice(
        &self,
        advice: &tcl_lsp_core::workspace_index::ConditionalNamespaceImportAdvice,
    ) -> DocumentSourceResult<Option<Vec<String>>> {
        retry_readonly_attempt(None, move || {
            self.finish_conditional_namespace_advice_attempt(advice)
        })
        .await
    }

    async fn finish_conditional_namespace_advice_attempt(
        &self,
        advice: &tcl_lsp_core::workspace_index::ConditionalNamespaceImportAdvice,
    ) -> DocumentSourceResult<ControlFlow<Option<Vec<String>>>> {
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let mut documents = Vec::new();
        for source in advice.documents() {
            let Some(text) = source.source() else {
                return Ok(ControlFlow::Break(None));
            };
            let Some(document) = self
                .capture_readonly_source_document(source.uri(), Arc::from(text))
                .await?
            else {
                return Ok(ControlFlow::Break(None));
            };
            if !source.matches_input(document.input.as_ref()) {
                return Ok(ControlFlow::Break(None));
            }
            documents.push(document);
        }
        let _inputs = self.analyser_inputs_gate.read().await;
        let generation = self.class_factory_generation.read().await;
        let open = self.documents.lock("namespace_advice_final_snapshot").await;
        let index = self.workspace_index.read().await;
        let Ok(db) = self.db.try_lock() else {
            return Ok(ControlFlow::Continue(()));
        };
        let Ok(files) = self.db_files.try_lock() else {
            return Ok(ControlFlow::Continue(()));
        };
        let published = self
            .source_path_admissions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let uri_generations = self
            .live_publication_uri_generations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.diag_inputs_epoch() != inputs_epoch
            || *generation != factory_generation
            || !advice.matches_index(&index)
            || documents.iter().any(|document| {
                !document.matches_current_snapshot(&uri_generations, &open, &db, &files, &published)
            })
        {
            return Ok(ControlFlow::Continue(()));
        }
        let admissions = documents
            .iter()
            .map(|document| document.admission.as_ref())
            .collect::<Vec<_>>();
        DocumentPathAdmission::with_current(&admissions, self.store.as_ref(), || {
            Some(
                advice
                    .compatibility_report()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
        })
        .map(ControlFlow::Break)
    }
}
