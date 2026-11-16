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

//! `workspace/didChangeWatchedFiles`: classification of a batch and its effect on open and closed files.

use super::*;

impl Backend {
    pub(in super::super) async fn partition_open_watched_paths(
        &self,
        last_kind: HashMap<Uri, FileChangeType>,
        deleted: &mut Vec<Uri>,
        deleted_while_open: &mut HashSet<Uri>,
        changed: &mut Vec<Uri>,
        revived_while_open: &mut Vec<Uri>,
    ) {
        for (uri, typ) in last_kind {
            let is_open = self
                .documents
                .lock("did_change_watched_files")
                .await
                .contains_key(&uri);
            if typ == FileChangeType::DELETED {
                if is_open {
                    deleted_while_open.insert(uri.clone());
                }
                // A DELETED event runs even for an open buffer.  The editor
                // sends no `didClose` when a file is renamed or removed
                // out-of-band (a terminal `mv`, a branch switch), so skipping
                // open URIs here would leave the dead path in `workspace_index`
                // and the salsa `Project` for the process's life — duplicate
                // workspace symbols and a go-to-definition jump to a file that
                // no longer exists, once the new path is indexed alongside it.
                // Only the
                // *index* is retired; `self.documents` is untouched, so the
                // still-open buffer keeps working exactly as `did_close`'s
                // reindex-from-disk leaves a closed-and-deleted file.
                deleted.push(uri);
            } else if !is_open {
                // CREATED or CHANGED: re-analyse from disk (a Tcl source file)
                // or drop it if it no longer reads as one.  Files the editor has
                // open are driven by did_open/did_change; their unsaved buffer
                // must not be clobbered by the on-disk copy.
                // `batch_reindex_from_disk` re-checks this under the lock as
                // well.
                changed.push(uri);
            } else {
                revived_while_open.push(uri);
            }
        }
    }

