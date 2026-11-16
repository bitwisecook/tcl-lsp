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

//! One diagnostic pass: its capture from the inputs, its entry in the publisher ledger, the delivery of its completion and the currency checks made against it.

use super::super::{
    AnalysisResult, Arc, Backend, CommandRegistry, DeliveryCtx, DiagCurrency, DiagInputs, DiagJob,
    DiagSlot, DiagnosticPublisher, DocumentState, HashMap, OrphanCommandDependencies,
    PublishedSourcePath, Uri, WorkspaceCommandQueries, is_apl_source,
};
use super::{
    DiagnosticCompletionEntry, DiagnosticPass, DiagnosticPassState, DiagnosticSuppression,
    DiagnosticUnavailable,
};

impl DiagInputs {
    pub(in super::super) async fn capture_diagnostic_pass(
        &self,
        uri: &Uri,
        job: &DiagJob,
    ) -> Option<Arc<DiagnosticPass>> {
        let DiagCurrency::Open(revision) = job.currency else {
            return None;
        };
        let epoch = self.inputs_epoch?;
        // Retain the original publication, rather than recapturing a mapping
        // that might have replaced the provider while this pass was running.
        let path_admission = if let Some(file) = job.file {
            let retained = self.source_path_admissions.lock().ok()?.get(uri)?.clone();
            if retained.file != file {
                return None;
            }
            retained.admission
        } else {
            self.documents
                .lock("diagnostic_pass_source_owner")
                .await
                .get(uri)?
                .path_admission
                .clone()?
        };
        path_admission.check_for(uri, self.store.as_ref()).ok()?;
        let generation = *self.class_factory_generation.read().await;
        let db = self.db.lock().await;
        let configuration = job.config.diagnostic_report_configuration(&*db);
        let input = if let Some(file) = job.file {
            if file.text(&*db) != tcl_lexer::normalise_lone_cr(&job.text).as_ref()
                || file.dialect(&*db) != &job.dialect
            {
                None
            } else {
                tcl_lsp_db::document_analysis_input(&*db, file, job.config).ok()
            }
        } else {
            None
        };
        drop(db);
        let f5_model =
            Backend::is_bigip_dialect(&job.dialect) || is_apl_source(uri, &job.language_id);
        // A scheduled run defers the checked open-document registry to this
        // drain: an analysis pass requires the document's current input.
        // Policy-clear receipts retain source currency independently.
        let registry = if self.registry_at_drain
            && !f5_model
            && self.toggles.diagnostics_enabled
            && !self.toggles.excluded
        {
            input
                .as_ref()?
                .borrowed_context_registry()
                .commands()
                .snapshot()
        } else {
            self.registry.snapshot()
        };
        let slot_generation = self
            .diag_slots
            .lock()
            .await
            .get(uri)
            .map(|slot| slot.generation);
        Some(Arc::new(DiagnosticPass {
            configuration,
            source_file: job.file,
            path_admission,
            source: Arc::clone(&job.text),
            dialect: job.dialect.clone(),
            revision,
            version: job.version,
            inputs_epoch: epoch,
            source_inputs_epoch: Arc::clone(&self.source_inputs_epoch),
            source_inputs_gate: Arc::clone(&self.source_inputs_gate),
            source_slots: Arc::downgrade(&self.diag_slots),
            source_factory_generation: Arc::clone(&self.class_factory_generation),
            slot_generation,
            class_factory_generation: generation,
            input,
            registry,
            f5_model,
        }))
    }
}

impl DiagnosticPublisher {
    pub(in super::super) async fn begin_diagnostic_pass(
        &self,
        uri: &Uri,
        pass: &Arc<DiagnosticPass>,
    ) {
        self.completion.lock().await.insert(
            uri.clone(),
            DiagnosticCompletionEntry {
                pass: Arc::clone(pass),
                state: DiagnosticPassState::Unavailable(DiagnosticUnavailable::Pending),
            },
        );
    }

    pub(super) async fn finish_diagnostic_pass(
        &self,
        uri: &Uri,
        pass: &Arc<DiagnosticPass>,
        state: DiagnosticPassState,
    ) {
        let mut ledger = self.completion.lock().await;
        if let Some(entry) = ledger.get_mut(uri)
            && Arc::ptr_eq(&entry.pass, pass)
        {
            entry.state = state;
        }
    }
}

impl DeliveryCtx<'_> {
    pub(in super::super) async fn deliver_suppressed_if_current(
        &self,
        reason: DiagnosticSuppression,
    ) -> bool {
        let Some(result_id) = self.delivered_non_tcl_report(Vec::new()).await else {
            return false;
        };
        if let Some(pass) = self.diagnostic_pass {
            self.diagnostic_publisher
                .finish_diagnostic_pass(
                    self.uri,
                    pass,
                    DiagnosticPassState::Suppressed { result_id, reason },
                )
                .await;
        }
        true
    }

    pub(in super::super) async fn diagnostic_pass_unavailable(
        &self,
        reason: DiagnosticUnavailable,
    ) {
        if let Some(pass) = self.diagnostic_pass {
            self.diagnostic_publisher
                .finish_diagnostic_pass(self.uri, pass, DiagnosticPassState::Unavailable(reason))
                .await;
        }
    }

    /// Complete this pass with the F5 model report delivered for `text`.
    /// Only an F5 model pass over exactly that source completes; anything else
    /// is a context mismatch, never a borrowed completion.
    pub(in super::super) async fn complete_f5_model_pass(&self, text: &str, result_id: String) {
        let Some(pass) = self.diagnostic_pass else {
            return;
        };
        if !pass.f5_model || pass.source.as_ref() != text {
            self.diagnostic_pass_unavailable(DiagnosticUnavailable::Context)
                .await;
            return;
        }
        self.diagnostic_publisher
            .finish_diagnostic_pass(self.uri, pass, DiagnosticPassState::Complete { result_id })
            .await;
    }

    pub(in super::super) async fn complete_diagnostic_pass(
        &self,
        analysis: &AnalysisResult,
        result_id: String,
    ) {
        let Some(pass) = self.diagnostic_pass else {
            return;
        };
        if !pass.matches_completed_analysis(analysis) {
            self.diagnostic_pass_unavailable(DiagnosticUnavailable::Context)
                .await;
            return;
        }
        self.diagnostic_publisher
            .finish_diagnostic_pass(self.uri, pass, DiagnosticPassState::Complete { result_id })
            .await;
    }
}

