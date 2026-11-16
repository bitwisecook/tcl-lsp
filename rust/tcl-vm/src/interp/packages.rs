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

//! The package database: provided versions, `package ifneeded` scripts, loading markers, the unknown handler and the version preference.

use super::{InterpId, PackageLoadingMarker, Vm};
use crate::value::Value;
use tcl_dialect::PackagePrefer;

/// Tcl's initial package-selection policy. Merely defining the environment
/// variable opts in; its value is intentionally ignored, matching Tcl 9.0.4.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub(super) fn initial_package_prefer() -> PackagePrefer {
    if std::env::var_os("TCL_PKG_PREFER_LATEST").is_some() {
        PackagePrefer::Latest
    } else {
        PackagePrefer::Stable
    }
}

/// A browser-WASM interpreter has no process environment from which to opt in.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(super) fn initial_package_prefer() -> PackagePrefer {
    PackagePrefer::Stable
}

impl Vm {
    /// Record a version under an already selected package-database key.
    pub(crate) fn provide_package(&mut self, name: impl AsRef<[u8]>, version: impl AsRef<[u8]>) {
        self.record_package_entry(name.as_ref());
        self.selected_package_state_mut().packages.insert(
            tcl_core_types::NameBytes::from(name.as_ref()),
            tcl_core_types::NameBytes::from(version.as_ref()),
        );
    }

    /// Withdraw an already selected provided key, retaining loader entries.
    pub(crate) fn forget_package(&mut self, name: impl AsRef<[u8]>) {
        self.selected_package_state_mut()
            .packages
            .remove(name.as_ref());
        self.selected_package_state_mut()
            .version_objects
            .remove(name.as_ref());
    }

    pub(crate) fn package_version_bytes(
        &self,
        name: impl AsRef<[u8]>,
    ) -> Option<&tcl_core_types::NameBytes> {
        self.selected_package_state().packages.get(name.as_ref())
    }

    pub(crate) fn package_version_object(&mut self, name: impl AsRef<[u8]>) -> Value {
        let Some(version) = self
            .selected_package_state()
            .packages
            .get(name.as_ref())
            .cloned()
        else {
            return Value::empty();
        };
        if self.selected_package_protocol().is_some_and(
            tcl_registry::native_package::NativePackageProtocol::retains_version_object,
        ) {
            self.selected_package_state_mut()
                .version_objects
                .entry(tcl_core_types::NameBytes::from(name.as_ref()))
                .or_insert_with(|| Value::from_native_string_bytes(version.as_bytes()))
                .clone()
        } else {
            Value::from_native_string_bytes(version.as_bytes())
        }
    }

    /// Record that the library `prefix` is loaded into this interpreter, from
    /// `file_name`. A prefix is listed once, under the file it was first loaded
    /// from.
    pub(crate) fn note_library_loaded(&mut self, file_name: &str, prefix: &str) {
        let libraries = &mut self.package_state.loaded_libraries;
        if !libraries.iter().any(|(_, loaded)| loaded == prefix) {
            libraries.push((file_name.to_owned(), prefix.to_owned()));
        }
    }

    /// Borrow the inventory of the actual interpreter selected by original path resolution.
    pub(crate) fn loaded_libraries_in(&self, selected: InterpId) -> Option<&[(String, String)]> {
        if !self.interp_alive(selected) {
            return None;
        }
        self.st_of(selected)
            .map(|state| state.package_state.loaded_libraries.as_slice())
    }

    #[cfg(test)]
    pub(crate) fn package_version(&self, name: impl AsRef<[u8]>) -> Option<&str> {
        self.package_version_bytes(name)
            .map(|version| version.try_utf8().expect("Unicode test version"))
    }

    #[cfg(test)]
    pub(crate) fn package_ifneeded_versions(&self, name: impl AsRef<[u8]>) -> Vec<String> {
        self.package_ifneeded_versions_bytes(name)
            .into_iter()
            .map(|version| version.try_utf8().expect("Unicode test version").to_owned())
            .collect()
    }