    pub(in super::super) async fn retire_watched_deleted_paths(
        &self,
        deleted: &[Uri],
        deleted_revisions: &HashMap<Uri, u64>,
        event_admissions: &HashMap<Uri, Arc<DocumentPathAdmission>>,
    ) -> bool {
        if !deleted.is_empty() {
            let batch = AdmittedDiskBatch {
                removal_admissions: deleted
                    .iter()
                    .map(|uri| Arc::clone(&event_admissions[uri]))
                    .collect(),
                ..Default::default()
            };
            match self
                .publish_scanned_disk_results(
                    &batch,
                    deleted,
                    DiskRemovalPolicy::IncludeOpen(deleted_revisions),
                    "did_change_watched_files: deleted",
                )
                .await
            {
                Ok(value) => value,
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return false;
                }
            };
            if let Err(error) = self
                .settle_deleted_open_paths(deleted_revisions, event_admissions)
                .await
            {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return false;
            }
        }
        true
    }

    pub(in super::super) async fn refresh_watched_configuration(&self) {
        match self.pull_and_apply_config().await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        match self.scan_workspace_folders().await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        // `tclLsp.specPacks` is part of what was just re-applied, so the
        // pack set may point somewhere else now.
        match self.reload_spec_packs(ReloadTrigger::Config).await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        };
        self.reschedule_all_open_documents().await;
        // Closed files that carry a badge follow the same reconfigured
        // disabled-code / master-switch state.
        self.reschedule_closed_file_diagnostics().await;
    }

    pub(in super::super) fn watched_closed_badge_uris(
        cache: &HashMap<Uri, PullDiagEntry>,
        deleted: &[Uri],
        changed: &[Uri],
        deleted_while_open: &HashSet<Uri>,
    ) -> (Vec<Uri>, Vec<Uri>) {
        (
            deleted
                .iter()
                .filter(|uri| cache.contains_key(uri) && !deleted_while_open.contains(*uri))
                .cloned()
                .collect(),
            changed
                .iter()
                .filter(|uri| cache.contains_key(uri))
                .cloned()
                .collect(),
        )
    }

    pub(in super::super) async fn restore_revived_live_documents(
        &self,
        revived_publications: Vec<(Uri, Arc<str>, String, u64)>,
    ) -> bool {
        for (uri, text, dialect, revision) in revived_publications {
            match self
                .publish_changed_document(&uri, &text, &dialect, revision, true)
                .await
            {
                Ok(value) => value,
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return false;
                }
            };
        }

        true
    }

    pub(in super::super) async fn capture_watched_event_admissions(
        &self,
        changes: &[tower_lsp_server::ls_types::FileEvent],
    ) -> Option<HashMap<Uri, Arc<DocumentPathAdmission>>> {
        let mut event_admissions = HashMap::new();
        for event in changes {
            match self.document_path_admission(&event.uri).await {
                Ok(admission) => {
                    event_admissions.insert(event.uri.clone(), admission);
                }
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return None;
                }
            }
        }
        Some(event_admissions)
    }

    /// Best-effort dynamic registration of
    /// `workspace/didChangeWatchedFiles` for Tcl source files, so external
    /// on-disk changes reach [`LanguageServer::did_change_watched_files`].
    /// A client
    /// without dynamic-registration support rejects the request; that is
    /// logged and ignored (the startup scan still seeds the index).
    pub(in super::super) async fn register_file_watchers(&self) {
        let all_kinds = Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete);
        let registration = Registration {
            id: "tcl-lsp-watched-files".to_owned(),
            method: "workspace/didChangeWatchedFiles".to_owned(),
            register_options: serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                watchers: vec![
                    FileSystemWatcher {
                        // Case-folded in the glob itself: watcher registrations
                        // carry no `ignoreCase` option and VS Code matches them
                        // case-sensitively on Linux.
                        //
                        // This also covers `.tclspec` SpecTcl packs, which are
                        // in `TCL_SOURCE_EXTENSIONS` — a pack *is* one Tcl
                        // script. A second watcher for them would only make the
                        // client send every pack event twice; what makes a pack
                        // event different is not the watch but what
                        // `did_change_watched_files` does with it (see
                        // `WatchedFileChanges::spec_pack_changed`).
                        glob_pattern: GlobPattern::String(tcl_source_watch_glob()),
                        kind: all_kinds,
                    },
                    FileSystemWatcher {
                        // Sidecars are external analysis inputs, not Tcl
                        // source documents. Their events advance the tracked
                        // sidecar epoch and republish affected open documents.
                        glob_pattern: GlobPattern::String("**/*.tcl.stubs".to_owned()),
                        kind: all_kinds,
                    },
                    FileSystemWatcher {
                        // The lockfile a pack's dependency tier is read from.
                        // `tclpkg.tcl` beside it is a `.tcl` file, so the source
                        // watcher above reports it; `did_change_watched_files`
                        // reloads the packs for either.
                        glob_pattern: GlobPattern::String(PACKAGE_LOCKFILE_GLOB.to_owned()),
                        kind: all_kinds,
                    },
                    FileSystemWatcher {
                        // The project config the layered settings live-reload
                        // from ([`is_config_file`]). Registered here rather than
                        // left to each client's own `synchronize.fileEvents`, so
                        // the live-reload works in every editor and there is one
                        // source of truth for what the server watches.
                        glob_pattern: GlobPattern::String(PROJECT_CONFIG_GLOB.to_owned()),
                        kind: all_kinds,
                    },
                ],
            })
            .ok(),
        };
        if let Err(err) =
            bounded_client_request(self.client.register_capability(vec![registration])).await
        {
            self.client
                .log_message(
                    MessageType::LOG,
                    format!("file-watcher registration declined by client: {err}"),
                )
                .await;
        }
    }
}

