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

//! The workspace's `SpecTcl` packs: discovery, reload, the registrations they claim and the notices they publish.

use super::*;

mod notices;
mod registrations;
mod reload;
mod reports;

#[cfg(test)]
pub(super) use notices::whole_line_range;
#[cfg(test)]
pub(super) use reload::install_pack_overlays;
#[cfg(test)]
pub(super) use reload::sync_overlay_epoch;

/// Which event asked for a `SpecTcl` pack reload.
///
/// The trigger distinguishes the **startup** reload from the others, which is what
/// [`STARTUP_RELOAD_HOLD_ENV`] needs: only the startup reload honours the
/// test-only hold that forces the two-reload interleaving this module is
/// careful about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReloadTrigger {
    /// The one `initialized` starts, which parses the bundled EDA loadables
    /// and is by far the slowest.
    Startup,
    /// A `.tclspec` appeared, changed, or was deleted.
    WatchedFile,
    /// An actual Studio session acquired, published or retired a source.
    StudioSource,
    /// `tclLsp.specPacks` may have moved, so the discovery inputs did.
    Config,
    /// The client reported a different Workspace Trust state, so a workspace
    /// pack's hook bodies start or stop running. The pack-set key mixes the
    /// state, so the reload re-installs even though no file moved.
    Trust,
}

/// The workspace's published pack set, stamped with the reload generation that
/// published it.
///
/// The stamp is what makes publish order a function of **when the disk was
/// read** rather than of who finished last: a reload takes its sequence number
/// before it looks at the filesystem, so a snapshot taken earlier carries a
/// smaller number no matter how long it then spends parsing. See
/// [`crate::BackendState::spec_pack_reload_seq`].
#[derive(Default)]
pub(super) struct PublishedPackSet {
    /// The sequence number of the reload that published `packs`; `0` before
    /// anything has been published.
    pub(super) seq: u64,
    pub(super) packs: Arc<tcl_spectcl::PackSet>,
}

#[derive(Clone, Copy)]
struct PackReloadSourceViews<'a> {
    loader_inputs: &'a tcl_spectcl::pack::LoadedSourceInputs,
    studio_snapshot: &'a studio_source::StudioSourceSnapshot,
    pub(super) overlays: &'a [(&'static tcl_dialect::DialectProfile, Arc<CommandRegistry>)],
}

/// A successfully settled pack generation and its actual discovery input.
pub(super) struct PackConfigurationPublication {
    pub(super) options: tcl_spectcl::DiscoveryOptions,
    pub(super) configured_sources: Vec<Vec<PathBuf>>,
    pub(super) seq: u64,
    pub(super) packs: Arc<tcl_spectcl::PackSet>,
    pub(super) studio_snapshot: studio_source::StudioSourceSnapshot,
    pub(super) loader_inputs: tcl_spectcl::pack::LoadedSourceInputs,
    pub(super) overlays: Vec<(&'static tcl_dialect::DialectProfile, Arc<CommandRegistry>)>,
}

impl PackConfigurationPublication {
    fn overlays_current(&self) -> bool {
        !self.overlays.is_empty()
            && self.overlays.iter().all(|(profile, installed)| {
                tcl_registry::model::ingress::resolve_environment(profile.name)
                    .context_registry(
                        &tcl_registry::model::KeyedVersions::default(),
                        self.packs.key,
                    )
                    .is_ok_and(|context| Arc::ptr_eq(context.commands(), installed))
            })
    }

    pub(super) fn matches_installed(
        &self,
        options: &tcl_spectcl::DiscoveryOptions,
        published: &PublishedPackSet,
    ) -> bool {
        options == &self.options
            && published.seq == self.seq
            && Arc::ptr_eq(&published.packs, &self.packs)
            && self.has_configured_contributions()
    }

    pub(super) fn has_configured_contributions(&self) -> bool {
        self.configured_sources.len() == self.options.configured.len()
            && self.packs.load_errors.is_empty()
            && self.configured_sources.iter().all(|sources| {
                !sources.is_empty()
                    && sources.iter().all(|source| {
                        self.packs
                            .packs
                            .iter()
                            .any(|pack| pack.files.contains(source))
                    })
            })
    }
}

impl Backend {
    /// The workspace's loaded `SpecTcl` packs.  Clones the `Arc` and drops the
    /// lock, so no request path holds it across an await.
    pub(super) async fn spec_packs(&self) -> Arc<tcl_spectcl::PackSet> {
        Arc::clone(&self.spec_packs.lock().await.packs)
    }
}

/// `tcl-lsp/specPacksReloaded` — sent once a `SpecTcl` pack reload has fully
/// landed, carrying the extensions the resulting pack set claims.
///
/// The push half of pack-declared extension registration. A
/// client cannot derive this moment for itself: it sees the same filesystem
/// event the server does, but not when the server has finished acting on it,
/// so a client that reacts to the event directly reads the *previous* pack
/// set and has nothing scheduled to ask again. It also cannot see an edit to
/// a pack under an absolute `tclLsp.specPacks` root at all, since a
/// workspace-scoped watcher never fires there — the same gap
/// [`Backend::refresh_external_pack_watchers`] exists to close on the server
/// side.
///
/// Carries the payload rather than only the fact, so the common case costs no
/// follow-up round trip; `tcl-lsp.getEffectiveConfig` remains the pull
/// equivalent for a client that would rather ask.
/// Params: `{changed, pack_file_extensions}`.
///
/// `changed` says whether the pack *content* actually moved — a client
/// reconciling derived state still wants the `false` notifications, since they
/// are how it learns the reload it was waiting on is over.
/// `pack_file_extensions` is exactly the array `tcl-lsp.getEffectiveConfig`
/// reports under that name, so a client can use one shape for both paths.
///
/// Carried as `serde_json::Value` rather than a derived struct: this crate
/// takes no direct `serde` dependency, and one outbound message is a thin
/// reason to add one when the payload is assembled by `serde_json::json!`
/// either way.
struct SpecPacksReloaded;

impl tower_lsp_server::ls_types::notification::Notification for SpecPacksReloaded {
    type Params = serde_json::Value;
    const METHOD: &'static str = "tcl-lsp/specPacksReloaded";
}
