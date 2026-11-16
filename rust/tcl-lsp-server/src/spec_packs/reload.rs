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

//! One pack reload: discovery, load, overlay installation and settlement.

use std::path::PathBuf;
use std::sync::Arc;

use tcl_registry::CommandRegistry;
use tower_lsp_server::ls_types::MessageType;

use super::{
    PackConfigurationPublication, PackReloadSourceViews, PublishedPackSet, ReloadTrigger,
    SpecPacksReloaded,
};
use crate::diagnostics::sync_evaluator_epoch;
use crate::documents::DocumentSourceResult;
use crate::protocol::{FoldingRangeRefreshRequest, bounded_client_request};
use crate::semantic_tokens::SemanticTokensRefreshReason;
use crate::tracked_locks::TrackedMutex;
use crate::uri_norm::DocumentPath as _;
use crate::{Backend, studio_source, vfs};

/// Install `packs` as the overlay of every registry key the ingress can ask
/// for: each catalogue profile and the permissive sink that `tk`, the lenient
/// `tcl` and a pack-declared environment read their store from
/// ([`tcl_registry::model::store_profiles`]). An environment left out would
/// answer every compile query with an overlay miss for the life of the
/// session.
pub(in super::super) fn install_pack_overlays(packs: &tcl_spectcl::PackSet) {
    for profile in tcl_registry::model::store_profiles() {
        let _ = tcl_spectcl::install::registry_with_packs(profile, packs);
    }
}

/// Make the registry cache's overlay epoch the salsa input
/// `tcl_lsp_db::OverlayEpoch`, which every query that resolves a pack overlay
/// reads. The cache is process-wide state the database cannot see change, so
/// a unit memoised while a workspace's packs were not installed — or against a
/// generation the cache has since retired — is asked again once the epoch
/// moves. Compare-then-set, like [`sync_evaluator_epoch`]: a sync that finds
/// the overlays unchanged writes nothing.
pub(in super::super) async fn sync_overlay_epoch(db: &TrackedMutex<tcl_lsp_db::TclDatabase>) {
    let epoch = tcl_registry::overlay_epoch();
    let mut db = db.lock().await;
    tcl_lsp_db::set_overlay_epoch(&mut db, epoch);
}

impl Backend {
    fn load_pack_configuration(
        pack_store: &Arc<dyn vfs::SourceStore>,
        mut studio_snapshot: studio_source::StudioSourceSnapshot,
        options: tcl_spectcl::DiscoveryOptions,
        held_tx: Option<tokio::sync::oneshot::Sender<()>>,
        hold: std::time::Duration,
    ) -> (
        tcl_spectcl::PackSet,
        studio_source::StudioSourceSnapshot,
        tcl_spectcl::DiscoveryOptions,
        Vec<Vec<PathBuf>>,
        tcl_spectcl::pack::LoadedSourceInputs,
    ) {
        let studio =
            studio_snapshot.retained_sources(pack_store.as_ref(), &options.workspace_roots);
        let active_paths = studio
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        let discovery = tcl_spectcl::discovery::discover_configuration_with_active_studio_in(
            pack_store.as_ref(),
            &options,
            &active_paths,
        );
        // `load_discovered`, not `pack::load`: a bare `tcl-lsp-server`
        // release binary (Zed, the standalone-editor instructions) has no
        // `specs/` directory beside it, so discovery finds no bundled tier
        // and the shipped EDA loadables have to come from the embedded
        // copy. A plain load would leave every EDA command unknown in
        // exactly those clients.

        let (loaded, loader_inputs) =
            tcl_spectcl::bundled::load_discovered_configuration_with_studio_sources_and_inputs_in(
                Arc::clone(pack_store),
                &discovery,
                &studio,
                options.workspace_trust,
            );

        // The test seam, and the only place it can go: the snapshot is
        // taken, the publish has not happened, so the world is free to
        // change underneath a view that is already stale. The signal goes
        // out first so the test knows the snapshot it has to invalidate
        // exists; then the thread sits on it.
        if let Some(tx) = held_tx {
            let _ = tx.send(());
            std::thread::sleep(hold);
        }
        (
            loaded,
            studio_snapshot,
            options,
            discovery.configured_sources,
            loader_inputs,
        )
    }