    /// Native table inventory; failed require shells affect table growth without
    /// becoming visible package names.
    pub(crate) fn package_names(
        &self,
    ) -> Result<Vec<tcl_core_types::NameBytes>, tcl_syntax::value::ValueError> {
        if self.package_table_is_authored() {
            let state = self
                .authored_packages
                .as_ref()
                .expect("selected authored table");
            return Ok(state
                .order
                .iter()
                .filter(|key| {
                    state.table.packages.contains_key(*key)
                        || state.table.package_ifneeded.contains_key(*key)
                })
                .cloned()
                .collect());
        }
        let keys = self
            .selected_package_state()
            .package_entry_order
            .keys()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "package table ABI",
            ))?;
        Ok(keys
            .into_iter()
            .filter(|key| {
                self.selected_package_state().packages.contains_key(*key)
                    || self
                        .selected_package_state()
                        .package_ifneeded
                        .contains_key(*key)
            })
            .map(tcl_core_types::NameBytes::from)
            .collect())
    }

    pub(crate) fn set_package_ifneeded(
        &mut self,
        name: impl AsRef<[u8]>,
        version: impl AsRef<[u8]>,
        script: &[u8],
        release: tcl_dialect::TclVersion,
    ) {
        self.record_package_entry(name.as_ref());
        let origin = self
            .selected_package_state()
            .package_source_paths
            .last()
            .cloned();
        let versions = self
            .selected_package_state_mut()
            .package_ifneeded
            .entry(tcl_core_types::NameBytes::from(name.as_ref()))
            .or_default();
        let comparable = tcl_core_types::c_string_extent(version.as_ref());
        let registered = if let Some((known, stored)) = versions.iter_mut().find(|(known, _)| {
            tcl_dialect::compare_versions_bytes_for(
                tcl_core_types::c_string_extent(known.as_bytes()),
                comparable,
                release,
            ) == std::cmp::Ordering::Equal
        }) {
            *stored = script.to_vec();
            known.clone()
        } else {
            versions.push((
                tcl_core_types::NameBytes::from(version.as_ref()),
                script.to_vec(),
            ));
            tcl_core_types::NameBytes::from(version.as_ref())
        };
        let key = (tcl_core_types::NameBytes::from(name.as_ref()), registered);
        if let Some(origin) = origin {
            self.selected_package_state_mut()
                .package_loader_origins
                .insert(key, origin);
        } else {
            self.selected_package_state_mut()
                .package_loader_origins
                .remove(&key);
        }
    }

    pub(crate) fn package_ifneeded(
        &self,
        name: impl AsRef<[u8]>,
        version: impl AsRef<[u8]>,
        release: tcl_dialect::TclVersion,
    ) -> Option<&Vec<u8>> {
        let comparable = tcl_core_types::c_string_extent(version.as_ref());
        self.selected_package_state()
            .package_ifneeded
            .get(name.as_ref())
            .and_then(|versions| {
                versions.iter().find_map(|(known, script)| {
                    (tcl_dialect::compare_versions_bytes_for(
                        tcl_core_types::c_string_extent(known.as_bytes()),
                        comparable,
                        release,
                    ) == std::cmp::Ordering::Equal)
                        .then_some(script)
                })
            })
    }

    pub(crate) fn package_ifneeded_versions_bytes(
        &self,
        name: impl AsRef<[u8]>,
    ) -> Vec<tcl_core_types::NameBytes> {
        self.selected_package_state()
            .package_ifneeded
            .get(name.as_ref())
            .map_or_default(|versions| {
                versions
                    .iter()
                    .map(|(version, _)| version.clone())
                    .collect()
            })
    }

    pub(crate) fn package_loading_version_bytes(
        &self,
        name: impl AsRef<[u8]>,
    ) -> Option<&tcl_core_types::NameBytes> {
        self.selected_package_state()
            .package_loading
            .iter()
            .rev()
            .find(|marker| !marker.forgotten && marker.name.as_bytes() == name.as_ref())
            .map(|marker| &marker.version)
    }

    pub(crate) fn begin_package_loading(
        &mut self,
        name: impl AsRef<[u8]>,
        version: impl AsRef<[u8]>,
    ) {
        if self
            .selected_package_protocol()
            .is_some_and(tcl_registry::native_package::NativePackageProtocol::tracks_files)
        {
            self.selected_package_state_mut()
                .package_file_inventory_active = true;
            self.selected_package_state_mut()
                .package_file_scopes
                .push(tcl_core_types::NameBytes::from(name.as_ref()));
        }
        self.selected_package_state_mut()
            .package_loading
            .push(PackageLoadingMarker {
                name: tcl_core_types::NameBytes::from(name.as_ref()),
                version: tcl_core_types::NameBytes::from(version.as_ref()),
                forgotten: false,
            });
    }

    /// Cleanup remains tolerant of a loader forgetting its own package.
    #[cfg(test)]
    pub(crate) fn end_package_loading(
        &mut self,
        name: impl AsRef<[u8]>,
        version: impl AsRef<[u8]>,
    ) {
        let tracks_files = self
            .selected_package_protocol()
            .is_some_and(tcl_registry::native_package::NativePackageProtocol::tracks_files);
        self.end_package_loading_using(name, version, tracks_files);
    }

    pub(in super::super) fn end_package_loading_using(
        &mut self,
        name: impl AsRef<[u8]>,
        version: impl AsRef<[u8]>,
        tracks_files: bool,
    ) {
        // A loader suspended in a coroutine can finish after a later one
        // began, so the marker retired is this loader's own, wherever it sits
        // (upstream's `PackageLoadToken`); the innermost matching one.
        let position = self
            .selected_package_state()
            .package_loading
            .iter()
            .rposition(|marker| {
                marker.name.as_bytes() == name.as_ref()
                    && marker.version.as_bytes() == version.as_ref()
            });
        if let Some(position) = position {
            self.selected_package_state_mut()
                .package_loading
                .remove(position);
            if tracks_files {
                let scopes = &mut self.selected_package_state_mut().package_file_scopes;
                if let Some(scope) = scopes
                    .iter()
                    .rposition(|scope| scope.as_bytes() == name.as_ref())
                {
                    scopes.remove(scope);
                }
            }
        }
    }

    /// Retire only the loading marker of a require whose loader activation
    /// was refused by the host.
    pub(crate) fn end_abandoned_package_loading(
        &mut self,
        name: &tcl_core_types::NameBytes,
        version: &tcl_core_types::NameBytes,
    ) {
        let tracks_files = self
            .selected_package_protocol()
            .is_some_and(tcl_registry::native_package::NativePackageProtocol::tracks_files);
        self.end_package_loading_using(name, version, tracks_files);
    }

    /// Retire the loading marker of a require abandoned while its loader was
    /// suspended (its coroutine was deleted).
    pub(crate) fn abandon_package_loading(
        &mut self,
        name: &tcl_core_types::NameBytes,
        version: &tcl_core_types::NameBytes,
    ) {
        let tracks_files = self
            .selected_package_protocol()
            .is_some_and(tcl_registry::native_package::NativePackageProtocol::tracks_files);
        self.end_package_loading_using(name, version, tracks_files);
        // The version the loader provided before it was suspended is
        // withdrawn, as a failed loader's is.
        self.forget_package(name);
    }

    pub(crate) fn forget_package_completely(&mut self, name: impl AsRef<[u8]>) {
        if self.package_table_is_authored() {
            self.authored_packages
                .as_mut()
                .expect("selected authored table")
                .order
                .retain(|key| key.as_bytes() != name.as_ref());
        }
        self.selected_package_state_mut()
            .package_entry_order
            .remove(name.as_ref());
        self.selected_package_state_mut()
            .packages
            .remove(name.as_ref());
        self.selected_package_state_mut()
            .version_objects
            .remove(name.as_ref());
        self.selected_package_state_mut()
            .package_ifneeded
            .remove(name.as_ref());
        self.selected_package_state_mut()
            .package_files
            .remove(name.as_ref());
        self.selected_package_state_mut()
            .package_loader_origins
            .retain(|(key, _), _| key.as_bytes() != name.as_ref());
        for marker in &mut self.selected_package_state_mut().package_loading {
            if marker.name.as_bytes() == name.as_ref() {
                marker.forgotten = true;
            }
        }
    }

    pub(crate) fn set_package_unknown(&mut self, script: Option<Vec<u8>>) {
        if self.package_table_is_authored() {
            self.authored_packages
                .as_mut()
                .expect("selected authored table")
                .unknown = script;
        } else {
            self.package_unknown = script;
        }
    }

    pub(crate) fn package_unknown(&self) -> Option<&Vec<u8>> {
        if self.package_table_is_authored() {
            self.authored_packages.as_ref()?.unknown.as_ref()
        } else {
            self.package_unknown.as_ref()
        }
    }

    pub(crate) fn package_prefer(&self) -> PackagePrefer {
        if self.package_table_is_authored() {
            self.authored_packages
                .as_ref()
                .expect("selected authored table")
                .prefer
        } else {
            self.package_prefer
        }
    }

    /// Raise the package selection policy to `latest`. Tcl deliberately does
    /// not provide the inverse transition: `package prefer stable` is a no-op
    /// after this latch has been raised.
    pub(crate) fn prefer_latest_packages(&mut self) {
        if self.package_table_is_authored() {
            self.authored_packages
                .as_mut()
                .expect("selected authored table")
                .prefer = PackagePrefer::Latest;
        } else {
            self.package_prefer = PackagePrefer::Latest;
        }
    }
}
