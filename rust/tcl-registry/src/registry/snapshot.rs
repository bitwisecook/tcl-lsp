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

//! An immutable, comparable snapshot of a registry and the keys that identify its semantics.

use super::CommandRegistry;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};
use tcl_dialect::model::{PackageFloor, SurfaceLayer};

/// Immutable registry snapshot for incremental consumers. Equality checks the
/// complete structural identity; a hash is only a table index. Specs and body
/// grammars have immutable process-lifetime identities, while registry indexes
/// and package/layer order are copied and owned by the snapshot.
#[derive(Clone, Debug)]
pub struct RegistrySnapshot(pub(super) Arc<RegistrySnapshotData>);

#[derive(Debug)]
pub(super) struct RegistrySnapshotData {
    pub(super) semantic_key: RegistrySemanticKey,
    pub(super) registry: Arc<CommandRegistry>,
    /// Pure projection of this immutable store's descriptor labels. The
    /// existing semantic key, equality and hash remain unchanged.
    pub(super) descriptor_coordinates: OnceLock<Arc<BTreeMap<String, String>>>,
}

/// Immutable structural registry identity without retained reader caches.
/// Equality checks every semantic field; fingerprints only select hash buckets.
#[derive(Clone, Debug)]
pub struct RegistrySemanticKey(pub(super) Arc<RegistrySemanticKeyData>);

#[derive(Debug)]
pub(super) struct RegistrySemanticKeyData {
    key: RegistrySnapshotKey,
    pub(super) fingerprint: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct RegistrySnapshotKey {
    pub(super) commands: Vec<(&'static str, Vec<usize>)>,
    overlays: Vec<usize>,
    layers: Vec<SurfaceLayer>,
    pub(super) profile: Option<tcl_dialect::DialectProfileKey>,
    pub(super) ambient_packages: Vec<(&'static str, &'static str)>,
    pub(super) own_packages: Vec<PackageFloor<'static>>,
    pub(super) special_vars: Vec<usize>,
    pub(super) overlay: Option<u64>,
    pub(super) pack_origins: Vec<(usize, crate::pack_origin::PackOrigin)>,
    pub(super) reference_texts: Vec<(usize, Arc<str>)>,
    pub(super) document_grammar: Option<usize>,
}

impl RegistrySnapshot {
    /// Collision-checked metadata coordinates from this exact frozen store.
    /// The table depends only on its descriptor labels and the pure coordinate
    /// rule; it reads no selection hooks, live registrations or source state.
    /// Reuse supplies no selected descriptor or Native command binding.
    #[must_use]
    pub fn command_descriptor_coordinates(&self) -> Arc<BTreeMap<String, String>> {
        Arc::clone(self.0.descriptor_coordinates.get_or_init(|| {
            Arc::new(
                crate::unique_command_descriptor_coordinates(
                    self.0.registry.command_descriptor_keys(),
                )
                .map(|(coordinate, key)| (coordinate, key.label().to_owned()))
                .collect(),
            )
        }))
    }

    /// Clone the cache-free identity for keys that do not need a registry reader.
    #[must_use]
    pub fn semantic_key(&self) -> RegistrySemanticKey {
        self.0.semantic_key.clone()
    }

    /// Read the retained registry rather than resolving its display name again.
    #[must_use]
    pub fn registry(&self) -> &CommandRegistry {
        &self.0.registry
    }

    /// Share the exact frozen command store with a retained analysis context.
    /// This preserves authored overrides and package/profile axes without
    /// reconstructing a catalogue or copying another registry generation.
    #[must_use]
    pub fn shared_registry(&self) -> Arc<CommandRegistry> {
        Arc::clone(&self.0.registry)
    }
}

impl PartialEq for RegistrySnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.semantic_key == other.0.semantic_key
    }
}

impl Eq for RegistrySnapshot {}

impl Hash for RegistrySnapshot {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.semantic_key.hash(state);
    }
}