    fn publish_pack_set_locked(
        guard: &mut PublishedPackSet,
        studio_sources: &mut studio_source::StudioSources,
        studio_snapshot: &studio_source::StudioSourceSnapshot,
        loaded: &Arc<tcl_spectcl::PackSet>,
        seq: u64,
        registration: &mut Option<tcl_spectcl::PackSetRegistration>,
    ) -> bool {
        // Newest snapshot wins, decided by when the disk was read. With
        // `spec_pack_reload` held this is always true — see
        // `spec_pack_reload_seq` for why it is checked anyway.
        let newest = seq > guard.seq;
        let changed =
            newest && (guard.packs.key != loaded.key || studio_sources.has_unpublished_changes());
        if changed {
            // Publish the hook plan *before* the new pack set becomes
            // visible, and under the same lock: slots are allocated once,
            // here, for the whole process, and each worker thread builds
            // its own sandboxed host from the plan the first time it
            // dispatches a hook.  A `HookHost` owns VM
            // instances, which are not `Send`, so this cannot install
            // anything itself — it records what to install.  Publishing
            // after the swap would leave a window in which a query sees
            // the pack's commands with their hooks still abstaining.
            tcl_spectcl::hooks::publish(loaded);
            let loaded = Arc::clone(loaded);
            // Publish the same set to every other consumer in this process
            // — the compiler explorer most of all, which built a plain
            // registry and so reported a pack's (and every EDA) command
            // unknown on the very line whose diagnostic resolved it.
            //
            // This is also where a pack's `environment` blocks go live:
            // the publish registers them into the one environment
            // registry the ingress resolves against, retires the ones a
            // pack that has left the workspace declared, and republishes
            // the extension routing detection reads — so a document with
            // a pack-declared extension resolves to that environment
            // through the ordinary ingress from here on.
            *registration = Some(tcl_spectcl::bundled::set_active(Some(Arc::clone(&loaded))));
            // The stamp moves with the content: it records which snapshot
            // the *current* set came from, so a reload that read the disk
            // earlier than that can never take its place.
            *guard = PublishedPackSet { seq, packs: loaded };
            studio_sources.mark_published(studio_snapshot);
        }
        if !changed {
            tcl_spectcl::registration::publish_pack_extension_routes(loaded);
            if newest {
                guard.seq = seq;
            }
        }
        changed
    }

