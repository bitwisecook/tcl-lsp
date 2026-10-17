// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Backend-owned publication of exact live Studio source images.

use super::*;
use tcl_lsp_core::wire_session::EditorWireSession;
#[cfg(test)]
use tower_lsp_server::ls_types::WorkDoneProgressParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StudioSourceLease {
    issuer: EditorWireSession,
    token: u32,
}

#[derive(serde::Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
enum StudioSourceRequest {
    Acquire {
        uri: Uri,
    },
    Publish {
        uri: Uri,
        lease: StudioSourceLease,
        revision: u32,
        source: String,
    },
    Retire {
        uri: Uri,
        lease: StudioSourceLease,
    },
}

struct StudioSource {
    admission: Arc<DocumentPathAdmission>,
    revision: u32,
    source: Option<Arc<str>>,
}

pub(super) struct StudioSources {
    issuer: Option<EditorWireSession>,
    next: u32,
    closed: bool,
    generation: u64,
    published_generation: u64,
    published: HashMap<u32, u32>,
    retired: HashMap<u32, Uri>,
    rows: HashMap<u32, StudioSource>,
}

impl Default for StudioSources {
    fn default() -> Self {
        Self {
            issuer: EditorWireSession::allocate(),
            next: 0,
            closed: false,
            generation: 0,
            published_generation: 0,
            published: HashMap::new(),
            retired: HashMap::new(),
            rows: HashMap::new(),
        }
    }
}

impl StudioSources {
    fn advance(&mut self) -> Option<()> {
        self.generation = self.generation.checked_add(1)?;
        Some(())
    }

    fn acquire(&mut self, admission: Arc<DocumentPathAdmission>) -> Option<StudioSourceLease> {
        if self.closed {
            return None;
        }
        let issuer = self.issuer?;
        let token = self.next.checked_add(1)?;
        self.advance()?;
        self.next = token;
        let replaced = self
            .rows
            .iter()
            .filter(|(_, row)| {
                row.admission.uri() == admission.uri() || row.admission.path() == admission.path()
            })
            .map(|(token, _)| *token)
            .collect::<Vec<_>>();
        for token in replaced {
            if let Some(row) = self.rows.remove(&token) {
                self.retired.insert(token, row.admission.uri().clone());
            }
        }
        self.rows.insert(
            token,
            StudioSource {
                admission,
                revision: 0,
                source: None,
            },
        );
        Some(StudioSourceLease { issuer, token })
    }

    fn selected(&self, lease: StudioSourceLease, uri: &Uri) -> Option<&StudioSource> {
        if self.closed || self.issuer != Some(lease.issuer) {
            return None;
        }
        self.rows
            .get(&lease.token)
            .filter(|row| row.admission.uri() == uri)
    }

    fn publish(
        &mut self,
        lease: StudioSourceLease,
        uri: &Uri,
        revision: u32,
        source: String,
    ) -> bool {
        let Some(row) = self.selected(lease, uri) else {
            return false;
        };
        if revision <= row.revision || self.advance().is_none() {
            return false;
        }
        let Some(row) = self.rows.get_mut(&lease.token) else {
            return false;
        };
        row.revision = revision;
        row.source = Some(Arc::from(source));
        true
    }

    fn retire(&mut self, lease: StudioSourceLease, uri: &Uri) -> bool {
        if self.closed || self.issuer != Some(lease.issuer) {
            return false;
        }
        if self.retired.get(&lease.token) == Some(uri) {
            return true;
        }
        if self.selected(lease, uri).is_none() || self.advance().is_none() {
            return false;
        }
        self.rows.remove(&lease.token);
        self.retired.insert(lease.token, uri.clone());
        true
    }

    fn retirement_is_published(&self, lease: StudioSourceLease, uri: &Uri) -> bool {
        self.issuer == Some(lease.issuer)
            && self.retired.get(&lease.token) == Some(uri)
            && !self.rows.contains_key(&lease.token)
            && !self.published.contains_key(&lease.token)
    }

    pub(super) fn close(&mut self) -> bool {
        if self.closed {
            return !self.shutdown_publication_complete();
        }
        let changed =
            !self.rows.is_empty() || !self.published.is_empty() || self.has_unpublished_changes();
        self.closed = true;
        for (token, row) in self.rows.drain() {
            self.retired.insert(token, row.admission.uri().clone());
        }
        self.generation = self.generation.saturating_add(1);
        if !changed {
            self.published_generation = self.generation;
        }
        changed
    }