impl RegistrySemanticKey {
    /// Share one allocation between every live snapshot of an equal store, so
    /// the hot equality check is a pointer comparison rather than a walk of
    /// the whole command inventory.
    pub(super) fn interned(key: RegistrySnapshotKey) -> Self {
        use std::collections::HashMap;
        use std::sync::{Mutex, Weak};
        static LIVE: OnceLock<Mutex<HashMap<u64, Vec<Weak<RegistrySemanticKeyData>>>>> =
            OnceLock::new();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        let fingerprint = hasher.finish();
        let mut live = LIVE
            .get_or_init(Mutex::default)
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let bucket = live.entry(fingerprint).or_default();
        bucket.retain(|weak| weak.strong_count() > 0);
        if let Some(shared) = bucket
            .iter()
            .filter_map(Weak::upgrade)
            .find(|data| data.key == key)
        {
            return Self(shared);
        }
        let data = Arc::new(RegistrySemanticKeyData { key, fingerprint });
        bucket.push(Arc::downgrade(&data));
        if live.len() > 256 {
            live.retain(|_, bucket| {
                bucket.retain(|weak| weak.strong_count() > 0);
                !bucket.is_empty()
            });
        }
        Self(data)
    }
}

impl PartialEq for RegistrySemanticKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.0.fingerprint == other.0.fingerprint && self.0.key == other.0.key)
    }
}

impl Eq for RegistrySemanticKey {}

impl Hash for RegistrySemanticKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.fingerprint.hash(state);
    }
}

impl CommandRegistry {
    /// Retain a frozen, shareable registry for caches that must preserve
    /// authored overrides, package pins, and dialect policies. Repeated calls
    /// share one snapshot; mutation creates a new snapshot while previous
    /// readers keep their original registry.
    #[must_use]
    pub fn snapshot(&self) -> RegistrySnapshot {
        self.snapshot
            .get_or_init(|| {
                // Exhaustive destructuring forces new registry axes to make an
                // explicit snapshot and identity decision at this shared seam.
                let Self {
                    by_name,
                    overlay_specs,
                    loaded_layers,
                    profile,
                    ambient_packages,
                    own_packages,
                    special_vars,
                    overlay,
                    pack_origins,
                    reference_texts,
                    generation,
                    document_grammar,
                    effective_semantics: _,
                    snapshot: _,
                    native_registrations: _,
                } = self;
                let mut commands: Vec<_> = by_name
                    .iter()
                    .map(|(&name, specs)| {
                        (
                            name,
                            specs
                                .iter()
                                .map(|&spec| std::ptr::from_ref(spec) as usize)
                                .collect(),
                        )
                    })
                    .collect();
                commands.sort_by_key(|(name, _)| *name);
                let mut pack_origins_key = pack_origins
                    .iter()
                    .map(|(&spec, origin)| (spec, origin.clone()))
                    .collect::<Vec<_>>();
                pack_origins_key.sort_by_key(|(spec, _)| *spec);
                let mut reference_texts_key = reference_texts
                    .iter()
                    .map(|(&spec, text)| (spec, Arc::clone(text)))
                    .collect::<Vec<_>>();
                reference_texts_key.sort_by_key(|(spec, _)| *spec);
                let key = RegistrySnapshotKey {
                    commands,
                    overlays: overlay_specs
                        .iter()
                        .map(|&spec| std::ptr::from_ref(spec) as usize)
                        .collect(),
                    layers: loaded_layers.clone(),
                    profile: profile.map(tcl_dialect::DialectProfile::cache_key),
                    ambient_packages: ambient_packages.clone(),
                    own_packages: own_packages.clone(),
                    special_vars: special_vars
                        .iter()
                        .map(|spec| std::ptr::from_ref(*spec).addr())
                        .collect(),
                    overlay: *overlay,
                    pack_origins: pack_origins_key,
                    reference_texts: reference_texts_key,
                    document_grammar: document_grammar
                        .map(|grammar| std::ptr::from_ref(grammar) as usize),
                };
                let registry = Self {
                    by_name: by_name.clone(),
                    overlay_specs: overlay_specs.clone(),
                    loaded_layers: loaded_layers.clone(),
                    profile: *profile,
                    ambient_packages: ambient_packages.clone(),
                    own_packages: own_packages.clone(),
                    special_vars: special_vars.clone(),
                    overlay: *overlay,
                    pack_origins: pack_origins.clone(),
                    reference_texts: reference_texts.clone(),
                    generation: *generation,
                    document_grammar: *document_grammar,
                    effective_semantics: OnceLock::from(self.effective_semantics()),
                    snapshot: OnceLock::new(),
                    native_registrations: Arc::clone(&self.native_registrations),
                };
                RegistrySnapshot(Arc::new(RegistrySnapshotData {
                    semantic_key: RegistrySemanticKey::interned(key),
                    registry: Arc::new(registry),
                    descriptor_coordinates: OnceLock::new(),
                }))
            })
            .clone()
    }
}
