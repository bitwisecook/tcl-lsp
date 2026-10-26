// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-bound whole-document results; transformation safety stays with each worker.


#[cfg(test)]
use super::Analyser;
use super::current_diagnostics::CurrentDiagnosticInput;
use super::{
    AnalysisResult, Arc, Backend, DocumentPathAdmission, DocumentSourceResult, DocumentState,
    FromStr, HashSet, NonAsciiMode, PolicyLayers, ResourceAnalyserInputs, Uri, jsonrpc,
};
use std::collections::VecDeque;
use tcl_lsp_core::wire_session::EditorWireSession;

pub(super) enum TransformRequest {
    Legacy(Uri),
    Snapshot {
        uri: Uri,
        version: i32,
        source: String,
    },
}

impl TransformRequest {
    pub(super) fn parse(value: Option<&serde_json::Value>) -> Option<Self> {
        let value = value?;
        if let Some(uri) = value.as_str() {
            return Some(Self::Legacy(Uri::from_str(uri).ok()?));
        }
        let object = value.as_object()?;
        Some(Self::Snapshot {
            uri: Uri::from_str(object.get("uri")?.as_str()?).ok()?,
            version: i32::try_from(object.get("version")?.as_i64()?).ok()?,
            source: object.get("source")?.as_str()?.to_owned(),
        })
    }

    pub(super) fn uri(&self) -> &Uri {
        match self {
            Self::Legacy(uri) | Self::Snapshot { uri, .. } => uri,
        }
    }
}

/// The actual input and complete source used by one existing worker pass.
/// A document worker must join the captured `SourceFile` input. The separately
/// configured safe-fix purpose instead retains its actual parameter producer.
pub(super) struct TransformWorkerSource {
    source: Arc<str>,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    registry: tcl_registry::RegistrySnapshot,
    purpose: TransformWorkerPurpose,
}

enum TransformWorkerPurpose {
    Document,
    FixAll(Arc<FixAllWorkerConfiguration>),
}

/// The actual old safe-fix constructor arguments. This is its parameter owner,
/// not the document analyser's input or a reconstructed profile/Registry.
pub(super) struct FixAllWorkerConfiguration {
    pub(super) disabled: HashSet<String>,
    pub(super) na_mode: NonAsciiMode,
    pub(super) extra: HashSet<String>,
    pub(super) packs: Arc<tcl_spectcl::PackSet>,
    pub(super) resource: ResourceAnalyserInputs,
    pub(super) policy: PolicyLayers,
    pub(super) dialect: String,
    pub(super) decode_report: Option<tcl_lsp_core::source_decode::DecodeReport>,
}

impl FixAllWorkerConfiguration {
    /// Invoke the existing configured analyser once, under these actual
    /// retained arguments. No additional prepare/resolve walk is introduced.
    pub(super) fn analyse(
        self: &Arc<Self>,
        source: &str,
    ) -> (AnalysisResult, Option<TransformWorkerSource>) {
        let mut analyser = Backend::configured_analyser(
            self.disabled.clone(),
            self.na_mode,
            self.extra.clone(),
            self.packs.key,
            self.resource.clone(),
        );
        let analysis = analyser.analyse(source, &self.dialect);
        let worker = TransformWorkerSource::from_analysis(
            source,
            &analysis,
            TransformWorkerPurpose::FixAll(Arc::clone(self)),
        );
        (analysis, worker)
    }
}

impl TransformWorkerSource {
    fn from_analysis(
        source: &str,
        analysis: &AnalysisResult,
        purpose: TransformWorkerPurpose,
    ) -> Option<Self> {
        tcl_compiler::source_graph::current_analysis(source, analysis)?;
        Some(Self {
            source: Arc::from(source),
            input: analysis.resolved_input.clone()?,
            registry: analysis.resolved_registry()?.snapshot(),
            purpose,
        })
    }

    pub(super) fn for_document(
        source: &str,
        analysis: &AnalysisResult,
        expected: Option<&TransformSource>,
    ) -> Option<Self> {
        let worker = Self::from_analysis(source, analysis, TransformWorkerPurpose::Document)?;
        if let Some(expected) = expected {
            worker.matches_document(expected).then_some(worker)
        } else {
            // Legacy URI results do not issue a source-associated handle.
            Some(worker)
        }
    }

