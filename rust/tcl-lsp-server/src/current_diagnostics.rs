// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-associated completion of the existing push diagnostic pipeline.

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub(super) struct DiagnosticPass {
    path_admission: Arc<DocumentPathAdmission>,
    source: Arc<str>,
    dialect: String,
    revision: u64,
    version: Option<i32>,
    inputs_epoch: u64,
    slot_generation: Option<u64>,
    class_factory_generation: u64,
    input: Option<Arc<tcl_compiler::analyser::ResolvedAnalysisInput>>,
    registry: tcl_registry::RegistrySnapshot,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum DiagnosticUnavailable {
    Pending,
    Stale,
    Context,
    ProviderError,
    Cancelled,
    Unsupported,
    NotOpen,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum DiagnosticSuppression {
    Disabled,
    Excluded,
}

enum DiagnosticPassState {
    Unavailable(DiagnosticUnavailable),
    Complete {
        result_id: String,
    },
    Suppressed {
        result_id: String,
        reason: DiagnosticSuppression,
    },
}

pub(super) struct DiagnosticCompletionEntry {
    pass: Arc<DiagnosticPass>,
    state: DiagnosticPassState,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticSourceExpectation {
    uri: Uri,
    version: i32,
    source: String,
}

struct CurrentDiagnosticInput {
    file: tcl_lsp_db::SourceFile,
    input: Arc<tcl_compiler::analyser::ResolvedAnalysisInput>,
    database_revision: salsa::Revision,
}

struct CurrentDiagnosticRead<'a> {
    _database: TrackedMutexGuard<'a, tcl_lsp_db::TclDatabase>,
    _files: TrackedMutexGuard<'a, HashMap<Uri, tcl_lsp_db::SourceFile>>,
}

impl CurrentDiagnosticInput {
    fn lock_current<'a>(
        &self,
        backend: &'a Backend,
        uri: &Uri,
        admission: &DocumentPathAdmission,
    ) -> Result<CurrentDiagnosticRead<'a>, DiagnosticUnavailable> {
        let db = backend
            .db
            .try_lock()
            .map_err(|_| DiagnosticUnavailable::Pending)?;
        let files = backend
            .db_files
            .try_lock()
            .map_err(|_| DiagnosticUnavailable::Pending)?;
        if tcl_lsp_db::database_revision(&db) != self.database_revision
            || files.get(uri) != Some(&self.file)
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        let published = backend
            .source_path_admissions
            .lock()
            .map_err(|_| DiagnosticUnavailable::Context)?;
        let retained = published.get(uri).ok_or(DiagnosticUnavailable::Context)?;
        if retained.file != self.file
            || retained.admission.as_ref() != admission
            || self.file.path(&*db).as_deref() != admission.analyser_path()
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        drop(published);
        Ok(CurrentDiagnosticRead {
            _database: db,
            _files: files,
        })
    }
}

impl DiagnosticCompletionEntry {
    fn current_result(
        &self,
        expected: &DiagnosticSourceExpectation,
        doc: &DocumentState,
        epoch: u64,
        slot: Option<&DiagSlot>,
    ) -> Result<(&str, Option<DiagnosticSuppression>), DiagnosticUnavailable> {
        let pass = &self.pass;
        if pass.source.as_ref() != expected.source
            || pass.version != Some(expected.version)
            || pass.revision != doc.revision
            || pass.dialect != doc.dialect
            || pass.inputs_epoch != epoch
            || pass.slot_generation != slot.map(|slot| slot.generation)
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        if slot.is_some_and(|slot| slot.dirty) {
            return Err(DiagnosticUnavailable::Pending);
        }
        match &self.state {
            DiagnosticPassState::Unavailable(reason) => Err(*reason),
            DiagnosticPassState::Complete { result_id } => Ok((result_id, None)),
            DiagnosticPassState::Suppressed { result_id, reason } => Ok((result_id, Some(*reason))),
        }
    }
}

