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

//! Changing the `tclLsp` configuration of a running server and waiting until the server reflects it.

use super::*;

impl Lsp {
    /// Poll `getEffectiveConfig` until every key of `requested` is reflected in
    /// the server's applied config.
    pub fn settle_config(&mut self, requested: &Value) {
        let deadline = Instant::now() + scaled_timeout(DEFAULT_TIMEOUT);
        loop {
            let effective = self.effective_config("");
            if config_reflected(requested, &effective) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "requested config was not applied within {:?} (load-scaled)\n  \
                 requested: {requested}\n  effective: {effective}{}",
                scaled_timeout(DEFAULT_TIMEOUT),
                latency_barrier_timeout_note()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// The server's *resolved* config for `uri` (`tcl-lsp.getEffectiveConfig`) —
    /// the view the analyser/formatter actually applies.
    pub fn effective_config(&mut self, uri: &str) -> Value {
        self.execute_command("tcl-lsp.getEffectiveConfig", json!([uri]))
    }

    /// Make the server adopt `config` as the `tclLsp` section: update the reply
    /// this client returns for `workspace/configuration` and notify
    /// `didChangeConfiguration` so the server re-pulls. Returns the resolved
    /// config for `""`. Because each test owns its server, there is no shared
    /// state to restore.
    /// Change what this client answers `workspace/configuration` with, without
    /// telling the server.  For tests that drive the notification themselves —
    /// e.g. a burst, where the point is to count how many pulls the server
    /// actually makes.
    pub fn set_config(&mut self, config: Value) {
        *self.shared.tcllsp_config.lock().unwrap() = config;
    }

    /// Delay configuration replies for a transport-liveness scenario.
    pub fn set_configuration_reply_delay(&self, delay: Duration) {
        *self.shared.configuration_reply_delay.lock().unwrap() = delay;
    }

    pub fn apply_configuration(&mut self, config: Value) -> Value {
        *self.shared.tcllsp_config.lock().unwrap() = config;
        self.notify(
            "workspace/didChangeConfiguration",
            json!({ "settings": {} }),
        );
        self.effective_config("")
    }

    /// Apply `config`, then poll `getEffectiveConfig` for `settle_uri` until
    /// `predicate` holds — a deterministic barrier, so the caller never races
    /// the asynchronous re-pull. Returns the settled config.
    pub fn apply_configuration_settle(
        &mut self,
        config: Value,
        settle_uri: &str,
        predicate: impl Fn(&Value) -> bool,
    ) -> Value {
        *self.shared.tcllsp_config.lock().unwrap() = config;
        self.notify(
            "workspace/didChangeConfiguration",
            json!({ "settings": {} }),
        );
        let deadline = Instant::now() + scaled_timeout(DEFAULT_TIMEOUT);
        let mut last = Value::Null;
        while Instant::now() < deadline {
            last = self.effective_config(settle_uri);
            if predicate(&last) {
                return last;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!(
            "config did not settle within {:?} (load-scaled); last: {last}{}",
            scaled_timeout(DEFAULT_TIMEOUT),
            latency_barrier_timeout_note()
        );
    }
}

/// Whether every key of the `tclLsp` config `requested` is reflected in the
/// `getEffectiveConfig` reply `effective`.
///
/// The two shapes differ (`optimiser.profile` is reported as a flat
/// `optimiser_profile`), so the mapping is explicit. An unmapped key **panics**
/// rather than being skipped: silently ignoring it would silently restore the
/// unsynchronised behaviour this barrier exists to remove, and the next author
/// to add a config key to a test would get a flake instead of an error.
pub(super) fn config_reflected(requested: &Value, effective: &Value) -> bool {
    let Some(req) = requested.as_object() else {
        return true;
    };
    req.iter().all(|(key, want)| match key.as_str() {
        // Nested feature toggles: each requested toggle must match.
        "features" => want.as_object().is_none_or(|feats| {
            feats.iter().all(|(feat, on)| {
                effective
                    .get("features")
                    .and_then(|f| f.get(feat))
                    .is_some_and(|got| got == on)
            })
        }),
        // Nested optimiser settings, reported flat.
        "optimiser" => want.as_object().is_none_or(|opt| {
            opt.iter().all(|(k, v)| {
                let flat = match k.as_str() {
                    "enabled" => "optimiser_enabled",
                    "profile" => "optimiser_profile",
                    // A per-code override (`optimiser.O109 = false`) lands in
                    // the disabled set, which is reported — but matching it
                    // properly means replicating the profile→disabled
                    // derivation, so refuse rather than pretend to check it.
                    other => panic!(
                        "config_reflected: no settle mapping for `optimiser.{other}` \
                         — add one (see getEffectiveConfig) so the config is a real barrier"
                    ),
                };
                effective.get(flat).is_some_and(|got| got == v)
            })
        }),
        "libraryPaths" => effective
            .get("library_paths")
            .is_some_and(|got| got == want),
        // Configured paths need the actual settled discovery/provider receipt.
        // A nonempty roster from an earlier generation cannot close this gate.
        "specPacks" => {
            effective.get("spec_packs").is_some_and(|got| got == want)
                && want.as_array().is_some_and(|paths| {
                    paths.is_empty()
                        || effective.get("spec_pack_publication").is_some_and(|publication| {
                            publication.get("status").and_then(Value::as_str) == Some("complete")
                                && publication.get("configured") == Some(want)
                        })
                })
        }
        // Nested formatting settings; only `docstringStyle` has a settle
        // signal (`getEffectiveConfig`'s `docstring_style`) today.
        "formatting" => want.as_object().is_none_or(|fmt| {
            fmt.iter().all(|(k, v)| {
                let flat = match k.as_str() {
                    "docstringStyle" => "docstring_style",
                    other => panic!(
                        "config_reflected: no settle mapping for `formatting.{other}` \
                         — add one (see getEffectiveConfig) so the config is a real barrier"
                    ),
                };
                effective.get(flat).is_some_and(|got| got == v)
            })
        }),
        // `tclLsp.iruleslx` is folder-scoped and reported resolved —
        // absolute paths, which the request does not carry — so the barrier is
        // that every declared plugin *name* has reached the applied config.
        // That is the thing a test then depends on: an unapplied declaration
        // resolves nothing at all.
        "iruleslx" => want
            .get("plugins")
            .and_then(Value::as_object)
            .is_none_or(|plugins| {
                let got = effective.get("iruleslx_plugins").and_then(Value::as_array);
                plugins.keys().all(|name| {
                    got.is_some_and(|list| {
                        list.iter()
                            .any(|entry| entry.get("plugin").is_some_and(|p| p == name))
                    })
                })
            }),
        // `tclLsp.workspaceScan.maxFiles` (#2021) is session-wide and reported
        // flat.  The barrier matters here as much as anywhere: the budget
        // decides which files the scan indexes, so a test that pushes one and
        // then reads the scan must not race the apply.
        "workspaceScan" => want.as_object().is_none_or(|scan| {
            scan.iter().all(|(k, v)| {
                let flat = match k.as_str() {
                    "maxFiles" => "workspace_scan_max_files",
                    other => panic!(
                        "config_reflected: no settle mapping for `workspaceScan.{other}`                          — add one (see getEffectiveConfig) so the config is a real barrier"
                    ),
                };
                effective.get(flat).is_some_and(|got| got == v)
            })
        }),
        // `tclLsp.notifications.*` is session-wide and reported flat.
        "notifications" => want.as_object().is_none_or(|notes| {
            notes.iter().all(|(k, v)| {
                let flat = match k.as_str() {
                    "environmentKind" => "notifications_environment_kind",
                    other => panic!(
                        "config_reflected: no settle mapping for `notifications.{other}` \
                         — add one (see getEffectiveConfig) so the config is a real barrier"
                    ),
                };
                effective.get(flat).is_some_and(|got| got == v)
            })
        }),
        "dialect" => effective.get("dialect").is_some_and(|got| got == want),
        "lineLength" => effective.get("line_length").is_some_and(|got| got == want),
        other => panic!(
            "config_reflected: no settle mapping for `{other}` — add one (see \
             getEffectiveConfig) so `Lsp::with_config` remains a real barrier"
        ),
    })
}
