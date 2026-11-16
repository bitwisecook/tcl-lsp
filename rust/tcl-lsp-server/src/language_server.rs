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

//! The `LanguageServer` implementation: every LSP request and notification handler.

use super::*;

impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        // A document can be opened, and analysed, before the start-up pack
        // reload has published anything; the compiled-in core surfaces are
        // part of what it is analysed against either way, so they are in
        // before the client can send one.
        tcl_spectcl::core_surfaces::ensure();
        self.apply_workspace_folders(&params).await;
        self.apply_initialization_options(&params).await;
        // Push is the sole diagnostics channel by default: pull is opt-in and
        // the server does not advertise `diagnosticProvider` (see
        // `build_server_capabilities`).  A client that *supports* pull will not
        // actually pull unless the server advertises it, so the
        // "stop pushing when the client pulls" suppression must stay OFF here —
        // otherwise a pull-capable client (VS Code advertises the capability)
        // gets neither push (suppressed) nor pull (unadvertised), i.e. zero
        // diagnostics.  When pull is opted back in this flag is set from the
        // client's actual `textDocument/diagnostic` support via
        // `client_supports_pull_diagnostics(&params)`.
        let _ = client_supports_pull_diagnostics(&params);
        self.client_supports_pull_diagnostics
            .store(false, std::sync::atomic::Ordering::Relaxed);
        self.client_supports_relative_watch_patterns.store(
            client_supports_relative_watch_patterns(&params),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.conditional_variable_rename_supported.store(
            client_supports_conditional_rename(&params),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.environment_notice.set_client_capabilities(
            client_supports_message_action_items(&params),
            client_supports_show_document(&params),
        );
        let position_encoding = negotiate_position_encoding(&params);
        if client_lacks_utf16_support(&params) {
            // The client advertised position encodings without UTF-16. The
            // server emits UTF-16 columns only, so it proceeds in UTF-16
            // regardless (rather than failing the session over a baseline
            // every conformant client supports) — but warn so a genuine
            // UTF-8/UTF-32-only client's mis-mapped non-ASCII ranges are
            // diagnosable instead of silent.
            self.client
                .log_message(
                    MessageType::WARNING,
                    "client advertised position encodings without UTF-16; this server \
                     emits UTF-16 columns and will use UTF-16 regardless",
                )
                .await;
        }
        Ok(InitializeResult {
            capabilities: build_server_capabilities(position_encoding),
            server_info: Some(ServerInfo {
                // Protocol identity must match the editor's
                // expectations ("tcl-lsp"), not the crate/binary name.
                name: "tcl-lsp".to_owned(),
                // The release version from the tag, not the manifest's 0.1.0.
                version: Some(tcl_version::VERSION.to_owned()),
            }),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let _startup_guard = WorkspaceStartupGuard(&self.workspace_scan_ready);
        self.client
            .log_message(MessageType::INFO, "tcl-lsp-server initialised")
            .await;
        if let Some(warning) = self.environment_notice.take_load_warning() {
            self.client.log_message(MessageType::WARNING, warning).await;
        }
        // Pull the resolved `tclLsp` config once the client is ready so
        // feature toggles / optimiser switch / analyser knobs are in effect
        // before the first request.
        match self.pull_and_apply_config().await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        // Ask the client to watch Tcl source files so external edits
        // (`git checkout`, generated files, deletions) reach
        // `did_change_watched_files` and refresh the cross-document index.
        // Best-effort: clients without dynamic-registration support reject
        // this, which is harmless — the startup scan still seeds the index.
        self.register_file_watchers().await;
        // Load the workspace's SpecTcl packs before the first analysis so a
        // pack-declared command is known from the first keystroke rather than
        // flashing W123 until a reload catches up.  `pull_and_apply_config`
        // above has already applied `tclLsp.specPacks`, so discovery sees the
        // configured paths on this first pass.
        match self.reload_spec_packs(ReloadTrigger::Startup).await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        };
        // Type-hierarchy support is advertised statically by the
        // `inject_type_hierarchy_provider` service layer (see `main.rs`) — a
        // dynamic `client/registerCapability` here would both duplicate it and
        // fail to surface in the client's `initializeResult.capabilities`.
        // Seed the cross-document index with on-disk project files
        // the editor hasn't opened yet.
        match self.scan_workspace_folders().await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        // `initialized` fires concurrently with any `textDocument/didOpen` the
        // client already queued for its restored tabs (`buffer_unordered`, see
        // `edit_serialize`'s doc comment) — a document's first debounced
        // diagnostics run (50ms) routinely completes before this scan does on
        // anything but a trivial workspace. The always-on W120/W123 workspace
        // refinement (`refine_workspace_w120`/`refine_workspace_w123`) reads
        // `workspace_index` / `package_resolver`, which this scan is what
        // populates — so a document opened at startup can publish a diagnostic
        // (e.g. a false W120 for a command whose `package require` lives in an
        // unopened `source` ancestor) computed against the still-empty stores,
        // and nothing republishes it once the scan finishes. Reschedule every
        // open document now so any diagnostics published before the scan
        // completed are refreshed against the now-populated workspace state —
        // the same pattern `did_change_watched_files` already uses after its
        // own `scan_workspace_folders` call on a config change. The populated
        // package database is a diagnostics input, so a report computed
        // against the empty one is stale rather than complete.
        self.invalidate_diag_inputs();
        self.reschedule_all_open_documents().await;
        // VS Code computes a document's sticky-scroll model once, when the
        // editor opens, and a folding provider that registers *after* that
        // first computation does not itself trigger a recompute — the tabs
        // restored at startup would keep an empty sticky bar until the next
        // edit.  `workspace/foldingRange/refresh` does trigger one (it fires
        // `SyntaxRangeProvider.onDidChange`), so send it once the provider is
        // genuinely live.  Best-effort and idempotent, exactly as in
        // `did_change_configuration`: a client without refresh support
        // rejects the request, which is harmless.
        let _ = bounded_client_request(self.client.send_request::<FoldingRangeRefreshRequest>(()))
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.workspace_scan_ready.shutdown.raise();
        let retired = self.studio_sources.lock().await.close();
        if retired {
            self.reload_spec_packs(ReloadTrigger::StudioSource).await?;
            if !self
                .studio_sources
                .lock()
                .await
                .shutdown_publication_complete()
            {
                return Err(jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: "Studio source withdrawal publication is unavailable".into(),
                    data: None,
                });
            }
        }
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        // Apply document-sync mutations in arrival order (see `EditOrder`). The
        // ticket must be drawn before the first await.
        let ticket = self
            .edit_order
            .take_ticket("didOpen", &params.text_document.uri);
        let turn = self.edit_order.wait_turn(ticket).await;
        let path_admission =
            match DocumentPathAdmission::capture(&params.text_document.uri, self.store.as_ref()) {
                Ok(admission) => Arc::new(admission),
                Err(error) => {
                    drop(turn);
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            };
        // Phase markers (see `EditTurn::at`): `did_open` is the handler a wedge
        // most often names as the turn holder, so every `await` it reaches
        // inside the turn is marked before it is entered.
        turn.at("did_open: dialect_for_open");
        let dialect = self
            .dialect_for_open(
                &params.text_document.uri,
                &params.text_document.language_id,
                &params.text_document.text,
            )
            .await;
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        let version = params.text_document.version;
        let dialect_for_diags = dialect.clone();
        // Make the authoritative live buffer visible in arrival order, then
        // release the *global* request barrier before waiting on either shared
        // store. Retaining this turn while suspended on a shared store blocks
        // every request in `edits_settled()` even though the transport is
        // still alive.
        let state = DocumentState::with_version(params.text_document.text, dialect, version)
            .with_path_admission(Arc::clone(&path_admission))
            .with_language_id(params.text_document.language_id.clone())
            .with_publication_pending();
        turn.at("did_open: documents.lock");
        let mut documents = self.documents.lock("did_open").await;
        let admitted = DocumentPathAdmission::with_current(
            &[path_admission.as_ref()],
            self.store.as_ref(),
            || {
                documents.insert(uri.clone(), state.clone());
            },
        );
        drop(documents);
        if let Err(error) = admitted {
            drop(turn);
            self.client
                .log_message(MessageType::WARNING, error.to_string())
                .await;
            return;
        }
        self.invalidate_live_publication(std::iter::once(&uri));

        // Poll the index writer once while the edit turn still excludes new
        // requests. If the lock is busy this registers our writer in Tokio's
        // fair queue without waiting for it; if it is free, retain the guard.
        // Either way, no request released by `drop(turn)` can acquire an index
        // read ahead of this stale-slot retirement, including when different
        // executor workers run concurrently.
        let mut index_write = std::pin::pin!(self.workspace_index.write());
        let mut index_guard = None;
        std::future::poll_fn(|cx| {
            if let Poll::Ready(guard) = index_write.as_mut().poll(cx) {
                index_guard = Some(guard);
            }
            Poll::Ready(())
        })
        .await;
        drop(turn);
        self.notify_environment_kind(&dialect_for_diags);

        // Await only after releasing the global barrier. This wait holds no
        // other store or edit turn, so a pre-existing index reader/writer
        // cannot recreate the whole-server wedge.
        let mut index = match index_guard {
            Some(guard) => guard,
            None => index_write.await,
        };
        let retired = DocumentPathAdmission::with_current(
            &[path_admission.as_ref()],
            self.store.as_ref(),
            || {
                index.remove_document(uri.as_str());
            },
        );
        drop(index);
        if let Err(error) = retired {
            self.client
                .log_message(MessageType::WARNING, error.to_string())
                .await;
            return;
        }

        // Publish the matching Salsa source after the turn is free. The
        // deferred commit re-checks text/dialect/version under the
        // document map before its no-await mutation, so a later edit or close
        // wins rather than being overwritten by this open. The ordinary
        // unindexed-open-document fallback covers the short gap until live
        // analysis republishes the buffer.
        if !(match Box::pin(self.commit_open_document(&uri, &text, &dialect_for_diags, version))
            .await
        {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }) {
            return;
        }
        // If a change lands while the disk read below runs, the version/text
        // check refuses to attach facts about the opened file to the edited
        // buffer.
        self.attach_open_disk_decode_report(&uri, &text, version)
            .await;
        // Opening a document is a config-context boundary, not an edit. A closed
        // URI keeps its `DiagSlot` (only the live document + index entry are
        // dropped on close), so `slot.latest_inputs` can carry the toggle values
        // captured when the file was last analysed — a stale
        // `features.diagnostics` / dialect / `optimiser.enabled` / disabled-code
        // set if any of those changed while the file was closed. The
        // reuse-cached-inputs fast path is valid only on the edit path
        // (`did_change`), where config genuinely has not changed. Force a fresh
        // input resolve so the open reads the *current* toggles; otherwise
        // reopening a file after the diagnostics master switch was turned off
        // republishes its pre-toggle squiggles from the stale inputs.
        self.reschedule_diagnostics(uri, dialect_for_diags).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // Incremental sync: each content change is either a full-document
        // replacement (`range == None`) or a ranged edit applied to the
        // current text. Apply them in order. (The re-analysis below is
        // still whole-document and is not bounded to `reparse_window`,
        // though the primitives exist in `tcl-lexer`.)
        // Apply document-sync mutations in arrival order (see `EditOrder`) —
        // otherwise two rapid incremental edits splice out of order and corrupt
        // the buffer. The ticket must be drawn before the first await.
        let ticket = self
            .edit_order
            .take_ticket("didChange", &params.text_document.uri);
        let turn = self.edit_order.wait_turn(ticket).await;
        let uri = params.text_document.uri.clone();
        if params.content_changes.is_empty() {
            return;
        }
        let path_admission = match DocumentPathAdmission::capture(&uri, self.store.as_ref()) {
            Ok(admission) => Arc::new(admission),
            Err(error) => {
                drop(turn);
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        };
        let change_version = params.text_document.version;
        let changed = {
            // Phase marker — see `EditTurn::at`.
            turn.at("did_change: documents.lock");
            let mut docs = self.documents.lock("did_change").await;
            // tower-lsp-server 0.23 dispatches notification handlers
            // concurrently (`buffer_unordered(4)`), so a `didChange` can be
            // processed before its `didOpen` or after its `didClose`. Only
            // mutate an already-open document: resurrecting a closed one (or
            // splicing a ranged edit against a phantom empty buffer that
            // `didOpen` then silently overwrites at the wrong version) would
            // corrupt state and keep publishing diagnostics for a closed
            // document. A dropped change is safe — `didOpen`
            // carries the authoritative full text, and a post-close change has
            // nothing to apply to.
            let Some(entry) = docs.get_mut(&uri) else {
                return;
            };
            if let Some(retained) = &entry.path_admission
                && let Err(error) = retained.check_for(&uri, self.store.as_ref())
            {
                drop(docs);
                drop(turn);
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
            let retained = entry
                .path_admission
                .clone()
                .unwrap_or_else(|| Arc::clone(&path_admission));
            DocumentPathAdmission::with_current(&[retained.as_ref()], self.store.as_ref(), || {
                Self::apply_document_content_changes(
                    entry,
                    &params.content_changes,
                    &path_admission,
                    change_version,
                )
            })
        };
        let (text, dialect, language_id, revision, index_needs_seed) = match changed {
            Ok(changed) => changed,
            Err(error) => {
                drop(turn);
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        };
        self.invalidate_live_publication(std::iter::once(&uri));
        // The ordered operation is the buffer splice. Hand the global turn on
        // before Salsa publication: a setter can wait for a snapshot, and
        // retaining this turn across such a wait wedges every request at
        // `edits_settled`. The deferred
        // publication checks revision/text/dialect under the document map, so
        // a newer edit or close wins without being overwritten.
        drop(turn);
        if !(match self
            .publish_changed_document(&uri, &text, &dialect, revision, index_needs_seed)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }) {
            return;
        }
        // An ordinarily indexed document deliberately keeps its previous
        // workspace facts until diagnostics republishes this revision:
        // that is less misleading than a per-keystroke absence. The exception
        // above is an edit that superseded a cold-open seed after its disk slot
        // was removed; there are no previous facts to retain, so it restores a
        // current view before advertising indexed readiness.
        self.schedule_diagnostics(uri.clone(), dialect).await;
        // Resolve a hint dirtied by this edit **or any superseded predecessor**.
        // The helper is a no-op when no generation is pending, so the ordinary
        // no-hint keystroke path still avoids a whole-document detector pass.
        if language_id == "tcl" {
            match self
                .resolve_dirty_open_document_dialect(&uri, "did_change_dialect")
                .await
            {
                Ok(value) => value,
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                }
            }
        }
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        // Accept ``{"tclLsp": {"dialect": "<name>"}}`` in either the
        // VS Code-style nested shape or the flat ``{"dialect":
        // "<name>"}`` shape (used by the MCP bridge). Update the
        // session default so newly opened documents pick up the change,
        // then re-resolve every already-open document under the new
        // default and reschedule diagnostics for any whose dialect
        // changed — `tclLsp.selectDialect` pushes a dialect-only config
        // change for a buffer that is already open, and without this the
        // buffer would keep being parsed/diagnosed under the dialect it
        // was opened with until reopened. This snapshots each open
        // document's resolved dialect and force-reanalyses the ones that
        // change.
        let dialect = params
            .settings
            .get("tclLsp")
            .and_then(|v| v.get("dialect"))
            .or_else(|| params.settings.get("dialect"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        // The editor's Workspace Trust state, when the client pushes it — VS
        // Code does when the user grants trust. Top-level only: a `tclLsp`
        // section here is the synchronised settings, which the workspace's
        // own settings file can write (`workspace_trust_in`).
        let analyser_inputs_guard = self.analyser_inputs_gate.write().await;
        let trust_changed = match workspace_trust_in(&params.settings) {
            Some(trust) => {
                let mut current = self.workspace_trust.lock().await;
                let changed = *current != trust;
                *current = trust;
                changed
            }
            None => false,
        };
        let mut dialect_warning = None;
        if let Some(d) = dialect {
            dialect_warning = self.apply_configured_session_dialect(&d).await;
            // No re-resolve here: the coalesced reload below ends with one
            // (`pull_and_apply_config`'s own, at the single point every pull
            // passes through), so doing it per notification would only
            // multiply the re-analyses a settings-editor burst causes.
        }
        // W108 mode + disabled-diagnostics reconfiguration. Existing
        // documents pick the change up on their next analyse (the
        // analyses cache is keyed per URI and refreshed on edit).
        if let Some(mode) = settings_non_ascii_mode(&params.settings) {
            *self.non_ascii_mode.lock().await = mode;
        }
        let skip = {
            let mut layers = self.policy_layers.lock().await;
            layers.editor = config_ini::merge_settings(
                &layers.editor,
                &normalize_config_payload(&params.settings),
            );
            layers.production_skip()
        };
        *self.disabled_diagnostics.lock().await = skip;
        if let Some(flag) = settings_environment_kind_enabled(&params.settings) {
            self.environment_notice.set_enabled(flag);
        }
        // The writes above land immediately, ahead of the coalesced
        // re-pull below, so they retire the scheduler's cached inputs on their
        // own — the flat MCP-bridge payload is the only thing that carries them
        // and it must not need a second notification to take effect on the next
        // keystroke.
        self.invalidate_diag_inputs();
        drop(analyser_inputs_guard);
        // A trust change starts or stops a workspace pack's hook bodies. The
        // pack-set key mixes the state, so this reload re-installs with no
        // file moved, and every open document re-analyses against the bodies
        // that now run — or no longer do.
        if trust_changed
            && (match self.reload_spec_packs(ReloadTrigger::Trust).await {
                Ok(value) => value,
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            })
        {
            self.reschedule_all_open_documents().await;
        }
        self.warn_configured_dialect(dialect_warning).await;
        // VS Code (and the e2e harness) push an empty/partial payload as a
        // signal to re-pull the full resolved config via
        // `workspace/configuration`.  Always re-pull so `features.*`, the
        // optimiser switch, and the analyser knobs reflect the latest editor
        // settings — the inline `params.settings` handling above covers the
        // flat MCP-bridge shape that carries the values directly.  Coalesced:
        // a settings-editor burst is one pull and one re-analysis, not one per
        // notification.
        match self.coalesced_config_reload().await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Apply document-sync mutations in arrival order (see `EditOrder`) so a
        // close can't be reordered ahead of an in-flight edit. The ticket must be
        // drawn before the first await.
        let ticket = self
            .edit_order
            .take_ticket("didClose", &params.text_document.uri);
        let turn = self.edit_order.wait_turn(ticket).await;
        let uri = &params.text_document.uri;
        let admission = match self.document_path_admission(uri).await {
            Ok(admission) => admission,
            Err(error) => {
                drop(turn);
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        };

        {
            // The ordered mutation is removal of the authoritative live
            // buffer. Index/caches are derived publications and happen only
            // after the turn is free.
            turn.at("did_close: documents.lock");
            let mut docs = self.documents.lock("did_close").await;
            let removed = DocumentPathAdmission::with_current(
                &[admission.as_ref()],
                self.store.as_ref(),
                || {
                    docs.remove(uri);
                },
            );
            drop(docs);
            if let Err(error) = removed {
                drop(turn);
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        self.invalidate_live_publication(std::iter::once(uri));
        drop(turn);
        self.live_publication_advanced.notify_waiters();

        // Optimistically acquire the index and then currency-check the closed
        // state without ever waiting while holding either lock. A racing open
        // is authoritative and cancels this stale close tail.
        match self.commit_closed_live_state(uri, &admission).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        // The transaction above also releases the diagnostics slot,
        // semantic-token baseline/refresh marker, and workspace-class memo.
        // Keeping those removals in the same final closed-state check prevents
        // a close superseded by `didOpen` from clearing the reopened buffer's
        // newly-armed state.
        // The ordering turn is already free before every derived cleanup and
        // the heavy tail below. `EditOrder` is a *global* barrier — every
        // request handler awaits `edits_settled`, and the next `did_change`
        // waits for this ticket — so holding it across a disk read, a full
        // uncached `Analyser::analyse`, an index rebuild and the closed-file
        // diagnostics pipeline would freeze hover / completion / semantic
        // tokens in *every* open document for the length of a close.
        // None of that tail is an ordered buffer mutation. The live document
        // removal is already visible; each derived publication checks that the
        // URI remains closed, so a `did_open` that now wins the race keeps it.
        // Re-index the file from disk rather than dropping it: the file still
        // exists on disk and was (or would be) part of the on-disk index, so
        // cross-document definition / references / rename / call-hierarchy — and
        // cross-file diagnostics (the salsa `Project`) — must keep seeing it after
        // the editor closes the buffer.  `scan_workspace_folders` only runs at
        // `initialized`, so dropping it here would make the file vanish until
        // restart.  The helper rechecks that this URI is still closed, then
        // refreshes both the salsa db source and the disk-backed index entry (or
        // drops both when the URI is not a readable file).
        match self
            .reindex_index_from_disk_using(uri, Some(Arc::clone(&admission)))
            .await
        {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        // Keep the file's Problems / File-Explorer badge after its editor tab
        // closes.  An unconditional empty publish would make a
        // closed-but-on-disk workspace file lose its diagnostics until it was
        // reopened, so republish its on-disk diagnostics through the same
        // pipeline the open path uses (the set is identical, kept accurate
        // against disk).  For a URI with no readable on-disk source (untitled
        // buffer, deleted file) this clears the squiggles and drops the
        // pull-cache entry.  The reindex above primed the salsa source it
        // reads; both re-check the document is still closed under the `documents`
        // lock, so a racing `did_open` can never have a stale closed publish land
        // on a freshly reopened buffer.
        self.publish_closed_file_diagnostics(uri).await;
        // Sync guarantee: a harness (`waitForDeepDiagnostics`) keys on the
        // `[timing] deep diagnostics (uri=…)` marker to know the close's
        // republish settled before it asserts the retained badge. That marker
        // is emitted *inside* the currency-gated publish, so a close run
        // legitimately superseded by a racing config / watched-file refresh —
        // which bumps the per-URI closed generation — or one that settles an
        // empty file emits none, and the harness would time out even though the
        // badge settled correctly. Emit an
        // unconditional completion marker here, ordered after the republish above
        // has delivered its publish, so the signal is reliable regardless of the
        // internal delivery outcome. Notifications are ordered on the client, so
        // any publish sent above is applied before this marker is observed; the
        // count is read back from the pull cache the publish primed. A duplicate
        // of the in-pipeline marker on the common (current) path is harmless —
        // the harness matches on presence, never a count.
        let uri_str = uri.to_string();
        let diag_count = self
            .pull_diag_cache
            .lock()
            .await
            .get(uri)
            .map_or(0, |entry| entry.diagnostics.len());
        self.client
            .log_message(
                MessageType::LOG,
                format!("[timing] deep diagnostics 0ms (uri={uri_str}, diags={diag_count})"),
            )
            .await;
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        // Update the tracked folder set, then reconcile the cross-document
        // index and configuration so multi-root behaviour is not frozen at the
        // `initialize` snapshot.  Drops removed-folder state, loads/scans
        // added folders, and re-pulls config.
        let removed: Vec<Uri> = params.event.removed.iter().map(|f| f.uri.clone()).collect();
        {
            let _inputs = self.analyser_inputs_gate.write().await;
            let mut folders = self.workspace_folders.lock().await;
            for uri in &removed {
                folders.retain(|f| f != uri);
            }
            for added in &params.event.added {
                if !folders.iter().any(|f| f == &added.uri) {
                    folders.push(added.uri.clone());
                }
            }
        }
        // Drop index entries for files under removed folders so their symbols
        // stop resolving across the workspace.  Open documents are left alone —
        // their live buffer is the source of truth regardless of folder set.
        if !removed.is_empty() {
            match self.drop_index_under_folders(&removed).await {
                Ok(value) => value,
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            }
        }
        // Pick up files under any newly-added folder.  `scan_workspace_folders`
        // walks the *current* folder list and skips open documents, so it is
        // safe to call here.
        if !params.event.added.is_empty() {
            match self.scan_workspace_folders().await {
                Ok(value) => value,
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            }
        }
        // A folder add/remove shifts the salsa `Project`, `workspace_index` and
        // `package_resolver` (the cross-file resolution domain *and* the
        // always-on W120/W123 workspace-refinement inputs) without an open
        // document's own edit, so reschedule every open document — otherwise a
        // push-diagnostic client keeps a stale suppressed-W123 / cross-file
        // arity, or a false W120 the refinement would now resolve, from a
        // now-removed (or newly-added) folder. Unconditional
        // `reschedule_all_open_documents` (not a narrower `crossFileResolution`-only
        // helper): the W120/W123 refinement runs for every document regardless
        // of that opt-in toggle.
        if !removed.is_empty() || !params.event.added.is_empty() {
            self.reschedule_all_open_documents().await;
        }
        // Per-folder config may differ between roots; re-pull so the resolved
        // settings reflect the new folder set.
        match self.pull_and_apply_config().await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
            }
        }
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let Some(event_admissions) = self.capture_watched_event_admissions(&params.changes).await
        else {
            return;
        };
        // tower-lsp-server dispatches notification handlers concurrently.
        // Preserve watched-event transaction order so an older deletion cannot
        // finish its index/Salsa removal after a newer create has revived the
        // same still-open path.
        let _watched_change = self.watched_file_change_gate.lock().await;
        // External (non-editor) file changes — `git checkout`, a generated
        // file, a deletion — must refresh the cross-document index so
        // definition / references / rename / call-hierarchy keep seeing the
        // project's true on-disk state between restarts.
        let WatchedFileChanges {
            config_changed,
            sidecar_changed,
            spec_pack_changed,
            last_kind,
        } = partition_watched_file_changes(params.changes);
        if sidecar_changed {
            self.invalidate_sidecar_stubs().await;
        }
        // A pack file moved: reload the whole set (a pack is a logical unit
        // spread over any number of files, so there is no such thing as
        // reloading one file of it) and, when the content really changed,
        // re-analyse every open document — a command that just became known,
        // or just stopped being known, changes what W123 and hover say about
        // code nobody touched.
        if spec_pack_changed
            && (match self.reload_spec_packs(ReloadTrigger::WatchedFile).await {
                Ok(value) => value,
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            })
        {
            self.reschedule_all_open_documents().await;
        }
        // Partition the (deduplicated) event set so the disk-backed work below
        // runs once per kind — a parallel read+analyse and one batched
        // index/db mutation — instead of once per file: a 500-file branch
        // switch would otherwise run 500 sequential full analyses here, each
        // with its own `documents.lock()` / pull-cache probe.
        let mut deleted: Vec<Uri> = Vec::new();
        // The subset of `deleted` whose buffer is still open. Their index entry
        // is retired like any other deletion, but they must not have their
        // *diagnostics* cleared as a closed file's badge would be — the editor
        // is still showing the buffer.
        let mut deleted_while_open: HashSet<Uri> = HashSet::new();
        let mut changed: Vec<Uri> = Vec::new();
        // Open buffers whose backing file reappeared (a branch switch that
        // deletes then restores a path emits CREATED/CHANGED for it): clear the
        // orphan mark so the buffer rejoins the cross-document index.
        let mut revived_while_open: Vec<Uri> = Vec::new();
        self.partition_open_watched_paths(
            last_kind,
            &mut deleted,
            &mut deleted_while_open,
            &mut changed,
            &mut revived_while_open,
        )
        .await;
        // Apply the orphan marks before any re-analysis is scheduled, so the
        // publish path (which consults the flag under the `documents` lock)
        // cannot re-add a just-retired path.
        let (revived_publications, deleted_revisions) = match self
            .update_watched_open_paths(&deleted_while_open, &revived_while_open, &event_admissions)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        };
        let domain_changed =
            !deleted.is_empty() || !changed.is_empty() || !revived_publications.is_empty();

        // Closed files that already carry a badge (a pull-cache entry) and whose
        // on-disk change must refresh or clear that badge — computed once
        // under one lock rather than once per file, and applied after the whole
        // batch has updated the resolution domain, so each refresh analyses
        // against the final on-disk state — never a half-applied one when
        // several files change together.
        let (closed_badge_clear, closed_badge_refresh): (Vec<Uri>, Vec<Uri>) = {
            let cache = self.pull_diag_cache.lock().await;
            Self::watched_closed_badge_uris(&cache, &deleted, &changed, &deleted_while_open)
        };

        // DELETED: atomically retire the batch from the index and salsa
        // `Project` so a deleted file stops suppressing W123 / driving the
        // arity error cross-file. Unlike scan/folder publication, deletion is
        // authoritative even for an open path: the buffer remains usable but
        // its dead filesystem identity must leave cross-file resolution.
        if !self
            .retire_watched_deleted_paths(&deleted, &deleted_revisions, &event_admissions)
            .await
        {
            return;
        }
        // CREATED / CHANGED: read + analyse every file across the bounded
        // worker pool, then one batched index/db merge.
        match self
            .batch_reindex_from_disk_using(&changed, Some(&event_admissions))
            .await
        {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        // An open buffer whose path was previously orphaned remains the source
        // of truth when that path reappears. Restore both its live Salsa input
        // and standalone index view; a racing edit/close/delete wins through
        // the ordinary revision and orphan currency checks.
        if !self
            .restore_revived_live_documents(revived_publications)
            .await
        {
            return;
        }

        // Refresh/clear the badges of the closed files that changed on disk, now
        // the resolution domain has settled.
        for uri in &closed_badge_clear {
            self.clear_closed_diagnostics(uri).await;
        }
        for uri in &closed_badge_refresh {
            self.publish_closed_file_diagnostics(uri).await;
        }
        // A watched (non-open) file's create/change/delete shifts the cross-file
        // resolution domain *and* `workspace_index` (the always-on W120/W123
        // workspace-refinement input), but no open document's own edit triggered
        // it — so a push-diagnostic client would keep stale cross-file results (a
        // suppressed W123, an arity error sourced from the now-changed file, or a
        // false W120 the refinement would now resolve via a `source` ancestor)
        // until the caller is next edited. Reschedule every open document — not
        // just `crossFileResolution`-enabled ones, since the W120/W123 refinement
        // is unconditional — so both the cross-file pass and the refinement re-run
        // against the new domain.
        if domain_changed {
            self.reschedule_all_open_documents().await;
        }
        // Re-read config.ini / .tcl-lsp.ini and re-apply with full precedence,
        // then rebuild the package database (libraryPaths may have changed) and
        // re-run open documents so the new settings take effect immediately.
        if config_changed {
            self.refresh_watched_configuration().await;
        }
    }

    async fn will_save_wait_until(
        &self,
        params: WillSaveTextDocumentParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        // Opt-in format-on-save, gated behind
        // `tclLsp.features.willSaveWaitUntil`, which is **off by default**.
        // The capability is advertised unconditionally; the runtime guard
        // here returns no edits when the toggle is unset, matching the
        // repo's "always register, guard in the body" feature pattern.
        if !self
            .will_save_format_enabled(&params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let Some(doc) = self.read_local_document(&params.text_document.uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(
                &params.text_document.uri,
                doc.text.clone(),
                doc.dialect.clone(),
            )
            .await?;
        // Honour the user's resolved `tclLsp.formatting` settings, exactly as
        // `formatting()` does — otherwise format-on-save re-indents with
        // defaults and fights an explicit Format Document.
        // `WillSaveTextDocumentParams` carries no `FormattingOptions`, so the
        // settings object is the sole source; the resolved formatter width is
        // then applied on top.
        let formatting = self.resolved_formatting(&params.text_document.uri).await;
        // The document's resolved dialect is the formatter's one dialect
        // fact: the lexer preset, the rewrite-candidate release, and the
        // version-range-aware forward range all follow from it.
        let mut config = core_formatting::FormatterConfig::for_dialect(&doc.dialect);
        if let Some(obj) = formatting.as_object() {
            apply_formatting_object(obj, &mut config);
        }
        config.max_line_length =
            self.resolved_line_length(&params.text_document.uri).await as usize;
        config.indent_size = config.indent_size.max(1);
        // Run on a worker so a formatter panic is contained as a JSON-RPC
        // error rather than unwinding the event loop, matching every sibling
        // handler.
        let text = doc.raw().to_owned();
        let edits = crate::rt::spawn_blocking(move || {
            core_formatting::formatting_with_analysis(&text, &config, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("will_save_wait_until worker panicked: {err}").into(),
            data: None,
        })?;
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        if !self
            .feature_enabled("folding", &params.text_document.uri)
            .await
        {
            // `None`, never `Some([])` — and the same holds for every other
            // return path below.  An *empty* folding result is
            // not neutral in VS Code: its sticky-scroll model provider treats
            // the folding candidate as valid whenever the model is non-null,
            // so an authoritative empty list is accepted as a valid, terminal
            // sticky model and the chain never falls through to the
            // indentation model — sticky scroll stays permanently blank for
            // the whole session.  `None` is the only result that both lets
            // the folding UI fall back to built-in indentation folding and
            // lets sticky scroll fall through to its indentation model.
            return Ok(None);
        }
        let Some(doc) = self.read_local_document(&params.text_document.uri).await? else {
            return Ok(None);
        };
        // BIG-IP config documents fold on their stanza tree rather than the
        // Tcl brace walk (which, on non-Tcl config text, finds only comment
        // blocks and leaves every stanza unfoldable) — the folding twin of
        // the outline split in `document_symbol`.
        let bigip = Self::is_bigip_dialect(&doc.dialect);
        let ranges = if bigip {
            let text = doc.text.clone();
            // Pure-CPU tokenise/segment work; run on a worker so a parser
            // panic is contained as a JSON-RPC error rather than unwinding
            // the event loop (defence in depth).
            crate::rt::spawn_blocking(move || core_bigip::folding_ranges(&text))
                .await
                .map_err(|err| jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: format!("folding worker panicked: {err}").into(),
                    data: None,
                })?
        } else if let Some(ranges) = self.db_folding_ranges(&params.text_document.uri).await {
            // Served from the salsa query graph (memoised); avoids a fresh
            // segmenter/registry walk on every request.
            ranges
        } else {
            // Cold / cancelled fallback: compute directly so the request
            // never returns empty due to a concurrent edit (behaviour
            // preserved).
            let analysis = self
                .analysis_for(
                    &params.text_document.uri,
                    doc.text.clone(),
                    doc.dialect.clone(),
                )
                .await?;
            crate::rt::spawn_blocking(move || {
                tcl_lsp_core::folding::folding_ranges_with_analysis(&doc.text, &analysis)
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("folding worker panicked: {err}").into(),
                data: None,
            })?
        };
        if ranges.is_empty() {
            return Ok(None);
        }
        Ok(Some(ranges.into_iter().map(lift_folding_range).collect()))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        if !self
            .feature_enabled("documentSymbols", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let Some(doc) = self.read_local_document(&params.text_document.uri).await? else {
            return Ok(None);
        };
        // BIG-IP config documents get a `module → kind → object` outline
        // built from their stanza tree rather than the Tcl scope walk
        // (which would find nothing in non-Tcl config text). Nameless
        // singletons fall back to their kind label so no outline symbol
        // ever carries an empty `name`.
        // The two compute branches run pure-CPU on a worker so a parser
        // panic is contained as a JSON-RPC error.
        let symbols = if Self::is_bigip_dialect(&doc.dialect) {
            let text = doc.text.clone();
            crate::rt::spawn_blocking(move || core_bigip::document_symbols(&text))
                .await
                .map_err(|err| jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: format!("document_symbol worker panicked: {err}").into(),
                    data: None,
                })?
        } else {
            // Resolve the declaration-vs-script route from the same current
            // document analysis that owns its workspace registry and lexer
            // configuration. A profile label alone can miss an installed
            // declaration grammar overlay.
            let analysis = self
                .analysis_for(
                    &params.text_document.uri,
                    doc.text.clone(),
                    doc.dialect.clone(),
                )
                .await?;
            let text = doc.text.clone();
            if tcl_lsp_core::declaration_outline::is_declaration_document_from_analysis(
                &text, &analysis,
            ) {
                crate::rt::spawn_blocking(move || {
                    core_symbols::declaration_symbols_from_analysis(&text, &analysis)
                })
                .await
                .map_err(|err| jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: format!("document_symbol worker panicked: {err}").into(),
                    data: None,
                })?
            } else if let Some(symbols) = self.db_document_symbols(&params.text_document.uri).await
            {
                // Served from the salsa query graph (memoised; reuses the
                // tracked file_analysis instead of re-running projection).
                symbols
            } else {
                // The direct analysis is current and complete for this request.
                crate::rt::spawn_blocking(move || {
                    core_symbols::document_symbols_from_analysis(&text, &analysis)
                })
                .await
                .map_err(|err| jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: format!("document_symbol worker panicked: {err}").into(),
                    data: None,
                })?
            }
        };
        let lifted: Vec<DocumentSymbol> = symbols.into_iter().map(lift_document_symbol).collect();
        Ok(Some(DocumentSymbolResponse::Nested(lifted)))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        if !self
            .feature_enabled(
                "completion",
                &params.text_document_position.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let pos = params.text_document_position.position;
        // Fetch the cached per-dialect registry on the async
        // path before spawning so the worker can read built-in
        // command names alongside the user-defined procs the
        // analyser surfaces.  Threads through as
        // Completion here only knows user procs / vars, not the
        // registry's command set.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        // Share the cross-document index with the worker so it can
        // enumerate procs from sibling files.  Move a refcounted
        // handle in and take the read lock inside the blocking
        // closure: the heavy walk holds a shared read lock, letting
        // other readers proceed concurrently while only writers wait.
        let workspace_index = Arc::clone(&self.workspace_index);
        // Pure-CPU work; spawn_blocking off the LSP event loop.
        let items = crate::rt::spawn_blocking(move || {
            let workspace = workspace_index.blocking_read();
            core_completion::completions(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
                Some(&*workspace),
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("completion worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted: Vec<CompletionItem> = items
            .into_iter()
            .map(|item| lift_completion_item(item, pos.line))
            .collect();
        Ok(Some(CompletionResponse::Array(lifted)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        if !self
            .feature_enabled(
                "definition",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        if let std::ops::ControlFlow::Break(locations) = self
            .original_source_view_navigation_route(&uri, pos, None)
            .await?
        {
            return Ok(locations
                .filter(|items| !items.is_empty())
                .map(GotoDefinitionResponse::Array));
        }
        let locations = Box::pin(self.compute_definition(&uri, pos)).await?;
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDefinitionResponse::Array(locations)))
    }

    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<GotoDeclarationResponse>> {
        if !self
            .feature_enabled(
                "declaration",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // For a `$var`, go-to-declaration resolves the
        // visible `global` / `variable` / `upvar` scoping statement that
        // declares the name; for any other symbol it defers to plain
        // go-to-definition (handled inside `core_declaration`).
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let ranges = crate::rt::spawn_blocking(move || {
            core_declaration::declaration(
                &text,
                pos.line,
                pos.character,
                tcl_lsp_core::profile_for_dialect(&dialect),
                &analysis,
                &registry,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("declaration worker panicked: {err}").into(),
            data: None,
        })?;
        let locations: Vec<Location> = if ranges.is_empty() {
            // No in-document declaration / definition — try the
            // cross-document index (a sibling-file proc / class), the
            // same fallback go-to-definition uses.
            Box::pin(self.compute_definition(&uri, pos)).await?
        } else {
            ranges
                .into_iter()
                .map(|r| Location {
                    uri: uri.clone(),
                    range: lift_lsp_range(r),
                })
                .collect()
        };
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDeclarationResponse::Array(locations)))
    }

    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoTypeDefinitionResponse>> {
        if !self
            .feature_enabled(
                "typeDefinition",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // Type-definition jumps to the class that types the symbol (a `$obj`
        // instance's class, or a method's owning class), never the plain
        // definition site.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let source_epoch = self.diag_inputs_epoch();
        let caller_generation = self.live_publication_uri_generation(&uri);
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            match self
                .original_method_candidate_at(&uri, &doc.text, &analysis, pos)
                .await?
            {
                ControlFlow::Break(Some(method)) => {
                    let Ok(owner) = method.uri().parse::<Uri>() else {
                        return Ok(None);
                    };
                    let target_generation = self.live_publication_uri_generation(&owner);
                    let Some(target) = self.read_document(&owner).await? else {
                        return Ok(None);
                    };
                    let target_analysis = self
                        .analysis_for(&owner, target.text.clone(), target.dialect.clone())
                        .await?;
                    let Some(class) = method.declaring_class_in(&target.text, &target_analysis)
                    else {
                        return Ok(None);
                    };
                    let Some(identity) =
                        tcl_lsp_core::original_declaration::OriginalDeclarationIdentity::for_class(
                            owner.as_str(),
                            &target.text,
                            &target_analysis,
                            class,
                        )
                    else {
                        return Ok(None);
                    };
                    let Some(context) = tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(&target_analysis) else { return Ok(None); };
                    let Some(location) =
                        OriginalIndexedSourceLocation::from_declaration(&context, &identity)
                    else {
                        return Ok(None);
                    };
                    if location
                        .validated_span(&target.text, &target_analysis)
                        .is_none()
                    {
                        return Ok(None);
                    }
                    let locations = self
                        .resolve_current_original_class_type_location(
                            (uri.clone(), doc.clone(), caller_generation),
                            (owner, target, target_analysis, target_generation),
                            location,
                            source_epoch,
                        )
                        .await?;
                    return Ok(Some(GotoTypeDefinitionResponse::Array(locations)));
                }
                ControlFlow::Break(None) => return Ok(None),
                ControlFlow::Continue(()) => {}
            }
        }
        let text = doc.text.clone();
        let ranges = crate::rt::spawn_blocking(move || {
            core_type_definition::type_definition(&text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("type-definition worker panicked: {err}").into(),
            data: None,
        })?;
        if ranges.is_empty() {
            return Ok(None);
        }
        let locations = type_definition_locations(&uri, ranges);
        Ok(Some(GotoTypeDefinitionResponse::Array(locations)))
    }

    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> jsonrpc::Result<Option<GotoImplementationResponse>> {
        if !self
            .feature_enabled(
                "implementation",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // Implementation queries retain exact source owners across the
        // complete current workspace declaration inventory.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            use std::ops::ControlFlow;
            let class = core_type_hierarchy::prepare_for_document(
                uri.as_str(),
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
            )
            .into_iter()
            .next()
            .and_then(|item| item.original_declaration);
            let method = if class.is_some() {
                None
            } else {
                match self
                    .original_method_candidate_at(&uri, &doc.text, &analysis, pos)
                    .await?
                {
                    ControlFlow::Break(Some(method)) => Some(method),
                    ControlFlow::Break(None) | ControlFlow::Continue(()) => return Ok(None),
                }
            };
            let documents = self.original_declaration_documents(&uri).await?;
            let targets = crate::rt::spawn_blocking(move || {
                let inventory = implementation_inventory(&documents);
                if let Some(class) = class {
                    core_type_hierarchy::related_from_inventory(&class, &inventory, true)
                        .into_iter()
                        .filter_map(|item| Some((item.original_declaration?, item.selection_range)))
                        .collect::<Vec<_>>()
                } else if let Some(method) = method {
                    core_implementation::method_implementations_in_inventory(&method, &inventory)
                        .into_iter()
                        .map(|target| (target.identity, target.range))
                        .collect()
                } else {
                    Vec::new()
                }
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("implementation worker panicked: {err}").into(),
                data: None,
            })?;
            let mut locations = Vec::new();
            for (identity, range) in targets {
                let Ok(target_uri) = Uri::from_str(identity.uri()) else {
                    continue;
                };
                let Some(target_doc) = self.read_document(&target_uri).await? else {
                    continue;
                };
                let target_analysis = self
                    .analysis_for(&target_uri, target_doc.text.clone(), target_doc.dialect)
                    .await?;
                if identity.is_current(target_uri.as_str(), &target_doc.text, &target_analysis) {
                    locations.push(Location {
                        uri: target_uri,
                        range: lift_lsp_range(range),
                    });
                }
            }
            return Ok(
                (!locations.is_empty()).then_some(GotoImplementationResponse::Array(locations))
            );
        }
        let locations = self.readonly_logical_implementation(&uri, pos).await?;
        Ok(locations
            .filter(|locations| !locations.is_empty())
            .map(GotoImplementationResponse::Array))
    }

    async fn references(&self, params: ReferenceParams) -> jsonrpc::Result<Option<Vec<Location>>> {
        if !self
            .feature_enabled(
                "references",
                &params.text_document_position.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let include_decl = params.context.include_declaration;
        let mut settled = None;
        if let std::ops::ControlFlow::Break(locations) = self
            .original_source_view_navigation_route(&uri, pos, Some(include_decl))
            .await?
        {
            match self.original_source_reference_supplement(&uri).await? {
                Some(sourced) => settled = Some((locations.unwrap_or_default(), sourced)),
                // A view that lists something is the answer. One that lists
                // nothing leaves the ordinary listing to answer.
                None if locations.as_ref().is_some_and(|found| !found.is_empty()) => {
                    return Ok(locations);
                }
                None => {}
            }
        }
        let ordinary = self.ordinary_references(&uri, pos, include_decl).await?;
        let Some((mut locations, sourced)) = settled else {
            return Ok(ordinary);
        };
        locations.extend(
            ordinary
                .into_iter()
                .flatten()
                .filter(|location| !sourced.contains(&location.uri)),
        );
        dedup_locations(&mut locations);
        Ok((!locations.is_empty()).then_some(locations))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> jsonrpc::Result<Option<Vec<DocumentHighlight>>> {
        if !self
            .feature_enabled(
                "documentHighlight",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        // Highlighting is find-references narrowed to one document, so it
        // takes the same whole-program export view —
        // otherwise a `-force`-shadowed call highlights as an occurrence of a
        // definition go-to-definition refuses to open.
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let entries = crate::rt::spawn_blocking(move || {
            // The kinded entry
            // point tags variable defining spans as `Write` and
            // their reads as `Read`; command-invocation heads
            // stay `Text`.
            core_references::document_highlights_in_program(
                &doc.text,
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
                pos.line,
                pos.character,
                &analysis,
                core_definition::CallResolution::document_only().in_program(
                    core_definition::ProgramExports {
                        uri: &uri_key,
                        oracle: exports.as_ref(),
                    },
                ),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("document_highlight worker panicked: {err}").into(),
            data: None,
        })?;
        if entries.is_empty() {
            return Ok(None);
        }
        let highlights = entries
            .into_iter()
            .map(|(r, kind)| DocumentHighlight {
                range: lift_lsp_range(r),
                kind: Some(match kind {
                    core_references::HighlightKind::Read => DocumentHighlightKind::READ,
                    core_references::HighlightKind::Write => DocumentHighlightKind::WRITE,
                    core_references::HighlightKind::Text => DocumentHighlightKind::TEXT,
                }),
            })
            .collect();
        Ok(Some(highlights))
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyItem>>> {
        if !self
            .feature_enabled(
                "callHierarchy",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        if let std::ops::ControlFlow::Break(items) = self
            .original_source_view_call_prepare_route(&uri, pos)
            .await?
        {
            return Ok(items.filter(|items| !items.is_empty()));
        }
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            let documents = self.original_declaration_documents(&uri).await?;
            let workspace = self.workspace_index.read().await;
            let inventory = documents
                .iter()
                .map(|(uri, source, analysis)| {
                    tcl_lsp_core::original_call_hierarchy::OriginalCallDocument {
                        uri,
                        source,
                        analysis,
                    }
                })
                .collect::<Vec<_>>();
            let items = tcl_lsp_core::original_call_hierarchy::prepare(
                uri.as_str(),
                pos.line,
                pos.character,
                &inventory,
                Some(&workspace),
            );
            drop(workspace);
            let mut lifted = Vec::new();
            for item in items {
                if let Some(item) = self.lift_original_call_item(item).await? {
                    lifted.push(item);
                }
            }
            return Ok((!lifted.is_empty()).then_some(lifted));
        }
        let items = self.readonly_logical_call_prepare(&uri, pos).await?;
        Ok(items.filter(|items| !items.is_empty()))
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyIncomingCall>>> {
        if !self
            .feature_enabled("callHierarchy", &params.item.uri)
            .await
        {
            return Ok(None);
        }
        let item = params.item;
        if item
            .data
            .as_ref()
            .and_then(|data| data.get("originalDeclaration"))
            .and_then(|handle| handle.get("schema"))
            .and_then(serde_json::Value::as_u64)
            == Some(5)
        {
            let edges = self.walk_original_source_view_calls(&item, true).await?;
            return Ok(Some(
                edges
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(from, from_ranges)| CallHierarchyIncomingCall { from, from_ranges })
                    .collect(),
            ));
        }
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            let Some(identity) = self
                .resolve_original_declaration_data(item.data.as_ref(), &uri, &doc.text, &analysis)
                .await
            else {
                return Ok(Some(Vec::new()));
            };
            let documents = self.original_declaration_documents(&uri).await?;
            let workspace = self.workspace_index.read().await;
            let inventory = documents
                .iter()
                .map(|(uri, source, analysis)| {
                    tcl_lsp_core::original_call_hierarchy::OriginalCallDocument {
                        uri,
                        source,
                        analysis,
                    }
                })
                .collect::<Vec<_>>();
            let edges = tcl_lsp_core::original_call_hierarchy::incoming(
                &identity,
                &inventory,
                Some(&workspace),
            );
            drop(workspace);
            let mut lifted = Vec::new();
            for (_, edge) in edges {
                if let Some(item) = self.lift_original_call_item(edge.from).await? {
                    lifted.push(CallHierarchyIncomingCall {
                        from: item,
                        from_ranges: edge.from_ranges.into_iter().map(lift_lsp_range).collect(),
                    });
                }
            }
            return Ok(Some(lifted));
        }
        let edges = self.readonly_logical_call_walk(&item, true).await?;
        Ok(edges.map(|edges| {
            edges
                .into_iter()
                .map(|(from, from_ranges)| CallHierarchyIncomingCall { from, from_ranges })
                .collect()
        }))
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        if !self
            .feature_enabled("callHierarchy", &params.item.uri)
            .await
        {
            return Ok(None);
        }
        let item = params.item;
        if item
            .data
            .as_ref()
            .and_then(|data| data.get("originalDeclaration"))
            .and_then(|handle| handle.get("schema"))
            .and_then(serde_json::Value::as_u64)
            == Some(5)
        {
            let edges = self.walk_original_source_view_calls(&item, false).await?;
            return Ok(Some(
                edges
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(to, from_ranges)| CallHierarchyOutgoingCall { to, from_ranges })
                    .collect(),
            ));
        }
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            let Some(identity) = self
                .resolve_original_declaration_data(item.data.as_ref(), &uri, &doc.text, &analysis)
                .await
            else {
                return Ok(Some(Vec::new()));
            };
            let documents = self.original_declaration_documents(&uri).await?;
            let workspace = self.workspace_index.read().await;
            let inventory = documents
                .iter()
                .map(|(uri, source, analysis)| {
                    tcl_lsp_core::original_call_hierarchy::OriginalCallDocument {
                        uri,
                        source,
                        analysis,
                    }
                })
                .collect::<Vec<_>>();
            let edges = tcl_lsp_core::original_call_hierarchy::outgoing(
                &identity,
                &inventory,
                Some(&workspace),
            );
            drop(workspace);
            let mut lifted = Vec::new();
            for (_, edge) in edges {
                if let Some(item) = self.lift_original_call_item(edge.to).await? {
                    lifted.push(CallHierarchyOutgoingCall {
                        to: item,
                        from_ranges: edge.from_ranges.into_iter().map(lift_lsp_range).collect(),
                    });
                }
            }
            return Ok(Some(lifted));
        }
        let edges = self.readonly_logical_call_walk(&item, false).await?;
        Ok(edges.map(|edges| {
            edges
                .into_iter()
                .map(|(to, from_ranges)| CallHierarchyOutgoingCall { to, from_ranges })
                .collect()
        }))
    }

    async fn prepare_type_hierarchy(
        &self,
        params: TypeHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if analysis.allows_lexical_declaration_advice() {
            let lifted = self.readonly_logical_hierarchy_prepare(&uri, pos).await?;
            return Ok((!lifted.is_empty()).then_some(lifted));
        }
        let owner = uri.to_string();
        let items = crate::rt::spawn_blocking(move || {
            core_type_hierarchy::prepare_for_document(
                &owner,
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("type_hierarchy worker panicked: {err}").into(),
            data: None,
        })?;
        if items.is_empty() {
            return Ok(None);
        }
        let lifted = self.lift_original_type_hierarchy_items(items).await?;
        Ok((!lifted.is_empty()).then_some(lifted))
    }

    async fn supertypes(
        &self,
        params: TypeHierarchySupertypesParams,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
        self.type_hierarchy_walk(params.item, false).await
    }

    async fn subtypes(
        &self,
        params: TypeHierarchySubtypesParams,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
        self.type_hierarchy_walk(params.item, true).await
    }

    async fn linked_editing_range(
        &self,
        params: LinkedEditingRangeParams,
    ) -> jsonrpc::Result<Option<LinkedEditingRanges>> {
        if !self
            .feature_enabled(
                "linkedEditingRange",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // `S-linked-editing-range-rich`: when the cursor sits
        // on a proc declaration's name or inside the proc's
        // body, return every range that should be edited in
        // lock-step (the declaration plus any self-call sites
        // inside the body).  Editors then paint these as
        // linked-edit chips so renaming one updates the others
        // as the user types.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let result = crate::rt::spawn_blocking(move || {
            core_linked_editing_range::linked_editing_ranges(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("linked_editing_range worker panicked: {err}").into(),
            data: None,
        })?;
        let Some(linked) = result else {
            return Ok(None);
        };
        Ok(Some(LinkedEditingRanges {
            ranges: linked.ranges.into_iter().map(lift_lsp_range).collect(),
            word_pattern: Some(linked.word_pattern),
        }))
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> jsonrpc::Result<DocumentDiagnosticReportResult> {
        let uri = params.text_document.uri;
        let report = self.ensure_current_diagnostic_report(&uri).await?;
        if params.previous_result_id.as_deref() == Some(report.result_id.as_str()) {
            return Ok(DocumentDiagnosticReportResult::Report(
                DocumentDiagnosticReport::Unchanged(RelatedUnchangedDocumentDiagnosticReport {
                    related_documents: None,
                    unchanged_document_diagnostic_report: UnchangedDocumentDiagnosticReport {
                        result_id: report.result_id,
                    },
                }),
            ));
        }
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: Some(report.result_id),
                    items: report.diagnostics,
                },
            }),
        ))
    }

    async fn workspace_diagnostic(
        &self,
        params: WorkspaceDiagnosticParams,
    ) -> jsonrpc::Result<WorkspaceDiagnosticReportResult> {
        let previous: HashMap<Uri, String> = params
            .previous_result_ids
            .into_iter()
            .map(|p| (p.uri, p.value))
            .collect();
        // The cache key set identifies requested participants, not completion.
        // Each payload must reborrow its own acknowledged original publisher.
        let uris = self
            .pull_diag_cache
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for uri in &uris {
            self.ensure_current_diagnostic_report(uri).await?;
        }
        let reports = self
            .current_cached_diagnostic_reports(&uris)
            .await?
            .map_err(Self::diagnostic_report_unavailable)?;
        // No await separates the jointly current source/input snapshot from
        // wire projection. Versions belong to those actual original reports.
        let items = reports
            .into_iter()
            .map(|report| {
                let uri = report.uri;
                let version = report.version.map(i64::from);
                if previous.get(&uri).map(String::as_str) == Some(report.result_id.as_str()) {
                    WorkspaceDocumentDiagnosticReport::Unchanged(
                        WorkspaceUnchangedDocumentDiagnosticReport {
                            uri,
                            version,
                            unchanged_document_diagnostic_report:
                                UnchangedDocumentDiagnosticReport {
                                    result_id: report.result_id,
                                },
                        },
                    )
                } else {
                    WorkspaceDocumentDiagnosticReport::Full(WorkspaceFullDocumentDiagnosticReport {
                        uri,
                        version,
                        full_document_diagnostic_report: FullDocumentDiagnosticReport {
                            result_id: Some(report.result_id),
                            items: report.diagnostics,
                        },
                    })
                }
            })
            .collect();
        Ok(WorkspaceDiagnosticReportResult::Report(
            WorkspaceDiagnosticReport { items },
        ))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        if !self
            .feature_enabled("semanticTokens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let target = &uri;
        readonly_reference_reports::retry_readonly_attempt(
            Err(coarse_semantic_source_changed()),
            move || self.semantic_tokens_full_attempt(target),
        )
        .await?
    }

    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> jsonrpc::Result<Option<SemanticTokensFullDeltaResult>> {
        if !self
            .feature_enabled("semanticTokens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let (target, request) = (&uri, &params);
        readonly_reference_reports::retry_readonly_attempt(
            Err(coarse_semantic_source_changed()),
            move || self.semantic_tokens_full_delta_attempt(target, request),
        )
        .await?
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<SemanticTokensRangeResult>> {
        // Gate on the same `semanticTokens` toggle as `full` / `full_delta`, so
        // disabling semantic tokens also silences viewport (range) requests —
        // otherwise range-request clients keep rendering highlights the user
        // turned off (issue 174).
        if !self
            .feature_enabled("semanticTokens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let Some(doc) = self
            .read_document_for_semantic_tokens(&params.text_document.uri)
            .await?
        else {
            return Ok(None);
        };
        // `S-semantic-tokens-rich`: filter tokens to those that
        // start inside the requested range.  Editors call this
        // when only the visible viewport needs colouring.
        let core_range = core_semantic_range(params.range);
        if let Some(tokens) = self
            .non_tcl_range_tokens(&params.text_document.uri, &doc, core_range)
            .await
        {
            // Settle here too: an APL / BIG-IP viewport never touches the Tcl
            // compilation unit, so there is no enriched tier to converge to —
            // and an observer waiting on the marker must not be left waiting for
            // a decision that will never be made.
            log_range_convergence_settled(
                &self.client,
                params.text_document.uri.as_str(),
                false,
                RangeSettleOutcome::NoAnalysis,
            )
            .await;
            return Ok(Some(SemanticTokensRangeResult::Tokens(LspSemanticTokens {
                result_id: None,
                data: lift_semantic_token_data(&tokens.data),
            })));
        }
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let uri = &params.text_document.uri;
        // Race the memoised unit + analysis against the fast-path budget; on
        // timeout `pending` carries the still-running reads to the convergence
        // continuation below (see `race_range_enriched_reads`).
        let (cached_cu, cached_analysis, pending, had_analysis_handles) =
            self.race_range_enriched_reads(uri).await;
        let cached_analysis = if had_analysis_handles {
            cached_analysis
        } else {
            Some(
                self.fresh_analysis_for(uri, Arc::clone(&doc.text), doc.dialect.clone())
                    .await?,
            )
        };
        // Distinguishes the two no-continuation cases for the settled marker
        // below: the enriched reads landed inside the budget (so this response
        // *is* the enriched viewport), versus the document having no analysis
        // handles at all.
        // Both inputs are required for the complete enriched tier. A Salsa
        // write can cancel one read after its sibling completed; that partial
        // result is useful for this response but remains retryable rather than
        // being mislabelled final.
        let served_enriched = cached_cu.is_some() && cached_analysis.is_some();
        // Pure-CPU tokenisation on a worker so a parser panic is contained
        // as a JSON-RPC error.  The text goes in behind an `Arc` so the
        // convergence continuation below shares this buffer instead of taking a
        // second full-document copy.
        let text: Arc<str> = Arc::clone(&doc.text);
        let dialect = doc.dialect.clone();
        let serve_text = Arc::clone(&text);
        let serve_dialect = dialect.clone();
        let serve_registry = Arc::clone(&registry);
        let core_data = crate::rt::spawn_blocking(move || {
            range_token_data(
                had_analysis_handles,
                cached_analysis.as_deref(),
                &serve_text,
                core_range,
                &serve_dialect,
                &serve_registry,
                cached_cu.as_deref(),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("semantic_tokens_range worker panicked: {err}").into(),
            data: None,
        })?;

        // Convergence: if the coarse tier was served because the
        // enriched reads overran the budget, detach a continuation that awaits
        // them and refreshes once the enriched viewport differs.
        if let Some(pending) = pending {
            // At most one continuation per document: a client scrolling
            // a cold file issues a viewport request per frame, and each detached
            // continuation holds the document and queues a blocking recompute
            // only to ask for the same workspace-wide refresh the pending one
            // will already ask for.
            if let Some(guard) = ConvergenceGuard::claim(&self.semantic_tokens_convergence, uri) {
                self.spawn_range_convergence(
                    RangeConvergenceInputs {
                        uri: uri.as_str().to_owned(),
                        served: core_data.clone(),
                        registry,
                        text,
                        dialect,
                        range: core_range,
                    },
                    pending,
                    guard,
                );
            } else {
                // Settle here: the in-flight continuation settles under its own
                // request's marker, so without this one an observer waiting on
                // *this* request's decision would wait for a line that never
                // comes.
                log_range_convergence_settled(
                    &self.client,
                    uri.as_str(),
                    false,
                    RangeSettleOutcome::Coalesced,
                )
                .await;
            }
        } else {
            // No continuation to detach, so settle here instead: this request's
            // convergence decision is already final and an observer waiting on
            // the marker must not be left waiting for a continuation that will
            // never run. Which of the two no-continuation cases it was is
            // recorded in `outcome`.
            let (refreshed, outcome) =
                self.inline_range_settlement(served_enriched, had_analysis_handles);
            log_range_convergence_settled(&self.client, uri.as_str(), refreshed, outcome).await;
        }

        Ok(Some(SemanticTokensRangeResult::Tokens(LspSemanticTokens {
            result_id: None,
            data: lift_semantic_token_data(&core_data),
        })))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<WorkspaceSymbolResponse>> {
        let diagnostic = WorkspaceSymbolDiagnostic::new();
        // Workspace-wide query (no document URI): gate on the process-global
        // `workspaceSymbols` toggle.
        if !self
            .feature_toggles
            .lock()
            .await
            .is_enabled("workspaceSymbols")
        {
            diagnostic.record("endpoint.disabled", &[]);
            diagnostic.emit(&self.client, "none").await;
            return Ok(None);
        }
        // Answered from the workspace index, not from the open-document map.
        // VS Code re-issues this request on every keystroke in its Ctrl+T box,
        // so cloning the whole document store — every buffer and its
        // `LineIndex` — and serially re-analysing each open document per
        // keystroke would be costly *and* would miss every symbol in a file
        // the editor had not opened.  The index already holds each of them,
        // filters before materialising, and caps the answer.
        //
        // Freshness: the index is refreshed on each document's diagnostics
        // publish, which the debounce puts ~50 ms behind an edit.  A symbol
        // picker tolerates that — it is the same staleness every other
        // cross-document feature answers with.  What it does *not* tolerate is
        // an index that has not been populated at all yet, so wait out the
        // initial folder scan first.  The `didOpen`/`didChange`
        // barrier comes first for the same reason every other reader takes it:
        // a query issued after an open must observe that open.
        self.edits_settled().await;
        self.wait_for_workspace_scan().await?;
        let query = params.query.as_str();
        // A document indexed again while its rows were being mapped is
        // captured again; only the last attempt answers without it.
        let attempts = std::sync::atomic::AtomicUsize::new(0);
        let result = readonly_reference_reports::retry_readonly_attempt(None, || {
            let attempt = attempts.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let moved = if attempt + 1 < readonly_reference_reports::READONLY_CURRENCY_ATTEMPTS {
                readonly_reference_reports::MovedIndexRows::Recapture
            } else {
                readonly_reference_reports::MovedIndexRows::Omit
            };
            let diagnostic = &diagnostic;
            async move {
                if attempt != 0 {
                    let _ = crate::rt::timeout(
                        readonly_reference_reports::source_views::SOURCE_VIEW_CONTENTION_BACKOFF,
                        self.live_publication_advanced.notified(),
                    )
                    .await;
                }
                self.workspace_symbol_attempt_diagnostic(query, diagnostic, moved)
                    .await
            }
        })
        .await
        .map_err(Into::into);
        diagnostic
            .emit(
                &self.client,
                match &result {
                    Ok(Some(_)) => "some",
                    Ok(None) => "none",
                    Err(_) => "error",
                },
            )
            .await;
        result
    }

    async fn document_link(
        &self,
        params: DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<DocumentLink>>> {
        if !self
            .feature_enabled("documentLinks", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        // BIG-IP `.conf`/`.scf`: links are object references (iRule-body and
        // supported TMSH property values) resolving to the target object's
        // stanza in the same document — a `uri#L<line>` fragment anchor. This
        // is a different engine from the Tcl `source`/`package require` link
        // scanner below.
        if Self::is_bigip_dialect(&doc.dialect) {
            let text = doc.text.clone();
            let bigip_links =
                crate::rt::spawn_blocking(move || tcl_bigip::links::document_links(&text))
                    .await
                    .map_err(|err| jsonrpc::Error {
                        code: jsonrpc::ErrorCode::InternalError,
                        message: format!("bigip document_link worker panicked: {err}").into(),
                        data: None,
                    })?;
            if bigip_links.is_empty() {
                return Ok(None);
            }
            let uri_str = uri.as_str();
            let lifted = bigip_links
                .into_iter()
                .map(|l| DocumentLink {
                    range: lift_bigip_range(l.range),
                    // Resolved → a `uri#L<1-based-line>` fragment the editor
                    // navigates to; unresolved → no target (hover-only).
                    target: l
                        .target_line
                        .and_then(|line| Uri::from_str(&format!("{uri_str}#L{}", line + 1)).ok()),
                    tooltip: Some(l.tooltip),
                    data: None,
                })
                .collect();
            return Ok(Some(lifted));
        }
        // Pass the document's
        // enclosing directory as the workspace root so
        // relative `source <path>` arguments resolve.  When
        // the URI isn't a `file://` URL we leave the workspace
        // root unset and only absolute paths surface as links.
        let doc_path = uri
            .document_path()
            .and_then(|p| p.to_str().map(str::to_owned));
        let workspace_root = uri
            .document_path()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
            .and_then(|p| p.to_str().map(str::to_owned));
        // The document's own path is what `[info script]` answers while it is
        // being sourced, so a computed `source [file dirname [info script]]`
        // path resolves through the same evaluator the source graph uses
        // instead of being fabricated from raw text.
        let home = std::env::var("HOME").ok();
        // The document's imported constants, so a link through
        // a value an ancestor document assigns resolves exactly as the same
        // line resolves for navigation.
        let imported = {
            let index = self.workspace_index.read().await;
            index.imported_path_constants_for(uri.as_str())
        };
        // Pure-CPU segmentation on a worker so a parser panic is contained
        // as a JSON-RPC error.
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let text = doc.text.clone();
        let links = crate::rt::spawn_blocking(move || {
            core_document_links::document_links_from_analysis(
                &text,
                &analysis,
                &core_document_links::LinkContext {
                    imported_constants: Some(&imported),
                    workspace_root: workspace_root.as_deref(),
                    home: home.as_deref(),
                    script_path: doc_path.as_deref(),
                },
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("document_link worker panicked: {err}").into(),
            data: None,
        })?;
        if links.is_empty() {
            return Ok(None);
        }
        let lifted = links
            .into_iter()
            .map(|l| DocumentLink {
                range: Range {
                    start: Position {
                        line: l.start_line,
                        character: l.start_character,
                    },
                    end: Position {
                        line: l.end_line,
                        character: l.end_character,
                    },
                },
                target: Uri::from_str(&l.target).ok(),
                tooltip: l.tooltip,
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> jsonrpc::Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri.clone();
        // Inlay hints are opt-in (default off), with the type-hint and
        // parameter-hint families gated independently.  The provider must
        // still answer with a well-formed (empty) list — never an error or
        // `null` — when both are disabled.
        let type_hints = self.inlay_family_enabled(&uri, "inlayTypeHints").await;
        let parameter_hints = self.inlay_family_enabled(&uri, "inlayParameterHints").await;
        if !type_hints && !parameter_hints {
            return Ok(Some(Vec::new()));
        }
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // Build analysis on a worker so the inlay-hints provider can surface
        // inferred-type hints (`inlayTypeHints`) and parameter-name hints at
        // user-proc call sites plus built-in command synopsis hints
        // (`inlayParameterHints`).  When the analyser surfaces an empty
        // all_procs map (no user procs in the document), the provider still
        // returns built-in hints from the registry.
        // The parameter labels name the reached proc's parameters, so the
        // hint provider needs the same whole-program export view definition
        // and hover use — a `-force` shadow changes which proc that is.
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let hints = crate::rt::spawn_blocking(move || {
            core_inlay_hints::inlay_hints_in_program(
                &doc.text,
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
                range,
                Some(&analysis),
                core_definition::CallResolution {
                    registry: Some(&registry),
                    program: Some(core_definition::ProgramExports {
                        uri: &uri_key,
                        oracle: exports.as_ref(),
                    }),
                },
                type_hints,
                parameter_hints,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("inlay_hint worker panicked: {err}").into(),
            data: None,
        })?;
        if hints.is_empty() {
            return Ok(None);
        }
        let lifted = hints
            .into_iter()
            .map(|h| InlayHint {
                position: Position {
                    line: h.position_line,
                    character: h.position_character,
                },
                label: InlayHintLabel::String(h.label),
                kind: Some(match h.kind {
                    core_inlay_hints::InlayHintKind::Type => InlayHintKind::TYPE,
                    core_inlay_hints::InlayHintKind::Parameter => InlayHintKind::PARAMETER,
                }),
                text_edits: None,
                tooltip: None,
                padding_left: h.padding_left.then_some(true),
                padding_right: None,
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn code_lens(&self, params: CodeLensParams) -> jsonrpc::Result<Option<Vec<CodeLens>>> {
        if !self
            .feature_enabled("codeLens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            let workspace = self.workspace_index.read().await;
            let lenses = core_code_lens::code_lenses(
                &doc.text,
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
                Some(&analysis),
                Some(&workspace),
                uri.as_str(),
            );
            drop(workspace);
            // The unresolved tag follows the same single-document census
            // scope as the resolved report.
            let retained_documents = if lenses.iter().any(|lens| {
                lens.reference_coverage
                    .is_some_and(readonly_reference_reports::is_source_census)
            }) {
                // An unavailable inventory degrades the tag, never the request.
                self.original_declaration_documents(&uri)
                    .await
                    .map_or(0, |documents| documents.len())
            } else {
                1
            };
            let mut lifted = Vec::new();
            for lens in lenses {
                let Some(mut data) = self
                    .retain_original_declaration_data(
                        &uri,
                        &doc.text,
                        &analysis,
                        lens.identity.as_ref(),
                    )
                    .await
                else {
                    continue;
                };
                data["uri"] = serde_json::Value::String(uri.to_string());
                // Presentation only: resolve reads the declaration handle.
                data["qname"] = serde_json::Value::String(lens.qname);
                if let Some(coverage) = lens.reference_coverage {
                    data["sourceReferenceCoverage"] =
                        serde_json::json!(readonly_reference_reports::coverage_tag(
                            readonly_reference_reports::single_document_coverage(
                                coverage,
                                retained_documents,
                            )
                        ));
                }
                lifted.push(CodeLens {
                    range: lift_lsp_range(lens.range),
                    data: Some(data),
                    command: None,
                });
            }
            return Ok((!lifted.is_empty()).then_some(lifted));
        }
        Ok(self
            .readonly_logical_lenses(&uri)
            .await?
            .filter(|lenses| !lenses.is_empty()))
    }

    /// Resolve a current readonly source lens by its retained opaque identity.
    /// Client labels and old ranges cannot select a declaration. Native and
    /// Logical schemas remain separate; partial positive counts never close a
    /// reference census, and a stale or unavailable selection stays inert.
    async fn code_lens_resolve(&self, lens: CodeLens) -> jsonrpc::Result<CodeLens> {
        if lens
            .data
            .as_ref()
            .is_some_and(|data| data.get("originalDeclaration").is_some())
        {
            let mut lens = lens;
            lens.command = None;
            let Some(uri) = lens
                .data
                .as_ref()
                .and_then(|data| data.get("uri"))
                .and_then(serde_json::Value::as_str)
                .and_then(|uri| uri.parse::<Uri>().ok())
            else {
                return Ok(lens);
            };
            let Some(doc) = self.read_document(&uri).await? else {
                return Ok(lens);
            };
            let analysis = self
                .analysis_for(&uri, doc.text.clone(), doc.dialect)
                .await?;
            let Some(identity) = self
                .resolve_original_declaration_data(lens.data.as_ref(), &uri, &doc.text, &analysis)
                .await
            else {
                return Ok(lens);
            };
            let index = tcl_lexer::LineIndex::new_lsp(&doc.text);
            let selected = index.position_at_utf16(identity.span().start(), &doc.text);
            let position = Position {
                line: selected.line,
                character: selected.character.get(),
            };
            let report = self
                .readonly_original_declaration_reference_report(&identity, &uri, false)
                .await?
                .unwrap_or_else(
                    readonly_reference_reports::OriginalReadonlyReferenceReport::unavailable,
                );
            if let Some(data) = lens.data.as_mut() {
                data["sourceReferenceCoverage"] =
                    serde_json::json!(readonly_reference_reports::coverage_tag(report.coverage));
            }
            lens.command = Some(report.lens_command(&uri, position));
            return Ok(lens);
        }
        let mut lens = lens;
        lens.command = None;
        let Some((uri, handle)) = lens.data.as_ref().and_then(|data| {
            let uri = data.get("uri")?.as_str()?.parse::<Uri>().ok()?;
            let handle = serde_json::from_value(data.get("logicalSourceLens")?.clone()).ok()?;
            Some((uri, handle))
        }) else {
            // Old qname/range payloads and unknown handles cannot reissue a
            // source declaration or borrow the current document's authority.
            return Ok(lens);
        };
        Ok(self
            .readonly_logical_lens_resolve(&uri, handle)
            .await?
            .unwrap_or(lens))
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<Vec<CodeActionOrCommand>>> {
        if !self
            .feature_enabled("codeActions", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let (target, request) = (&uri, &params);
        readonly_reference_reports::retry_readonly_attempt(None, move || {
            self.code_action_attempt(target, request)
        })
        .await
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "tcl-lsp.minifyDocument" => self.minify_document_command(&params.arguments).await,
            "tcl-lsp.captureDocumentSourceContext" => {
                self.capture_document_source_context_command(&params.arguments)
                    .await
            }
            "tcl-lsp.validateDocumentSourceContext" => {
                self.validate_document_source_context_command(&params.arguments)
                    .await
            }
            "tcl-lsp.validateDocumentTransform" => {
                self.validate_document_transform_command(&params.arguments)
                    .await
            }
            "tcl-lsp.optimiseDocument" => self.optimise_document_command(&params.arguments).await,
            "tcl-lsp.unminifyError" => Ok(Self::unminify_error_command(&params.arguments)),
            "tcl-lsp.describeIruleEvent" => {
                self.describe_irule_event_command(&params.arguments).await
            }
            "tcl-lsp.describeIruleCommand" => {
                self.describe_irule_command_command(&params.arguments).await
            }
            "tcl-lsp.listIruleEvents" => Ok(Some(Self::list_irule_events_command())),
            "tcl-lsp.diagramData" => Ok(self.diagram_data_command(&params.arguments).await),
            "tcl-lsp.xcTranslate" => Ok(self.xc_translate_command(&params.arguments).await),
            "tcl-lsp.xcTranslateDocument" => {
                self.xc_translate_document_command(&params.arguments).await
            }
            "tcl-lsp.listRules" => self.list_rules_command(&params.arguments).await,
            "tcl-lsp.extractRule" => self.extract_rule_command(&params.arguments).await,
            "tcl-lsp.writeRuleBack" => self.write_rule_back_command(&params.arguments).await,
            "tcl-lsp.extractLinkedObjects" => {
                self.extract_linked_objects_command(&params.arguments).await
            }
            "tcl-lsp.bigipCleanup" => self.bigip_cleanup_command(&params.arguments).await,
            "tcl-lsp.minimizeDiagnostic" => {
                self.minimize_diagnostic_command(&params.arguments).await
            }
            "tcl-lsp.renamePartition" => self.rename_partition_command(&params.arguments).await,
            "tcl-lsp.validateDiagnosticPublication" => {
                self.validate_diagnostic_publication_command(&params.arguments)
                    .await
            }
            "tcl-lsp.currentDiagnostics" => {
                self.current_diagnostics_command(&params.arguments).await
            }
            "tcl-lsp.proposalDiagnostics" => {
                self.proposal_diagnostics_command(&params.arguments).await
            }
            "tcl-lsp.getEffectiveConfig" => {
                self.get_effective_config_command(&params.arguments).await
            }
            "tcl-lsp.fixAllSafeIssues" => self.fix_all_safe_issues_command(&params.arguments).await,
            "tcl-lsp.listSubcommands" => Ok(self.list_subcommands_command(&params.arguments).await),
            "tcl-lsp.insertPackageRequire" => {
                self.insert_package_require_command(&params.arguments).await
            }
            "tcl-lsp.listKnownPackages" => Ok(Some(Self::list_known_packages_command())),
            "tcl-lsp.suggestPackagesForSymbol" => Ok(Some(
                Self::suggest_packages_for_symbol_command(&params.arguments),
            )),
            "tcl-lsp.exportConfig" => Ok(Some(self.export_config_command().await)),
            "tcl-lsp.listTclInstallations" => Ok(Some(self.list_tcl_installations_command().await)),
            "tcl-lsp.listDialects" => Ok(Some(Self::list_dialects_command())),
            "tcl-lsp.setDialect" => self.set_dialect_command(&params.arguments).await,
            "tcl-lsp.setSessionDialectOverride" => {
                self.set_session_dialect_override_command(&params.arguments)
                    .await
            }
            "tcl-lsp.studioSource" => self.studio_source_command(&params.arguments).await,
            "tcl-lsp.setDocumentDialectOverride" => {
                self.set_document_dialect_override_command(&params.arguments)
                    .await
            }
            "tcl-lsp.compilerExplorer" => self.compiler_explorer_command(&params.arguments).await,
            "tcl-lsp.compilerExplorerView" => {
                self.compiler_explorer_view_command(&params.arguments).await
            }
            "tcl-lsp.tkPreview" => self.tk_preview_command(&params.arguments).await,
            "tcl-lsp.ilxReferences" => Ok(self.ilx_references_command(&params.arguments).await),
            _ => Ok(None),
        }
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_local_document(&params.text_document.uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(
                &params.text_document.uri,
                doc.text.clone(),
                doc.dialect.clone(),
            )
            .await?;
        // Build from the resolved `tclLsp.formatting` settings; the client's
        // `FormattingOptions.tabSize` / `insertSpaces` override indentation by
        // LSP contract.
        let formatting = self.resolved_formatting(&params.text_document.uri).await;
        let config = formatter_config_from(
            &formatting,
            &params.options,
            tcl_lsp_core::profile_for_dialect(&doc.dialect),
        );
        // Pure-CPU formatting on a worker so a parser panic is contained as
        // a JSON-RPC error.  The formatter is one of the two consumers that
        // must see the document's *real* terminators (`lineEnding: auto`
        // sniffs them, and the whole-document replace range is measured
        // against them) — see `DocumentState::raw`.
        let text = doc.raw().to_owned();
        let edits = crate::rt::spawn_blocking(move || {
            core_formatting::formatting_with_analysis(&text, &config, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("formatting worker panicked: {err}").into(),
            data: None,
        })?;
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_local_document(&params.text_document.uri).await? else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let analysis = self
            .analysis_for(
                &params.text_document.uri,
                doc.text.clone(),
                doc.dialect.clone(),
            )
            .await?;
        let formatting = self.resolved_formatting(&params.text_document.uri).await;
        let config = formatter_config_from(
            &formatting,
            &params.options,
            tcl_lsp_core::profile_for_dialect(&doc.dialect),
        );
        // Pure-CPU formatting on a worker so a parser panic is contained as
        // a JSON-RPC error.
        let text = doc.raw().to_owned();
        let edits = crate::rt::spawn_blocking(move || {
            core_formatting::range_formatting_with_analysis(&text, range, &config, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("range_formatting worker panicked: {err}").into(),
            data: None,
        })?;
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<SelectionRange>>> {
        if !self
            .feature_enabled("selectionRange", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let positions = params.positions;
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let result = crate::rt::spawn_blocking(move || {
            positions
                .into_iter()
                .map(|pos| {
                    let chain = core_selection_range::selection_range_for_dialect(
                        &doc.text,
                        pos.line,
                        pos.character,
                        Some(&analysis),
                        tcl_lsp_core::profile_for_dialect(&doc.dialect),
                    );
                    // The LSP spec requires `result[i]` to answer
                    // `positions[i]`, so every position must yield a range.
                    // When no chain is found (e.g. the cursor sits on empty
                    // space), fall back to a degenerate range at the cursor
                    // itself rather than dropping the entry, which would
                    // misalign the client's cursor-to-range pairing.
                    materialise_selection_range(&chain).unwrap_or(SelectionRange {
                        range: Range {
                            start: pos,
                            end: pos,
                        },
                        parent: None,
                    })
                })
                .collect::<Vec<SelectionRange>>()
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("selection_range worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(result))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<PrepareRenameResponse>> {
        let conditional_uri = params.text_document.uri.clone();
        let conditional_uri_generation = self.live_publication_uri_generation(&conditional_uri);
        if !self
            .feature_enabled("rename", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let pos = params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if let std::ops::ControlFlow::Break(prepared) = self
            .original_command_prepare_tier(&uri, &doc, &analysis, pos)
            .await?
        {
            return Ok(prepared);
        }
        let text = doc.text.clone();
        let analysis_for_worker = analysis.clone();
        // A rename started from a `-force`-shadowed call must offer the
        // definition that call reaches, not the local one the import deleted —
        // the same view definition uses.
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let result = crate::rt::spawn_blocking(move || {
            core_rename::prepare_rename_in_program(
                &text,
                pos.line,
                pos.character,
                &analysis_for_worker,
                core_definition::CallResolution::document_only().in_program(
                    core_definition::ProgramExports {
                        uri: &uri_key,
                        oracle: exports.as_ref(),
                    },
                ),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("prepare_rename worker panicked: {err}").into(),
            data: None,
        })?;
        if let Some(p) = result {
            return Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                range: lift_lsp_range(p.range),
                placeholder: p.placeholder,
            }));
        }
        if let ControlFlow::Break(prepared) = self
            .conditional_variable_prepare_tier(&uri, &doc, pos, conditional_uri_generation)
            .await?
        {
            return Ok(prepared);
        }
        if Self::original_namespace_cursor_retained(&doc.text, &analysis, pos)
            || tcl_lsp_core::variable_symbol::select(&doc.text, &analysis, pos.line, pos.character)
                .is_break()
            || tcl_lsp_core::method_symbol::select(&doc.text, &analysis, pos.line, pos.character)
                .is_break()
        {
            return Ok(None);
        }
        // Consumer-document fall-through: the local analysis has no
        // declaration to anchor prepare on, but the cursor symbol may still
        // resolve through the workspace index (a sibling document or an
        // autoloaded library defines it) — exactly the case the rename
        // handler's workspace-resolved branch serves.  VS Code gates every
        // rename behind prepare, so refusing here would make that branch
        // unreachable from the editor.  Accept with the call-site word's own
        // range when the workspace resolves it.
        let Some(word_p) = core_rename::word_prepare_at(&doc.text, pos.line, pos.character) else {
            return Ok(None);
        };
        if !self
            .resolve_workspace_symbols_for_binding(&uri, &doc.text, &analysis, pos)
            .await?
            .is_empty()
        {
            return Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                range: lift_lsp_range(word_p.range),
                placeholder: word_p.placeholder,
            }));
        }
        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        // A refusal computed against a workspace snapshot that moved during
        // the request says nothing about the current workspace: plan again.
        for attempt in 0..readonly_reference_reports::READONLY_CURRENCY_ATTEMPTS {
            if attempt != 0 {
                let _ = crate::rt::timeout(
                    readonly_reference_reports::source_views::SOURCE_VIEW_CONTENTION_BACKOFF,
                    self.live_publication_advanced.notified(),
                )
                .await;
            }
            let mut plan = None;
            let result = self.rename_attempt(params.clone(), &mut plan).await?;
            let Some(plan) = plan else {
                return Ok(result);
            };
            if result.is_some() || self.workspace_source_plan_is_current(plan).await {
                return Ok(result);
            }
        }
        Ok(None)
    }

    /// Project source-reference edits for an explicit host file rename.
    /// Each literal proposal retains its actual FILE role and source owner;
    /// current document/index joins precede versioned edit publication.
    async fn will_rename_files(
        &self,
        params: RenameFilesParams,
    ) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        let Some(inputs) = self.file_rename_input_snapshot().await else {
            return Ok(None);
        };
        // Workspace-wide file operation: gate on the process-global
        // `workspaceFileOps` toggle.
        if !self
            .feature_toggles
            .lock()
            .await
            .is_enabled("workspaceFileOps")
        {
            return Ok(None);
        }
        let Some(roots) = self
            .workspace_folder_urls()
            .await
            .iter()
            .map(uri_norm::file_action_path)
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };
        let Some(renames) = Self::file_rename_paths(&params) else {
            return Ok(None);
        };
        // The roster schedules work only. Refresh each document from its
        // genuine current analysis, then derive proposals from that same owner.
        let (candidates, mut generation) = {
            let index = self.workspace_index.read().await;
            let mut candidates: Vec<_> = index.sources().map(|row| row.uri.clone()).collect();
            candidates.sort();
            candidates.dedup();
            (candidates, index.generation())
        };
        let mut current_sources = HashMap::new();
        for owner in candidates {
            if self
                .refresh_file_rename_source(&owner, &mut current_sources, &mut generation, &inputs)
                .await?
                .is_break()
            {
                return Ok(None);
            }
        }
        let raw_edits: Vec<core_file_ops::RenameEdit> = {
            let index = self.workspace_index.read().await;
            if index.generation() != generation {
                return Ok(None);
            }
            renames
                .iter()
                .flat_map(|(old, new)| {
                    core_file_ops::FileRenameSourceAction::new(old, new, &roots).map_or_default(
                        |action| {
                            core_file_ops::compute_rename_edits(&action, &index, &|owner| {
                                uri_norm::file_action_path(&owner.parse::<Uri>().ok()?)
                            })
                        },
                    )
                })
                .collect()
        };
        if raw_edits.is_empty() {
            return Ok(None);
        }
        let mut by_dep: HashMap<Uri, Vec<core_file_ops::RenameEdit>> = HashMap::new();
        for edit in raw_edits {
            let Ok(uri) = edit.uri.parse::<Uri>() else {
                return Ok(None);
            };
            by_dep.entry(uri).or_default().push(edit);
        }
        let Some(held) = self
            .hold_file_rename_edits(by_dep, &mut current_sources)
            .await?
        else {
            return Ok(None);
        };
        // Closed files have no client version; re-read them before the final
        // open-document/index join rather than publishing stale indexed spans.
        for (uri, document, _, _) in &held {
            let Some(current) = self.read_document(uri).await? else {
                return Ok(None);
            };
            if current.text != document.text
                || current.dialect != document.dialect
                || current.revision != document.revision
                || current.version != document.version
            {
                return Ok(None);
            }
        }
        Ok(self
            .file_rename_edit_if_current(held, generation, &inputs)
            .await)
    }

    /// `workspace/didRenameFiles`: after the client applies a
    /// rename on disk, refresh the workspace — retire the old URI entirely
    /// and re-index the renamed file from its new path so cross-document
    /// features (including future renames) stay current.
    async fn did_rename_files(&self, params: RenameFilesParams) {
        // Same workspace-file-ops gate as `will_rename_files`.
        if !self
            .feature_toggles
            .lock()
            .await
            .is_enabled("workspaceFileOps")
        {
            return;
        }
        let mut prepared = Vec::with_capacity(params.files.len());
        for file in &params.files {
            let mut pair = Vec::with_capacity(2);
            for raw in [&file.old_uri, &file.new_uri] {
                let parsed = match Uri::from_str(raw) {
                    Ok(uri) => uri,
                    Err(error) => {
                        self.client
                            .log_message(
                                MessageType::WARNING,
                                format!("file rename URI is unavailable: {error}"),
                            )
                            .await;
                        return;
                    }
                };
                match self.document_path_admission(&parsed).await {
                    Ok(admission) => pair.push((parsed, admission)),
                    Err(error) => {
                        self.client
                            .log_message(MessageType::WARNING, error.to_string())
                            .await;
                        return;
                    }
                }
            }
            prepared.push(pair);
        }
        for pair in &prepared {
            for (uri, admission) in pair {
                if let Err(error) = admission.check_for(uri, self.store.as_ref()) {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            }
        }
        for pair in prepared {
            let mut pair = pair.into_iter();
            let (old, old_admission) = pair
                .next()
                .expect("the prepared pair includes the original URI");
            let (new, new_admission) = pair.next().expect("the prepared pair includes the new URI");
            if let Err(error) = self.retire_renamed_uri(&old, old_admission).await {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
            if let Err(error) = self
                .reindex_index_from_disk_using(&new, Some(new_admission))
                .await
            {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> jsonrpc::Result<Option<SignatureHelp>> {
        if !self
            .feature_enabled(
                "signatureHelp",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let pos = position_within_line(&doc.text, params.text_document_position_params.position);
        let disabled_commands = self.resolved_signature_help_disabled_commands(&uri).await;
        // Per-dialect cached registry — same source the
        // completion handler uses.  Threading it through lets
        // `S-signature-help-rich` surface signatures for
        // built-in commands (e.g. `puts`, `lsearch`) without
        // requiring a user proc with the same name.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        // Same whole-program view as hover: the signature rendered is the
        // reached proc's parameter list, which a `-force` shadow changes.
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let result = crate::rt::spawn_blocking(move || {
            core_sig::signature_help_in_program_with_options(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                core_definition::CallResolution {
                    registry: Some(&registry),
                    program: Some(core_definition::ProgramExports {
                        uri: &uri_key,
                        oracle: exports.as_ref(),
                    }),
                },
                core_sig::SignatureHelpOptions {
                    disabled_builtin_commands: &disabled_commands,
                },
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("signature_help worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(result.map(lift_signature_help))
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        if !self
            .feature_enabled(
                "hover",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let source_epoch = self.diag_inputs_epoch();
        let source_generation = self.live_publication_uri_generation(&uri);
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        // Run the provider over the memoised `file_analysis` query (no separate
        // hover result cache — walking the cached analysis is cheap, and the
        // query graph already avoids re-analysing).  Worker-offload via
        // `spawn_blocking` keeps the LSP event loop responsive.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if let std::ops::ControlFlow::Break(hover) = self
            .original_source_view_hover_route_for_snapshot(&uri, &doc.text, &analysis, pos)
            .await?
        {
            return Ok(hover.map(lift_hover));
        }

        // `profile_for_dialect`, not `by_name`: this is a dialect ingress, and
        // `by_name` sinks the additive set-only spelling `tk` to the plain
        // fallback. That is not a repair — hover answers a `tk` document
        // correctly either way, because the registry beside this profile is
        // keyed by the dialect *name* and the plain-Tcl fallback is permissive
        // enough to resolve its rows. Resolving here forecloses the divergence
        // instead: the profile and the registry now agree about what `tk` is,
        // so a future gate that does consult the profile cannot silently
        // disagree with the command surface. See
        // `hover_on_a_tk_document_resolves_a_tk_command`.
        let hover_profile = tcl_lsp_core::profile_for_dialect(&doc.dialect);
        // The cross-document fallback must only fire on a command head, the
        // same gate `compute_definition` applies — otherwise an argument word
        // that happens to share a sibling proc's name would pop up that proc's
        // signature.  Computed here, while `analysis` is still in scope.
        let on_command_head = position_is_command_head(&doc.text, pos, &analysis, &doc.line_index);
        if let ControlFlow::Break(hover) = self
            .position_hover_before_method(&uri, &doc, &analysis, pos)
            .await?
        {
            return Ok(hover);
        }
        if let Some(hover) = self
            .conditional_foreign_factory_source_hover(
                &uri,
                &doc,
                source_epoch,
                source_generation,
                pos,
                &analysis,
            )
            .await
        {
            return Ok(Some(lift_hover(hover)));
        }
        // A member route with no card still lets the document's own hover
        // answer from its declared classes; only the workspace fallback,
        // which knows nothing of the member, is withheld.
        let member_route_declined = match Box::pin(self.original_method_hover_for_snapshot(
            (uri.clone(), doc.clone(), source_generation),
            &analysis,
            pos,
            source_epoch,
        ))
        .await?
        {
            std::ops::ControlFlow::Break(Some(hover)) => return Ok(Some(lift_hover(hover))),
            std::ops::ControlFlow::Break(None) => true,
            std::ops::ControlFlow::Continue(()) => false,
        };
        let text = doc.text.clone();
        let analysis_worker = Arc::clone(&analysis);
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let result = crate::rt::spawn_blocking(move || {
            core_hover::hover_in_program(
                &text,
                pos.line,
                pos.character,
                &analysis_worker,
                Some(&registry),
                hover_profile,
                Some(core_definition::ProgramExports {
                    uri: &uri_key,
                    oracle: exports.as_ref(),
                }),
            )
        })
        .await
        .map_err(|err| Self::hover_worker_error(&err))?;
        if let Some(hover) = result {
            return Ok(Some(lift_hover(hover)));
        }
        if member_route_declined {
            return Ok(None);
        }
        self.hover_workspace_fallback(&uri, &doc, &analysis, pos, on_command_head)
            .await
    }
}