    fn matches_document(&self, expected: &TransformSource) -> bool {
        matches!(self.purpose, TransformWorkerPurpose::Document)
            && self.source.as_ref() == tcl_lexer::normalise_lone_cr(&expected.source).as_ref()
            && self.input == *expected.input.input
            && self.registry.semantic_key()
                == expected
                    .input
                    .input
                    .borrowed_context_registry()
                    .commands()
                    .snapshot()
                    .semantic_key()
    }
}

pub(super) struct TransformSource {
    uri: Uri,
    version: i32,
    source: Arc<str>,
    revision: u64,
    dialect: String,
    language_id: String,
    decode_report: Option<tcl_lsp_core::source_decode::DecodeReport>,
    admission: Arc<DocumentPathAdmission>,
    input: CurrentDiagnosticInput,
    configuration: tcl_lsp_db::DiagnosticConfigurationSnapshot,
    policy: PolicyLayers,
    epoch: u64,
    factory_generation: u64,
    overlay_epoch: u64,
    evaluator_epoch: u64,
}

struct TransformResult {
    source: Arc<TransformSource>,
    replacement: String,
    workers: Arc<[TransformWorkerSource]>,
}

/// A bounded, allocation-local ledger. Handles only recover their original
/// result; caller-provided labels or replacement bytes cannot issue authority.
pub(super) struct TransformLedger {
    issuer: Option<EditorWireSession>,
    next: u64,
    entries: VecDeque<(String, Arc<TransformResult>)>,
    source_contexts: VecDeque<(String, Arc<TransformSource>)>,
}

impl Default for TransformLedger {
    fn default() -> Self {
        Self {
            // An independently allocated transport issuer for this ledger.
            // Entropy failure withholds new bound results, never a donor ID.
            issuer: EditorWireSession::allocate(),
            next: 0,
            entries: VecDeque::new(),
            source_contexts: VecDeque::new(),
        }
    }
}

impl TransformLedger {
    fn insert(&mut self, result: TransformResult) -> Option<(String, EditorWireSession)> {
        let issuer = self.issuer?;
        self.next = self.next.checked_add(1)?;
        let id = self.next.to_string();
        if self.entries.len() == 128 {
            self.entries.pop_front();
        }
        self.entries.push_back((id.clone(), Arc::new(result)));
        Some((id, issuer))
    }
}

/// Metadata publication is separate from a worker-issued edit result. Only
/// the transform variant checks/retains workers or can issue edit receipts.
#[derive(Clone, Copy)]
enum SourcePublication<'a> {
    Context,
    Transform(&'a [TransformWorkerSource]),
}

fn stale_transform() -> jsonrpc::Error {
    jsonrpc::Error {
        code: jsonrpc::ErrorCode::ContentModified,
        message: "Document transform source or analysis context changed".into(),
        data: None,
    }
}

fn transform_document_changed(
    doc: &DocumentState,
    source: &TransformSource,
    purpose: &SourcePublication<'_>,
) -> bool {
    (matches!(purpose, SourcePublication::Context) && doc.dialect_resolution_pending())
        || doc.version != Some(source.version)
        || doc.revision != source.revision
        || doc.raw() != source.source.as_ref()
        || doc.dialect != source.dialect
        || doc.language_id != source.language_id
        || doc.decode_report != source.decode_report
        || doc.path_admission.as_deref() != Some(source.admission.as_ref())
}

impl Backend {
    pub(super) async fn capture_transform_source(
        &self,
        request: &TransformRequest,
        doc: &DocumentState,
    ) -> jsonrpc::Result<Option<Arc<TransformSource>>> {
        let TransformRequest::Snapshot {
            uri,
            version,
            source,
        } = request
        else {
            return Ok(None);
        };
        if doc.version != Some(*version) || doc.raw() != source.as_str() {
            return Err(stale_transform());
        }
        let _inputs = self.analyser_inputs_gate.read().await;
        let epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let admission = doc.path_admission.clone().ok_or_else(stale_transform)?;
        admission.check_for(uri, self.store.as_ref())?;
        let input = self
            .current_diagnostic_input_parts(uri, source, &admission)
            .await
            .map_err(|_| stale_transform())?;
        let config = self.resolved_db_config(uri).await;
        let configuration = config.diagnostic_report_configuration(&*self.db.lock().await);
        let policy = self.resolved_policy_layers(uri).await;
        Ok(Some(Arc::new(TransformSource {
            uri: uri.clone(),
            version: *version,
            source: Arc::from(source.as_str()),
            revision: doc.revision,
            dialect: doc.dialect.clone(),
            language_id: doc.language_id.clone(),
            decode_report: doc.decode_report,
            admission,
            input,
            configuration,
            policy,
            epoch,
            factory_generation,
            overlay_epoch: tcl_registry::overlay_epoch(),
            evaluator_epoch: tcl_registry::pack_hooks::evaluator_epoch(),
        })))
    }

