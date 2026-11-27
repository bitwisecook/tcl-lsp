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

//! Retained source inventories shared by exact body projections.

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, Weak};

/// Analysis writes detach a shared inventory; retained body selection shares it.
#[derive(Debug, Clone, Default)]
#[doc(hidden)]
pub struct SharedSourceInventory<T>(Arc<T>);

impl<T: PartialEq> PartialEq for SharedSourceInventory<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}

impl<T: Eq> Eq for SharedSourceInventory<T> {}

impl<T> SharedSourceInventory<T> {
    /// Whether both handles still share one undetached inventory, so neither
    /// side has written since the copy.
    pub(super) fn shares(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> From<T> for SharedSourceInventory<T> {
    fn from(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl<T> Deref for SharedSourceInventory<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> DerefMut for SharedSourceInventory<T> {
    fn deref_mut(&mut self) -> &mut T {
        Arc::make_mut(&mut self.0)
    }
}

impl<'a, T> IntoIterator for &'a SharedSourceInventory<T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.as_ref().into_iter()
    }
}

impl<T: Clone + IntoIterator> IntoIterator for SharedSourceInventory<T> {
    type Item = T::Item;
    type IntoIter = T::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        Arc::unwrap_or_clone(self.0).into_iter()
    }
}

impl<T: Clone> SharedSourceInventory<T> {
    /// Detach only when an exact retained selection changes this inventory.
    /// The caller's predicate observes the whole current collection first.
    pub(super) fn update_if_needed(
        &mut self,
        changes: impl FnOnce(&T) -> bool,
        update: impl FnOnce(&mut T),
    ) {
        if changes(&self.0) {
            update(Arc::make_mut(&mut self.0));
        }
    }
}

impl<T> SharedSourceInventory<T> {
    pub(super) fn shared(&self) -> Arc<T> {
        Arc::clone(&self.0)
    }
}

/// A derived index of one shared inventory under one root origin, built when
/// the same undetached inventory is queried a second time, so a walk that
/// writes between queries never pays for an index it cannot reuse. A clone
/// starts empty.
///
/// The slot keeps a weak handle: that reserves the allocation's address, and
/// an inventory with a weak handle is never written in place (a write moves
/// it to a fresh allocation), so an equal address is the same contents.
pub(super) struct InventoryIndexCache<T, I>(Mutex<Option<InventoryIndexSlot<T, I>>>);

struct InventoryIndexSlot<T, I> {
    inventory: Weak<T>,
    origin: Arc<tcl_names::origin::SourceOriginId>,
    index: Option<Arc<I>>,
}

impl<T, I> Default for InventoryIndexCache<T, I> {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

impl<T, I> Clone for InventoryIndexCache<T, I> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<T, I> std::fmt::Debug for InventoryIndexCache<T, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InventoryIndexCache")
            .finish_non_exhaustive()
    }
}

impl<T, I> InventoryIndexCache<T, I> {
    /// The index of `inventory`, or `None` on the first query of an inventory;
    /// the caller then scans the whole inventory as it would without an index.
    pub(super) fn get(
        &self,
        inventory: &SharedSourceInventory<T>,
        origin: &Arc<tcl_names::origin::SourceOriginId>,
        build: impl FnOnce(&T) -> I,
    ) -> Option<Arc<I>> {
        let mut slot = self.0.lock().expect("inventory index cache poisoned");
        if let Some(current) = slot.as_mut().filter(|current| {
            std::ptr::eq(current.inventory.as_ptr(), Arc::as_ptr(&inventory.0))
                && (Arc::ptr_eq(&current.origin, origin) || &current.origin == origin)
        }) {
            return Some(Arc::clone(
                current
                    .index
                    .get_or_insert_with(|| Arc::new(build(&inventory.0))),
            ));
        }
        *slot = Some(InventoryIndexSlot {
            inventory: Arc::downgrade(&inventory.0),
            origin: Arc::clone(origin),
            index: None,
        });
        None
    }
}

/// The derived indexes of one bindings handle. They are allocated on first
/// use, so the handle itself grows by one pointer; a clone starts empty.
#[derive(Default)]
pub(super) struct DerivedIndexes(std::sync::OnceLock<Box<DerivedIndexSet>>);

#[derive(Default)]
pub(super) struct DerivedIndexSet {
    pub(super) root_layout_extents: super::root_layout_extents::RootLayoutExtentCache,
    pub(super) jim_stock_continuations: super::jim_stock_class::JimStockContinuationMemo,
    pub(super) variable_access_owners: super::variable_access_owners::VariableAccessOwnerCache,
    pub(super) point_extents: super::point_extents::PointExtentCache,
}

impl DerivedIndexes {
    pub(super) fn get(&self) -> &DerivedIndexSet {
        self.0.get_or_init(Box::default)
    }
}

impl Clone for DerivedIndexes {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for DerivedIndexes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DerivedIndexes").finish_non_exhaustive()
    }
}

#[cfg(test)]
impl<T> SharedSourceInventory<T> {
    pub(super) fn shares_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[cfg(test)]
mod index_cache_tests {
    use super::{InventoryIndexCache, SharedSourceInventory};
    use std::sync::Arc;

    #[test]
    fn an_index_is_built_on_the_second_query_and_dropped_by_a_write() {
        let origin = Arc::new(tcl_names::origin::SourceOriginId::authored(&Arc::from(
            "set x 1",
        )));
        let mut inventory = SharedSourceInventory::from(vec![1_u32, 2]);
        let cache = InventoryIndexCache::<Vec<u32>, usize>::default();
        let index = |cache: &InventoryIndexCache<Vec<u32>, usize>,
                     inventory: &SharedSourceInventory<Vec<u32>>| {
            cache
                .get(inventory, &origin, Vec::len)
                .map(|length| *length)
        };
        assert_eq!(index(&cache, &inventory), None);
        assert_eq!(index(&cache, &inventory), Some(2));
        assert_eq!(index(&cache, &inventory), Some(2));

        // A write moves the inventory, so the old index is never read again.
        inventory.push(3);
        assert_eq!(index(&cache, &inventory), None);
        assert_eq!(index(&cache, &inventory), Some(3));

        // A clone of the cache starts empty.
        assert_eq!(index(&cache.clone(), &inventory), None);

        // Another origin is another index.
        let other = Arc::new(tcl_names::origin::SourceOriginId::authored(&Arc::from(
            "set y 2",
        )));
        assert_eq!(cache.get(&inventory, &other, Vec::len), None);
    }
}