#[derive(Serialize)]
struct CurrentDiagnosticReport {
    uri: Uri,
    version: i32,
    source: String,
    revision: String,
    #[serde(rename = "inputsEpoch")]
    inputs_epoch: String,
    #[serde(rename = "resultId")]
    result_id: String,
    diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum CurrentDiagnostics {
    Complete {
        #[serde(flatten)]
        report: CurrentDiagnosticReport,
    },
    Suppressed {
        reason: DiagnosticSuppression,
        #[serde(flatten)]
        report: CurrentDiagnosticReport,
    },
    Unavailable {
        reason: DiagnosticUnavailable,
    },
}

impl DiagInputs {
    pub(super) async fn capture_diagnostic_pass(
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
        let input = if let Some(file) = job.file {
            let db = self.db.lock().await;
            if file.text(&*db) != tcl_lexer::normalise_lone_cr(&job.text).as_ref()
                || file.dialect(&*db) != &job.dialect
            {
                None
            } else {
                with_pack_hooks(|| tcl_lsp_db::document_analysis_input(&*db, file, job.config)).ok()
            }
        } else {
            None
        };
        let slot_generation = self
            .diag_slots
            .lock()
            .await
            .get(uri)
            .map(|slot| slot.generation);
        Some(Arc::new(DiagnosticPass {
            path_admission,
            source: Arc::clone(&job.text),
            dialect: job.dialect.clone(),
            revision,
            version: job.version,
            inputs_epoch: epoch,
            slot_generation,
            class_factory_generation: generation,
            input,
            registry: self.registry.snapshot(),
        }))
    }
}

impl DiagnosticPublisher {
    pub(super) async fn begin_diagnostic_pass(&self, uri: &Uri, pass: &Arc<DiagnosticPass>) {
        self.completion.lock().await.insert(
            uri.clone(),
            DiagnosticCompletionEntry {
                pass: Arc::clone(pass),
                state: DiagnosticPassState::Unavailable(DiagnosticUnavailable::Pending),
            },
        );
    }