impl DiagnosticPass {
    pub(super) fn matches_source_publication(
        &self,
        db: &tcl_lsp_db::TclDatabase,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &HashMap<Uri, PublishedSourcePath>,
    ) -> bool {
        let Some(file) = self.source_file else {
            return self.f5_model;
        };
        let uri = self.path_admission.uri();
        files.get(uri) == Some(&file)
            && published.get(uri).is_some_and(|retained| {
                retained.file == file && retained.admission.as_ref() == self.path_admission.as_ref()
            })
            && file.path(db).as_deref() == self.path_admission.analyser_path()
            && file.text(db) == tcl_lexer::normalise_lone_cr(&self.source).as_ref()
            && file.dialect(db) == &self.dialect
    }

    /// The checked open-document registry this pass's input carries.
    pub(in super::super) fn checked_registry(&self) -> Option<Arc<CommandRegistry>> {
        if self.f5_model {
            return None;
        }
        self.input
            .as_ref()
            .map(|input| Arc::clone(input.borrowed_context_registry().commands()))
    }

    pub(in super::super) async fn workspace_source_input_guards(
        &self,
    ) -> Option<(
        tokio::sync::RwLockReadGuard<'_, ()>,
        tokio::sync::RwLockReadGuard<'_, u64>,
    )> {
        let inputs = self.source_inputs_gate.read().await;
        let factory = self.source_factory_generation.read().await;
        if self
            .source_inputs_epoch
            .load(std::sync::atomic::Ordering::Acquire)
            != self.inputs_epoch
            || *factory != self.class_factory_generation
        {
            return None;
        }
        Some((inputs, factory))
    }

    pub(super) fn matches_current_source(
        &self,
        source: &str,
        version: Option<i32>,
        doc: &DocumentState,
        epoch: u64,
        slot: Option<&DiagSlot>,
    ) -> Result<(), DiagnosticUnavailable> {
        if self.source.as_ref() != source
            || self.version != version
            || self.revision != doc.revision
            || self.dialect != doc.dialect
            || self.inputs_epoch != epoch
            || self.slot_generation != slot.map(|slot| slot.generation)
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        if slot.is_some_and(|slot| slot.dirty) {
            return Err(DiagnosticUnavailable::Pending);
        }
        Ok(())
    }

    pub(in super::super) fn rewrite_source_association(
        &self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<serde_json::Value> {
        if self.source.as_ref() != source || !self.matches_completed_analysis(analysis) {
            return None;
        }
        Some(serde_json::json!({
            "schema": 1,
            "purpose": "original-document-optimiser-rewrite",
            "uri": self.path_admission.uri(),
            "version": self.version,
            "source": source,
            "analysisSource": tcl_lexer::normalise_lone_cr(source),
            "revision": self.revision.to_string(),
            "inputsEpoch": self.inputs_epoch.to_string(),
            "dialectId": self.dialect,
        }))
    }

    pub(super) fn matches_completed_analysis(&self, analysis: &AnalysisResult) -> bool {
        let Some(input) = self.input.as_ref() else {
            return false;
        };
        let source = tcl_lexer::normalise_lone_cr(&self.source);
        tcl_compiler::source_graph::current_analysis(&source, analysis).is_some()
            && analysis.resolved_input.as_ref() == Some(input.as_ref())
            && analysis.resolved_registry().is_some_and(|registry| {
                registry.snapshot().semantic_key() == self.registry.semantic_key()
            })
    }
}

impl OrphanCommandDependencies {
    pub(in super::super) fn from_pass(
        uri: &Uri,
        pass: &Arc<DiagnosticPass>,
        analysis: &Arc<AnalysisResult>,
        queries: WorkspaceCommandQueries,
    ) -> Option<Self> {
        if pass.path_admission.uri() != uri || !pass.matches_completed_analysis(analysis) {
            return None;
        }
        let _inputs = pass.source_inputs_gate.try_read().ok()?;
        if pass
            .source_inputs_epoch
            .load(std::sync::atomic::Ordering::Acquire)
            != pass.inputs_epoch
        {
            return None;
        }
        Some(Self {
            uri: uri.clone(),
            source: Arc::clone(&pass.source),
            dialect: pass.dialect.clone(),
            revision: pass.revision,
            version: pass.version,
            analysis: Arc::clone(analysis),
            expected_epoch: pass.inputs_epoch,
            current_epoch: Arc::clone(&pass.source_inputs_epoch),
            inputs_gate: Arc::clone(&pass.source_inputs_gate),
            slots: pass.source_slots.clone(),
            slot_generation: pass.slot_generation,
            factory_generation: Arc::clone(&pass.source_factory_generation),
            expected_factory_generation: pass.class_factory_generation,
            queries,
        })
    }
}
