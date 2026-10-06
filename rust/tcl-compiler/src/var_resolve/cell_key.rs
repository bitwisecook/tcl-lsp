// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Domain-separated variable storage keys and compatibility-only text queries.

use crate::command_binding::SourceNamespaceKey;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::{Deref, DerefMut},
};

/// Namespace membership for diagnostics, keeping authored scope advice separate
/// from a retained namespace owner. This supplies no contents or execution fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableNamespaceMembership<'a> {
    /// Original namespace incarnation and component geometry.
    Retained(&'a SourceNamespaceKey),
    /// Explicit symbolic namespace scope; never a native namespace token.
    Authored(&'a str),
}

/// Exact variable storage address. A written string cannot impersonate a
/// retained namespace slot, even when its text equals a diagnostic label.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariableCellKey {
    /// Compatibility storage key from an authored symbolic frame.
    Authored(String),
    /// Original namespace table and already selected simple variable name.
    Namespace {
        /// Retained namespace incarnation and component boundaries.
        identity: SourceNamespaceKey,
        /// Selected variable-table key, without written-name reinterpretation.
        simple: String,
    },
    /// Cell in an actual source-proved activation.
    Activation {
        /// Original activation identity.
        identity: String,
        /// Selected local key.
        simple: String,
    },
    /// Cell in an explicitly selected foreign stack frame.
    SelectedFrame {
        /// Original validated selector.
        selector: tcl_registry::FrameLevel,
        /// Selected cell key.
        simple: String,
    },
    /// Cell in an actual object instance storage table.
    Instance {
        /// Original instance identity.
        identity: String,
        /// Selected variable key.
        simple: String,
    },
    /// Cell belonging to an actual bounded object allocation.
    AllocatedInstance {
        /// Original allocation identity.
        allocation: Box<crate::command_binding::SourceObjectAllocation>,
        /// Selected member key.
        simple: String,
    },
    /// Actual retained raw wrapper or callable-owned static allocation.
    RetainedSlot(Box<crate::raw_binding::RawBindingSlotId>),
    /// A particular physical lifetime of an otherwise identical storage slot.
    Lifetime {
        /// Reached operation which replaced the previous lifetime.
        source: u32,
        /// Original typed cell slot.
        cell: Box<VariableCellKey>,
    },
    /// An original array member distinct from the array's root slot.
    Element {
        /// Original typed array root.
        cell: Box<VariableCellKey>,
        /// Exact selected element name.
        index: String,
    },
}
impl VariableCellKey {
    /// Compatibility spelling only; never a lookup or equality proof.
    #[must_use]
    pub fn compatibility_name(&self) -> String {
        match self {
            Self::Authored(name) => name.clone(),
            Self::Namespace { identity, simple } => format!("{identity:?}::{simple}"),
            Self::Activation { identity, simple } => {
                format!("@frame:{}:{identity}:{simple}", identity.len())
            }
            Self::SelectedFrame { selector, simple } => format!("@selected:{selector:?}:{simple}"),
            Self::Instance { identity, simple } => {
                format!("@instance:{}:{identity}:{simple}", identity.len())
            }
            Self::AllocatedInstance { allocation, simple } => format!("{allocation:?}:{simple}"),
            Self::RetainedSlot(slot) => format!("{slot:?}"),
            Self::Lifetime { source, cell } => {
                format!("@lifetime:{source}:{}", cell.compatibility_name())
            }
            Self::Element { cell, index } => format!("{}({index})", cell.compatibility_name()),
        }
    }
    /// Symbolic authored key, excluding every native or allocated slot.
    #[must_use]
    pub fn authored_spelling(&self) -> Option<&str> {
        if let Self::Authored(name) = self {
            Some(name)
        } else {
            None
        }
    }
    /// Root slot, excluding element and lifetime wrappers.
    #[must_use]
    pub fn root(&self) -> &Self {
        match self {
            Self::Lifetime { cell, .. } | Self::Element { cell, .. } => cell.root(),
            key => key,
        }
    }
    /// Whether this address is an array member of the exact original root.
    #[must_use]
    pub fn is_member_of(&self, root: &Self) -> bool {
        match (self, root) {
            (Self::Element { cell, .. }, root) => cell.as_ref() == root,
            (
                Self::Lifetime { source, cell },
                Self::Lifetime {
                    source: root_source,
                    cell: root_cell,
                },
            ) if source == root_source => cell.is_member_of(root_cell),
            _ => false,
        }
    }
    /// Exact namespace membership; rendered text cannot select native slots.
    #[must_use]
    pub fn is_in_namespace(&self, namespace: &SourceNamespaceKey) -> bool {
        match (self.root(), namespace) {
            (Self::Namespace { identity, .. }, namespace) => {
                namespace_contains(namespace, identity)
            }
            (Self::Authored(name), SourceNamespaceKey::Authored(namespace)) => {
                let root = name.as_str();
                if namespace == "::" {
                    root.starts_with("::")
                } else {
                    root.starts_with(namespace)
                        && root
                            .get(namespace.len()..)
                            .is_some_and(|tail| tail.starts_with("::"))
                }
            }
            (Self::RetainedSlot(slot), namespace) => match slot.as_ref() {
                crate::raw_binding::RawBindingSlotId::Variable(cell) => match &cell.owner {
                    crate::place::CellOwner::NamespaceIdentity(owner) => {
                        namespace_contains(namespace, owner)
                    }
                    crate::place::CellOwner::Namespace(owner) => {
                        namespace_contains(namespace, &SourceNamespaceKey::authored(owner))
                    }
                    _ => false,
                },
                crate::raw_binding::RawBindingSlotId::Callable { .. } => false,
            },
            _ => false,
        }
    }
    /// Exact containing namespace incarnation, independent of element and lifetime wrappers.
    #[must_use]
    pub fn namespace_identity(&self) -> Option<&SourceNamespaceKey> {
        match self.root() {
            Self::Namespace { identity, .. } => Some(identity),
            Self::RetainedSlot(slot) => match slot.as_ref() {
                crate::raw_binding::RawBindingSlotId::Variable(cell) => match &cell.owner {
                    crate::place::CellOwner::NamespaceIdentity(identity) => Some(identity),
                    _ => None,
                },
                crate::raw_binding::RawBindingSlotId::Callable { .. } => None,
            },
            _ => None,
        }
    }
    /// Structural namespace visibility for diagnostics. Only an authored key
    /// uses compatibility naming rules; native labels are never decoded.
    #[must_use]
    pub fn namespace_membership_for_advice(&self) -> Option<VariableNamespaceMembership<'_>> {
        if let Some(namespace) = self.namespace_identity() {
            return Some(match namespace {
                SourceNamespaceKey::Authored(scope) => VariableNamespaceMembership::Authored(scope),
                _ => VariableNamespaceMembership::Retained(namespace),
            });
        }
        match self.root() {
            Self::Authored(name) if tcl_syntax::naming::is_qualified(name.as_bytes()) => {
                let (namespace, _) = tcl_syntax::naming::key_holder_and_tail(name);
                Some(VariableNamespaceMembership::Authored(namespace))
            }
            Self::RetainedSlot(slot) => match slot.as_ref() {
                crate::raw_binding::RawBindingSlotId::Variable(cell) => match &cell.owner {
                    crate::place::CellOwner::Namespace(scope) => {
                        Some(VariableNamespaceMembership::Authored(scope))
                    }
                    _ => None,
                },
                crate::raw_binding::RawBindingSlotId::Callable { .. } => None,
            },
            _ => None,
        }
    }

    /// Exact retired table; authored namespace retirement remains a symbolic tree query.
    #[must_use]
    pub fn is_owned_by_namespace(&self, namespace: &SourceNamespaceKey) -> bool {
        match namespace {
            SourceNamespaceKey::Authored(_) => self.is_in_namespace(namespace),
            _ => self.namespace_identity() == Some(namespace),
        }
    }
    /// Whether this slot belongs to a namespace table, without parsing a native label.
    #[must_use]
    pub fn is_namespace_storage(&self) -> bool {
        match self.root() {
            Self::Namespace { .. } => true,
            Self::Authored(name) => name.starts_with("::"),
            Self::RetainedSlot(slot) => {
                matches!(slot.as_ref(), crate::raw_binding::RawBindingSlotId::Variable(cell) if matches!(cell.owner, crate::place::CellOwner::Namespace(_) | crate::place::CellOwner::NamespaceIdentity(_)))
            }
            _ => false,
        }
    }
    /// Select an array element without modifying its root identity.
    #[must_use]
    pub fn with_index(self, index: impl Into<String>) -> Self {
        Self::Element {
            cell: Box::new(self),
            index: index.into(),
        }
    }
    /// Retain a replaced lifetime without encoding its identity as text.
    #[must_use]
    pub fn with_lifetime(self, source: u32) -> Self {
        Self::Lifetime {
            source,
            cell: Box::new(self),
        }
    }
}
impl From<String> for VariableCellKey {
    fn from(name: String) -> Self {
        Self::Authored(name)
    }
}
impl From<&str> for VariableCellKey {
    fn from(name: &str) -> Self {
        Self::Authored(name.to_owned())
    }
}
impl From<&String> for VariableCellKey {
    fn from(name: &String) -> Self {
        name.as_str().into()
    }
}
impl std::fmt::Display for VariableCellKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.compatibility_name())
    }
}

