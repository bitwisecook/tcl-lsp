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

//! The document lifecycle request handlers: open, change, close and save.

use crate::{
    Arc, Backend, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentPathAdmission, DocumentState, MessageType, Poll, TextEdit,
    WillSaveTextDocumentParams, apply_formatting_object, core_formatting, jsonrpc, lift_lsp_range,
};

impl Backend {
    pub(crate) async fn serve_did_open(&self, params: DidOpenTextDocumentParams) {
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
        if Self::is_bigip_dialect(&dialect_for_diags) {
            self.refresh_data_group_diagnostics(&uri).await;
        }
        self.reschedule_diagnostics(uri, dialect_for_diags).await;
    }

    pub(crate) async fn serve_did_change(&self, params: DidChangeTextDocumentParams) {
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
        if Self::is_bigip_dialect(&dialect) {
            self.refresh_data_group_diagnostics(&uri).await;
        }
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

    pub(crate) async fn serve_did_close(&self, params: DidCloseTextDocumentParams) {
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

        let was_configuration = {
            // The ordered mutation is removal of the authoritative live
            // buffer. Index/caches are derived publications and happen only
            // after the turn is free.
            turn.at("did_close: documents.lock");
            let mut docs = self.documents.lock("did_close").await;
            let removed = DocumentPathAdmission::with_current(
                &[admission.as_ref()],
                self.store.as_ref(),
                || {
                    docs.remove(uri)
                        .is_some_and(|doc| Self::is_bigip_dialect(&doc.dialect))
                },
            );
            drop(docs);
            match removed {
                Ok(was_configuration) => was_configuration,
                Err(error) => {
                    drop(turn);
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            }
        };
        self.invalidate_live_publication(std::iter::once(uri));
        drop(turn);
        self.live_publication_advanced.notify_waiters();
        if was_configuration {
            self.refresh_data_group_diagnostics(uri).await;
        }

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

    pub(crate) async fn serve_will_save_wait_until(
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
}
