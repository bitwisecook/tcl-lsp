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

//! Reports reborrowed from the publisher ledger: each is validated against its current source before it is handed out.

use super::super::{
    Arc, Backend, DiagSlot, DocumentPathAdmission, DocumentPublication, DocumentState, HashMap,
    PullDiagEntry, Uri, WorkspaceDiagnosticSources, jsonrpc,
};
use super::{
    CurrentDiagnosticInput, DiagnosticCacheOrigin, DiagnosticCacheQuery, DiagnosticCacheReceipt,
    DiagnosticCompletionEntry, DiagnosticPass, DiagnosticReportExpectation, DiagnosticSuppression,
    DiagnosticUnavailable, PublishedDiagnosticReport,
};
use serde::Deserialize;

/// Caller-held identity for read-only reborrowing of the existing publisher ledger.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DiagnosticPublicationExpectation {
    pub(super) uri: Uri,
    pub(super) version: i32,
    pub(super) source: String,
    pub(super) dialect_id: String,
    pub(super) revision: String,
    pub(super) inputs_epoch: String,
    pub(super) result_id: String,
}

#[derive(Clone, Copy)]
struct CurrentPublishedReportBatch<'a, 'b> {
    pub(super) expected: &'a [DiagnosticReportExpectation],
    admissions: &'a [Arc<DocumentPathAdmission>],
    current: &'a [Result<CurrentDiagnosticInput, DiagnosticUnavailable>],
    configurations: &'b [tcl_lsp_db::AnalyserConfig],
    purpose: DiagnosticCacheQuery<'b>,
    epoch: u64,
    pub(super) factory_generation: u64,
    docs: &'b HashMap<Uri, DocumentState>,
    slots: &'b HashMap<Uri, DiagSlot>,
    ledger: &'b HashMap<Uri, DiagnosticCompletionEntry>,
    cache: &'b HashMap<Uri, PullDiagEntry>,
}

struct CurrentPublishedReportItem<'a, 'b> {
    pub(super) source: &'a DiagnosticReportExpectation,
    pub(super) admission: &'a Arc<DocumentPathAdmission>,
    current: &'a Result<CurrentDiagnosticInput, DiagnosticUnavailable>,
    configuration: &'b tcl_lsp_db::AnalyserConfig,
    purpose: DiagnosticCacheQuery<'b>,
    epoch: u64,
    pub(super) factory_generation: u64,
    doc: &'b DocumentState,
    slots: &'b HashMap<Uri, DiagSlot>,
    ledger: &'b HashMap<Uri, DiagnosticCompletionEntry>,
    cache: &'b HashMap<Uri, PullDiagEntry>,
    checks: &'b mut CurrentPublishedReportChecks<'a>,
}

#[derive(Clone, Copy)]
struct CurrentPublishedSuppression<'a> {
    pub(super) report: &'a PullDiagEntry,
    pub(super) receipt: &'a DiagnosticCacheReceipt,
    pub(super) source: &'a DiagnosticReportExpectation,
    purpose: DiagnosticCacheQuery<'a>,
    epoch: u64,
    doc: &'a DocumentState,
    slots: &'a HashMap<Uri, DiagSlot>,
    ledger: &'a HashMap<Uri, DiagnosticCompletionEntry>,
}

struct CurrentPublishedReportChecks<'a> {
    reports: Vec<PublishedDiagnosticReport>,
    pub(super) workspace_sources: Vec<Arc<WorkspaceDiagnosticSources>>,
    checked_policy_sources: Vec<Arc<DiagnosticPass>>,
    checked_inputs: Vec<(
        &'a CurrentDiagnosticInput,
        &'a DiagnosticReportExpectation,
        &'a Arc<DocumentPathAdmission>,
    )>,
    checked_configurations: Vec<(
        tcl_lsp_db::AnalyserConfig,
        tcl_lsp_db::DiagnosticConfigurationSnapshot,
    )>,
}

struct CurrentPublishedReportFinalization<'a, 'b> {
    pub(super) expected: &'a [DiagnosticReportExpectation],
    admissions: &'a [Arc<DocumentPathAdmission>],
    docs: &'b HashMap<Uri, DocumentState>,
    cache: &'b mut HashMap<Uri, PullDiagEntry>,
    purpose: DiagnosticCacheQuery<'b>,
    checks: CurrentPublishedReportChecks<'a>,
}