    async fn finish_diagnostic_pass(
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
    pub(super) async fn deliver_suppressed_if_current(
        &self,
        reason: DiagnosticSuppression,
    ) -> bool {
        let Some(result_id) = self.delivered_current_report(Vec::new()).await else {
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

    pub(super) async fn diagnostic_pass_unavailable(&self, reason: DiagnosticUnavailable) {
        if let Some(pass) = self.diagnostic_pass {
            self.diagnostic_publisher
                .finish_diagnostic_pass(self.uri, pass, DiagnosticPassState::Unavailable(reason))
                .await;
        }
    }

    pub(super) async fn complete_diagnostic_pass(
        &self,
        analysis: &AnalysisResult,
        result_id: String,
    ) {
        let Some(pass) = self.diagnostic_pass else {
            return;
        };
        let Some(input) = pass.input.as_ref() else {
            self.diagnostic_pass_unavailable(DiagnosticUnavailable::Context)
                .await;
            return;
        };
        let source = tcl_lexer::normalise_lone_cr(&pass.source);
        if tcl_compiler::source_graph::current_analysis(&source, analysis).is_none()
            || analysis.resolved_input.as_ref() != Some(input.as_ref())
            || analysis.resolved_registry().is_none_or(|registry| {
                registry.snapshot().semantic_key() != pass.registry.semantic_key()
            })
        {
            self.diagnostic_pass_unavailable(DiagnosticUnavailable::Context)
                .await;
            return;
        }
        self.diagnostic_publisher
            .finish_diagnostic_pass(self.uri, pass, DiagnosticPassState::Complete { result_id })
            .await;
    }
}

impl Backend {
    /// Read a source-associated result from the actual completed push ledger.
    /// The single argument must retain the open document URI, client version
    /// and complete source. This never schedules analysis or grants execution.
    pub async fn current_diagnostics_command(
        &self,
        arguments: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let [argument] = arguments else {
            return Err(jsonrpc::Error::invalid_params(
                "Expected one diagnostic source expectation",
            ));
        };
        let expected: DiagnosticSourceExpectation = serde_json::from_value(argument.clone())
            .map_err(|_| jsonrpc::Error::invalid_params("Expected {uri, version, source}"))?;
        let answer = self.current_diagnostics(&expected).await?;
        serde_json::to_value(answer)
            .map(Some)
            .map_err(|_| jsonrpc::Error::internal_error())
    }

    async fn current_diagnostic_input(
        &self,
        expected: &DiagnosticSourceExpectation,
        admission: &DocumentPathAdmission,
    ) -> Result<CurrentDiagnosticInput, DiagnosticUnavailable> {
        let config = self.resolved_db_config(&expected.uri).await;
        let db = self.db.lock().await;
        let files = self.db_files.lock().await;
        let file = files
            .get(&expected.uri)
            .copied()
            .ok_or(DiagnosticUnavailable::Context)?;
        let published = self
            .source_path_admissions
            .lock()
            .map_err(|_| DiagnosticUnavailable::Context)?;
        let retained = published
            .get(&expected.uri)
            .ok_or(DiagnosticUnavailable::Context)?;
        if retained.file != file
            || retained.admission.as_ref() != admission
            || file.path(&*db).as_deref() != admission.analyser_path()
        {
            return Err(DiagnosticUnavailable::Context);
        }
        drop(published);
        if file.text(&*db) != tcl_lexer::normalise_lone_cr(&expected.source).as_ref() {
            return Err(DiagnosticUnavailable::Stale);
        }
        let input = with_pack_hooks(|| tcl_lsp_db::document_analysis_input(&*db, file, config))
            .map_err(|_| DiagnosticUnavailable::Context)?;
        Ok(CurrentDiagnosticInput {
            file,
            input,
            database_revision: tcl_lsp_db::database_revision(&db),
        })
    }

    async fn current_diagnostics(
        &self,
        expected: &DiagnosticSourceExpectation,
    ) -> jsonrpc::Result<CurrentDiagnostics> {
        let unavailable = |reason| Ok(CurrentDiagnostics::Unavailable { reason });
        let admission = self.document_path_admission(&expected.uri).await?;
        let _inputs = self.analyser_inputs_gate.read().await;
        let epoch = self.diag_inputs_epoch();
        let factory_generation = self.class_factory_generation.read().await;
        let current = self.current_diagnostic_input(expected, &admission).await;
        let docs = self.documents.lock("current_diagnostics").await;
        let Some(doc) = docs.get(&expected.uri) else {
            return unavailable(DiagnosticUnavailable::NotOpen);
        };
        if doc.version != Some(expected.version) || doc.raw() != expected.source {
            return unavailable(DiagnosticUnavailable::Stale);
        }
        if tcl_lsp_core::source_decode::should_abstain(doc.decode_report.as_ref()) {
            return unavailable(DiagnosticUnavailable::Context);
        }
        if doc.publication != DocumentPublication::Indexed || self.diag_inputs_epoch() != epoch {
            return unavailable(DiagnosticUnavailable::Pending);
        }
        let slots = self.diag_slots.lock().await;
        let slot = slots.get(&expected.uri);
        let ledger = self.diagnostic_publisher.completion.lock().await;
        let Some(entry) = ledger.get(&expected.uri) else {
            return unavailable(DiagnosticUnavailable::Pending);
        };
        let (result_id, suppression) = match entry.current_result(expected, doc, epoch, slot) {
            Ok(current) => current,
            Err(reason) => return unavailable(reason),
        };
        let pass = &entry.pass;
        if pass.path_admission.as_ref() != admission.as_ref() {
            return unavailable(DiagnosticUnavailable::Stale);
        }
        let cache = self.pull_diag_cache.lock().await;
        let Some(report) = cache
            .get(&expected.uri)
            .filter(|report| report.result_id == result_id && report.revision == pass.revision)
        else {
            return unavailable(DiagnosticUnavailable::Stale);
        };
        // A suppression receipt proves the delivered policy clear only.
        // Completed analysis additionally retains the actual prepared DB input.
        let current = if suppression.is_none() {
            let current = match current {
                Ok(current) => current,
                Err(reason) => return unavailable(reason),
            };
            if pass.input.as_deref() != Some(current.input.as_ref())
                || pass.class_factory_generation != *factory_generation
            {
                return unavailable(DiagnosticUnavailable::Stale);
            }
            Some(current)
        } else {
            None
        };
        let guards = match current
            .as_ref()
            .map(|current| current.lock_current(self, &expected.uri, &admission))
            .transpose()
        {
            Ok(guards) => guards,
            Err(reason) => return unavailable(reason),
        };
        admission.check_for(&expected.uri, self.store.as_ref())?;
        let report = CurrentDiagnosticReport {
            uri: expected.uri.clone(),
            version: expected.version,
            source: pass.source.to_string(),
            revision: pass.revision.to_string(),
            inputs_epoch: epoch.to_string(),
            result_id: result_id.to_owned(),
            diagnostics: report.diagnostics.clone(),
        };
        let result = match suppression {
            Some(reason) => CurrentDiagnostics::Suppressed { reason, report },
            None => CurrentDiagnostics::Complete { report },
        };
        drop(guards);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn open_source(backend: &Backend, expected: &DiagnosticSourceExpectation) {
        backend
            .documents
            .lock("diagnostic_completion_fixture")
            .await
            .insert(
                expected.uri.clone(),
                DocumentState::with_version(
                    expected.source.clone(),
                    "tcl8.6".into(),
                    expected.version,
                ),
            );
        backend
            .db_set_source(&expected.uri, &expected.source, "tcl8.6".into())
            .await
            .unwrap();
    }

    async fn run_provider(backend: &Backend, uri: &Uri) {
        let inputs = backend
            .diag_inputs(uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let job = inputs.capture_job(uri).await.unwrap();
        assert!(
            job.diagnostic_pass.is_some(),
            "fixture retains an actual source input"
        );
        assert!(run_diagnostics_core(inputs, uri, job).await);
    }

    #[tokio::test]
    async fn current_diagnostics_requires_an_actual_completed_provider_even_for_empty_reports() {
        let backend = crate::tests::test_backend();
        let mut expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-completion.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Pending
            }
        ));
        // A published/displayed empty collection supplies no provider receipt.
        backend.pull_diag_cache.lock().await.insert(
            expected.uri.clone(),
            PullDiagEntry {
                result_id: next_pull_diag_result_id(),
                revision: 0,
                diagnostics: Vec::new(),
            },
        );
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(
            matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { report } if report.diagnostics.is_empty())
        );

        // A fresh admission cannot authenticate an unretained publication.
        let admission = backend
            .document_path_admission(&expected.uri)
            .await
            .unwrap();
        let Ok(current) = backend
            .current_diagnostic_input(&expected, &admission)
            .await
        else {
            panic!("fixture retains the actual published SourceFile input");
        };
        let retained = backend
            .source_path_admissions
            .lock()
            .unwrap()
            .remove(&expected.uri)
            .unwrap();
        assert!(matches!(
            current.lock_current(&backend, &expected.uri, &admission),
            Err(DiagnosticUnavailable::Context)
        ));
        assert!(matches!(
            backend
                .current_diagnostic_input(&expected, &admission)
                .await,
            Err(DiagnosticUnavailable::Context)
        ));
        assert!(backend.current_diagnostics(&expected).await.is_err());
        backend
            .source_path_admissions
            .lock()
            .unwrap()
            .insert(expected.uri.clone(), retained);
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));

