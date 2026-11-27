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

//! Command tables and namespace maps keyed by the source namespace and
//! command keys, with their compatibility text queries.

use tcl_names::command::SourceCommandKey;
use tcl_names::namespace::SourceNamespaceKey;

pub(crate) trait CommandKeyQuery {
    fn command_key(&self) -> std::borrow::Cow<'_, SourceCommandKey>;
}
impl CommandKeyQuery for SourceCommandKey {
    fn command_key(&self) -> std::borrow::Cow<'_, SourceCommandKey> {
        std::borrow::Cow::Borrowed(self)
    }
}
impl CommandKeyQuery for str {
    fn command_key(&self) -> std::borrow::Cow<'_, SourceCommandKey> {
        std::borrow::Cow::Owned(self.into())
    }
}
impl CommandKeyQuery for String {
    fn command_key(&self) -> std::borrow::Cow<'_, SourceCommandKey> {
        self.as_str().command_key()
    }
}

/// The primary mutable command map. Legacy string queries address only the
/// authored domain; actual slots must supply their retained typed identity.
///
/// Values are shared: a state fork that writes one name copies the table's
/// keys and value handles, not every name's alternatives.
#[derive(Debug, Clone)]
pub(super) struct SourceCommandTable<V>(
    rustc_hash::FxHashMap<SourceCommandKey, std::sync::Arc<V>>,
    std::sync::Arc<DerivedRows<V>>,
);

/// Facts derived from a table's rows on first use. A clone shares them,
/// because its rows are the same; every write starts empty ones first, so
/// they never keep a row shared.
#[derive(Debug)]
struct DerivedRows<V> {
    /// The rows that name each command token, by the token's hash.
    token_rows: std::sync::OnceLock<rustc_hash::FxHashMap<u64, Vec<std::sync::Arc<V>>>>,
    /// The keys in the authored domain.
    authored: std::sync::OnceLock<Vec<SourceCommandKey>>,
}

impl<V> Default for DerivedRows<V> {
    fn default() -> Self {
        Self {
            token_rows: std::sync::OnceLock::new(),
            authored: std::sync::OnceLock::new(),
        }
    }
}

fn token_hash(token: &tcl_names::command::CommandIdentity) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    token.hash(&mut hasher);
    hasher.finish()
}

impl<V: PartialEq> PartialEq for SourceCommandTable<V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<V: Eq> Eq for SourceCommandTable<V> {}
impl<V> Default for SourceCommandTable<V> {
    fn default() -> Self {
        Self(rustc_hash::FxHashMap::default(), std::sync::Arc::default())
    }
}
impl<V> SourceCommandTable<V> {
    pub(super) fn get<Q: CommandKeyQuery + ?Sized>(&self, key: &Q) -> Option<&V> {
        self.0.get(key.command_key().as_ref()).map(AsRef::as_ref)
    }
    pub(super) fn contains_key<Q: CommandKeyQuery + ?Sized>(&self, key: &Q) -> bool {
        self.get(key).is_some()
    }
    pub(super) fn insert(&mut self, key: impl Into<SourceCommandKey>, value: V) {
        self.1 = std::sync::Arc::default();
        self.0.insert(key.into(), std::sync::Arc::new(value));
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = (&SourceCommandKey, &V)> {
        self.0.iter().map(|(key, value)| (key, value.as_ref()))
    }
    pub(super) fn values(&self) -> impl Iterator<Item = &V> {
        self.0.values().map(AsRef::as_ref)
    }
    /// At least every row `token_of` files under `token`; a row of another
    /// token appears only when the two tokens hash alike. `token_of` must be
    /// the one function this table is ever asked with: the first call decides
    /// which rows each token lists.
    pub(super) fn rows_naming<'a>(
        &'a self,
        token: &tcl_names::command::CommandIdentity,
        token_of: impl Fn(&V) -> Option<&tcl_names::command::CommandIdentity>,
    ) -> impl Iterator<Item = &'a V> {
        self.1
            .token_rows
            .get_or_init(|| {
                let mut rows = rustc_hash::FxHashMap::<u64, Vec<std::sync::Arc<V>>>::default();
                for value in self.0.values() {
                    if let Some(token) = token_of(value) {
                        rows.entry(token_hash(token))
                            .or_default()
                            .push(std::sync::Arc::clone(value));
                    }
                }
                rows
            })
            .get(&token_hash(token))
            .into_iter()
            .flatten()
            .map(AsRef::as_ref)
    }
    pub(super) fn keys(&self) -> impl Iterator<Item = &SourceCommandKey> {
        self.0.keys()
    }
    /// Every key with an authored spelling, in no particular order.
    pub(super) fn authored_keys(&self) -> &[SourceCommandKey] {
        self.1.authored.get_or_init(|| {
            self.0
                .keys()
                .filter(|key| key.authored_spelling().is_some())
                .cloned()
                .collect()
        })
    }
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Every key of either table except those both hold through one shared
    /// value allocation; such a value is equal on both sides.
    pub(super) fn keys_not_sharing_a_value<'a>(
        &'a self,
        other: &'a Self,
    ) -> impl Iterator<Item = &'a SourceCommandKey> {
        self.0
            .iter()
            .filter(|(key, value)| {
                other
                    .0
                    .get(*key)
                    .is_none_or(|shared| !std::sync::Arc::ptr_eq(value, shared))
            })
            .map(|(key, _)| key)
            .chain(other.0.keys().filter(|key| !self.0.contains_key(*key)))
    }
    pub(super) fn extend<K: Into<SourceCommandKey>>(
        &mut self,
        entries: impl IntoIterator<Item = (K, V)>,
    ) {
        self.1 = std::sync::Arc::default();
        self.0.extend(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), std::sync::Arc::new(value))),
        );
    }
}
impl<V: Clone + Default> SourceCommandTable<V> {
    /// The value at `key` for writing, inserted empty when absent.
    pub(super) fn entry_or_default(&mut self, key: impl Into<SourceCommandKey>) -> &mut V {
        self.1 = std::sync::Arc::default();
        std::sync::Arc::make_mut(self.0.entry(key.into()).or_default())
    }
}
impl<K: Into<SourceCommandKey>, V> FromIterator<(K, V)> for SourceCommandTable<V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut table = Self::default();
        table.extend(iter);
        table
    }
}