impl Backend {
    /// Reborrow the authentic publisher ledger without scheduling analysis,
    /// publishing/reissuing a report, or accepting a direct pull-cache receipt.
    pub(in super::super) async fn validate_diagnostic_publication_command(
        &self,
        arguments: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let [argument] = arguments else {
            return Err(jsonrpc::Error::invalid_params(
                "Expected one publication identity",
            ));
        };
        let expected: DiagnosticPublicationExpectation = serde_json::from_value(argument.clone())
            .map_err(|_| {
            jsonrpc::Error::invalid_params("Expected a complete publication identity")
        })?;
        let source = DiagnosticReportExpectation {
            uri: expected.uri.clone(),
            version: Some(expected.version),
            source: Arc::from(expected.source.as_str()),
        };
        let current = self
            .current_published_diagnostic_reports(&[source], DiagnosticCacheQuery::PublisherOnly)
            .await?;
        let valid = current.is_ok_and(|reports| {
            let [report] = reports.as_slice() else {
                return false;
            };
            report.suppression.is_none()
                && report.uri == expected.uri
                && report.version == Some(expected.version)
                && report.source.as_ref() == expected.source
                && report.dialect == expected.dialect_id
                && report.revision.to_string() == expected.revision
                && report.inputs_epoch.to_string() == expected.inputs_epoch
                && report.result_id == expected.result_id
        });
        if !valid {
            return Ok(Some(serde_json::json!({"status":"unavailable"})));
        }
        Ok(Some(serde_json::json!({
            "status":"current", "uri":expected.uri, "version":expected.version,
            "source":expected.source, "dialectId":expected.dialect_id,
            "revision":expected.revision, "inputsEpoch":expected.inputs_epoch,
            "resultId":expected.result_id,
        })))
    }

    /// Reborrow acknowledged publisher reports for their actual current sources.
    /// An unavailable participant refuses the batch rather than donating absence.
    pub(in super::super) async fn current_cached_diagnostic_reports(
        &self,
        uris: &[Uri],
    ) -> jsonrpc::Result<Result<Vec<PublishedDiagnosticReport>, DiagnosticUnavailable>> {
        let expected = {
            let docs = self.documents.lock("cached_diagnostic_sources").await;
            let mut expected = Vec::with_capacity(uris.len());
            for uri in uris {
                let Some(doc) = docs.get(uri) else {
                    return Ok(Err(DiagnosticUnavailable::NotOpen));
                };
                expected.push(DiagnosticReportExpectation {
                    uri: uri.clone(),
                    version: doc.version,
                    source: Arc::from(doc.raw()),
                });
            }
            expected
        };
        self.current_published_diagnostic_reports(&expected, DiagnosticCacheQuery::Reborrow)
            .await
    }

    pub(super) async fn current_published_diagnostic_reports(
        &self,
        expected: &[DiagnosticReportExpectation],
        purpose: DiagnosticCacheQuery<'_>,
    ) -> jsonrpc::Result<Result<Vec<PublishedDiagnosticReport>, DiagnosticUnavailable>> {
        if matches!(purpose, DiagnosticCacheQuery::PublishDirect(_)) && expected.len() != 1 {
            return Ok(Err(DiagnosticUnavailable::Context));
        }
        let mut admissions = Vec::with_capacity(expected.len());
        for source in expected {
            admissions.push(self.document_path_admission(&source.uri).await?);
        }
        let _inputs = self.analyser_inputs_gate.read().await;
        let epoch = self.diag_inputs_epoch();
        let factory_generation = self.class_factory_generation.read().await;
        let mut current = Vec::with_capacity(expected.len());
        let mut configurations = Vec::with_capacity(expected.len());
        for (source, admission) in expected.iter().zip(&admissions) {
            let config = self.resolved_db_config(&source.uri).await;
            configurations.push(config);
            current.push(
                self.current_diagnostic_input_parts(&source.uri, &source.source, admission)
                    .await,
            );
        }
        let docs = self.documents.lock("current_diagnostic_reports").await;
        let slots = self.diag_slots.lock().await;
        let ledger = self.diagnostic_publisher.completion.lock().await;
        let mut cache = self.pull_diag_cache.lock().await;
        let checks = match self.validate_current_published_reports(CurrentPublishedReportBatch {
            expected,
            admissions: &admissions,
            current: &current,
            configurations: &configurations,
            purpose,
            epoch,
            factory_generation: *factory_generation,
            docs: &docs,
            slots: &slots,
            ledger: &ledger,
            cache: &cache,
        }) {
            Ok(checks) => checks,
            Err(reason) => return Ok(Err(reason)),
        };
        self.finish_current_published_reports(CurrentPublishedReportFinalization {
            expected,
            admissions: &admissions,
            docs: &docs,
            cache: &mut cache,
            purpose,
            checks,
        })
    }

