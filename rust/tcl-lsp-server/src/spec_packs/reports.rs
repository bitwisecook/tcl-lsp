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

//! What `getEffectiveConfig` reports about the configured and discovered packs.

use super::Backend;

impl Backend {
    /// `tclLsp.specPacks` and what discovery made of it, for
    /// `tcl-lsp.getEffectiveConfig`: the configured paths, and one entry per
    /// loaded pack.
    ///
    /// Both halves are reported because they answer different questions. The
    /// paths are the *settle signal* a test polls after pushing a config. The
    /// loaded packs are what a user needs when the answer to "why is my pack
    /// not loading?" is that discovery never saw it — visible without reading
    /// a server log, which is the only other place that fact exists.
    pub(in super::super) async fn spec_pack_report(
        &self,
    ) -> (Vec<String>, Vec<serde_json::Value>, Vec<serde_json::Value>) {
        let configured = self.spec_pack_paths.lock().await.clone();
        let loaded = self.spec_packs().await;
        // Built from the same `Arc`, so `getEffectiveConfig` takes the
        // `spec_packs` lock once rather than twice. It is only ever held for a
        // short synchronous critical section — the reload's publish does no
        // `await` inside it — so a second acquisition could not deadlock; but
        // a reload storm makes it contended, and doubling the acquisitions on
        // a request path the liveness probe watches bought nothing.
        let extensions = Self::pack_file_extension_rows(&loaded);
        let report = loaded
            .packs
            .iter()
            .map(|pack| {
                serde_json::json!({
                    "name": pack.name,
                    "tier": pack.tier.label(),
                    "commands": pack.commands.len(),
                    "files": pack
                        .files
                        .iter()
                        .map(|f| f.display().to_string())
                        .collect::<Vec<String>>(),
                })
            })
            .collect();
        (configured, report, extensions)
    }

    /// Reborrow one actual configured discovery after its provider has settled.
    pub(in super::super) async fn spec_pack_configuration_report(
        &self,
    ) -> Option<serde_json::Value> {
        let reload = self.spec_pack_reload.try_lock().ok()?;
        let publication = reload.as_ref()?;
        let _inputs = self.analyser_inputs_gate.read().await;
        let current_options = self.spec_pack_discovery().await;
        let studio = self.studio_sources.lock().await;
        if !studio.matches_snapshot(&publication.studio_snapshot)
            || studio.has_unpublished_changes()
        {
            return None;
        }
        let published = self.spec_packs.lock().await;
        if !publication.matches_installed(&current_options, &published) {
            return None;
        }
        let config = *self.db_config.lock().await;
        let db = self.db.lock().await;
        if config.spec_pack_key(&*db) != published.packs.key {
            return None;
        }
        if !publication.overlays_current() {
            return None;
        }
        let retained_paths = publication.loader_inputs.retained_paths();
        let ordinary_paths = retained_paths
            .iter()
            .filter(|path| !publication.studio_snapshot.supplies_saved_source(path))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let loader_sources = publication.loader_inputs.selected(&ordinary_paths)?;
        publication
            .studio_snapshot
            .with_current_saved_source_inputs(self.store.as_ref(), &loader_sources, || {
                serde_json::json!({
                    "status": "complete",
                    "configured": publication.options.configured,
                    "generation": publication.seq.to_string(),
                    "registry_key": publication.packs.key.to_string(),
                })
            })
    }

    /// The file extensions the *discovered* packs claim, each resolved to the
    /// editor language id a client should associate it with.
    ///
    /// The dynamic half of extension registration. A pack's
    /// `file_extension NAME -dialect D` row already routes the extension
    /// server-side — `dialect_from_extension` consults pack routing before the
    /// static catalogue — but an editor learns its associations from a static
    /// manifest written long before the pack existed, so the file never
    /// associates, the client never attaches, and in `VS Code` the extension may
    /// not even activate. Advertising the pairs here lets a client that *can*
    /// register at runtime do so.
    ///
    /// No new editor *language* can be created at runtime, so each extension
    /// resolves onto an existing one: the owning dialect's
    /// `editor_language_id` when it has a dedicated language, and plain `tcl`
    /// otherwise (including for a row with no `-dialect` at all, where the
    /// server's own detection decides the dialect once the file opens).
    ///
    /// Extensions the shipped catalogue already owns are **excluded**: those
    /// are registered statically by every editor, and re-advertising them
    /// would invite a client to write a redundant `files.associations` entry
    /// that it would then have to distinguish from a real one on cleanup.
    pub(super) async fn pack_file_extension_report(&self) -> Vec<serde_json::Value> {
        Self::pack_file_extension_rows(self.spec_packs().await.as_ref())
    }

