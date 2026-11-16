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

//! Queries over the command registry and the discovered packages and installations.

use std::path::PathBuf;
use std::sync::Arc;

use tcl_lsp_core::tcl_install as core_tcl_install;
use tower_lsp_server::jsonrpc;

use crate::Backend;
use crate::uri_norm::DocumentPath as _;
use crate::workspace::effective_auto_path;

impl Backend {
    /// Handle `tcl-lsp.describeIruleEvent`: deterministic registry metadata
    /// for an iRules event.
    /// `validCommandCount` counts the iRules commands (those carrying
    /// `event_requires`) not excluded from the event.
    ///
    /// The answer is the standalone iRules catalogue (`catalogue`), not any
    /// document's configured release or input: it says which events and
    /// commands the registry declares, not which apply to a given BIG-IP.
    pub(in super::super) async fn describe_irule_event_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let event = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_owned();
        let events = tcl_registry::events::EventRegistry::build();
        let known = !event.is_empty() && events.is_known(&event);
        let (count, sample) = if known {
            let registry = self.registry_for_dialect("f5-irules").await;
            let profiles = tcl_registry::profiles::ProfileRegistry::build();
            // Every f5-irules command valid
            // in the event, not just those that carry `event_requires`.
            let names = registry.valid_irules_commands_for_event(&event, &events, &profiles, None);
            let count = names.len();
            let sample: Vec<String> = names.into_iter().take(80).map(str::to_owned).collect();
            (count, sample)
        } else {
            (0, Vec::new())
        };
        // The contract derives `deprecated` from the `when` event-completion
        // detail (`get_event_detail`), which never contains the word — so the
        // describe-event surface always reports false, deliberately *not*
        // `EventProps.deprecated` (the orthogonal F5 deprecation fact).
        // `deprecatedSource` says so, and `lifecycle` carries the declared
        // BIG-IP release data under its own name, so a reader cannot take the
        // completion flag for an F5 deprecation verdict.
        let deprecated = false;
        let lifecycle = if known {
            events.event_lifecycle(&event).map(|life| {
                serde_json::json!({
                    "axis": "BIG-IP",
                    "introduced": life.introduced,
                    "deprecated": life.deprecated,
                    "retired": life.retired,
                })
            })
        } else {
            None
        };
        Ok(Some(serde_json::json!({
            "event": event,
            "known": known,
            "deprecated": deprecated,
            "deprecatedSource": "eventCompletionDetail",
            "lifecycle": lifecycle,
            "catalogue": "standaloneIrules",
            "validCommandCount": count,
            "sampleCommands": sample,
        })))
    }

    /// Handle `tcl-lsp.describeIruleCommand`: registry metadata for an iRules
    /// command (exact match, then case-insensitive).
    pub(in super::super) async fn describe_irule_command_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let name = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned();
        let registry = self.registry_for_dialect("f5-irules").await;
        let context = tcl_lsp_core::context_for_dialect("f5-irules");
        let resolved = context.context().resolve_spec_case_advice(&registry, &name);
        let value = match resolved {
            Some(spec) => serde_json::json!({
                "found": true,
                "command": spec.name,
                "summary": spec.hover.as_ref().map(|hover| hover.summary.to_owned()),
            }),
            None => serde_json::json!({ "found": false, "command": name }),
        };
        Ok(Some(value))
    }

    /// Handle `tcl-lsp.listIruleEvents`: all known iRules event names (sorted).
    /// Handle `tcl-lsp.listIruleEvents`: the sorted list of all registry
    /// event names.
    pub(in super::super) fn list_irule_events_command() -> serde_json::Value {
        let events = tcl_registry::events::EventRegistry::build();
        let mut names: Vec<&str> = events.all_event_names();
        names.sort_unstable();
        serde_json::json!({ "events": names })
    }

    /// Handle `tcl-lsp.listSubcommands`: subcommand metadata for `command`
    /// from the registry.
    /// An unknown command (or one with no subcommands) yields an empty list.
    pub(in super::super) async fn list_subcommands_command(
        &self,
        args: &[serde_json::Value],
    ) -> Option<serde_json::Value> {
        let name = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let dialect = self.session_dialect().await;
        let registry = self.registry_for_dialect(&dialect).await;
        // Dialect ingress — see the note on the hover path. Same reasoning and
        // the same non-claim: subcommands resolve for a `tk` session under
        // either resolution, since they come from the name-keyed registry
        // above. Resolving through the ingress keeps the profile and that
        // registry describing the same dialect.
        let mut subs: Vec<serde_json::Value> = tcl_lsp_core::document_context_for_dialect(&dialect)
            .resolve_spec(&registry, &name)
            .map_or_default(|spec| {
                spec.subcommands
                    .iter()
                    .map(|sub| {
                        serde_json::json!({
                            "name": sub.name,
                            "detail": sub.detail,
                            "synopsis": sub.synopsis,
                            "pure": sub.pure,
                            "mutator": sub.mutator,
                            // The registry tracks deprecation at command level,
                            // not per subcommand — report the shape with a
                            // conservative default.
                            "deprecated": false,
                        })
                    })
                    .collect()
            });
        subs.sort_by(|a, b| {
            a.get("name")
                .and_then(serde_json::Value::as_str)
                .cmp(&b.get("name").and_then(serde_json::Value::as_str))
        });
        Some(serde_json::json!({ "command": name, "subcommands": subs }))
    }

    /// Handle `tcl-lsp.listKnownPackages`.  The native server has no on-disk
    /// `package require` resolver, so it reports the empty
    /// set — the contract is a `packages` list, which downstream callers can
    /// rely on regardless of population.
    pub(in super::super) fn list_known_packages_command() -> serde_json::Value {
        serde_json::json!({ "packages": serde_json::Value::Array(vec![]) })
    }

    /// `tcl-lsp.listTclInstallations` — report the Tcl installations discovered
    /// on disk (for the editor's "select a Tcl installation" picker) plus the
    /// `auto_path` currently feeding the package database. The editor writes
    /// the chosen `tcl_library` (or a custom path) back to
    /// `tclLsp.libraryPaths`.
    pub(in super::super) async fn list_tcl_installations_command(&self) -> serde_json::Value {
        let discovered = Arc::clone(&self.discovered_tcl);
        let editor_paths = self.editor_library_paths.lock().await.clone();
        let roots: Vec<PathBuf> = self
            .workspace_folder_urls()
            .await
            .iter()
            .filter_map(|f| f.document_path().map(std::borrow::Cow::into_owned))
            .collect();
        let store = Arc::clone(&self.store);
        crate::rt::spawn_blocking(move || {
            let installs = discovered.get_or_init(|| {
                core_tcl_install::discover(&core_tcl_install::default_search_bases())
            });
            let active = effective_auto_path(store.as_ref(), &roots, &editor_paths, installs);
            serde_json::json!({
                "installations": installs
                    .iter()
                    .map(|i| {
                        let selection = match tcl_installation_selection_paths(i) {
                            Ok((tcl_library, auto_path)) => serde_json::json!({
                                "status": "available",
                                "tclLibrary": tcl_library,
                                "autoPath": auto_path,
                            }),
                            Err(reason) => serde_json::json!({
                                "status": "unavailable",
                                "reason": reason.as_str(),
                            }),
                        };
                        serde_json::json!({
                            "version": i.version,
                            // These legacy fields are presentation only. The picker
                            // writes only the explicitly checked `selection` payload.
                            "tclLibrary": i.tcl_library.to_string_lossy(),
                            "autoPath": i.auto_path.iter()
                                .map(|p| p.to_string_lossy().into_owned())
                                .collect::<Vec<_>>(),
                            "selection": selection,
                        })
                    })
                    .collect::<Vec<_>>(),
                "activeAutoPath": active.iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect::<Vec<_>>(),
                "editorLibraryPaths": editor_paths,
            })
        })
        .await
        .unwrap_or(serde_json::Value::Null)
    }

    /// Handle `tcl-lsp.suggestPackagesForSymbol`.  Echoes the symbol and an
    /// (empty, pending the package resolver) suggestion list.
    pub(in super::super) fn suggest_packages_for_symbol_command(
        args: &[serde_json::Value],
    ) -> serde_json::Value {
        let symbol = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        serde_json::json!({ "symbol": symbol, "suggestions": serde_json::Value::Array(vec![]) })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum TclInstallationSelectionUnavailable {
    NonUnicodeLibraryPath,
    NonUnicodeAutoPath,
}

impl TclInstallationSelectionUnavailable {
    pub(in super::super) const fn as_str(self) -> &'static str {
        match self {
            Self::NonUnicodeLibraryPath => "nonUnicodeLibraryPath",
            Self::NonUnicodeAutoPath => "nonUnicodeAutoPath",
        }
    }
}

/// Project an actual discovered installation into the exact Unicode paths the
/// editor can write back to `libraryPaths`. Lossy path strings remain display
/// fields only and can never be used as selection identity.
pub(in super::super) fn tcl_installation_selection_paths(
    installation: &core_tcl_install::TclInstallation,
) -> Result<(String, Vec<String>), TclInstallationSelectionUnavailable> {
    let tcl_library = installation
        .tcl_library
        .to_str()
        .ok_or(TclInstallationSelectionUnavailable::NonUnicodeLibraryPath)?
        .to_owned();
    let auto_path = installation
        .auto_path
        .iter()
        .map(|path| {
            path.to_str()
                .map(str::to_owned)
                .ok_or(TclInstallationSelectionUnavailable::NonUnicodeAutoPath)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((tcl_library, auto_path))
}