    pub(super) fn shutdown_publication_complete(&self) -> bool {
        self.closed && self.published.is_empty() && !self.has_unpublished_changes()
    }

    pub(super) fn has_unpublished_changes(&self) -> bool {
        self.generation != self.published_generation
    }

    pub(super) fn mark_published(&mut self, snapshot: &StudioSourceSnapshot) {
        self.published_generation = self.generation;
        self.published = snapshot
            .sources
            .iter()
            .map(|image| (image.token, image.revision))
            .collect();
    }

    fn published_revision(&self, lease: StudioSourceLease, revision: u32) -> bool {
        self.issuer == Some(lease.issuer) && self.published.get(&lease.token) == Some(&revision)
    }

    pub(super) fn snapshot(&self) -> StudioSourceSnapshot {
        StudioSourceSnapshot {
            generation: self.generation,
            closed: self.closed,
            sources: self
                .rows
                .iter()
                .filter_map(|(token, row)| {
                    Some(StudioSourceImage {
                        token: *token,
                        revision: row.revision,
                        admission: Arc::clone(&row.admission),
                        source: Arc::clone(row.source.as_ref()?),
                    })
                })
                .collect(),
        }
    }

    pub(super) fn matches_snapshot(&self, snapshot: &StudioSourceSnapshot) -> bool {
        self.generation == snapshot.generation && self.closed == snapshot.closed
    }
}

#[derive(Clone)]
struct StudioSourceImage {
    token: u32,
    revision: u32,
    admission: Arc<DocumentPathAdmission>,
    source: Arc<str>,
}

pub(super) struct StudioSourceSnapshot {
    generation: u64,
    closed: bool,
    sources: Vec<StudioSourceImage>,
}

impl DocumentPathAdmission {
    pub(super) fn current_saved_source_path<'a>(
        &'a self,
        store: &dyn vfs::SourceStore,
    ) -> DocumentSourceResult<&'a vfs::SourceUriPath> {
        self.check_current(store)?;
        match self {
            Self::Native { selected, .. }
            | Self::Virtual { selected, .. }
            | Self::WasiGuest { selected, .. } => Ok(selected),
            Self::NoFilesystemPath { uri } => Err(DocumentSourceUnavailable::Path {
                uri: uri.clone(),
                cause: uri_norm::DocumentFilePathUnavailable::NotFile,
            }),
        }
    }

    pub(super) fn with_current_saved_sources<T>(
        sources: &[(&Self, &[u8])],
        store: &dyn vfs::SourceStore,
        mut publish: impl FnMut() -> T,
    ) -> DocumentSourceResult<T> {
        if sources.is_empty() {
            return Ok(publish());
        }
        let mut selected = Vec::with_capacity(sources.len());
        for (admission, bytes) in sources {
            admission.check_current(store)?;
            match admission {
                Self::Native { selected: path, .. }
                | Self::Virtual { selected: path, .. }
                | Self::WasiGuest { selected: path, .. } => selected.push((path, *bytes)),
                Self::NoFilesystemPath { uri } => {
                    return Err(DocumentSourceUnavailable::Path {
                        uri: uri.clone(),
                        cause: uri_norm::DocumentFilePathUnavailable::NotFile,
                    });
                }
            }
        }
        let mut result = None;
        store
            .with_current_document_sources(&selected, &mut || result = Some(publish()))
            .map_err(|cause| DocumentSourceUnavailable::Store {
                uri: sources[0].0.uri().clone(),
                cause,
            })?;
        result.ok_or_else(|| DocumentSourceUnavailable::Store {
            uri: sources[0].0.uri().clone(),
            cause: vfs::SourceUriPathUnavailable::ProviderUnavailable,
        })
    }
}