    /// [`Backend::pack_file_extension_report`] over a pack set already in
    /// hand, so a caller holding one builds the rows without re-acquiring the
    /// `spec_packs` lock.
    fn pack_file_extension_rows(packs: &tcl_spectcl::PackSet) -> Vec<serde_json::Value> {
        let mut seen: Vec<String> = Vec::new();
        let mut out: Vec<serde_json::Value> = Vec::new();
        for pack in &packs.packs {
            for row in &pack.file_extensions {
                if seen.contains(&row.extension) {
                    continue;
                }
                // A statically-registered extension needs no dynamic
                // association; the catalogue's own routing is what the
                // editors already ship.
                //
                // The environment model carries the same claim in
                // `DetectionFacts::file_extensions`, but the two sets are not
                // identical — the lenient `tcl` environment claims
                // `.tcl`/`.tk`/`.itcl`/`.tm`/`.test` where the fallback
                // profile claims none — so reading it instead would silently
                // suppress dynamic associations a pack registers.
                if tcl_dialect::DialectProfile::all().iter().any(|p| {
                    p.file_extensions
                        .iter()
                        .any(|e| e.extension == row.extension)
                }) {
                    continue;
                }
                seen.push(row.extension.clone());
                // The environment's own contributed editor identity: the same
                // value the profile's `editor_language_id` carries for every
                // catalogue entry, reached through the one name seam instead
                // of a second `find`.
                let language_id = row
                    .dialect
                    .and_then(tcl_registry::model::resolve_known_environment)
                    .and_then(|environment| environment.definition.editor_identity)
                    .map_or("tcl", tcl_dialect::model::EditorLanguageIdentityId::as_str);
                out.push(serde_json::json!({
                    "extension": row.extension,
                    "association_glob": tcl_registry::dialects::extension_globs_any_case([row.extension.as_str()])
                            .ok()
                            .and_then(|patterns| patterns.into_iter().next())
                            .map(|pattern| pattern.strip_prefix("**/").unwrap_or(&pattern).to_owned()),
                    "dialect": row.dialect,
                    "language_id": language_id,
                    "display_name": row.display_name,
                    "pack": pack.name,
                }));
            }
            // The same advertisement for the extensions a pack-declared
            // `environment` block claims. Server-side these already route
            // (`registration::extension_routes` publishes them alongside the
            // `-dialect` rows), but an editor that never associates the
            // extension still opens the file as plain text and never
            // attaches — so an environment a pack declares would be
            // reachable by every path except the one a user actually takes.
            for environment in &pack.environments {
                let resolved = tcl_registry::model::resolve_known_environment(&environment.id);
                let language_id = resolved
                    .as_ref()
                    .and_then(|environment| environment.definition.editor_identity)
                    .map_or("tcl", tcl_dialect::model::EditorLanguageIdentityId::as_str);
                for claim in &environment.file_extensions {
                    let extension = claim.extension.as_ref();
                    if seen.iter().any(|prior| prior == extension)
                        || tcl_dialect::DialectProfile::all()
                            .iter()
                            .any(|p| p.file_extensions.iter().any(|e| e.extension == extension))
                    {
                        continue;
                    }
                    seen.push(extension.to_owned());
                    out.push(serde_json::json!({
                        "extension": extension,
                        "dialect": environment.id,
                        "language_id": language_id,
                        "display_name": claim.display_name.as_ref(),
                        "pack": pack.name,
                    }));
                }
            }
        }
        out.sort_by(|a, b| a["extension"].as_str().cmp(&b["extension"].as_str()));
        out
    }
}