    async fn install_reload_pack_overlays(
        &self,
        packs: &Arc<tcl_spectcl::PackSet>,
    ) -> std::ops::ControlFlow<(), Vec<(&'static tcl_dialect::DialectProfile, Arc<CommandRegistry>)>>
    {
        std::ops::ControlFlow::Continue(if packs.key == 0 {
            // Loader-only notices still need the actual default registry owner
            // used by ordinary analysis when there is no pack set.
            let installed = tcl_registry::model::store_profiles()
                .map(|profile| {
                    let registry = tcl_lsp_core::environment_for_dialect(profile.name)
                        .default_context_registry();
                    Some((profile, Arc::clone(registry.commands())))
                })
                .collect::<Option<Vec<_>>>();
            let Some(installed) = installed else {
                return std::ops::ControlFlow::Break(());
            };
            installed
        } else {
            // Build the `(profile, key)` registry for every dialect now.
            //
            // The analyser resolves its own registry and can only *look up*
            // that entry (`Analyser::with_pack_overlay` /
            // `registry_for_profile_if_built`) — building it needs the pack
            // contents, which only this crate's loader has. Doing it here means
            // the entry is always there by the time a document is analysed, so
            // the very first diagnostics pass after a load already knows a
            // bundled EDA library's commands rather than reporting every one of
            // them unknown until some other provider happened to build it.
            //
            // The set of profiles is closed and each registry is built once for
            // the process, so this is a bounded one-off — and every one of them
            // would have been built lazily by the first document of that
            // dialect anyway. It runs on a worker: it is parse-free but not
            // free.
            let for_worker = Arc::clone(packs);
            let installed = crate::rt::spawn_blocking(move || {
                install_pack_overlays(&for_worker);

                tcl_registry::model::store_profiles()
                    .map(|profile| {
                        let context =
                            tcl_registry::model::ingress::resolve_environment(profile.name)
                                .context_registry(
                                    &tcl_registry::model::KeyedVersions::default(),
                                    for_worker.key,
                                )
                                .ok()?;
                        Some((profile, Arc::clone(context.commands())))
                    })
                    .collect::<Option<Vec<_>>>()
            })
            .await;
            let Ok(Some(installed)) = installed else {
                return std::ops::ControlFlow::Break(());
            };
            // The analyser reads the key off the salsa config, so publish it
            // before anything re-analyses, with the overlay epoch that says
            // the generations behind it are installed.
            self.sync_db_config().await;
            sync_overlay_epoch(&self.db).await;
            installed
        })
    }

    fn pack_reload_hold_channels(
        trigger: ReloadTrigger,
    ) -> (
        std::time::Duration,
        Option<tokio::sync::oneshot::Sender<()>>,
        Option<tokio::sync::oneshot::Receiver<()>>,
    ) {
        match trigger {
            ReloadTrigger::Startup if !startup_reload_hold().is_zero() => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                (startup_reload_hold(), Some(tx), Some(rx))
            }
            _ => (std::time::Duration::ZERO, None, None),
        }
    }

    fn next_spec_pack_reload_sequence(&self) -> u64 {
        self.spec_pack_reload_seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1
    }

    /// What [`tcl_spectcl::discover`] should look at for this workspace:
    /// every open folder, plus whatever `tclLsp.specPacks` names, and the
    /// editor's Workspace Trust state the workspace tier loads under.
    ///
    /// The user and bundled tiers take their defaults — the platform config
    /// directory and the shipped loadables — so a workspace need declare
    /// nothing to get them.
    pub(in super::super) async fn spec_pack_discovery(&self) -> tcl_spectcl::DiscoveryOptions {
        let workspace_trust = *self.workspace_trust.lock().await;
        let workspace_roots: Vec<PathBuf> = self
            .workspace_folders
            .lock()
            .await
            .iter()
            .filter_map(|uri| uri.document_path().map(std::borrow::Cow::into_owned))
            .collect();
        let configured = self
            .spec_pack_paths
            .lock()
            .await
            .iter()
            .map(PathBuf::from)
            .collect();
        // Folder-scoped `tclLsp.specPacks`, each kept with the folder it was
        // pulled for so a relative entry resolves there and nowhere else.
        let folder_configured: Vec<(PathBuf, Vec<PathBuf>)> = self
            .folder_configs
            .lock()
            .await
            .iter()
            .filter_map(|(uri, config)| {
                let paths = config.spec_packs.as_ref()?;
                let folder = uri.document_path()?.into_owned();
                Some((folder, paths.iter().map(PathBuf::from).collect()))
            })
            .collect();
        tcl_spectcl::DiscoveryOptions {
            workspace_roots,
            configured,
            folder_configured,
            workspace_trust,
            ..tcl_spectcl::DiscoveryOptions::default()
        }
    }