impl StudioSourceSnapshot {
    /// Reborrow the retained Studio image and its original admission for a loader path.
    pub(super) fn source_for_saved_path(
        &self,
        path: &Path,
    ) -> Option<(Arc<DocumentPathAdmission>, Arc<str>)> {
        self.sources.iter().find_map(|image| {
            image
                .admission
                .path()
                .is_some_and(|selected| selected.as_os_str() == path.as_os_str())
                .then(|| (Arc::clone(&image.admission), Arc::clone(&image.source)))
        })
    }
    pub(super) fn supplies_saved_source(&self, path: &std::path::Path) -> bool {
        self.sources.iter().any(|image| {
            image
                .admission
                .path()
                .is_some_and(|selected| selected.as_os_str() == path.as_os_str())
        })
    }

    pub(super) fn with_current_saved_source_inputs<T>(
        &self,
        store: &dyn vfs::SourceStore,
        paths: &[(&vfs::SourcePathOrigin, &[u8])],
        mut publish: impl FnMut() -> T,
    ) -> Option<T> {
        let documents = self
            .sources
            .iter()
            .map(|image| {
                Some((
                    image.admission.current_saved_source_path(store).ok()?,
                    image.source.as_bytes(),
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        let mut result = None;
        store
            .with_current_saved_source_inputs(&documents, paths, &mut || result = Some(publish()))
            .ok()?;
        result
    }

    /// Reborrow only original provider selections and exact saved images.
    /// Missing/changed materialisation never supplies a replacement image.
    pub(super) fn retained_sources(
        &mut self,
        store: &dyn vfs::SourceStore,
        roots: &[PathBuf],
    ) -> Vec<(PathBuf, String)> {
        self.sources.retain(|image| {
            image.admission.check_current(store).is_ok()
                && image.admission.path().is_some_and(|path| {
                    tcl_spectcl::discovery::is_workspace_studio_source(path, roots)
                        && store
                            .read(path)
                            .is_ok_and(|bytes| bytes == image.source.as_bytes())
                })
        });
        self.sources
            .iter()
            .filter_map(|image| {
                Some((
                    image.admission.path()?.to_path_buf(),
                    image.source.to_string(),
                ))
            })
            .collect()
    }

    pub(super) fn with_current_saved_sources<T>(
        &self,
        store: &dyn vfs::SourceStore,
        publish: impl FnMut() -> T,
    ) -> DocumentSourceResult<T> {
        let sources = self
            .sources
            .iter()
            .map(|image| (image.admission.as_ref(), image.source.as_bytes()))
            .collect::<Vec<_>>();
        DocumentPathAdmission::with_current_saved_sources(&sources, store, publish)
    }
}

struct StudioSourceConfirmation {
    lease: StudioSourceLease,
    uri: Uri,
    revision: Option<u32>,
    retired: bool,
}

type StudioSourceOutcome = (serde_json::Value, Option<StudioSourceConfirmation>);

impl Backend {
    async fn acquire_studio_source(&self, uri: Uri) -> DocumentSourceResult<StudioSourceOutcome> {
        let admission = Arc::new(DocumentPathAdmission::capture(&uri, self.store.as_ref())?);
        let options = self.spec_pack_discovery().await;
        if !admission.path().is_some_and(|path| {
            tcl_spectcl::discovery::is_workspace_studio_source(path, &options.workspace_roots)
        }) {
            return Ok((serde_json::json!({"accepted": false}), None));
        }
        let mut sources = self.studio_sources.lock().await;
        let mut confirmation = None;
        let reply = DocumentPathAdmission::with_current(
            &[admission.as_ref()],
            self.store.as_ref(),
            || {
                sources.acquire(Arc::clone(&admission)).map_or_else(
                    || serde_json::json!({"accepted": false}),
                    |lease| {
                        confirmation = Some(StudioSourceConfirmation {
                            lease,
                            uri: uri.clone(),
                            revision: None,
                            retired: false,
                        });
                        serde_json::json!({"accepted": true, "lease": lease})
                    },
                )
            },
        )?;
        Ok((reply, confirmation))
    }

    async fn publish_studio_source(
        &self,
        uri: Uri,
        lease: StudioSourceLease,
        revision: u32,
        source: String,
    ) -> DocumentSourceResult<StudioSourceOutcome> {
        let admission = {
            let sources = self.studio_sources.lock().await;
            let Some(row) = sources.selected(lease, &uri) else {
                return Ok((serde_json::json!({"accepted": false}), None));
            };
            Arc::clone(&row.admission)
        };
        let Some(path) = admission.path().map(Path::to_path_buf) else {
            return Ok((serde_json::json!({"accepted": false}), None));
        };
        let store = Arc::clone(&self.store);
        let expected = source.clone();
        let matches = crate::rt::spawn_blocking(move || {
            store
                .read(&path)
                .is_ok_and(|bytes| bytes == expected.as_bytes())
        })
        .await
        .unwrap_or(false);
        if !matches {
            return Ok((serde_json::json!({"accepted": false}), None));
        }
        let mut sources = self.studio_sources.lock().await;
        let mut source = Some(source);
        let accepted = DocumentPathAdmission::with_current(
            &[admission.as_ref()],
            self.store.as_ref(),
            || {
                source
                    .take()
                    .is_some_and(|source| sources.publish(lease, &uri, revision, source))
            },
        )?;
        let confirmation = accepted.then_some(StudioSourceConfirmation {
            lease,
            uri,
            revision: Some(revision),
            retired: false,
        });
        Ok((serde_json::json!({"accepted": accepted}), confirmation))
    }

    pub(super) async fn studio_source_command(
        &self,
        arguments: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let request: StudioSourceRequest = arguments
            .first()
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .filter(|_| arguments.len() == 1)
            .ok_or_else(|| {
                jsonrpc::Error::invalid_params(
                    "Expected one Studio acquire, publish or retire request",
                )
            })?;
        let (reply, confirmation) = match request {
            StudioSourceRequest::Acquire { uri } => self.acquire_studio_source(uri).await?,
            StudioSourceRequest::Publish {
                uri,
                lease,
                revision,
                source,
            } => {
                self.publish_studio_source(uri, lease, revision, source)
                    .await?
            }
            StudioSourceRequest::Retire { uri, lease } => {
                let accepted = self.studio_sources.lock().await.retire(lease, &uri);
                (
                    serde_json::json!({"accepted": accepted}),
                    accepted.then_some(StudioSourceConfirmation {
                        lease,
                        uri,
                        revision: None,
                        retired: true,
                    }),
                )
            }
        };
        if reply.get("accepted").and_then(serde_json::Value::as_bool) == Some(true)
            && self.reload_spec_packs(ReloadTrigger::StudioSource).await?
        {
            self.reschedule_all_open_documents().await;
        }
        if let Some(confirmation) = confirmation {
            let sources = self.studio_sources.lock().await;
            if confirmation.retired {
                return Ok(Some(
                    if sources.retirement_is_published(confirmation.lease, &confirmation.uri) {
                        reply
                    } else {
                        serde_json::json!({"accepted": false})
                    },
                ));
            }
            let Some(row) = sources.selected(confirmation.lease, &confirmation.uri) else {
                return Ok(Some(serde_json::json!({"accepted": false})));
            };
            if sources.has_unpublished_changes()
                || confirmation.revision.is_some_and(|revision| {
                    row.revision != revision
                        || !sources.published_revision(confirmation.lease, revision)
                })
            {
                return Ok(Some(serde_json::json!({"accepted": false})));
            }
            let reply = if confirmation.revision.is_some() {
                let Some(source) = row.source.as_ref() else {
                    return Ok(Some(serde_json::json!({"accepted": false})));
                };
                DocumentPathAdmission::with_current_saved_sources(
                    &[(row.admission.as_ref(), source.as_bytes())],
                    self.store.as_ref(),
                    || reply.clone(),
                )?
            } else {
                DocumentPathAdmission::with_current(
                    &[row.admission.as_ref()],
                    self.store.as_ref(),
                    || reply.clone(),
                )?
            };
            return Ok(Some(reply));
        }
        Ok(Some(reply))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfs::SourceStore as _;

    const URI: &str = "file:///studio/.tcl-lsp/.spec-studio/panel/active.tclspec";
    const PATH: &str = "/studio/.tcl-lsp/.spec-studio/panel/active.tclspec";
    const SOURCE: &str = "speclib studio_live 1 {command studio_live_command {arity 0}}";

    async fn backend(store: Arc<vfs::MemoryStore>) -> Backend {
        let (service, _socket) = tower_lsp_server::LspService::new(Backend::new);
        let backend = Backend::with_store(service.inner().client.clone(), store);
        *backend.workspace_folders.lock().await = vec![Uri::from_str("file:///studio").unwrap()];
        backend
    }

    async fn request(backend: &Backend, request: serde_json::Value) -> serde_json::Value {
        backend
            .execute_command(ExecuteCommandParams {
                command: "tcl-lsp.studioSource".to_owned(),
                arguments: vec![request],
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .unwrap()
            .unwrap()
    }

    async fn acquire(backend: &Backend) -> serde_json::Value {
        let result = request(
            backend,
            serde_json::json!({"operation":"acquire","uri":URI}),
        )
        .await;
        assert_eq!(result["accepted"], true);
        result["lease"].clone()
    }

    async fn has_live_pack(backend: &Backend) -> bool {
        backend
            .spec_packs()
            .await
            .packs
            .iter()
            .any(|pack| pack.name == "studio_live")
    }

    #[tokio::test]
    async fn active_saved_source_retires_without_deleting_and_restart_stays_closed() {
        let store = Arc::new(vfs::MemoryStore::new());
        store.upsert_uri(URI, PATH, SOURCE.as_bytes().to_vec());
        store.upsert(
            "/studio/.tcl-lsp/ordinary.tclspec",
            b"speclib ordinary 1 {command ordinary_command {arity 0}}".to_vec(),
        );
        let backend = backend(Arc::clone(&store)).await;
        backend
            .reload_spec_packs(ReloadTrigger::Startup)
            .await
            .unwrap();
        assert!(!has_live_pack(&backend).await);
        let lease = acquire(&backend).await;
        assert!(
            !has_live_pack(&backend).await,
            "acquisition alone is not publication"
        );
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":lease,"revision":1,"source":"unwritten"})).await["accepted"], false);
        let epoch = backend
            .diag_inputs_epoch
            .load(std::sync::atomic::Ordering::Acquire);
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":lease,"revision":1,"source":SOURCE})).await["accepted"], true);
        assert!(has_live_pack(&backend).await);
        assert!(
            backend
                .diag_inputs_epoch
                .load(std::sync::atomic::Ordering::Acquire)
                > epoch
        );
        assert_eq!(
            request(
                &backend,
                serde_json::json!({"operation":"retire","uri":URI,"lease":lease})
            )
            .await["accepted"],
            true
        );
        assert!(!has_live_pack(&backend).await);
        assert!(
            store.is_file(Path::new(PATH)),
            "retirement never deletes author files"
        );
        assert!(
            backend
                .spec_packs()
                .await
                .packs
                .iter()
                .any(|pack| pack.name == "ordinary")
        );
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":lease,"revision":2,"source":SOURCE})).await["accepted"], false);
        let restarted = self::backend(Arc::clone(&store)).await;
        restarted
            .reload_spec_packs(ReloadTrigger::Startup)
            .await
            .unwrap();
        assert!(!has_live_pack(&restarted).await);
    }

    #[tokio::test]
    async fn acquire_before_creation_and_shutdown_retire_without_filesystem_cleanup() {
        let store = Arc::new(vfs::MemoryStore::new());
        let backend = backend(Arc::clone(&store)).await;
        let lease = acquire(&backend).await;
        assert!(!store.is_file(Path::new(PATH)));
        store.upsert(PATH, SOURCE.as_bytes().to_vec());
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":lease,"revision":1,"source":SOURCE})).await["accepted"], true);
        assert!(has_live_pack(&backend).await);
        backend.shutdown().await.unwrap();
        assert_eq!(
            request(
                &backend,
                serde_json::json!({"operation":"acquire","uri":URI})
            )
            .await["accepted"],
            false
        );
        assert!(!has_live_pack(&backend).await);
        assert!(store.is_file(Path::new(PATH)));
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":lease,"revision":2,"source":SOURCE})).await["accepted"], false);
    }

    #[tokio::test]
    async fn saved_image_changed_while_reload_is_queued_cannot_acknowledge_publication() {
        let store = Arc::new(vfs::MemoryStore::new());
        store.upsert_uri(URI, PATH, SOURCE.as_bytes().to_vec());
        let backend = Arc::new(backend(Arc::clone(&store)).await);
        let lease = acquire(&backend).await;
        let reload = backend.spec_pack_reload.lock().await;
        let publishing = Arc::clone(&backend);
        let task = crate::rt::spawn(async move {
            request(&publishing, serde_json::json!({"operation":"publish","uri":URI,"lease":lease,"revision":1,"source":SOURCE})).await
        });
        crate::rt::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if !backend
                    .studio_sources
                    .lock()
                    .await
                    .snapshot()
                    .sources
                    .is_empty()
                {
                    break;
                }
                crate::rt::yield_now().await;
            }
        })
        .await
        .expect("the actual accepted source waits behind the reload lock");
        store.upsert(
            PATH,
            b"speclib replacement 1 {command replacement_command {arity 0}}".to_vec(),
        );
        drop(reload);
        assert_eq!(task.await.unwrap()["accepted"], false);
        assert!(!has_live_pack(&backend).await);
    }

    #[tokio::test]
    async fn saved_image_changed_after_load_before_publication_is_unavailable() {
        let store = Arc::new(vfs::MemoryStore::new());
        store.upsert_uri(URI, PATH, SOURCE.as_bytes().to_vec());
        let backend = Arc::new(backend(Arc::clone(&store)).await);
        let lease = acquire(&backend).await;
        let inputs = backend.analyser_inputs_gate.read().await;
        let publishing = Arc::clone(&backend);
        let task = crate::rt::spawn(async move {
            publishing.execute_command(ExecuteCommandParams {
                command: "tcl-lsp.studioSource".to_owned(),
                arguments: vec![serde_json::json!({"operation":"publish","uri":URI,"lease":lease,"revision":1,"source":SOURCE})],
                work_done_progress_params: WorkDoneProgressParams::default(),
            }).await
        });
        crate::rt::timeout(std::time::Duration::from_secs(30), async {
            loop {
                // Tokio's fair read gate refuses a new reader once the actual
                // reload writer is queued after its blocking load has returned.
                if backend.analyser_inputs_gate.try_read().is_err() {
                    break;
                }
                crate::rt::yield_now().await;
            }
        })
        .await
        .expect("the actual loaded source reaches final publication");
        let admission = backend.studio_sources.lock().await.snapshot().sources[0]
            .admission
            .clone();
        store.upsert(
            PATH,
            b"speclib replacement 1 {command replacement_command {arity 0}}".to_vec(),
        );
        admission.check_current(store.as_ref()).unwrap();
        drop(inputs);
        assert!(
            task.await.unwrap().is_err(),
            "unchanged mapping cannot certify changed saved source"
        );
        assert!(!has_live_pack(&backend).await);
    }

    #[tokio::test]
    async fn cross_backend_same_uri_token_cannot_publish_or_retire_new_owner() {
        let store = Arc::new(vfs::MemoryStore::new());
        store.upsert_uri(URI, PATH, SOURCE.as_bytes().to_vec());
        let old = backend(Arc::clone(&store)).await;
        let new = backend(Arc::clone(&store)).await;
        let old_lease = acquire(&old).await;
        let new_lease = acquire(&new).await;
        assert_eq!(old_lease["token"], new_lease["token"]);
        assert_ne!(old_lease["issuer"], new_lease["issuer"]);
        assert_eq!(request(&new, serde_json::json!({"operation":"publish","uri":URI,"lease":new_lease,"revision":1,"source":SOURCE})).await["accepted"], true);
        assert_eq!(request(&new, serde_json::json!({"operation":"publish","uri":URI,"lease":old_lease,"revision":2,"source":SOURCE})).await["accepted"], false);
        assert_eq!(
            request(
                &new,
                serde_json::json!({"operation":"retire","uri":URI,"lease":old_lease})
            )
            .await["accepted"],
            false
        );
        assert!(has_live_pack(&new).await);
    }

    #[tokio::test]
    async fn original_provider_mapping_cannot_rebind_across_equal_byte_aba() {
        let store = Arc::new(vfs::MemoryStore::new());
        store.upsert_uri(URI, PATH, SOURCE.as_bytes().to_vec());
        let backend = backend(Arc::clone(&store)).await;
        let lease = acquire(&backend).await;
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":lease,"revision":1,"source":SOURCE})).await["accepted"], true);
        assert!(has_live_pack(&backend).await);
        assert!(store.remove_uri(URI));
        store.upsert_uri(URI, PATH, SOURCE.as_bytes().to_vec());
        let result = backend.studio_source_command(&[serde_json::json!({"operation":"publish","uri":URI,"lease":lease,"revision":2,"source":SOURCE})]).await;
        assert!(
            result.is_err(),
            "the old path allocation is not the replacement mapping"
        );
        assert_eq!(
            request(
                &backend,
                serde_json::json!({"operation":"retire","uri":URI,"lease":lease})
            )
            .await["accepted"],
            true
        );
        assert!(!has_live_pack(&backend).await);
        let replacement = acquire(&backend).await;
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":replacement,"revision":1,"source":SOURCE})).await["accepted"], true);
        assert!(has_live_pack(&backend).await);
    }

    #[tokio::test]
    async fn retirement_invalidates_captured_reload_before_waiting_for_reload_lock() {
        let store = Arc::new(vfs::MemoryStore::new());
        store.upsert_uri(URI, PATH, SOURCE.as_bytes().to_vec());
        let backend = Arc::new(backend(Arc::clone(&store)).await);
        let lease = acquire(&backend).await;
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":lease,"revision":1,"source":SOURCE})).await["accepted"], true);
        let snapshot = backend.studio_sources.lock().await.snapshot();
        let reload = backend.spec_pack_reload.lock().await;
        let retiring = Arc::clone(&backend);
        let task = crate::rt::spawn(async move {
            request(
                &retiring,
                serde_json::json!({"operation":"retire","uri":URI,"lease":lease}),
            )
            .await
        });
        crate::rt::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if !backend
                    .studio_sources
                    .lock()
                    .await
                    .matches_snapshot(&snapshot)
                {
                    break;
                }
                crate::rt::yield_now().await;
            }
        })
        .await
        .expect("retirement invalidates the original snapshot before queued reload");
        assert!(
            !backend
                .studio_sources
                .lock()
                .await
                .matches_snapshot(&snapshot)
        );
        drop(reload);
        assert_eq!(task.await.unwrap()["accepted"], true);
        assert!(!has_live_pack(&backend).await);
        assert!(store.is_file(Path::new(PATH)));
    }

    #[tokio::test]
    async fn replaced_or_stale_revision_and_missing_entropy_never_donate_a_live_lease() {
        let store = Arc::new(vfs::MemoryStore::new());
        let backend = backend(Arc::clone(&store)).await;
        let old = acquire(&backend).await;
        store.upsert_uri(URI, PATH, SOURCE.as_bytes().to_vec());
        let current = acquire(&backend).await;
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":old,"revision":1,"source":SOURCE})).await["accepted"], false);
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":current,"revision":2,"source":SOURCE})).await["accepted"], true);
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":URI,"lease":current,"revision":1,"source":SOURCE})).await["accepted"], false);
        assert!(has_live_pack(&backend).await);
        let unavailable = self::backend(Arc::clone(&store)).await;
        unavailable.studio_sources.lock().await.issuer = None;
        assert_eq!(
            request(
                &unavailable,
                serde_json::json!({"operation":"acquire","uri":URI})
            )
            .await["accepted"],
            false
        );
        assert_eq!(
            request(
                &unavailable,
                serde_json::json!({"operation":"retire","uri":URI,"lease":current})
            )
            .await["accepted"],
            false
        );
    }
}

#[cfg(test)]
mod retirement_publication_tests {
    use super::*;