/// Domain-aware query; text addresses only the authored domain.
pub trait VariableCellKeyQuery {
    /// Retain an exact key or construct an authored query.
    fn variable_cell_key(&self) -> Cow<'_, VariableCellKey>;
}
impl VariableCellKeyQuery for VariableCellKey {
    fn variable_cell_key(&self) -> Cow<'_, Self> {
        Cow::Borrowed(self)
    }
}
impl VariableCellKeyQuery for str {
    fn variable_cell_key(&self) -> Cow<'_, VariableCellKey> {
        Cow::Owned(self.into())
    }
}
impl VariableCellKeyQuery for String {
    fn variable_cell_key(&self) -> Cow<'_, VariableCellKey> {
        self.as_str().variable_cell_key()
    }
}
impl<T: VariableCellKeyQuery + ?Sized> VariableCellKeyQuery for &T {
    fn variable_cell_key(&self) -> Cow<'_, VariableCellKey> {
        (*self).variable_cell_key()
    }
}

/// Mutable cell facts keyed by exact storage identity. String queries select
/// only authored facts; they cannot find or overwrite native namespace facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableCellTable<V>(HashMap<VariableCellKey, V>);
impl<V> Default for VariableCellTable<V> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}
impl<V> Deref for VariableCellTable<V> {
    type Target = HashMap<VariableCellKey, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<V> DerefMut for VariableCellTable<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<V> VariableCellTable<V> {
    /// Query one exact domain-separated cell.
    pub fn get<Q: VariableCellKeyQuery + ?Sized>(&self, key: &Q) -> Option<&V> {
        self.0.get(key.variable_cell_key().as_ref())
    }
    /// Mutate one exact domain-separated cell.
    pub fn get_mut<Q: VariableCellKeyQuery + ?Sized>(&mut self, key: &Q) -> Option<&mut V> {
        self.0.get_mut(key.variable_cell_key().as_ref())
    }
    /// Whether this domain-separated cell is present.
    pub fn contains_key<Q: VariableCellKeyQuery + ?Sized>(&self, key: &Q) -> bool {
        self.get(key).is_some()
    }
    /// Publish facts for an exact cell or an explicitly authored string key.
    pub fn insert(&mut self, key: impl Into<VariableCellKey>, value: V) -> Option<V> {
        self.0.insert(key.into(), value)
    }
    /// Remove facts for only the selected domain.
    pub fn remove<Q: VariableCellKeyQuery + ?Sized>(&mut self, key: &Q) -> Option<V> {
        self.0.remove(key.variable_cell_key().as_ref())
    }
    /// Select an exact slot for incremental fact publication.
    pub fn entry(
        &mut self,
        key: impl Into<VariableCellKey>,
    ) -> std::collections::hash_map::Entry<'_, VariableCellKey, V> {
        self.0.entry(key.into())
    }
}
impl<V, K: Into<VariableCellKey>> FromIterator<(K, V)> for VariableCellTable<V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }
}
impl<V, K: Into<VariableCellKey>> Extend<(K, V)> for VariableCellTable<V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.0
            .extend(iter.into_iter().map(|(key, value)| (key.into(), value)));
    }
}
impl<'a, V> IntoIterator for &'a VariableCellTable<V> {
    type Item = (&'a VariableCellKey, &'a V);
    type IntoIter = std::collections::hash_map::Iter<'a, VariableCellKey, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<'a, V> IntoIterator for &'a mut VariableCellTable<V> {
    type Item = (&'a VariableCellKey, &'a mut V);
    type IntoIter = std::collections::hash_map::IterMut<'a, VariableCellKey, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}
impl<V> IntoIterator for VariableCellTable<V> {
    type Item = (VariableCellKey, V);
    type IntoIter = std::collections::hash_map::IntoIter<VariableCellKey, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Exact cell inventory with authored-only string queries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariableCellSet(HashSet<VariableCellKey>);
impl Deref for VariableCellSet {
    type Target = HashSet<VariableCellKey>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for VariableCellSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl VariableCellSet {
    /// Test membership in only the selected key domain.
    pub fn contains<Q: VariableCellKeyQuery + ?Sized>(&self, key: &Q) -> bool {
        self.0.contains(key.variable_cell_key().as_ref())
    }
    /// Retain one exact cell slot.
    pub fn insert(&mut self, key: impl Into<VariableCellKey>) -> bool {
        self.0.insert(key.into())
    }
    /// Retire one exact cell slot.
    pub fn remove<Q: VariableCellKeyQuery + ?Sized>(&mut self, key: &Q) -> bool {
        self.0.remove(key.variable_cell_key().as_ref())
    }
}
impl<K: Into<VariableCellKey>> FromIterator<K> for VariableCellSet {
    fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
        Self(iter.into_iter().map(Into::into).collect())
    }
}
impl<K: Into<VariableCellKey>> Extend<K> for VariableCellSet {
    fn extend<T: IntoIterator<Item = K>>(&mut self, iter: T) {
        self.0.extend(iter.into_iter().map(Into::into));
    }
}
impl<K: Into<VariableCellKey>, const N: usize> From<[K; N]> for VariableCellSet {
    fn from(items: [K; N]) -> Self {
        items.into_iter().collect()
    }
}
impl<'a> IntoIterator for &'a VariableCellSet {
    type Item = &'a VariableCellKey;
    type IntoIter = std::collections::hash_set::Iter<'a, VariableCellKey>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl IntoIterator for VariableCellSet {
    type Item = VariableCellKey;
    type IntoIter = std::collections::hash_set::IntoIter<VariableCellKey>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Whether an original namespace table lies under an exact selected parent.
/// Native descendants must belong to the same interpreter; equality retains incarnation.
#[must_use]
pub fn namespace_contains(parent: &SourceNamespaceKey, child: &SourceNamespaceKey) -> bool {
    if parent == child {
        return true;
    }
    match (parent, child) {
        (SourceNamespaceKey::Authored(parent), SourceNamespaceKey::Authored(child)) => {
            parent == "::" || child.starts_with(&format!("{parent}::"))
        }
        (SourceNamespaceKey::Native(parent), SourceNamespaceKey::Native(child))
            if parent.interpreter != child.interpreter =>
        {
            false
        }
        _ => match (parent.exact_native_path(), child.exact_native_path()) {
            (Some(parent), Some(child)) => {
                child.as_segments().len() > parent.as_segments().len()
                    && child.as_segments().starts_with(parent.as_segments())
            }
            _ => false,
        },
    }
}

fn hash_unordered<I: IntoIterator<Item = T>, T: Hash, H: std::hash::Hasher>(
    items: I,
    state: &mut H,
) {
    use std::hash::Hasher;
    let mut hashes: Vec<_> = items
        .into_iter()
        .map(|item| {
            let mut item_state = std::collections::hash_map::DefaultHasher::new();
            item.hash(&mut item_state);
            item_state.finish()
        })
        .collect();
    hashes.sort_unstable();
    hashes.hash(state);
}
impl<V: Hash> Hash for VariableCellTable<V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hash_unordered(self.0.iter(), state);
    }
}
impl Hash for VariableCellSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hash_unordered(self.0.iter(), state);
    }
}