    /// Discover, load and install the workspace's `SpecTcl` packs, then publish
    /// each pack file's load notices as diagnostics on that file.
    ///
    /// Runs at `initialized` and again whenever a `.tclspec` changes on disk or
    /// `tclLsp.specPacks` moves.  Everything about it is workspace-scoped and
    /// off the per-edit path: discovery walks the filesystem and loading parses
    /// Tcl, neither of which belongs anywhere near a keystroke.
    ///
    /// Returns `true` when the loaded set actually changed, so a caller can
    /// decide whether re-analysing every open document is warranted — the pack
    /// set's content hash makes "the watcher fired but the bytes are the same"
    /// (a save with no edit, a `touch`, a branch switch that restores a file)
    /// free.
    ///
    /// `trigger` says which event asked, and is used for exactly one thing:
    /// the startup reload honours [`STARTUP_RELOAD_HOLD_ENV`], the test-only
    /// hold that lets the e2e suite force the two-reload interleaving.
    /// Wait for [`Self::reload_spec_packs`]'s loader to signal that it has read
    /// the disk, and say so on the client's log channel.
    ///
    /// The log line is the one ordering signal an e2e client can actually wait
    /// on: it means "this reload's view of the disk is now fixed", which is the
    /// instant the test has to write its pack file at. `held_rx` is `None` in
    /// every real session (see [`STARTUP_RELOAD_HOLD_ENV`]), and then this
    /// returns without awaiting anything.
    async fn announce_pack_snapshot_hold(
        &self,
        hold: std::time::Duration,
        held_rx: Option<tokio::sync::oneshot::Receiver<()>>,
    ) {
        let Some(rx) = held_rx else { return };
        if rx.await.is_err() {
            return;
        }
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "{STARTUP_RELOAD_HELD_LOG} for {}ms ({STARTUP_RELOAD_HOLD_ENV})",
                    hold.as_millis()
                ),
            )
            .await;
    }

    pub(in super::super) fn reload_spec_packs(
        &self,
        trigger: ReloadTrigger,
    ) -> std::pin::Pin<Box<impl std::future::Future<Output = DocumentSourceResult<bool>> + '_>>
    {
        Box::pin(self.reload_spec_packs_inner(trigger))
    }

    async fn reload_spec_packs_inner(&self, trigger: ReloadTrigger) -> DocumentSourceResult<bool> {
        // One reload at a time — see `spec_pack_reload`. The guard spans
        // discovery *and* the publish below, because it is the gap between
        // them that lets a slower reload overwrite a newer one's result.
        let mut reload = self.spec_pack_reload.lock().await;

        *reload = None;
        // Taken here, under the guard and *before* discovery: the number has
        // to order reloads by when they read the disk. See
        // `spec_pack_reload_seq`.
        let seq = self.next_spec_pack_reload_sequence();
        let options = self.spec_pack_discovery().await;
        // Before loading: the watch set follows the *configuration*, so it has
        // to be refreshed even when the discovered files turn out unchanged.
        self.refresh_external_pack_watchers(&options).await;

        // Discovery and loading are blocking filesystem + parse work; keeping
        // them off the LSP event loop is the same courtesy every other scan
        // here extends.
        // The test hold and its "snapshot taken" signal — absent in every real
        // session, because [`startup_reload_hold`] is zero unless the e2e
        // suite asked for it. See [`STARTUP_RELOAD_HOLD_ENV`].
        let (hold, held_tx, held_rx) = Self::pack_reload_hold_channels(trigger);
        let pack_store = Arc::clone(&self.store);
        let studio_snapshot = self.studio_sources.lock().await.snapshot();
        let load = crate::rt::spawn_blocking(move || {
            Self::load_pack_configuration(&pack_store, studio_snapshot, options, held_tx, hold)
        });
        self.announce_pack_snapshot_hold(hold, held_rx).await;
        let loaded = load.await;
        let Ok((loaded, studio_snapshot, options, configured_sources, loader_inputs)) = loaded
        else {
            // A panic in the loader must not take the session with it: the
            // workspace keeps whatever pack set it already had.
            self.client
                .log_message(
                    MessageType::WARNING,
                    "SpecTcl pack load failed; keeping the previously loaded packs",
                )
                .await;
            return Ok(false);
        };

        // Keep the pack registry and its Salsa key one analyser-input
        // generation. A standalone index seed either commits before this
        // writer or observes the bumped epoch after the complete transition.
        let analyser_inputs_guard = self.analyser_inputs_gate.write().await;

        let mut studio_sources = self.studio_sources.lock().await;
        if !studio_sources.matches_snapshot(&studio_snapshot) {
            return Ok(false);
        }
        // What publishing the set did to the live environment registry, when
        // it was published — `None` when this reload changed nothing.
        let mut registration: Option<tcl_spectcl::PackSetRegistration> = None;
        let loaded = Arc::new(loaded);
        let changed = {
            let mut guard = self.spec_packs.lock().await;
            studio_snapshot.with_current_saved_sources(self.store.as_ref(), || {
                Self::publish_pack_set_locked(
                    &mut guard,
                    &mut studio_sources,
                    &studio_snapshot,
                    &loaded,
                    seq,
                    &mut registration,
                )
            })?
        };

        let packs = self.spec_packs().await;
        let std::ops::ControlFlow::Continue(overlays) =
            self.install_reload_pack_overlays(&packs).await
        else {
            return Ok(false);
        };

        if changed {
            // The publish above moved the process's evaluator epoch, whether
            // or not the new set is empty: take it before anything
            // re-analyses, so no memoised lattice outlives the plan whose
            // hosts computed it.
            sync_evaluator_epoch(&self.db).await;
            // The new set decides `registry_for_dialect`, and every cached
            // `DiagInputs` holds a registry handle resolved from the old one.
            // Bump only after the registry and Salsa config key are both live.
            self.invalidate_diag_inputs();
        }
        drop(studio_sources);
        drop(analyser_inputs_guard);

        self.report_pack_reload(
            &packs,
            registration.as_ref(),
            changed,
            seq,
            PackReloadSourceViews {
                loader_inputs: &loader_inputs,
                studio_snapshot: &studio_snapshot,
                overlays: &overlays,
            },
        )
        .await?;

        self.settle_pack_reload(changed, trigger).await?;

        *reload = Some(PackConfigurationPublication {
            options,
            configured_sources,
            seq,
            packs,
            studio_snapshot,
            loader_inputs,
            overlays,
        });
        Ok(changed)
    }

    /// Publish everything a finished reload has to *say*: shipped-command
    /// collisions, the environments the live registry refused, and the
    /// one-line summary on the log channel.
    ///
    /// Split out of [`Backend::reload_spec_packs`] because it is the
    /// reporting half — it changes nothing about what is loaded, and every
    /// input it needs is already settled by the time it runs.
    async fn report_pack_reload(
        &self,
        packs: &Arc<tcl_spectcl::PackSet>,
        registration: Option<&tcl_spectcl::PackSetRegistration>,
        changed: bool,
        seq: u64,
        sources: PackReloadSourceViews<'_>,
    ) -> DocumentSourceResult<()> {
        let PackReloadSourceViews {
            loader_inputs,
            studio_snapshot,
            overlays,
        } = sources;
        // Shipped-command collisions are not load notices — the loader has no
        // registry to compare against, so `PackSet::notices` cannot carry
        // them. They are computed here, against the session dialect's plain
        // registry, and published alongside. Without this the author of a pack
        // declaring `lsort` gets silence: the declaration is dropped (or the
        // shipped spec replaced, with `-override`) and nothing in the editor
        // says which, though the loader promised to report both.
        //
        // Session dialect, not per-folder: a pack's commands are installed at
        // workspace scope, and the notice is about the pack file, which has no
        // dialect of its own.
        let dialect = self.session_dialect().await;
        let for_collisions = Arc::clone(packs);
        let mut collisions = crate::rt::spawn_blocking(move || {
            tcl_spectcl::pack::collision_notices(
                &for_collisions,
                tcl_lsp_core::registry_for_dialect(&dialect),
            )
        })
        .await
        .unwrap_or_default();
        // A pack whose `environment` blocks the registry refused is a pack
        // the author has to hear about, and the pack file is where they are
        // looking. The reason is the registry's own message — the reserved
        // name, the untrusted extension, the collision — so the notice says
        // which rule and which pack rather than "some environments did not
        // load".
        collisions.extend(Self::environment_rejection_notices(packs, registration));
        self.publish_spec_pack_notices(
            packs,
            &collisions,
            seq,
            loader_inputs,
            studio_snapshot,
            overlays,
        )
        .await?;

        let _: () = if changed {
            let commands: usize = packs.packs.iter().map(|p| p.commands.len()).sum();
            let environments = registration.map_or_else(String::new, |r| {
                format!(
                    ", {} environment(s), {} extension(s), {} retired, {} refused",
                    r.declared,
                    r.extended,
                    r.retired,
                    r.rejected.len()
                )
            });
            self.client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "SpecTcl: {} pack(s), {commands} command(s), {} notice(s), \
                         {} shipped-command collision(s){environments}",
                        packs.packs.len(),
                        packs.notices.len(),
                        collisions.len()
                    ),
                )
                .await;
        };
        Ok(())
    }

    /// A [`tcl_spectcl::PackNotice`] on each pack file whose environments
    /// the live registry refused.
    ///
    /// Attached to the pack's *first* file: an environment block can come
    /// from any file of a multi-file pack, and the registration seam works
    /// on the merged pack, so the pack — not a line — is what the refusal
    /// is about.
    fn environment_rejection_notices(
        packs: &tcl_spectcl::PackSet,
        registration: Option<&tcl_spectcl::PackSetRegistration>,
    ) -> Vec<tcl_spectcl::PackNotice> {
        let Some(registration) = registration else {
            return Vec::new();
        };
        registration
            .rejected
            .iter()
            .filter_map(|rejection| {
                let pack = packs
                    .packs
                    .iter()
                    .find(|pack| pack.name == rejection.pack)?;
                let path = pack.files.first()?;
                Some(tcl_spectcl::PackNotice::whole_file(
                    path,
                    format!(
                        "the `environment` blocks of pack `{}` are not loaded: {}",
                        rejection.pack, rejection.error
                    ),
                    tcl_spectcl::pack::Severity::Warning,
                ))
            })
            .collect()
    }

    /// Everything that has to happen *after* a pack set is published, in the
    /// order a client can rely on.
    ///
    /// Split from [`Backend::reload_spec_packs`] because it is a distinct
    /// phase with a contract of its own: by the time the notification at the
    /// end is sent, every consequence of the reload has already landed.
    async fn settle_pack_reload(
        &self,
        changed: bool,
        trigger: ReloadTrigger,
    ) -> DocumentSourceResult<()> {
        // Reconciled on **every** reload, not only a `changed` one: `changed`
        // compares the pack *content* key, and a reload leaving content
        // identical can still be the first to reach a client that had not yet
        // registered (a restarted client, a folder added). The refresh is a
        // no-op when the set has genuinely not moved, so this costs a vec
        // comparison.
        let extensions_changed = self.refresh_pack_extension_registrations().await;
        // Files already on disk under a newly-claimed extension are invisible
        // to the index until something walks them again — except at startup,
        // where `initialized` scans immediately after the reload returns and a
        // scan here would be the same walk twice.
        if extensions_changed && trigger != ReloadTrigger::Startup {
            self.scan_workspace_folders().await?;
        }

        // Everything a client caches per document and only re-requests on an
        // edit is now stale, because the registry is what classifies it. A
        // pack decides whether a command head is a `function` at all, whether
        // it carries `defaultLibrary`, and — through `arg_role` — whether a
        // braced word is tokenised as code or as one opaque string. Diagnostics
        // and hover already recover: the callers re-analyse open documents, and
        // hover is pulled per request. Semantic tokens, code lenses and folding
        // ranges do not; without a push they stay on screen exactly as they
        // were until the user types in the buffer, which is what made a Spec
        // Studio pack edit leave the sample's colours frozen.
        //
        // Here rather than at the `didChangeWatchedFiles` call site because
        // this is the one place every trigger passes through, and `run_config_reload`
        // already does the code-lens/folding half for its own path only.
        // Best-effort: a client without refresh support rejects the request.
        if changed {
            // A new pack generation re-classifies every document, so no
            // earlier per-stream refresh ask still describes what is on screen.
            self.semantic_tokens_refresh_asked.lock().await.clear();
            self.request_semantic_tokens_retry(SemanticTokensRefreshReason::PackReload);
            let _ =
                bounded_client_request(self.client.send_request::<FoldingRangeRefreshRequest>(()))
                    .await;
            let _ = bounded_client_request(self.client.code_lens_refresh()).await;
        }

        // Tell the client the reload is finished. A client that reacts to the
        // same filesystem event the server did has no way to know when the
        // server is done with it, so it races: it asks for the new pack set,
        // is answered from the old one, and has nothing scheduled to ask
        // again. A client watching only its workspace folders cannot even see
        // an edit under an absolute `tclLsp.specPacks` root, which the server
        // watches and it does not. One push after the fact settles both.
        // Sent unconditionally: "nothing
        // changed" is exactly what a racing client needs to hear to stop
        // waiting.
        self.client
            .send_notification::<SpecPacksReloaded>(serde_json::json!({
                "changed": changed,
                "pack_file_extensions": self.pack_file_extension_report().await,
            }))
            .await;

        Ok(())
    }
}