pub(crate) trait NamespaceKeyQuery {
    fn namespace_key(&self) -> std::borrow::Cow<'_, SourceNamespaceKey>;
}
impl NamespaceKeyQuery for SourceNamespaceKey {
    fn namespace_key(&self) -> std::borrow::Cow<'_, SourceNamespaceKey> {
        std::borrow::Cow::Borrowed(self)
    }
}
impl NamespaceKeyQuery for str {
    fn namespace_key(&self) -> std::borrow::Cow<'_, SourceNamespaceKey> {
        std::borrow::Cow::Owned(self.into())
    }
}
impl NamespaceKeyQuery for String {
    fn namespace_key(&self) -> std::borrow::Cow<'_, SourceNamespaceKey> {
        self.as_str().namespace_key()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SourceNamespaceMap<V>(std::collections::BTreeMap<SourceNamespaceKey, V>);
impl<V> Default for SourceNamespaceMap<V> {
    fn default() -> Self {
        Self(std::collections::BTreeMap::new())
    }
}
impl<V> SourceNamespaceMap<V> {
    pub(super) fn get<Q: NamespaceKeyQuery + ?Sized>(&self, key: &Q) -> Option<&V> {
        self.0.get(key.namespace_key().as_ref())
    }
    pub(super) fn insert(&mut self, key: impl Into<SourceNamespaceKey>, value: V) -> Option<V> {
        self.0.insert(key.into(), value)
    }
    pub(super) fn keys(&self) -> impl Iterator<Item = &SourceNamespaceKey> {
        self.0.keys()
    }
    pub(super) fn retain(&mut self, mut keep: impl FnMut(&SourceNamespaceKey, &mut V) -> bool) {
        self.0.retain(|key, value| keep(key, value));
    }
    pub(super) fn extend<K: Into<SourceNamespaceKey>>(
        &mut self,
        entries: impl IntoIterator<Item = (K, V)>,
    ) {
        self.0
            .extend(entries.into_iter().map(|(key, value)| (key.into(), value)));
    }
    pub(super) fn entry(
        &mut self,
        key: impl Into<SourceNamespaceKey>,
    ) -> std::collections::btree_map::Entry<'_, SourceNamespaceKey, V> {
        self.0.entry(key.into())
    }
}
impl<K: Into<SourceNamespaceKey>, V> FromIterator<(K, V)> for SourceNamespaceMap<V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = Self::default();
        map.extend(iter);
        map
    }
}

/// The source namespace inventory. Its advisory spelling list is derived
/// once and shared with every binding that reports it; any change drops it.
#[derive(Debug, Clone, Default)]
pub(super) struct SourceNamespaceSet(
    std::collections::BTreeSet<SourceNamespaceKey>,
    std::sync::OnceLock<std::sync::Arc<[String]>>,
    std::sync::OnceLock<std::sync::Arc<NamespacePathIndex>>,
);

/// Native and allocated keys by their constructed path.
type NamespacePathIndex =
    rustc_hash::FxHashMap<tcl_core_types::ByteNamespacePath, Vec<SourceNamespaceKey>>;