impl Backend {
    /// Apply watched delete/revive state to buffers that remain open and
    /// return the revived snapshots that need live Salsa/index publication and
    /// the exact orphan revisions a deletion transaction may fully retire.
    pub(in super::super) async fn update_watched_open_paths(
        &self,
        deleted: &HashSet<Uri>,
        revived: &[Uri],
        event_admissions: &HashMap<Uri, Arc<DocumentPathAdmission>>,
    ) -> DocumentSourceResult<(Vec<(Uri, Arc<str>, String, u64)>, HashMap<Uri, u64>)> {
        if deleted.is_empty() && revived.is_empty() {
            return Ok((Vec::new(), HashMap::new()));
        }
        let mut docs = self.documents.lock("did_change_watched_files").await;
        let mut admissions = Vec::new();
        for uri in deleted.iter().chain(revived) {
            let admission =
                event_admissions
                    .get(uri)
                    .ok_or_else(|| DocumentSourceUnavailable::Store {
                        uri: uri.clone(),
                        cause: vfs::SourceUriPathUnavailable::ProviderUnavailable,
                    })?;
            admission.check_uri(uri)?;
            admissions.push(Arc::clone(admission));
            if let Some(retained) = docs.get(uri).and_then(|doc| doc.path_admission.as_ref()) {
                retained.check_uri(uri)?;
                admissions.push(Arc::clone(retained));
            }
        }
        let retained = admissions.iter().map(Arc::as_ref).collect::<Vec<_>>();
        let result = DocumentPathAdmission::with_current(&retained, self.store.as_ref(), || {
            let mut publications = Vec::new();
            let mut deleted_revisions = HashMap::new();
            for uri in deleted {
                if let Some(doc) = docs.get_mut(uri) {
                    doc.backing_file_deleted = true;
                    // For an orphan the current cross-document index view is
                    // deliberately absent. A revision that already reached Salsa
                    // can become fully settled immediately; a pending revision
                    // must remain pending until the deletion transaction retires
                    // the preceding Salsa handle, otherwise local providers can
                    // combine these live bytes with stale memoised analysis.
                    if doc.publication != DocumentPublication::Pending {
                        doc.publication = DocumentPublication::Indexed;
                    }
                    deleted_revisions.insert(uri.clone(), doc.revision);
                }
            }
            for uri in revived {
                if let Some(doc) = docs.get_mut(uri)
                    && doc.backing_file_deleted
                {
                    doc.backing_file_deleted = false;
                    // Retire every deferred publication captured before the
                    // deletion, then republish this unchanged editor buffer as a
                    // new analysis generation. Merely clearing the orphan flag
                    // would leave the deliberately absent index view marked
                    // settled when diagnostics are disabled/excluded.
                    doc.bump_analysis_generation();
                    publications.push((
                        uri.clone(),
                        Arc::clone(&doc.text),
                        doc.dialect.clone(),
                        doc.revision,
                    ));
                }
            }
            // A normal CREATED/CHANGED notification for an already-live open
            // buffer changes no authoritative state and starts no replacement
            // publisher. Invalidating it here would retire a contended didOpen or
            // didChange forever. Only actual orphan-to-live transitions appear in
            // `publications`; pair those with the genuine deletions.
            if !deleted.is_empty() || !publications.is_empty() {
                self.invalidate_live_publication(
                    deleted
                        .iter()
                        .chain(publications.iter().map(|(uri, ..)| uri)),
                );
                self.live_publication_advanced.notify_waiters();
            }
            (publications, deleted_revisions)
        });
        drop(docs);
        result
    }

    /// Release pending local readers only after the watched-delete transaction
    /// has retired the stale Salsa handle for the same orphan revision.
    async fn settle_deleted_open_paths(
        &self,
        deleted_revisions: &HashMap<Uri, u64>,
        event_admissions: &HashMap<Uri, Arc<DocumentPathAdmission>>,
    ) -> DocumentSourceResult<()> {
        let mut docs = self
            .documents
            .lock("did_change_watched_files: settle deleted")
            .await;
        let mut admissions = Vec::new();
        for uri in deleted_revisions.keys() {
            let admission =
                event_admissions
                    .get(uri)
                    .ok_or_else(|| DocumentSourceUnavailable::Store {
                        uri: uri.clone(),
                        cause: vfs::SourceUriPathUnavailable::ProviderUnavailable,
                    })?;
            admission.check_uri(uri)?;
            admissions.push(Arc::clone(admission));
            if let Some(retained) = docs.get(uri).and_then(|doc| doc.path_admission.as_ref()) {
                retained.check_uri(uri)?;
                admissions.push(Arc::clone(retained));
            }
        }
        let retained = admissions.iter().map(Arc::as_ref).collect::<Vec<_>>();
        let advanced = DocumentPathAdmission::with_current(&retained, self.store.as_ref(), || {
            let mut advanced = false;
            for (uri, revision) in deleted_revisions {
                if let Some(doc) = docs.get_mut(uri)
                    && doc.backing_file_deleted
                    && doc.revision == *revision
                    && doc.publication == DocumentPublication::Pending
                {
                    doc.publication = DocumentPublication::Indexed;
                    advanced = true;
                }
            }
            advanced
        })?;
        drop(docs);
        if advanced {
            self.live_publication_advanced.notify_waiters();
        }
        Ok(())
    }
}

/// Whether `uri` names a tcl-lsp config file (`.tcl-lsp.ini` project config or
/// the user `config.ini`) — a watched-file change to one triggers a config
/// re-apply.
/// Watcher glob for the project config file [`is_config_file`] recognises.
///
/// The user `config.ini` lives outside the workspace, so no workspace-relative
/// glob can reach it; it is re-read on the next pull instead.
pub(in super::super) const PROJECT_CONFIG_GLOB: &str = "**/.tcl-lsp.ini";

