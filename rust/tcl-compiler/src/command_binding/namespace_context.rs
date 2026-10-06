// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Domain-separated source namespace and command identities.
//!
//! Native context equality uses original interpreter and namespace incarnation,
//! with retained component boundaries. Presentation strings never select a
//! native namespace. Source allocations remain distinct from native tokens.

use super::{AllocationIncarnation, CommandAllocationSite};
use tcl_core_types::{ByteNamespacePath, NameBytes};
use tcl_runtime_api::native_compilation::NativeNamespaceContext;

/// Exact source-analysis namespace identity. An authored symbolic context
/// supplies no actual namespace token or globally callable source spelling.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceNamespaceKey {
    /// Symbolic compatibility context from a Unicode document.
    Authored(String),
    /// Actual retained namespace incarnation and component geometry.
    Native(NativeNamespaceContext),
    /// A reached source allocation, distinct from every actual native token.
    Allocated {
        /// Original allocation instruction and its source instance.
        site: CommandAllocationSite,
        /// Distinct bounded executions of the allocation instruction.
        incarnation: AllocationIncarnation,
        /// Constructed child path retained before rendering.
        path: ByteNamespacePath,
    },
}

impl SourceNamespaceKey {
    /// Retain an authored symbolic context; this does not parse a native path.
    pub fn authored(namespace: impl Into<String>) -> Self {
        Self::Authored(namespace.into())
    }

    /// Retain the actual current namespace from an immutable runtime entry.
    ///
    /// # Errors
    /// Returns unavailable provenance for a missing or conflicting namespace row.
    pub fn from_native_entry(
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self, tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable> {
        entry
            .retained_namespace_context(entry.current_namespace)
            .map(Self::Native)
    }

    /// Exact native or allocated component geometry. Authored symbolic labels
    /// cannot donate boundaries by reparsing their presentation.
    #[must_use]
    pub fn exact_native_path(&self) -> Option<&ByteNamespacePath> {
        match self {
            Self::Native(context) => Some(&context.path),
            Self::Allocated { path, .. } => Some(path),
            Self::Authored(_) => None,
        }
    }

    /// Optional compatibility key whose analytical splitter agrees with the
    /// retained geometry. Even this view carries no callable-name authority.
    pub(crate) fn advisory_key(&self) -> Option<String> {
        let display = self.display()?;
        if let Some(path) = self.exact_native_path() {
            let components = tcl_syntax::naming::checked_namespace_path_utf8(path).ok()?;
            if tcl_syntax::naming::key_segments(&display) != components {
                return None;
            }
        }
        Some(display)
    }

    /// Retained compiled namespace geometry without a displayed-name lookup.
    #[must_use]
    pub fn to_compiled_context(&self) -> Option<tcl_runtime_api::CompiledNamespaceContext> {
        match self {
            Self::Authored(_) => None,
            Self::Native(context) => Some(tcl_runtime_api::CompiledNamespaceContext::Native(
                context.clone(),
            )),
            Self::Allocated { path, .. } => Some(
                tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(path.clone()),
            ),
        }
    }

    /// Actual retained native context, excluding source-created allocations.
    #[must_use]
    pub fn native_context(&self) -> Option<&NativeNamespaceContext> {
        if let Self::Native(context) = self {
            Some(context)
        } else {
            None
        }
    }

    pub(super) fn is_root(&self) -> bool {
        match self {
            Self::Authored(text) => text == "::",
            _ => self
                .exact_native_path()
                .is_some_and(ByteNamespacePath::is_root),
        }
    }

    pub(super) fn is_descendant_of(&self, parent: &Self) -> bool {
        match (self, parent) {
            (Self::Authored(child), Self::Authored(parent)) => {
                child.starts_with(&format!("{parent}::"))
            }
            _ => match (self.exact_native_path(), parent.exact_native_path()) {
                (Some(child), Some(parent)) => {
                    child.as_segments().len() > parent.as_segments().len()
                        && child.as_segments().starts_with(parent.as_segments())
                }
                _ => false,
            },
        }
    }

    /// Optional Unicode presentation, with no lookup or callable-name authority.
    #[must_use]
    pub fn display(&self) -> Option<String> {
        match self {
            Self::Authored(text) => Some(text.clone()),
            Self::Native(context) => display_path(&context.path),
            Self::Allocated { path, .. } => display_path(path),
        }
    }
}

fn display_path(path: &ByteNamespacePath) -> Option<String> {
    let components = tcl_syntax::naming::checked_namespace_path_utf8(path).ok()?;
    Some(if components.is_empty() {
        "::".to_owned()
    } else {
        format!("::{}", components.join("::"))
    })
}

/// Primary command-table key. A written source spelling cannot impersonate an
/// actual native slot, even when it equals that slot's display spelling.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SourceCommandKey {
    /// Symbolic command key in an authored document's namespace model.
    Authored(String),
    /// Exact counted simple key in a retained or allocated namespace table.
    Slot {
        /// Namespace incarnation, independent of its printed full name.
        namespace: SourceNamespaceKey,
        /// Already selected native simple-name bytes.
        simple: NameBytes,
    },
}