/// Env var holding the **startup** pack reload's already-taken filesystem
/// snapshot for this many milliseconds before it is published.
///
/// It exists solely so the e2e suite can force the interleaving that
/// [`crate::BackendState::spec_pack_reload`] and [`crate::BackendState::spec_pack_reload_seq`] exist to
/// survive: a pack file that appears *after* the startup reload has read the
/// disk but *before* it publishes. Real sessions never set it, and unset (or
/// unparseable, or `0`) is a no-op — the hold is read once, and a zero hold
/// costs nothing but the comparison.
///
/// Only the startup reload honours it ([`ReloadTrigger::Startup`]); holding a
/// watched-file or config reload would delay the very snapshot the test needs
/// to land first, which is the opposite of the point.
pub(super) const STARTUP_RELOAD_HOLD_ENV: &str = "TCL_LSP_TEST_STARTUP_RELOAD_HOLD_MS";

/// Logged (once, and only under [`STARTUP_RELOAD_HOLD_ENV`]) the moment the
/// held startup reload has finished reading the disk.
///
/// The e2e test waits on this before writing its pack file: without a signal
/// it would be guessing whether the snapshot it means to invalidate had been
/// taken yet, and a guess that lands early produces a startup snapshot that
/// already contains the pack — a race that proves nothing.
const STARTUP_RELOAD_HELD_LOG: &str = "SpecTcl: startup reload holding its snapshot";

/// How long the startup pack reload sits on its snapshot, from
/// [`STARTUP_RELOAD_HOLD_ENV`]. Read once per process; zero unless a test
/// asked for it.
fn startup_reload_hold() -> std::time::Duration {
    static HOLD: std::sync::OnceLock<std::time::Duration> = std::sync::OnceLock::new();
    *HOLD.get_or_init(|| {
        std::env::var(STARTUP_RELOAD_HOLD_ENV)
            .ok()
            .and_then(|raw| raw.trim().parse::<u64>().ok())
            .map_or(std::time::Duration::ZERO, std::time::Duration::from_millis)
    })
}