    /// Reborrow only source/input/publication metadata. This does not consult
    /// diagnostic completion or claim the worker's result is equivalent.
    async fn with_current_transform_source<T>(
        &self,
        source: &TransformSource,
        purpose: SourcePublication<'_>,
        mut publish: impl FnMut() -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        let _inputs = self.analyser_inputs_gate.read().await;
        let generation = self.class_factory_generation.read().await;
        let config = self.resolved_db_config(&source.uri).await;
        let policy = self.resolved_policy_layers(&source.uri).await;
        let current = self
            .current_diagnostic_input_parts(&source.uri, &source.source, &source.admission)
            .await;
        let Ok(current) = current else {
            return Ok(None);
        };
        if current.input != source.input.input || policy != source.policy {
            return Ok(None);
        }
        if let SourcePublication::Transform(workers) = purpose {
            let Some(first) = workers.first() else {
                return Ok(None);
            };
            match &first.purpose {
                TransformWorkerPurpose::Document => {
                    if workers.len() != 1 || !first.matches_document(source) {
                        return Ok(None);
                    }
                }
                TransformWorkerPurpose::FixAll(configuration) => {
                    let na_mode = *self.non_ascii_mode.lock().await;
                    let extra = self
                        .extra_commands
                        .lock()
                        .await
                        .iter()
                        .cloned()
                        .collect::<HashSet<_>>();
                    let packs = self.spec_packs().await;
                    let resource = self.resource_analyser_inputs(Some(&source.uri)).await;
                    if first.source.as_ref() != tcl_lexer::normalise_lone_cr(&source.source).as_ref()
                        || configuration.dialect != source.dialect
                        || configuration.decode_report != source.decode_report
                        || configuration.policy != policy
                        || configuration.disabled != policy.production_skip()
                        || configuration.na_mode != na_mode
                        || configuration.extra != extra
                        || !Arc::ptr_eq(&configuration.packs, &packs)
                        || configuration.resource != resource
                        || workers.iter().any(|worker| {
                            !matches!(&worker.purpose,
                            TransformWorkerPurpose::FixAll(other) if Arc::ptr_eq(other, configuration))
                        })
                    {
                        return Ok(None);
                    }
                }
            }
        }
        let documents = self
            .documents
            .lock("document_transform_current_source")
            .await;
        let Some(doc) = documents.get(&source.uri) else {
            return Ok(None);
        };
        if transform_document_changed(doc, source, &purpose)
            || self.diag_inputs_epoch() != source.epoch
            || *generation != source.factory_generation
            || tcl_registry::overlay_epoch() != source.overlay_epoch
            || tcl_registry::pack_hooks::evaluator_epoch() != source.evaluator_epoch
        {
            return Ok(None);
        }
        let Ok(db) = self.db.try_lock() else {
            return Ok(None);
        };
        let Ok(files) = self.db_files.try_lock() else {
            return Ok(None);
        };
        let published = self
            .source_path_admissions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if config.diagnostic_report_configuration(&*db) != source.configuration
            || source
                .input
                .matches_locked(&source.uri, &source.admission, &db, &files, &published)
                .is_err()
            || current
                .matches_locked(&source.uri, &source.admission, &db, &files, &published)
                .is_err()
        {
            return Ok(None);
        }
        DocumentPathAdmission::with_current(
            &[source.admission.as_ref()],
            self.store.as_ref(),
            &mut publish,
        )
    }