/// `true` when `uri` names a `.tclspec` `SpecTcl` pack.
pub(in super::super) fn is_spec_pack_file(uri: &Uri) -> bool {
    uri.document_path()
        .is_some_and(|path| tcl_spectcl::discovery::is_pack_file(&path))
}

/// Watcher glob for the lockfile a pack's dependency tier is read from
/// ([`is_package_metadata_file`]). The manifest beside it is a `.tcl` file, so
/// the source watcher already reports it.
pub(in super::super) const PACKAGE_LOCKFILE_GLOB: &str = "**/tclpkg.lock";

/// `true` when `uri` names a package manifest or lockfile: the files a pack's
/// dependency tier is read from at discovery, so a change to one changes what
/// the packs beside a manifest may declare without any pack file moving.
pub(in super::super) fn is_package_metadata_file(uri: &Uri) -> bool {
    uri.document_path()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .is_some_and(|name| {
            name.eq_ignore_ascii_case(tcl_spectcl::discovery::PACKAGE_MANIFEST)
                || name.eq_ignore_ascii_case(tcl_spectcl::discovery::PACKAGE_LOCKFILE)
        })
}

pub(in super::super) fn is_config_file(uri: &Uri) -> bool {
    let s = uri.as_str();
    s.ends_with("/.tcl-lsp.ini")
        || s.ends_with("\\.tcl-lsp.ini")
        || s.ends_with("/tcl-lsp/config.ini")
        || s.ends_with("\\tcl-lsp\\config.ini")
}

/// Whether `uri` is a workspace sidecar declaration bundle. It is purposely
/// kept outside [`is_tcl_source`]: a `.tcl.stubs` file controls analysis but is
/// never itself indexed or diagnosed as Tcl source.
fn is_sidecar_stubs_file(uri: &Uri) -> bool {
    uri.document_path()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .is_some_and(|name| {
            let name = name.to_ascii_lowercase();
            name.ends_with(".tcl.stubs") && name.len() > ".tcl.stubs".len()
        })
}

/// Split watched events into config / sidecar flags and the final event for
/// each Tcl source.  A batch may contain a path more than once, so the last
/// event wins.
pub(in super::super) fn partition_watched_file_changes(
    changes: Vec<FileEvent>,
) -> WatchedFileChanges {
    let mut partition = WatchedFileChanges::default();
    for change in changes {
        // A manifest also goes on as the Tcl source it is; the lockfile is
        // no source at all. Either moving can change a pack's tier.
        if is_package_metadata_file(&change.uri) {
            partition.spec_pack_changed = true;
        }
        if is_config_file(&change.uri) {
            partition.config_changed = true;
        } else if is_sidecar_stubs_file(&change.uri) {
            partition.sidecar_changed = true;
        } else if is_spec_pack_file(&change.uri) {
            // A `.tclspec` is an analysis *input*, never an indexed document:
            // it declares commands, it does not contain any. Routing it here
            // rather than into `last_kind` is what keeps the workspace index
            // from trying to analyse a spec pack as Tcl source.
            partition.spec_pack_changed = true;
        } else if change
            .uri
            .document_path()
            .is_none_or(|path| is_tcl_source(&path))
        {
            partition.last_kind.insert(change.uri, change.typ);
        }
    }
    partition
}

/// One `workspace/didChangeWatchedFiles` batch, split by what each path *is*.
///
/// Deduplicated to one event per URI (`last_kind`) so the disk-backed work runs
/// once per file per batch: without it a 500-file branch switch runs 500
/// sequential full analyses.
#[derive(Debug, Default)]
pub(in super::super) struct WatchedFileChanges {
    /// A layered-settings file moved.
    pub(in super::super) config_changed: bool,
    /// A `.tcl.stubs` sidecar moved.
    pub(in super::super) sidecar_changed: bool,
    /// A `.tclspec` `SpecTcl` pack moved, or a `tclpkg.tcl` / `tclpkg.lock`
    /// did, which is where the tier of the package shipping a pack is read from.
    pub(in super::super) spec_pack_changed: bool,
    /// The Tcl source files, one entry per URI, with its last event kind.
    pub(in super::super) last_kind: HashMap<Uri, FileChangeType>,
}
