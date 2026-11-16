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

//! The configured analyser, the dialect registry and the analyser configuration input.

use super::super::Backend;
use super::super::configuration::{PolicyLayers, ResourceAnalyserInputs};
use std::collections::HashSet;
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, NonAsciiMode};
use tcl_registry::CommandRegistry;

impl Backend {
    /// Mirror the editor analyser config (disabled diagnostics + non-ASCII
    /// mode) onto the salsa `AnalyserConfig` input.  The disabled set is
    /// sorted so the input value is stable (a `HashSet` iteration order change
    /// must not look like an edit to salsa).
    ///
    /// The pack key also reaches every folder's own handle: packs are a
    /// workspace fact, never a per-folder one (see
    /// [`Self::apply_folder_configs`]), and a pack reload lands after the
    /// configuration pull that created those handles.
    pub(in super::super) async fn sync_db_config(&self) {
        use salsa::Setter as _;
        let mut disabled: Vec<String> = self
            .disabled_diagnostics
            .lock()
            .await
            .iter()
            .cloned()
            .collect();
        disabled.sort();
        let mode = *self.non_ascii_mode.lock().await;
        let extra = self.extra_commands.lock().await.clone();
        let generic = self.generic_variable_patterns.lock().await.clone();
        // The loaded pack set's content identity: the analyser reads the
        // registry this key was installed under, so an EDA document sees the
        // bundled vendor library and a workspace pack reaches the diagnostics
        // it was written to change.
        let pack_key = self.spec_packs().await.key;
        let folder_handles: Vec<tcl_lsp_db::AnalyserConfig> = self
            .folder_db_configs
            .lock()
            .await
            .iter()
            .map(|(_, handle)| *handle)
            .collect();
        let config = *self.db_config.lock().await;
        let mut db = self.db.lock().await;
        config.set_disabled_diagnostics(&mut *db).to(disabled);
        config.set_non_ascii_mode(&mut *db).to(mode);
        config.set_spec_pack_key(&mut *db).to(pack_key);
        for handle in folder_handles {
            handle.set_spec_pack_key(&mut *db).to(pack_key);
        }
        config.set_extra_commands(&mut *db).to(extra);
        config.set_generic_variable_patterns(&mut *db).to(generic);
        let bigip = self.bigip_version.lock().await.clone();
        config.set_bigip_version(&mut *db).to(bigip);
        let targets = self.declared_targets.lock().await.clone();
        config.set_targets(&mut *db).to(targets);
        let provides = self.package_provides.lock().await.clone();
        config.set_package_provides(&mut *db).to(provides);
        let encoding = self.declared_script_encoding();
        config.set_script_encoding(&mut *db).to(encoding);
        for (_, handle) in self.folder_db_configs.lock().await.iter() {
            handle.set_script_encoding(&mut *db).to(encoding);
        }
    }

    /// Build an `Analyser` carrying the configured disabled-diagnostics
    /// set and W108 mode.
    pub(in super::super) fn configured_analyser(
        disabled: HashSet<String>,
        mode: NonAsciiMode,
        extra_commands: HashSet<String>,
        pack_key: u64,
        resource: ResourceAnalyserInputs,
    ) -> Analyser {
        resource.apply(
            Analyser::with_disabled_diagnostics(disabled)
                .with_non_ascii_mode(mode)
                .with_extra_commands(extra_commands)
                .with_pack_overlay(pack_key),
        )
    }

    pub(in super::super) fn configured_analyser_from_db_config(
        config: tcl_lsp_db::AnalyserConfig,
        db: &dyn salsa::Database,
    ) -> Analyser {
        Self::configured_analyser(
            config.disabled_diagnostics(db).iter().cloned().collect(),
            config.non_ascii_mode(db),
            config.extra_commands(db).iter().cloned().collect(),
            config.spec_pack_key(db),
            ResourceAnalyserInputs::from_db_config(config, db),
        )
    }

    /// [`Self::configured_analyser`] taking the recovery path's **shared**
    /// widened known-command set (see [`RecoveryNameCache`]), so the cached set
    /// is handed over as a refcount bump instead of being deep-copied into the
    /// analyser on every keystroke.
    pub(in super::super) fn recovery_analyser(
        disabled: HashSet<String>,
        mode: NonAsciiMode,
        extra_commands: Arc<HashSet<String>>,
        pack_key: u64,
        resource: ResourceAnalyserInputs,
    ) -> Analyser {
        // The resource-scoped inputs must survive an incomplete buffer. They
        // decide Tk activation, what `package_requires` contains and which
        // release the version gate reads against, and this analysis is
        // *published* — and indexed — so dropping them for the keystrokes
        // between an opening brace and its match makes those diagnostics
        // flicker and republishes a degraded index entry.
        resource.apply(
            Analyser::with_disabled_diagnostics(disabled)
                .with_non_ascii_mode(mode)
                .with_shared_extra_commands(extra_commands)
                .with_pack_overlay(pack_key),
        )
    }

    /// Return the command registry with `dialect` loaded on top of the
    /// default Tcl + stdlib + tcllib specs, **plus the workspace's `SpecTcl`
    /// packs**.
    ///
    /// The registry is the process-wide per-profile cache in `tcl-registry`,
    /// built once per dialect profile and shared for the process lifetime —
    /// an unknown or unparseable dialect string resolves to the plain-Tcl
    /// profile rather than leaking a fresh registry per typo.
    ///
    /// With packs loaded the entry is keyed by `(profile, pack-set content
    /// hash)` instead, which is the whole of what "workspace scope" means in
    /// `docs/design/registry/spec-packs.md`: one registry per pack *content*, shared by
    /// every document, rebuilt when the pack changes and never per edit of the
    /// code that uses it.  With no packs — the overwhelmingly common case —
    /// this is byte for byte the lookup it always was.
    ///
    /// Returned as an owning handle, and the caller is expected to keep it for
    /// the length of the request it serves. That is what makes a pack reload
    /// safe: the superseded generation stays alive and readable for every
    /// request already walking with it, and is freed once the last of them
    /// finishes (`tcl_registry::cache`'s memory note). Both arms hand back the
    /// same underlying instance the process already had — the no-packs arm
    /// re-enters the same cache the salsa query reads, so this is a change of
    /// ownership, not of identity.
    pub(in super::super) async fn registry_for_dialect(
        &self,
        dialect: &str,
    ) -> Arc<CommandRegistry> {
        let packs = self.spec_packs().await;
        if packs.key != 0 {
            return tcl_spectcl::install::registry_for_dialect_with_packs(dialect, &packs);
        }
        // The same per-dialect cache entry the salsa `registry` query returns
        // — that query is a forwarder to this cache, and both spellings
        // resolve `dialect` through the same catalogue — asked for as a
        // handle so this function has one return type.  Reaching the cache
        // directly also spares the db mutex a lock on a path that only ever
        // forwarded.
        Arc::clone(
            tcl_lsp_core::environment_for_dialect(dialect)
                .default_context_registry()
                .commands(),
        )
    }
}

/// The analyser-config query input a fresh backend starts from: the
/// production skip of the default policy layers and every other knob unset.
pub(in super::super) fn default_analyser_config(
    db: &tcl_lsp_db::TclDatabase,
) -> tcl_lsp_db::AnalyserConfig {
    tcl_lsp_db::AnalyserConfig::new(
        db,
        PolicyLayers::default().sorted_production_skip(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    )
}
