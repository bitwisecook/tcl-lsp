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
use std::sync::Arc;

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

#[cfg(test)]
impl<T> SharedSourceInventory<T> {
    pub(super) fn shares_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