    fn validate_current_published_reports<'a>(
        &self,
        batch: CurrentPublishedReportBatch<'a, '_>,
    ) -> Result<CurrentPublishedReportChecks<'a>, DiagnosticUnavailable> {
        let CurrentPublishedReportBatch {
            expected,
            admissions,
            current,
            configurations,
            purpose,
            epoch,
            factory_generation,
            docs,
            slots,
            ledger,
            cache,
        } = batch;
        let mut checks = CurrentPublishedReportChecks {
            reports: Vec::with_capacity(expected.len()),
            workspace_sources: Vec::new(),
            checked_inputs: Vec::new(),
            checked_policy_sources: Vec::new(),
            checked_configurations: Vec::with_capacity(expected.len()),
        };
        for (((source, admission), current), configuration) in expected
            .iter()
            .zip(admissions.iter())
            .zip(current.iter())
            .zip(configurations.iter())
        {
            let doc = docs
                .get(&source.uri)
                .ok_or(DiagnosticUnavailable::NotOpen)?;
            self.validate_current_published_report(CurrentPublishedReportItem {
                source,
                admission,
                current,
                configuration,
                purpose,
                epoch,
                factory_generation,
                doc,
                slots,
                ledger,
                cache,
                checks: &mut checks,
            })?;
        }
        Ok(checks)
    }

    fn validate_current_published_report(
        &self,
        item: CurrentPublishedReportItem<'_, '_>,
    ) -> Result<(), DiagnosticUnavailable> {
        let CurrentPublishedReportItem {
            source,
            admission,
            current,
            configuration,
            purpose,
            epoch,
            factory_generation,
            doc,
            slots,
            ledger,
            cache,
            checks,
        } = item;
        if doc.version != source.version || doc.raw() != source.source.as_ref() {
            return Err(DiagnosticUnavailable::Stale);
        }
        if doc.publication != DocumentPublication::Indexed || self.diag_inputs_epoch() != epoch {
            return Err(DiagnosticUnavailable::Pending);
        }
        let report = match purpose {
            DiagnosticCacheQuery::PublishDirect(candidate) => candidate,
            _ => cache
                .get(&source.uri)
                .ok_or(DiagnosticUnavailable::Pending)?,
        };
        let receipt = report
            .currency
            .as_ref()
            .ok_or(DiagnosticUnavailable::Stale)?;
        let pass = &receipt.source;
        // An abstaining (non-text) source has no analysis report. The F5
        // model report applies that abstention itself, so it stays the
        // document's answer.
        if !pass.f5_model && tcl_lsp_core::source_decode::should_abstain(doc.decode_report.as_ref())
        {
            return Err(DiagnosticUnavailable::Context);
        }
        if let Some(sources) = receipt.workspace_sources.as_ref() {
            checks.workspace_sources.push(Arc::clone(sources));
        }
        let suppression = Self::current_published_suppression(CurrentPublishedSuppression {
            report,
            receipt,
            source,
            purpose,
            epoch,
            doc,
            slots,
            ledger,
        })?;
        if report.revision != pass.revision || pass.path_admission.as_ref() != admission.as_ref() {
            return Err(DiagnosticUnavailable::Stale);
        }
        checks
            .checked_configurations
            .push((*configuration, pass.configuration.clone()));
        if suppression.is_some() {
            checks.checked_policy_sources.push(Arc::clone(pass));
        }
        if suppression.is_none() && !pass.f5_model {
            let current = current.as_ref().map_err(|reason| *reason)?;
            if pass.source_file != Some(current.file)
                || pass.input.as_deref() != Some(current.input.as_ref())
                || pass.class_factory_generation != factory_generation
            {
                return Err(DiagnosticUnavailable::Stale);
            }
            checks.checked_inputs.push((current, source, admission));
        }
        checks.reports.push(PublishedDiagnosticReport {
            uri: source.uri.clone(),
            version: pass.version,
            source: Arc::clone(&pass.source),
            dialect: pass.dialect.clone(),
            revision: pass.revision,
            inputs_epoch: epoch,
            result_id: report.result_id.clone(),
            diagnostics: report.diagnostics.clone(),
            suppression,
        });
        Ok(())
    }

    fn current_published_suppression(
        input: CurrentPublishedSuppression<'_>,
    ) -> Result<Option<DiagnosticSuppression>, DiagnosticUnavailable> {
        let CurrentPublishedSuppression {
            report,
            receipt,
            source,
            purpose,
            epoch,
            doc,
            slots,
            ledger,
        } = input;
        let pass = &receipt.source;
        match receipt.origin {
            DiagnosticCacheOrigin::Publisher => {
                let entry = ledger
                    .get(&source.uri)
                    .ok_or(DiagnosticUnavailable::Pending)?;
                let (result_id, suppression) = entry.current_result(
                    &source.source,
                    source.version,
                    doc,
                    epoch,
                    slots.get(&source.uri),
                )?;
                // The authentic current publisher pass owns unavailable
                // outcomes before any older cached report is considered.
                // Success still requires this exact pass and issued ID.
                if !Arc::ptr_eq(&entry.pass, pass) || result_id != report.result_id {
                    return Err(DiagnosticUnavailable::Stale);
                }
                Ok(suppression)
            }
            DiagnosticCacheOrigin::Direct { policy }
                if !matches!(purpose, DiagnosticCacheQuery::PublisherOnly) =>
            {
                pass.matches_current_source(
                    &source.source,
                    source.version,
                    doc,
                    epoch,
                    slots.get(&source.uri),
                )?;
                Ok(policy)
            }
            DiagnosticCacheOrigin::Direct { .. } => Err(DiagnosticUnavailable::Stale),
        }
    }

    fn finish_current_published_reports(
        &self,
        finalization: CurrentPublishedReportFinalization<'_, '_>,
    ) -> jsonrpc::Result<Result<Vec<PublishedDiagnosticReport>, DiagnosticUnavailable>> {
        let CurrentPublishedReportFinalization {
            expected,
            admissions,
            docs,
            cache,
            purpose,
            checks:
                CurrentPublishedReportChecks {
                    reports,
                    workspace_sources,
                    checked_inputs,
                    checked_policy_sources,
                    checked_configurations,
                },
        } = finalization;
        // No participant can change its SourceFile/input while the complete
        // batch is projected. Never wait for DB writers under documents.
        let guards = if expected.is_empty() {
            None
        } else {
            let Ok(db) = self.db.try_lock() else {
                return Ok(Err(DiagnosticUnavailable::Pending));
            };
            // Configuration values are checked under this same final database
            // guard, including policy clears that require no prepared analysis.
            // A prior snapshot or editing-input equality cannot cover a later
            // diagnostic-only option mutation such as declared target ranges.
            for (config, retained) in checked_configurations {
                if config.diagnostic_report_configuration(&*db) != retained {
                    return Ok(Err(DiagnosticUnavailable::Stale));
                }
            }
            let Ok(files) = self.db_files.try_lock() else {
                return Ok(Err(DiagnosticUnavailable::Pending));
            };
            let published = self
                .source_path_admissions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for pass in checked_policy_sources {
                if !pass.matches_source_publication(&db, &files, &published) {
                    return Ok(Err(DiagnosticUnavailable::Stale));
                }
            }
            for (current, source, admission) in checked_inputs {
                if let Err(reason) =
                    current.matches_locked(&source.uri, admission, &db, &files, &published)
                {
                    return Ok(Err(reason));
                }
            }
            drop(published);
            Some((db, files))
        };
        let _workspace_source_index = if workspace_sources.iter().any(|sources| !sources.is_empty())
        {
            let Ok(index) = self.workspace_index.try_read() else {
                return Ok(Err(DiagnosticUnavailable::Pending));
            };
            if workspace_sources
                .iter()
                .any(|sources| !sources.matches(docs, &index))
            {
                return Ok(Err(DiagnosticUnavailable::Stale));
            }
            Some(index)
        } else {
            None
        };
        let admissions = admissions.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        let workspace_sources = workspace_sources
            .iter()
            .map(Arc::as_ref)
            .collect::<Vec<_>>();
        let mut reports = Some(reports);
        let result = WorkspaceDiagnosticSources::with_current_batch(
            &workspace_sources,
            &admissions,
            self.store.as_ref(),
            || {
                let Some(reports) = reports.take() else {
                    return Err(DiagnosticUnavailable::Stale);
                };
                if let DiagnosticCacheQuery::PublishDirect(candidate) = purpose {
                    cache.insert(expected[0].uri.clone(), candidate.clone());
                }
                Ok(reports)
            },
        )?;
        drop(guards);
        Ok(result)
    }
}