impl PartialEq for SourceNamespaceSet {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for SourceNamespaceSet {}
impl std::hash::Hash for SourceNamespaceSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl SourceNamespaceSet {
    pub(super) fn contains<Q: NamespaceKeyQuery + ?Sized>(&self, key: &Q) -> bool {
        self.0.contains(key.namespace_key().as_ref())
    }
    pub(super) fn insert(&mut self, key: impl Into<SourceNamespaceKey>) -> bool {
        let inserted = self.0.insert(key.into());
        if inserted {
            self.1 = std::sync::OnceLock::new();
            self.2 = std::sync::OnceLock::new();
        }
        inserted
    }
    pub(super) fn remove<Q: NamespaceKeyQuery + ?Sized>(&mut self, key: &Q) -> bool {
        let removed = self.0.remove(key.namespace_key().as_ref());
        if removed {
            self.1 = std::sync::OnceLock::new();
            self.2 = std::sync::OnceLock::new();
        }
        removed
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = &SourceNamespaceKey> {
        self.0.iter()
    }
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }
    pub(super) fn retain(&mut self, keep: impl FnMut(&SourceNamespaceKey) -> bool) {
        let before = self.0.len();
        self.0.retain(keep);
        if self.0.len() != before {
            self.1 = std::sync::OnceLock::new();
            self.2 = std::sync::OnceLock::new();
        }
    }
    pub(super) fn extend<K: Into<SourceNamespaceKey>>(
        &mut self,
        keys: impl IntoIterator<Item = K>,
    ) {
        let before = self.0.len();
        self.0.extend(keys.into_iter().map(Into::into));
        if self.0.len() != before {
            self.1 = std::sync::OnceLock::new();
            self.2 = std::sync::OnceLock::new();
        }
    }
    /// The retained key equal to `key`.
    pub(super) fn get(&self, key: &SourceNamespaceKey) -> Option<&SourceNamespaceKey> {
        self.0.get(key)
    }
    /// Native and allocated keys whose constructed path is `path`.
    pub(super) fn keys_at_path(
        &self,
        path: &tcl_core_types::ByteNamespacePath,
    ) -> &[SourceNamespaceKey] {
        self.2
            .get_or_init(|| {
                let mut index = NamespacePathIndex::default();
                for key in &self.0 {
                    let path = match key {
                        SourceNamespaceKey::Native(context) => &context.path,
                        SourceNamespaceKey::Allocated { path, .. } => path,
                        SourceNamespaceKey::Authored(_) => continue,
                    };
                    index.entry(path.clone()).or_default().push(key.clone());
                }
                std::sync::Arc::new(index)
            })
            .get(path)
            .map_or(&[], Vec::as_slice)
    }
    /// Advisory spellings of every namespace, in key order, shared.
    pub(super) fn advisory_names(&self) -> std::sync::Arc<[String]> {
        std::sync::Arc::clone(self.1.get_or_init(|| {
            self.0
                .iter()
                .filter_map(SourceNamespaceKey::advisory_key)
                .collect()
        }))
    }
}
impl<K: Into<SourceNamespaceKey>> FromIterator<K> for SourceNamespaceSet {
    fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
        let mut set = Self::default();
        set.extend(iter);
        set
    }
}

impl<V> SourceNamespaceMap<std::collections::BTreeSet<V>>
where
    V: Clone + Ord,
{
    pub(super) fn join_with_default(&mut self, other: &Self, default: &V) -> bool {
        let keys = self
            .keys()
            .chain(other.keys())
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let mut changed = false;
        for key in keys {
            let mut joined = self
                .get(&key)
                .cloned()
                .unwrap_or_else(|| std::collections::BTreeSet::from([default.clone()]));
            joined.extend(
                other
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| std::collections::BTreeSet::from([default.clone()])),
            );
            if self.get(&key) != Some(&joined) {
                self.insert(key, joined);
                changed = true;
            }
        }
        changed
    }
}
impl IntoIterator for SourceNamespaceSet {
    type Item = SourceNamespaceKey;
    type IntoIter = std::collections::btree_set::IntoIter<SourceNamespaceKey>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<Q: CommandKeyQuery + ?Sized, V> std::ops::Index<&Q> for SourceCommandTable<V> {
    type Output = V;
    fn index(&self, key: &Q) -> &Self::Output {
        self.get(key).expect("retained command-table key")
    }
}

impl super::SourceInvocationBinding {
    /// Original root namespace row, independent of the source entry's current frame.
    pub(crate) fn original_native_root_namespace_context(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<SourceNamespaceKey> {
        let advice = self.original_conditional_operand_layout_advice(tokens, registry)?;
        let metadata = advice
            .original_declaration_metadata_context(registry, tokens)
            .ok()??;
        metadata.matches_registry(registry).then_some(())?;
        metadata.source_analysis_input()?;
        advice.closed_lookup().then_some(())?;
        advice.snapshot.state.native_root_namespace_key()
    }
}

#[cfg(test)]
mod native_root_tests {
    use super::*;

    #[test]
    fn original_native_root_requires_the_actual_entry_incarnation() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let namespace = SourceNamespaceKey::from_native_entry(&entry).unwrap();
        assert!(namespace.is_native_root_in_entry(&entry));
        assert!(!SourceNamespaceKey::authored("::").is_native_root_in_entry(&entry));
        let (_other_owner, other) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        assert!(!namespace.is_native_root_in_entry(&other));
    }
}