        let old = DiagnosticSourceExpectation {
            uri: expected.uri.clone(),
            version: 1,
            source: String::new(),
        };
        expected.version = 2;
        expected.source = "proc check {} {}\ncheck unexpected\n".into();
        open_source(&backend, &expected).await;
        assert!(matches!(
            backend.current_diagnostics(&old).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(
            matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { report } if !report.diagnostics.is_empty())
        );
    }

    #[tokio::test]
    async fn current_diagnostics_cannot_reborrow_a_replaced_equal_path_publication() {
        let mut backend = crate::tests::test_backend();
        let store = Arc::new(vfs::MemoryStore::new());
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-publication-aba.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        let path = "/same/diagnostic-publication-aba.tcl";
        store.upsert_uri(expected.uri.as_str(), path, Vec::new());
        backend.store = store.clone();
        open_source(&backend, &expected).await;
        run_provider(&backend, &expected.uri).await;
        let old_pass = backend
            .diagnostic_publisher
            .completion
            .lock()
            .await
            .get(&expected.uri)
            .unwrap()
            .pass
            .clone();
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
        store.upsert_uri(
            expected.uri.as_str(),
            "/different/diagnostic-publication-aba.tcl",
            Vec::new(),
        );
        store.upsert_uri(expected.uri.as_str(), path, Vec::new());
        // Republish the same bytes/version/configuration under the replacement
        // provider binding without completing another diagnostic pass.
        open_source(&backend, &expected).await;
        let admission = backend
            .document_path_admission(&expected.uri)
            .await
            .unwrap();
        assert_ne!(old_pass.path_admission.as_ref(), admission.as_ref());
        let Ok(current) = backend
            .current_diagnostic_input(&expected, &admission)
            .await
        else {
            panic!("replacement publication has an actual current input");
        };
        assert_eq!(old_pass.input.as_deref(), Some(current.input.as_ref()));
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Stale
            }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
    }

