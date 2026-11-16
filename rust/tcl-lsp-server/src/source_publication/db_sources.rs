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

//! The query database's source inputs and project membership.

use super::super::Backend;
use super::super::configuration::longest_folder_match;
use super::super::documents::DocumentPathAdmission;
#[cfg(test)]
use super::super::documents::DocumentSourceResult;
#[cfg(test)]
use super::super::vfs;
use super::{PublishedSourcePath, PublishedSourcePaths};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tower_lsp_server::ls_types::Uri;

impl Backend {
    /// Apply source replacements with all salsa maps already locked, returning
    /// whether the `Project` membership changed.
    ///
    /// This is the single mutation primitive shared by foreground edits and
    /// background disk publication.  Keeping normalisation, tombstone revival
    /// and class-factory seeding here prevents the optimistic background path
    /// from becoming a subtly different salsa writer.
    #[cfg(test)]
    fn set_db_sources_locked<'a>(
        db: &mut tcl_lsp_db::TclDatabase,
        files: &mut HashMap<Uri, tcl_lsp_db::SourceFile>,
        tombstones: &mut HashMap<Uri, tcl_lsp_db::SourceFile>,
        entries: impl IntoIterator<Item = (&'a Uri, &'a str, &'a str)>,
        store: &dyn vfs::SourceStore,
        published_paths: &PublishedSourcePaths,
    ) -> DocumentSourceResult<bool> {
        // Admit the complete write set before the first Salsa/tombstone mutation.
        let entries = entries
            .into_iter()
            .map(|(uri, text, dialect)| {
                Ok((DocumentPathAdmission::capture(uri, store)?, text, dialect))
            })
            .collect::<DocumentSourceResult<Vec<_>>>()?;
        let admissions = entries
            .iter()
            .map(|(admission, _, _)| admission)
            .collect::<Vec<_>>();
        DocumentPathAdmission::with_current(&admissions, store, || {
            Self::set_admitted_db_sources_locked(
                db,
                files,
                tombstones,
                entries
                    .iter()
                    .map(|(admission, text, dialect)| (admission, *text, *dialect)),
                published_paths,
            )
        })
    }

    pub(super) fn set_admitted_db_sources_locked<'a>(
        db: &mut tcl_lsp_db::TclDatabase,
        files: &mut HashMap<Uri, tcl_lsp_db::SourceFile>,
        tombstones: &mut HashMap<Uri, tcl_lsp_db::SourceFile>,
        entries: impl IntoIterator<Item = (&'a DocumentPathAdmission, &'a str, &'a str)>,
        published_paths: &PublishedSourcePaths,
    ) -> bool {
        use salsa::Setter as _;

        let oracle = Self::published_class_factories(db, files);
        let mut membership_changed = false;
        for (admission, text, dialect) in entries {
            let uri = admission.uri();
            // Salsa's `SourceFile.text` is the analyser's input, so it stores
            // the analysis form on every ingress path.
            let text = tcl_lexer::normalise_lone_cr(text).into_owned();
            if let Some(&file) = files.get(uri) {
                file.set_path(db)
                    .to(admission.analyser_path().map(str::to_owned));
                file.set_text(db).to(text);
                file.set_dialect(db).to(dialect.to_owned());
            } else {
                let file = Self::revive_or_create_db_source(
                    db,
                    tombstones,
                    admission,
                    text,
                    dialect.to_owned(),
                    oracle.clone(),
                );
                files.insert(uri.clone(), file);
                membership_changed = true;
            }
            let file = *files.get(uri).expect("the source was just published");
            published_paths
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(
                    uri.clone(),
                    PublishedSourcePath {
                        file,
                        admission: Arc::new(admission.clone()),
                    },
                );
        }
        Self::debug_assert_published_paths_in_step(db, files, published_paths);
        membership_changed
    }

    /// `published_document_path_admission` reads the admission map under
    /// `db_files` alone. That is sound only while every writer keeps both maps
    /// and each source's Salsa path in step before releasing `db_files`.
    pub(in super::super) fn debug_assert_published_paths_in_step(
        db: &tcl_lsp_db::TclDatabase,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
        published_paths: &PublishedSourcePaths,
    ) {
        if !cfg!(debug_assertions) {
            return;
        }
        let published = published_paths
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        debug_assert_eq!(published.len(), files.len());
        for (uri, file) in files {
            let retained = published.get(uri);
            debug_assert!(retained.is_some_and(|retained| retained.file == *file
                && file.path(db).as_deref() == retained.admission.analyser_path()));
        }
    }