impl SourceCommandKey {
    pub fn authored(key: impl Into<String>) -> Self {
        Self::Authored(key.into())
    }

    pub(crate) fn slot(namespace: SourceNamespaceKey, simple: NameBytes) -> Self {
        Self::Slot { namespace, simple }
    }

    pub(super) fn holder(&self) -> std::borrow::Cow<'_, SourceNamespaceKey> {
        match self {
            Self::Slot { namespace, .. } => std::borrow::Cow::Borrowed(namespace),
            Self::Authored(text) => {
                let (holder, _) = tcl_syntax::naming::key_holder_and_tail(text);
                std::borrow::Cow::Owned(SourceNamespaceKey::authored(if holder.is_empty() {
                    "::"
                } else {
                    holder
                }))
            }
        }
    }

    pub(super) fn simple_utf8(&self) -> Option<&str> {
        match self {
            Self::Slot { simple, .. } => simple.try_utf8().ok(),
            Self::Authored(text) => Some(tcl_syntax::naming::key_holder_and_tail(text).1),
        }
    }

    /// Symbolic document spelling only. Native slots require an independent
    /// callable-spelling receipt rather than a rendered path.
    pub fn authored_spelling(&self) -> Option<&str> {
        match self {
            Self::Authored(text) => Some(text),
            Self::Slot { .. } => None,
        }
    }
}

impl From<String> for SourceNamespaceKey {
    fn from(value: String) -> Self {
        Self::authored(value)
    }
}
impl From<&str> for SourceNamespaceKey {
    fn from(value: &str) -> Self {
        Self::authored(value)
    }
}
impl From<String> for SourceCommandKey {
    fn from(value: String) -> Self {
        Self::authored(value)
    }
}
impl From<&str> for SourceCommandKey {
    fn from(value: &str) -> Self {
        Self::authored(value)
    }
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceCommandTable<V>(std::collections::HashMap<SourceCommandKey, V>);
impl<V> Default for SourceCommandTable<V> {
    fn default() -> Self {
        Self(std::collections::HashMap::new())
    }
}
impl<V> SourceCommandTable<V> {
    pub(super) fn get<Q: CommandKeyQuery + ?Sized>(&self, key: &Q) -> Option<&V> {
        self.0.get(key.command_key().as_ref())
    }
    pub(super) fn contains_key<Q: CommandKeyQuery + ?Sized>(&self, key: &Q) -> bool {
        self.get(key).is_some()
    }
    pub(super) fn insert(&mut self, key: impl Into<SourceCommandKey>, value: V) -> Option<V> {
        self.0.insert(key.into(), value)
    }
    pub(super) fn entry(
        &mut self,
        key: impl Into<SourceCommandKey>,
    ) -> std::collections::hash_map::Entry<'_, SourceCommandKey, V> {
        self.0.entry(key.into())
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = (&SourceCommandKey, &V)> {
        self.0.iter()
    }
    pub(super) fn values(&self) -> impl Iterator<Item = &V> {
        self.0.values()
    }
    pub(super) fn keys(&self) -> impl Iterator<Item = &SourceCommandKey> {
        self.0.keys()
    }
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub(super) fn extend<K: Into<SourceCommandKey>>(
        &mut self,
        entries: impl IntoIterator<Item = (K, V)>,
    ) {
        self.0
            .extend(entries.into_iter().map(|(key, value)| (key.into(), value)));
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(super) struct SourceNamespaceSet(std::collections::BTreeSet<SourceNamespaceKey>);
impl SourceNamespaceSet {
    pub(super) fn contains<Q: NamespaceKeyQuery + ?Sized>(&self, key: &Q) -> bool {
        self.0.contains(key.namespace_key().as_ref())
    }
    pub(super) fn insert(&mut self, key: impl Into<SourceNamespaceKey>) -> bool {
        self.0.insert(key.into())
    }
    pub(super) fn remove<Q: NamespaceKeyQuery + ?Sized>(&mut self, key: &Q) -> bool {
        self.0.remove(key.namespace_key().as_ref())
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = &SourceNamespaceKey> {
        self.0.iter()
    }
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }
    pub(super) fn retain(&mut self, keep: impl FnMut(&SourceNamespaceKey) -> bool) {
        self.0.retain(keep);
    }
    pub(super) fn extend<K: Into<SourceNamespaceKey>>(
        &mut self,
        keys: impl IntoIterator<Item = K>,
    ) {
        self.0.extend(keys.into_iter().map(Into::into));
    }
}
impl<K: Into<SourceNamespaceKey>> FromIterator<K> for SourceNamespaceSet {
    fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
        let mut set = Self::default();
        set.extend(iter);
        set
    }
}

impl Default for SourceNamespaceKey {
    fn default() -> Self {
        Self::authored("::")
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