    #[derive(Debug, Default)]
    struct ReloadFailureStore {
        source: vfs::MemoryStore,
        fail_reload: std::sync::atomic::AtomicBool,
    }

    impl vfs::SourceStore for ReloadFailureStore {
        fn document_uri_path(
            &self,
            uri: &str,
        ) -> Result<vfs::SourceUriPath, vfs::SourceUriPathUnavailable> {
            self.source.document_uri_path(uri)
        }
        fn validate_document_uri_path(
            &self,
            path: &vfs::SourceUriPath,
        ) -> Result<(), vfs::SourceUriPathUnavailable> {
            self.source.validate_document_uri_path(path)
        }
        fn with_current_document_paths(
            &self,
            paths: &[&vfs::SourceUriPath],
            publish: &mut dyn FnMut(),
        ) -> Result<(), vfs::SourceUriPathUnavailable> {
            self.source.with_current_document_paths(paths, publish)
        }
        fn with_current_document_sources(
            &self,
            sources: &[(&vfs::SourceUriPath, &[u8])],
            publish: &mut dyn FnMut(),
        ) -> Result<(), vfs::SourceUriPathUnavailable> {
            self.source.with_current_document_sources(sources, publish)
        }
        fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
            self.source.read(path)
        }
        fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
            self.source.read_to_string(path)
        }
        fn read_dir(&self, path: &Path) -> std::io::Result<Vec<vfs::DirEntry>> {
            assert!(
                !self.fail_reload.load(std::sync::atomic::Ordering::Acquire),
                "actual reload source failure"
            );
            self.source.read_dir(path)
        }
        fn is_dir(&self, path: &Path) -> bool {
            self.source.is_dir(path)
        }
        fn is_file(&self, path: &Path) -> bool {
            self.source.is_file(path)
        }
        fn exists(&self, path: &Path) -> bool {
            self.source.exists(path)
        }
        fn metadata(&self, path: &Path) -> std::io::Result<vfs::Metadata> {
            self.source.metadata(path)
        }
        fn write(&self, path: &Path, contents: &[u8]) -> std::io::Result<()> {
            self.source.write(path, contents)
        }
        fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
            self.source.create_dir_all(path)
        }
    }

    async fn request(backend: &Backend, object: serde_json::Value) -> serde_json::Value {
        backend
            .execute_command(ExecuteCommandParams {
                command: "tcl-lsp.studioSource".to_owned(),
                arguments: vec![object],
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn retire_and_shutdown_cannot_acknowledge_failed_withdrawal_and_can_retry() {
        let uri = "file:///studio/.tcl-lsp/.spec-studio/panel/active.tclspec";
        let store = Arc::new(ReloadFailureStore::default());
        let source = "speclib retirement_live 1 {command retirement_command {arity 0}}";
        store.source.upsert_uri(
            uri,
            "/studio/.tcl-lsp/.spec-studio/panel/active.tclspec",
            source.as_bytes().to_vec(),
        );
        let (service, socket) = tower_lsp_server::LspService::new(Backend::new);
        // Match the existing test helper: no client transport is driven here.
        drop(socket);
        let backend = Backend::with_store(service.inner().client.clone(), store.clone());
        *backend.workspace_folders.lock().await = vec![Uri::from_str("file:///studio").unwrap()];
        let acquired = request(
            &backend,
            serde_json::json!({"operation":"acquire","uri":uri}),
        )
        .await;
        assert_eq!(acquired["accepted"], true);
        let lease = &acquired["lease"];
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":uri,"lease":lease,"revision":1,"source":source})).await["accepted"], true);
        store
            .fail_reload
            .store(true, std::sync::atomic::Ordering::Release);
        assert_eq!(
            request(
                &backend,
                serde_json::json!({"operation":"retire","uri":uri,"lease":lease})
            )
            .await["accepted"],
            false
        );
        assert!(
            backend
                .spec_packs()
                .await
                .packs
                .iter()
                .any(|pack| pack.name == "retirement_live")
        );
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":uri,"lease":lease,"revision":2,"source":source})).await["accepted"], false);
        store
            .fail_reload
            .store(false, std::sync::atomic::Ordering::Release);
        assert_eq!(
            request(
                &backend,
                serde_json::json!({"operation":"retire","uri":uri,"lease":lease})
            )
            .await["accepted"],
            true
        );
        assert!(
            !backend
                .spec_packs()
                .await
                .packs
                .iter()
                .any(|pack| pack.name == "retirement_live")
        );
        let acquired = request(
            &backend,
            serde_json::json!({"operation":"acquire","uri":uri}),
        )
        .await;
        let lease = &acquired["lease"];
        assert_eq!(request(&backend, serde_json::json!({"operation":"publish","uri":uri,"lease":lease,"revision":1,"source":source})).await["accepted"], true);
        store
            .fail_reload
            .store(true, std::sync::atomic::Ordering::Release);
        assert!(backend.shutdown().await.is_err());
        assert_eq!(
            request(
                &backend,
                serde_json::json!({"operation":"acquire","uri":uri})
            )
            .await["accepted"],
            false
        );
        store
            .fail_reload
            .store(false, std::sync::atomic::Ordering::Release);
        backend.shutdown().await.unwrap();
        assert!(
            !backend
                .spec_packs()
                .await
                .packs
                .iter()
                .any(|pack| pack.name == "retirement_live")
        );
    }
}