    /// Retire sources with all salsa maps already locked, returning whether
    /// the `Project` membership changed.
    pub(super) fn remove_db_sources_locked<'a>(
        db: &mut tcl_lsp_db::TclDatabase,
        files: &mut HashMap<Uri, tcl_lsp_db::SourceFile>,
        tombstones: &mut HashMap<Uri, tcl_lsp_db::SourceFile>,
        uris: impl IntoIterator<Item = &'a Uri>,
        published_paths: &PublishedSourcePaths,
    ) -> bool {
        let mut membership_changed = false;
        for uri in uris {
            // Do not short-circuit: every URI must be retired even after the
            // first one proves that the project membership changed.
            membership_changed =
                Self::retire_db_source(db, files, tombstones, uri, published_paths)
                    || membership_changed;
        }
        membership_changed
    }

    /// Create or update a salsa `SourceFile` input in tests that need to seed
    /// the query graph directly.
    /// Lock order is always `db` → `db_files` → `db_tombstones` →
    /// `db_project_members` → `db_project`.
    ///
    /// Borrows the text rather than taking it by value: callers on the edit
    /// path hold the document snapshot's shared `Arc<str>` and must not have to
    /// own a separate copy of it.  The salsa `SourceFile::text` input is a
    /// `String`, so exactly one copy is made here.
    /// # INVARIANT (no wedged sessions): never hold a salsa snapshot across an await
    ///
    /// The `set_text` / `SourceFile` calls below take `&mut` on the database,
    /// which salsa 0.28 grants through `Storage::cancel_others`:
    ///
    /// ```text
    /// let mut clones = self.handle.coordinate.clones.lock();
    /// while *clones != 1 { clones = self.handle.coordinate.cvar.wait(clones); }
    /// ```
    ///
    /// That is a **blocking** condvar wait, on a Tokio worker thread, for every
    /// other `DatabaseImpl` clone in the process to be dropped. Called while
    /// holding the [`EditOrder`] turn, then for as long as it waits:
    ///
    /// * `now_serving` does not advance, so every later document-sync
    ///   notification blocks in `wait_turn`;
    /// * every request handler blocks in [`Backend::edits_settled`] and holds
    ///   one of `DeferredConcurrency`'s four permits;
    /// * the server answers nothing, writes nothing, and burns no CPU, while
    ///   its stdin reader goes on draining input.
    ///
    /// That is the signature of a wedged session. Live setters therefore run
    /// after the turn is released and only once the snapshot census reaches
    /// zero; the same lifetime rule is mandatory for every clone. A
    /// `db.lock().await.clone()` anywhere in this crate must be **moved straight
    /// into the worker it was cloned for** and never held across an unrelated
    /// `.await`. `scratchpad`-style audits will not catch it either — the mutex
    /// is released the instant the clone is made, so it is invisible to a
    /// lock-guard analysis; it is the *clone* Salsa waits on.
    #[cfg(test)]
    pub(in super::super) async fn db_set_source(
        &self,
        uri: &Uri,
        text: &str,
        dialect: String,
    ) -> DocumentSourceResult<()> {
        let mut db = self.db.lock().await;
        let mut files = self.db_files.lock().await;
        let mut tombstones = self.db_tombstones.lock().await;
        let membership_changed = Self::set_db_sources_locked(
            &mut db,
            &mut files,
            &mut tombstones,
            std::iter::once((uri, text, dialect.as_str())),
            self.store.as_ref(),
            self.source_path_admissions.as_ref(),
        )?;
        let mut members = self.db_project_members.lock().await;
        let member_added = members.insert(uri.clone());
        drop(tombstones);
        let _: () = if membership_changed || member_added {
            // Membership changed — re-set the `Project` file set. A text-only
            // edit leaves it untouched (and does not backdate its aggregates).
            let mut project = self.db_project.lock().await;
            let global_config = *self.db_config.lock().await;
            let folder_configs = self.folder_db_configs.lock().await;
            Self::sync_db_project_with_configurations(
                &mut db,
                &files,
                &members,
                &mut project,
                global_config,
                &folder_configs,
            );
        };
        Ok(())
    }

    /// Invalidate every tracked analysis after a sidecar-stub change. A source
    /// may inherit its nearest `<dialect>.tcl.stubs` from any ancestor, so the
    /// rare event deliberately advances every loaded source instead of trying
    /// to maintain a second, fallible ancestor dependency graph.
    pub(in super::super) async fn invalidate_sidecar_stubs(&self) {
        use salsa::Setter as _;
        let inputs = self.analyser_inputs_gate.write().await;
        self.invalidate_diag_inputs();
        let sidecar_revision = {
            let mut cache = self
                .fresh_analysis_cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            cache.clear();
            cache.revision()
        };
        let tracked: Vec<Uri> = {
            let mut db = self.db.lock().await;
            let files = self.db_files.lock().await;
            let mut uris = Vec::with_capacity(files.len());
            for (uri, file) in files.iter() {
                let next = file.sidecar_stubs_epoch(&*db).wrapping_add(1);
                file.set_sidecar_stubs_epoch(&mut *db).to(next);
                uris.push(uri.clone());
            }
            uris
        };
        self.fresh_analysis_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .complete_sidecar_invalidation(&sidecar_revision);
        drop(inputs);
        let open: Vec<(Uri, String)> = {
            let docs = self.documents.lock("invalidate_sidecar_stubs").await;
            tracked
                .into_iter()
                .filter_map(|uri| docs.get(&uri).map(|doc| (uri, doc.dialect.clone())))
                .collect()
        };
        for (uri, dialect) in open {
            self.reschedule_diagnostics(uri, dialect).await;
        }
    }

    /// Retire the salsa `SourceFile` input for `uri` (on `did_close`).  Lock
    /// order is `db` → `db_files` → `db_tombstones` →
    /// `db_project_members` → `db_project`, matching [`Self::db_set_source`].
    #[cfg(test)]
    pub(in super::super) async fn db_remove_source(&self, uri: &Uri) {
        let mut db = self.db.lock().await;
        let mut files = self.db_files.lock().await;
        let retired = {
            let mut tombstones = self.db_tombstones.lock().await;
            Self::retire_db_source(
                &mut db,
                &mut files,
                &mut tombstones,
                uri,
                self.source_path_admissions.as_ref(),
            )
        };
        let mut members = self.db_project_members.lock().await;
        let member_removed = members.remove(uri);
        if retired || member_removed {
            let mut project = self.db_project.lock().await;
            let global_config = *self.db_config.lock().await;
            let folder_configs = self.folder_db_configs.lock().await;
            Self::sync_db_project_with_configurations(
                &mut db,
                &files,
                &members,
                &mut project,
                global_config,
                &folder_configs,
            );
        }
    }

    /// Move `uri`'s live `SourceFile` handle into the tombstone map with its
    /// payload released, returning whether there was one.
    ///
    /// Salsa 0.27 allocates inputs and never frees them, so dropping the handle
    /// would strand the text and every memo keyed on it.  Emptying the text and
    /// clearing the cross-file evidence releases the payload *and* backdates the
    /// memos that read them, which is the closest a caller can get to reclaiming
    /// the input.  `path` and `dialect` are left alone: both are small, and the
    /// path is a pure function of the URI this handle is filed under, so a
    /// revival re-derives the same value.
    fn retire_db_source(
        db: &mut tcl_lsp_db::TclDatabase,
        files: &mut HashMap<Uri, tcl_lsp_db::SourceFile>,
        tombstones: &mut HashMap<Uri, tcl_lsp_db::SourceFile>,
        uri: &Uri,
        published_paths: &PublishedSourcePaths,
    ) -> bool {
        use salsa::Setter as _;
        let Some(file) = files.remove(uri) else {
            return false;
        };
        published_paths
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(uri);
        file.set_text(db).to(String::new());
        file.set_external_call_sites(db).to(None);
        tombstones.insert(uri.clone(), file);
        Self::debug_assert_published_paths_in_step(db, files, published_paths);
        true
    }

    /// The `SourceFile` handle to file under `uri`: its retired one revived with
    /// the new text, or a fresh input when the session has never seen this URI
    /// Reviving keeps one input per distinct URI however many
    /// delete/re-create cycles the URI goes through.
    fn revive_or_create_db_source(
        db: &mut tcl_lsp_db::TclDatabase,
        tombstones: &mut HashMap<Uri, tcl_lsp_db::SourceFile>,
        admission: &DocumentPathAdmission,
        text: String,
        dialect: String,
        oracle: Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>>,
    ) -> tcl_lsp_db::SourceFile {
        use salsa::Setter as _;
        let uri = admission.uri();
        let path = admission.analyser_path().map(str::to_owned);
        let file = if let Some(file) = tombstones.remove(uri) {
            file.set_path(db).to(path);
            file.set_text(db).to(text);
            file.set_dialect(db).to(dialect);
            file
        } else {
            tcl_lsp_db::SourceFile::new(&*db, text, dialect, path)
        };
        // Hand the arriving file the workspace class-factory oracle the rest of
        // the project already carries, rather than leaving it at `None` for
        // `sync_workspace_class_factories` to correct a publish later. A
        // default-`None` arrival is analysed — and *published* — as if the
        // workspace declared no metaclass at all, so every class a cross-file
        // metaclass manufactures in it is missing from that first result and
        // from the workspace index built out of it, and only a second,
        // rescheduled pass would repair it. The value is a project-wide fact
        // every
        // other file is already entitled to, so seeding it asserts nothing the
        // next sync would not assert anyway.
        if oracle.is_some() {
            file.set_workspace_class_factories(db).to(oracle);
        }
        file
    }

    /// Re-set the salsa [`tcl_lsp_db::Project`] input to the admitted member
    /// set, sorted by URI for stable iteration. `db_files` may additionally
    /// contain an edited open orphan whose local providers still need a Salsa
    /// handle; only `members` defines its cross-document identity.
    #[cfg(test)]
    pub(in super::super) fn sync_db_project(
        db: &mut tcl_lsp_db::TclDatabase,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
        members: &HashSet<Uri>,
        project: &mut Option<tcl_lsp_db::Project>,
    ) {
        use salsa::Setter as _;
        let mut entries: Vec<(&Uri, &tcl_lsp_db::SourceFile)> = files
            .iter()
            .filter(|(uri, _)| members.contains(*uri))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        let sources: Vec<tcl_lsp_db::SourceFile> = entries.into_iter().map(|(_, &f)| f).collect();
        match *project {
            Some(p) => {
                p.set_files(db).to(sources);
            }
            None => *project = Some(tcl_lsp_db::Project::new(&*db, sources)),
        }
    }

    /// Publish each admitted member's resolved configuration alongside its
    /// source handle. The caller owns the source publication or configuration
    /// transaction; semantic-token readers never mutate this input.
    pub(in super::super) fn sync_db_project_with_configurations(
        db: &mut tcl_lsp_db::TclDatabase,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
        members: &HashSet<Uri>,
        project: &mut Option<tcl_lsp_db::Project>,
        global_config: tcl_lsp_db::AnalyserConfig,
        folder_configs: &[(Uri, tcl_lsp_db::AnalyserConfig)],
    ) {
        use salsa::Setter as _;
        let mut entries = files
            .iter()
            .filter(|(uri, _)| members.contains(*uri))
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        let configurations = entries
            .into_iter()
            .map(|(uri, &file)| {
                let config = longest_folder_match(folder_configs, uri)
                    .copied()
                    .unwrap_or(global_config);
                (file, config)
            })
            .collect::<Vec<_>>();
        let sources = configurations.iter().map(|&(file, _)| file).collect();
        match *project {
            Some(handle) => {
                handle.set_files(db).to(sources);
                handle.set_token_configurations(db).to(Some(configurations));
            }
            None => {
                *project = Some(tcl_lsp_db::Project::with_token_configurations(
                    &*db,
                    sources,
                    Some(configurations),
                ));
            }
        }
    }

    /// Apply admitted project membership changes together with each member's
    /// actual folder/global configuration. Input handles stay reusable across
    /// source edits and configuration changes.
    pub(super) fn sync_db_project_membership_with_configurations<'a>(
        db: &mut tcl_lsp_db::TclDatabase,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
        members: &mut HashSet<Uri>,
        project: &mut Option<tcl_lsp_db::Project>,
        configurations: (
            tcl_lsp_db::AnalyserConfig,
            &[(Uri, tcl_lsp_db::AnalyserConfig)],
        ),
        changes: (
            impl IntoIterator<Item = &'a Uri>,
            impl IntoIterator<Item = &'a Uri>,
            bool,
        ),
    ) {
        let (removals, additions, source_membership_changed) = changes;
        let mut membership_changed = false;
        for uri in removals {
            membership_changed = members.remove(uri) || membership_changed;
        }
        for uri in additions {
            membership_changed = members.insert(uri.clone()) || membership_changed;
        }
        if source_membership_changed || membership_changed {
            Self::sync_db_project_with_configurations(
                db,
                files,
                members,
                project,
                configurations.0,
                configurations.1,
            );
        }
    }
}