    pub(super) async fn finish_document_transform(
        &self,
        source: Option<Arc<TransformSource>>,
        workers: Vec<TransformWorkerSource>,
        value: serde_json::Value,
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(source) = source else {
            return Ok(Some(value));
        };
        if workers.is_empty() {
            return Err(stale_transform());
        }
        let replacement = value
            .get("source")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(stale_transform)?
            .to_owned();
        let result = Arc::clone(&source);
        let workers: Arc<[TransformWorkerSource]> = workers.into();
        let mut replacement = Some(replacement);
        let mut value = Some(value);
        self.with_current_transform_source(&source, SourcePublication::Transform(&workers), || {
            let mut value = value.take()?;
            let (id, issuer) = self.document_transforms.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(TransformResult { source: Arc::clone(&result), replacement: replacement.take()?, workers: Arc::clone(&workers) })?;
            value["editReceipt"] = serde_json::json!({
                "id": id, "issuer": issuer, "uri": source.uri.as_str(), "version": source.version, "source": source.source.as_ref(),
            });
            Some(value)
        }).await?.map(Some).ok_or_else(stale_transform)
    }

    /// Receive the actual open-document sync publication, rather than asking a
    /// host to invent an LSP version from its editor modification stamp. The
    /// supplied version must be the actual selected SDK sync publication.
    pub(super) async fn capture_document_source_context_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri) = args
            .first()
            .and_then(serde_json::Value::as_str)
            .and_then(|value| Uri::from_str(value).ok())
        else {
            return Ok(None);
        };
        let Some(expected_version) = args
            .get(1)
            .and_then(serde_json::Value::as_i64)
            .and_then(|value| i32::try_from(value).ok())
        else {
            return Ok(None);
        };
        let Some(expected) = args.get(2).and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let Some(version) = doc.version else {
            return Ok(None);
        };
        if version != expected_version || doc.raw() != expected || doc.dialect_resolution_pending()
        {
            return Ok(None);
        }
        let request = TransformRequest::Snapshot {
            uri,
            version,
            source: expected.to_owned(),
        };
        let Some(source) = self.capture_transform_source(&request, &doc).await? else {
            return Ok(None);
        };
        self.with_current_transform_source(&source, SourcePublication::Context, || {
            let mut ledger = self
                .document_transforms
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let issuer = ledger.issuer?;
            ledger.next = ledger.next.checked_add(1)?;
            let id = ledger.next.to_string();
            if ledger.source_contexts.len() == 128 {
                ledger.source_contexts.pop_front();
            }
            ledger
                .source_contexts
                .push_back((id.clone(), Arc::clone(&source)));
            Some(serde_json::json!({
                "schema": 1, "purpose": "current-document-source-context",
                "id": id, "issuer": issuer, "uri": source.uri.as_str(),
                "version": source.version, "source": source.source.as_ref(),
                "analysisSource": tcl_lexer::normalise_lone_cr(&source.source).as_ref(),
                "dialect": source.dialect, "inputsEpoch": source.epoch,
            }))
        })
        .await
        .map_err(Into::into)
    }

    /// This metadata-only receipt never recovers a transform result or edit.
    pub(super) async fn validate_document_source_context_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let current = self.current_document_source_context(args.first()).await?;
        Ok(Some(serde_json::Value::Bool(current)))
    }

    async fn current_document_source_context(
        &self,
        value: Option<&serde_json::Value>,
    ) -> jsonrpc::Result<bool> {
        let Some(value) = value else {
            return Ok(false);
        };
        if value.get("schema").and_then(serde_json::Value::as_u64) != Some(1)
            || value.get("purpose").and_then(serde_json::Value::as_str)
                != Some("current-document-source-context")
        {
            return Ok(false);
        }
        let Some(id) = value.get("id").and_then(serde_json::Value::as_str) else {
            return Ok(false);
        };
        let Some(issuer) = value
            .get("issuer")
            .and_then(|value| serde_json::from_value::<EditorWireSession>(value.clone()).ok())
        else {
            return Ok(false);
        };
        let source = {
            let ledger = self
                .document_transforms
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if ledger.issuer != Some(issuer) {
                return Ok(false);
            }
            ledger
                .source_contexts
                .iter()
                .find(|(key, _)| key == id)
                .map(|(_, source)| Arc::clone(source))
        };
        let Some(source) = source else {
            return Ok(false);
        };
        if value.get("uri").and_then(serde_json::Value::as_str) != Some(source.uri.as_str())
            || value.get("version").and_then(serde_json::Value::as_i64)
                != Some(i64::from(source.version))
            || value.get("source").and_then(serde_json::Value::as_str)
                != Some(source.source.as_ref())
            || value
                .get("analysisSource")
                .and_then(serde_json::Value::as_str)
                != Some(tcl_lexer::normalise_lone_cr(&source.source).as_ref())
            || value.get("dialect").and_then(serde_json::Value::as_str)
                != Some(source.dialect.as_str())
            || value.get("inputsEpoch").and_then(serde_json::Value::as_u64) != Some(source.epoch)
        {
            return Ok(false);
        }
        Ok(self
            .with_current_transform_source(&source, SourcePublication::Context, || Some(true))
            .await?
            .unwrap_or(false))
    }

    /// Validate the original result after the client's dialog and immediately
    /// before its snapshot-checked edit. No replacement is recomputed here.
    pub(super) async fn validate_document_transform_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(request) = args.first().and_then(serde_json::Value::as_object) else {
            return Ok(Some(serde_json::Value::Bool(false)));
        };
        let Some(id) = request.get("id").and_then(serde_json::Value::as_str) else {
            return Ok(Some(serde_json::Value::Bool(false)));
        };
        let Some(issuer) = request
            .get("issuer")
            .and_then(|value| serde_json::from_value::<EditorWireSession>(value.clone()).ok())
        else {
            return Ok(Some(serde_json::Value::Bool(false)));
        };
        let result = {
            let ledger = self
                .document_transforms
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            // Check original ledger/session before even considering the local
            // numeric handle; another Backend can have the same first ID.
            if ledger.issuer != Some(issuer) {
                return Ok(Some(serde_json::Value::Bool(false)));
            }
            ledger
                .entries
                .iter()
                .find(|(key, _)| key == id)
                .map(|(_, result)| Arc::clone(result))
        };
        let Some(result) = result else {
            return Ok(Some(serde_json::Value::Bool(false)));
        };
        let matches = request.get("uri").and_then(serde_json::Value::as_str)
            == Some(result.source.uri.as_str())
            && request.get("version").and_then(serde_json::Value::as_i64)
                == Some(i64::from(result.source.version))
            && request.get("source").and_then(serde_json::Value::as_str)
                == Some(result.source.source.as_ref())
            && request
                .get("replacement")
                .and_then(serde_json::Value::as_str)
                == Some(result.replacement.as_str());
        let current = if matches {
            self.with_current_transform_source(
                &result.source,
                SourcePublication::Transform(&result.workers),
                || Some(true),
            )
            .await?
            .unwrap_or(false)
        } else {
            false
        };
        Ok(Some(serde_json::Value::Bool(current)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn open(backend: &Backend, uri: &Uri, source: &str, version: i32) {
        let admission = backend.document_path_admission(uri).await.unwrap();
        backend.documents.lock("transform_fixture").await.insert(
            uri.clone(),
            DocumentState::with_version(source.to_owned(), "tcl8.6".into(), version)
                .with_path_admission(admission),
        );
        backend
            .db_set_source(uri, source, "tcl8.6".into())
            .await
            .unwrap();
    }

    fn snapshot(uri: &Uri, source: &str, version: i32) -> serde_json::Value {
        serde_json::json!({ "uri": uri.as_str(), "version": version, "source": source })
    }

    async fn validate(backend: &Backend, result: &serde_json::Value) -> bool {
        let mut request = result["editReceipt"].clone();
        request["replacement"] = result["source"].clone();
        backend
            .validate_document_transform_command(&[request])
            .await
            .unwrap()
            == Some(serde_json::Value::Bool(true))
    }

    #[tokio::test]
    async fn original_document_transforms_keep_actual_result_and_refuse_changed_source() {
        let backend = crate::tests::test_backend();
        let uri = Uri::from_str("file:///source-bound-transform.tcl").unwrap();
        let source = "# original\r\nputs   hello\r\n";
        open(&backend, &uri, source, 7).await;
        let result = backend
            .minify_document_command(&[snapshot(&uri, source, 7)])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result["source"], "puts hello");
        assert_eq!(result["editReceipt"]["source"], source);
        assert!(validate(&backend, &result).await);
        let mut forged = result.clone();
        forged["source"] = serde_json::json!("puts different");
        assert!(!validate(&backend, &forged).await);
        open(&backend, &uri, "puts newer", 8).await;
        assert!(!validate(&backend, &result).await);
        assert!(
            backend
                .minify_document_command(&[snapshot(&uri, source, 7)])
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn original_document_transforms_bind_each_worker_and_leave_legacy_results_unbound() {
        let backend = crate::tests::test_backend();
        let uri = Uri::from_str("file:///source-bound-transform-workers.tcl").unwrap();
        let source = "puts hello\n";
        open(&backend, &uri, source, 2).await;
        let request = snapshot(&uri, source, 2);
        let minified = backend
            .minify_document_command(std::slice::from_ref(&request))
            .await
            .unwrap()
            .unwrap();
        assert!(validate(&backend, &minified).await);
        let optimised = backend
            .optimise_document_command(std::slice::from_ref(&request))
            .await
            .unwrap()
            .unwrap();
        assert!(validate(&backend, &optimised).await);
        let fixed = backend
            .fix_all_safe_issues_command(std::slice::from_ref(&request))
            .await
            .unwrap()
            .unwrap();
        assert!(validate(&backend, &fixed).await);
        let legacy = backend
            .minify_document_command(&[serde_json::json!(uri.as_str())])
            .await
            .unwrap()
            .unwrap();
        assert!(legacy.get("editReceipt").is_none());
        assert!(
            TransformRequest::parse(Some(
                &serde_json::json!({ "uri": uri.as_str(), "source": source })
            ))
            .is_none()
        );
    }

    #[tokio::test]
    async fn original_document_transforms_refuse_configuration_and_reopened_source() {
        let backend = crate::tests::test_backend();
        let uri = Uri::from_str("file:///source-bound-transform-config.tcl").unwrap();
        let source = "puts hello\n";
        open(&backend, &uri, source, 3).await;
        let result = backend
            .minify_document_command(&[snapshot(&uri, source, 3)])
            .await
            .unwrap()
            .unwrap();
        assert!(validate(&backend, &result).await);
        let epoch = backend.diag_inputs_epoch();
        {
            use salsa::Setter as _;
            let config = backend.resolved_db_config(&uri).await;
            let mut db = backend.db.lock().await;
            config
                .set_targets(&mut *db)
                .to(vec![("tcl".into(), "8.4-9.0".into())]);
        }
        assert_eq!(backend.diag_inputs_epoch(), epoch);
        assert!(!validate(&backend, &result).await);
        let current = backend
            .minify_document_command(&[snapshot(&uri, source, 3)])
            .await
            .unwrap()
            .unwrap();
        assert!(validate(&backend, &current).await);
        backend
            .documents
            .lock("transform_close_fixture")
            .await
            .remove(&uri);
        backend.db_remove_source(&uri).await;
        open(&backend, &uri, source, 3).await;
        assert!(!validate(&backend, &current).await);
    }

    #[tokio::test]
    async fn original_document_transform_receipts_cannot_cross_fresh_backend_issuers() {
        let first = crate::tests::test_backend();
        let second = crate::tests::test_backend();
        let uri = Uri::from_str("file:///source-bound-transform-issuer.tcl").unwrap();
        let source = "# source\nputs   hello\n";
        open(&first, &uri, source, 1).await;
        open(&second, &uri, source, 1).await;
        let request = snapshot(&uri, source, 1);
        let a = first
            .minify_document_command(std::slice::from_ref(&request))
            .await
            .unwrap()
            .unwrap();
        let b = second
            .minify_document_command(&[request])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(a["source"], b["source"]);
        assert_eq!(a["editReceipt"]["uri"], b["editReceipt"]["uri"]);
        assert_eq!(a["editReceipt"]["version"], b["editReceipt"]["version"]);
        assert_eq!(a["editReceipt"]["source"], b["editReceipt"]["source"]);
        assert_eq!(
            a["editReceipt"]["id"], b["editReceipt"]["id"],
            "both genuine first local handles"
        );
        assert_ne!(a["editReceipt"]["issuer"], b["editReceipt"]["issuer"]);
        assert!(validate(&first, &a).await);
        assert!(validate(&second, &b).await);
        assert!(!validate(&second, &a).await);
        assert!(!validate(&first, &b).await);
        let mut missing = a.clone();
        missing["editReceipt"]
            .as_object_mut()
            .unwrap()
            .remove("issuer");
        assert!(!validate(&first, &missing).await);
        let mut malformed = a.clone();
        malformed["editReceipt"]["issuer"] = serde_json::json!([256]);
        assert!(!validate(&first, &malformed).await);
    }
}

#[cfg(test)]
mod worker_purpose_tests {
    use super::*;

    #[tokio::test]
    async fn document_transform_worker_requires_the_actual_complete_original_input() {
        let backend = crate::tests::test_backend();
        let uri = Uri::from_str("file:///source-bound-worker-input.tcl").unwrap();
        let source = "puts hello\n";
        let admission = backend.document_path_admission(&uri).await.unwrap();
        backend
            .documents
            .lock("worker_purpose_fixture")
            .await
            .insert(
                uri.clone(),
                DocumentState::with_version(source.to_owned(), "tcl8.6".into(), 1)
                    .with_path_admission(admission),
            );
        backend
            .db_set_source(&uri, source, "tcl8.6".into())
            .await
            .unwrap();
        let request = TransformRequest::parse(Some(&serde_json::json!({
            "uri": uri.as_str(), "version": 1, "source": source,
        })))
        .unwrap();
        let doc = backend.read_local_document(&uri).await.unwrap().unwrap();
        let retained = backend
            .capture_transform_source(&request, &doc)
            .await
            .unwrap()
            .unwrap();
        let input = retained.input.input.as_ref().clone();
        let genuine = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "tcl8.6");
        assert!(TransformWorkerSource::for_document(source, &genuine, Some(&retained)).is_some());
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let foreign = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        let other = Analyser::new()
            .with_resolved_input(foreign.clone())
            .analyse(source, "tcl8.6");
        assert!(tcl_compiler::source_graph::current_analysis(source, &other).is_some());
        assert_eq!(other.resolved_input.as_ref(), Some(&foreign));
        assert_ne!(other.resolved_input.as_ref(), Some(&input));
        assert!(TransformWorkerSource::for_document(source, &other, Some(&retained)).is_none());
    }

    #[tokio::test]
    async fn safe_fix_transform_retains_its_actual_configured_worker_parameters() {
        let backend = crate::tests::test_backend();
        let uri = Uri::from_str("file:///source-bound-worker-config.tcl").unwrap();
        let source = "puts hello\n";
        let admission = backend.document_path_admission(&uri).await.unwrap();
        backend
            .documents
            .lock("configured_worker_fixture")
            .await
            .insert(
                uri.clone(),
                DocumentState::with_version(source.to_owned(), "tcl8.6".into(), 1)
                    .with_path_admission(admission),
            );
        backend
            .db_set_source(&uri, source, "tcl8.6".into())
            .await
            .unwrap();
        let request = serde_json::json!({ "uri": uri.as_str(), "version": 1, "source": source });
        let result = backend
            .fix_all_safe_issues_command(std::slice::from_ref(&request))
            .await
            .unwrap()
            .unwrap();
        let mut validation = result["editReceipt"].clone();
        validation["replacement"] = result["source"].clone();
        assert_eq!(
            backend
                .validate_document_transform_command(std::slice::from_ref(&validation))
                .await
                .unwrap(),
            Some(serde_json::json!(true))
        );
        let epoch = backend.diag_inputs_epoch();
        // This genuine worker parameter is independent of SourceFile input:
        // holding the latter alone cannot cover a changed configured pass.
        backend
            .extra_commands
            .lock()
            .await
            .push("configured_worker_command".into());
        assert_eq!(backend.diag_inputs_epoch(), epoch);
        assert_eq!(
            backend
                .validate_document_transform_command(&[validation])
                .await
                .unwrap(),
            Some(serde_json::json!(false))
        );
        let fresh = backend
            .fix_all_safe_issues_command(&[request])
            .await
            .unwrap()
            .unwrap();
        let mut current = fresh["editReceipt"].clone();
        current["replacement"] = fresh["source"].clone();
        assert_eq!(
            backend
                .validate_document_transform_command(&[current])
                .await
                .unwrap(),
            Some(serde_json::json!(true))
        );
    }
}

#[cfg(test)]
mod source_context_tests;