    #[tokio::test]
    async fn current_diagnostics_withdraws_changed_inputs_generation_and_source() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-input-currency.tcl").unwrap(),
            version: 4,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
        // Even an unchanged revision/epoch cannot donate a different full input.
        {
            use salsa::Setter as _;
            let config = backend.resolved_db_config(&expected.uri).await;
            let mut db = backend.db.lock().await;
            config
                .set_targets(&mut *db)
                .to(vec![("tcl".into(), "8.4-9.0".into())]);
        }
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
        *backend.class_factory_generation.write().await += 1;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), false);
        backend.invalidate_diag_inputs();
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed {
                reason: DiagnosticSuppression::Disabled,
                ..
            }
        ));
        let admission = backend
            .document_path_admission(&expected.uri)
            .await
            .unwrap();
        backend
            .documents
            .lock("completion-close")
            .await
            .remove(&expected.uri);
        backend
            .evict_diag_slot(&expected.uri, &admission)
            .await
            .unwrap();
        assert!(
            backend
                .diagnostic_publisher
                .completion
                .lock()
                .await
                .get(&expected.uri)
                .is_none()
        );
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
    }

    #[tokio::test]
    async fn cancelled_and_failed_passes_cannot_reuse_a_prior_complete_report() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-pass-outcomes.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        run_provider(&backend, &expected.uri).await;
        let inputs = backend
            .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        let pass = job.diagnostic_pass.as_ref().unwrap();
        backend
            .diagnostic_publisher
            .begin_diagnostic_pass(&expected.uri, pass)
            .await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Pending
            }
        ));
        for reason in [
            DiagnosticUnavailable::Cancelled,
            DiagnosticUnavailable::ProviderError,
        ] {
            backend
                .diagnostic_publisher
                .finish_diagnostic_pass(
                    &expected.uri,
                    pass,
                    DiagnosticPassState::Unavailable(reason),
                )
                .await;
            assert!(matches!(
                backend.current_diagnostics(&expected).await.unwrap(),
                CurrentDiagnostics::Unavailable { .. }
            ));
        }
        // A superseded pass cannot overwrite the actual newer pass state.
        let successor = inputs.capture_job(&expected.uri).await.unwrap();
        let successor = successor.diagnostic_pass.as_ref().unwrap();
        backend
            .diagnostic_publisher
            .begin_diagnostic_pass(&expected.uri, successor)
            .await;
        backend
            .diagnostic_publisher
            .finish_diagnostic_pass(
                &expected.uri,
                pass,
                DiagnosticPassState::Unavailable(DiagnosticUnavailable::ProviderError),
            )
            .await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Pending
            }
        ));
    }

    async fn await_pending_notification(backend: &Backend, uri: &Uri) {
        crate::rt::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let pending = backend
                    .diagnostic_publisher
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .pending
                    .contains_key(uri);
                if pending {
                    break;
                }
                crate::rt::yield_now().await;
            }
        })
        .await
        .expect("the actual provider must enqueue its notification");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn suppression_requires_delivered_clear_and_superseded_clear_cannot_settle() {
        use std::sync::atomic::Ordering;
        let backend = Arc::new(crate::tests::test_backend());
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-clear-receipt.tcl").unwrap(),
            version: 7,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), false);
        backend.invalidate_diag_inputs();
        // Reuse the existing held-mailbox seam: no new production instrumentation.
        backend
            .diagnostic_publisher
            .started
            .store(true, Ordering::Release);
        let runner = Arc::clone(&backend);
        let uri = expected.uri.clone();
        let first = crate::rt::spawn(async move { run_provider(&runner, &uri).await });
        await_pending_notification(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Pending
            }
        ));
        // This real mailbox replacement settles the first clear as Superseded.
        let replacement = backend.diagnostic_publisher.enqueue_assembled(
            expected.uri.clone(),
            Vec::new(),
            Some(expected.version),
            false,
            false,
        );
        first.await.unwrap();
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        let runner = Arc::clone(&backend);
        let uri = expected.uri.clone();
        let second = crate::rt::spawn(async move { run_provider(&runner, &uri).await });
        // The second actual clear must supersede this queued notification.
        assert_eq!(
            crate::rt::timeout(std::time::Duration::from_secs(5), replacement)
                .await
                .expect("the second clear must reach the mailbox")
                .unwrap(),
            DiagnosticPublishOutcome::Superseded
        );
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        backend
            .diagnostic_publisher
            .started
            .store(false, Ordering::Release);
        backend.diagnostic_publisher.ensure_started();
        second.await.unwrap();
        assert!(
            matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed { reason: DiagnosticSuppression::Disabled, report } if report.diagnostics.is_empty())
        );

        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), true);
        *backend.diagnostics_exclude.lock().await = vec!["*.tcl".into()];
        backend.invalidate_diag_inputs();
        run_provider(&backend, &expected.uri).await;
        assert!(
            matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed { reason: DiagnosticSuppression::Excluded, report } if report.diagnostics.is_empty())
        );
    }

    #[tokio::test]
    async fn issued_publication_id_cannot_be_replaced_by_the_actual_legacy_pull_handler() {
        use std::sync::atomic::Ordering;
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-issued-publication.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        let inputs = backend
            .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        let pass = job.diagnostic_pass.as_ref().unwrap();
        let analysis = backend
            .analysis_for(&expected.uri, Arc::clone(&job.text), job.dialect.clone())
            .await
            .unwrap();
        backend
            .diagnostic_publisher
            .begin_diagnostic_pass(&expected.uri, pass)
            .await;
        let delivery = DeliveryCtx {
            diagnostic_pass: Some(pass),
            client: &backend.client,
            diagnostic_publisher: &backend.diagnostic_publisher,
            documents: &backend.documents,
            store: &backend.store,
            diag_slots: &backend.diag_slots,
            pull_diag_cache: &backend.pull_diag_cache,
            closed_diag_gen: &backend.closed_diag_gen,
            uri: &expected.uri,
            currency: job.currency,
            version: job.version,
            client_supports_pull: false,
        };
        // Hold actual delivery while the legacy handler independently replaces
        // the cache. No production scheduling or publisher hook is added.
        backend
            .diagnostic_publisher
            .started
            .store(true, Ordering::Release);
        let publication = {
            let docs = backend.documents.lock("issued-publication-control").await;
            assert!(delivery.is_current(&docs).await);
            delivery.cache_and_enqueue(Vec::new()).await
        };
        assert_eq!(
            backend
                .pull_diag_cache
                .lock()
                .await
                .get(&expected.uri)
                .unwrap()
                .result_id,
            publication.result_id
        );
        backend.pull_diag_cache.lock().await.remove(&expected.uri);
        let params = serde_json::from_value(serde_json::json!({
            "textDocument": {"uri": expected.uri}
        }))
        .unwrap();
        let legacy = backend.diagnostic(params).await.unwrap();
        let legacy_id = match legacy {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(report)) => {
                report.full_document_diagnostic_report.result_id.unwrap()
            }
            _ => panic!("a real cache-miss pull must publish a new full report"),
        };
        assert_ne!(publication.result_id, legacy_id);
        backend
            .diagnostic_publisher
            .started
            .store(false, Ordering::Release);
        backend.diagnostic_publisher.ensure_started();
        assert_eq!(
            publication.receipt.await.unwrap(),
            DiagnosticPublishOutcome::Delivered
        );
        // Exercise the actual post-ack issuer facade with its own transported
        // publication ID. This control isolates publication identity; it does
        // not claim another complete provider pass was run by this direct call.
        delivery
            .complete_diagnostic_pass(&analysis, publication.result_id.clone())
            .await;
        {
            let ledger = backend.diagnostic_publisher.completion.lock().await;
            let entry = ledger.get(&expected.uri).unwrap();
            assert!(matches!(
                &entry.state,
                DiagnosticPassState::Complete { result_id }
                    if result_id == &publication.result_id && result_id != &legacy_id
            ));
        }
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Stale
            }
        ));
        // A new genuine provider pass remains able to issue current completion.
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
    }

    #[tokio::test]
    async fn actual_failed_lift_worker_cannot_issue_completed_empty_diagnostics() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-failed-lift.tcl").unwrap(),
            version: 2,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        run_provider(&backend, &expected.uri).await;
        let inputs = backend
            .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        let pass = job.diagnostic_pass.as_ref().unwrap();
        backend
            .diagnostic_publisher
            .begin_diagnostic_pass(&expected.uri, pass)
            .await;
        let analysis = backend
            .analysis_for(&expected.uri, Arc::clone(&job.text), job.dialect.clone())
            .await
            .unwrap();
        let delivery = DeliveryCtx {
            diagnostic_pass: Some(pass),
            client: &backend.client,
            diagnostic_publisher: &backend.diagnostic_publisher,
            documents: &backend.documents,
            store: &backend.store,
            diag_slots: &backend.diag_slots,
            pull_diag_cache: &backend.pull_diag_cache,
            closed_diag_gen: &backend.closed_diag_gen,
            uri: &expected.uri,
            currency: job.currency,
            version: job.version,
            client_supports_pull: false,
        };
        let failed =
            crate::rt::spawn_blocking(|| -> Vec<tower_lsp_server::ls_types::Diagnostic> {
                panic!("actual diagnostic lift worker failure control")
            })
            .await;
        assert!(failed.is_err());
        assert!(
            publish_diagnostics_result(
                &delivery,
                &backend.workspace_index,
                &backend.rehomed_source_seeds,
                &backend.rehoming_gate,
                &analysis,
                (Ok(()), failed),
                PublishTiming {
                    started: crate::rt::Instant::now(),
                    uri_str: expected.uri.as_str(),
                    line_count: 1
                }
            )
            .await
        );
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::ProviderError
            }
        ));
    }

    #[tokio::test]
    async fn actual_foreign_provider_registry_cannot_borrow_completed_analysis_currency() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-provider-registry.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
        let mut inputs = backend
            .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let foreign = inputs
            .registry
            .project_for_profile(tcl_lsp_core::profile_for_dialect("tcl8.4"));
        assert_ne!(
            inputs.registry.snapshot().semantic_key(),
            foreign.snapshot().semantic_key()
        );
        inputs.registry = Arc::new(foreign);
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        assert!(run_diagnostics_core(inputs, &expected.uri, job).await);
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Context
            }
        ));
    }

    #[tokio::test]
    async fn delivered_policy_suppression_does_not_claim_missing_analysis_context() {
        use salsa::Setter as _;
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-disabled-missing-context.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        {
            let config = backend.resolved_db_config(&expected.uri).await;
            let mut db = backend.db.lock().await;
            config.set_spec_pack_key(&mut *db).to(u64::MAX - 390_037);
        }
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), false);
        backend.invalidate_diag_inputs();
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed {
                reason: DiagnosticSuppression::Disabled,
                ..
            }
        ));
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), true);
        backend.invalidate_diag_inputs();
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
    }
}