/// Namespace query whose text form addresses only authored symbolic contexts.
pub trait VariableNamespaceQuery {
    /// Borrow an exact owner or create an authored query without native parsing.
    fn variable_namespace_key(&self) -> Cow<'_, SourceNamespaceKey>;
}
impl VariableNamespaceQuery for SourceNamespaceKey {
    fn variable_namespace_key(&self) -> Cow<'_, SourceNamespaceKey> {
        Cow::Borrowed(self)
    }
}
impl VariableNamespaceQuery for str {
    fn variable_namespace_key(&self) -> Cow<'_, SourceNamespaceKey> {
        Cow::Owned(SourceNamespaceKey::authored(self))
    }
}
impl VariableNamespaceQuery for String {
    fn variable_namespace_key(&self) -> Cow<'_, SourceNamespaceKey> {
        self.as_str().variable_namespace_key()
    }
}
impl<T: VariableNamespaceQuery + ?Sized> VariableNamespaceQuery for &T {
    fn variable_namespace_key(&self) -> Cow<'_, SourceNamespaceKey> {
        (*self).variable_namespace_key()
    }
}

/// Reached namespace effects retaining original owners and component geometry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariableNamespaceSet(HashSet<SourceNamespaceKey>);
impl Deref for VariableNamespaceSet {
    type Target = HashSet<SourceNamespaceKey>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for VariableNamespaceSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl VariableNamespaceSet {
    /// Test only the queried namespace domain.
    pub fn contains<Q: VariableNamespaceQuery + ?Sized>(&self, key: &Q) -> bool {
        self.0.contains(key.variable_namespace_key().as_ref())
    }
    /// Retain an exact owner or an explicitly authored symbolic key.
    pub fn insert(&mut self, key: impl Into<SourceNamespaceKey>) -> bool {
        self.0.insert(key.into())
    }
    /// Retire only the exact selected owner.
    pub fn remove<Q: VariableNamespaceQuery + ?Sized>(&mut self, key: &Q) -> bool {
        self.0.remove(key.variable_namespace_key().as_ref())
    }
}
impl<K: Into<SourceNamespaceKey>> Extend<K> for VariableNamespaceSet {
    fn extend<T: IntoIterator<Item = K>>(&mut self, iter: T) {
        self.0.extend(iter.into_iter().map(Into::into));
    }
}
impl<K: Into<SourceNamespaceKey>> FromIterator<K> for VariableNamespaceSet {
    fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
        Self(iter.into_iter().map(Into::into).collect())
    }
}
impl<K: Into<SourceNamespaceKey>, const N: usize> From<[K; N]> for VariableNamespaceSet {
    fn from(items: [K; N]) -> Self {
        items.into_iter().collect()
    }
}
impl Hash for VariableNamespaceSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hash_unordered(self.0.iter(), state);
    }
}
impl<'a> IntoIterator for &'a VariableNamespaceSet {
    type Item = &'a SourceNamespaceKey;
    type IntoIter = std::collections::hash_set::Iter<'a, SourceNamespaceKey>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_advice_preserves_owner_geometry_and_authored_separation() {
        let namespace = SourceNamespaceKey::Native(
            tcl_runtime_api::native_compilation::NativeNamespaceContext {
                interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                    owner: 17,
                    interpreter: 23,
                },
                token: 29,
                path: tcl_core_types::ByteNamespacePath::default().with_child("a::b"),
            },
        );
        let cell = VariableCellKey::Namespace {
            identity: namespace.clone(),
            simple: "value".into(),
        }
        .with_lifetime(31)
        .with_index("member");
        assert_eq!(
            cell.namespace_membership_for_advice(),
            Some(VariableNamespaceMembership::Retained(&namespace))
        );
        let nested = SourceNamespaceKey::Native(
            tcl_runtime_api::native_compilation::NativeNamespaceContext {
                interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                    owner: 17,
                    interpreter: 23,
                },
                token: 30,
                path: tcl_core_types::ByteNamespacePath::from_segments(["a", "b"]),
            },
        );
        assert_eq!(namespace.display(), nested.display());
        let nested_cell = VariableCellKey::Namespace {
            identity: nested.clone(),
            simple: "value".into(),
        };
        assert_eq!(
            nested_cell.namespace_membership_for_advice(),
            Some(VariableNamespaceMembership::Retained(&nested))
        );
        assert_ne!(
            cell.namespace_membership_for_advice(),
            nested_cell.namespace_membership_for_advice()
        );
        let collision = VariableCellKey::Authored(cell.compatibility_name());
        assert!(!matches!(
            collision.namespace_membership_for_advice(),
            Some(VariableNamespaceMembership::Retained(_))
        ));
        let local = VariableCellKey::Activation {
            identity: "actual-call".into(),
            simple: "::named::value".into(),
        };
        assert_eq!(local.namespace_membership_for_advice(), None);
        let authored = VariableCellKey::Authored("named::value".into());
        assert_eq!(
            authored.namespace_membership_for_advice(),
            Some(VariableNamespaceMembership::Authored("named"))
        );
        assert!(!authored.is_namespace_storage());
    }

    #[test]
    fn authored_names_cannot_forge_lifetimes_or_array_elements() {
        let cell = VariableCellKey::Authored("::N::a".into());
        let lifetime = cell.clone().with_lifetime(42);
        let element = lifetime.clone().with_index("k");
        let forged_lifetime = VariableCellKey::Authored(lifetime.compatibility_name());
        let forged_element = VariableCellKey::Authored(element.compatibility_name());
        assert_ne!(lifetime, forged_lifetime);
        assert_ne!(element, forged_element);
        assert!(element.is_member_of(&lifetime));
        assert!(!element.is_member_of(&cell));
        assert!(!element.is_member_of(&cell.clone().with_lifetime(43)));
        assert!(!forged_element.is_member_of(&lifetime));
        assert!(element.is_in_namespace(&SourceNamespaceKey::authored("::N")));
        assert!(!forged_lifetime.is_in_namespace(&SourceNamespaceKey::authored("::N")));
        let mut facts = VariableCellTable::default();
        facts.insert(lifetime.clone(), "ACTUAL");
        facts.insert(forged_lifetime, "WRITTEN");
        assert_eq!(facts.get(&lifetime), Some(&"ACTUAL"));
        assert_eq!(facts.get(&lifetime.compatibility_name()), Some(&"WRITTEN"));
    }
}
