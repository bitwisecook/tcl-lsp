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

//! A bounded small set — the primitive behind "small union of possibilities"
//! lattice carriers whose dedup is plain equality.
//!
//! The type lattice's shape unions ([`crate::types::TypeLattice`], cap
//! [`crate::types::MAX_TYPE_UNION`]) are built on this. Two other carriers
//! look similar but deliberately keep their own storage, because their dedup
//! is **not** plain `==`: SCCP's constant sets
//! ([`crate::analyses::LatticeValue::ConstSet`]) merge under a numeric
//! cross-equality (`cv_eq` — `Int(1)` and `Float(1.0)` unify) with a custom
//! sort key, and the analyser's class sets ride `BTreeSet`'s always-sorted
//! persistent-set semantics. Fold either under this primitive and those
//! domain rules would silently change; a new carrier whose alternatives
//! compare by ordinary equality should reach for this type instead of
//! growing a fourth ad-hoc vec-with-cap.
//!
//! Semantics:
//! - **Dedup by equality** — inserting an element already present is a no-op.
//! - **Insertion-ordered** — iteration order is the order of first insertion,
//!   deterministic given deterministic inputs. Carriers whose equality must be
//!   order-independent (the type lattice) call [`BoundedSet::canonicalise`]
//!   after building.
//! - **Capacity is the carrier's contract** — [`BoundedSet::insert`] reports
//!   whether the element fit; the *caller* widens its own lattice to top on
//!   overflow (this type never silently drops).

/// A deduplicated, insertion-ordered set holding at most `CAP` elements.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundedSet<T, const CAP: usize> {
    items: Vec<T>,
}

impl<T, const CAP: usize> Default for BoundedSet<T, CAP> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T: PartialEq, const CAP: usize> BoundedSet<T, CAP> {
    /// The empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A one-element set. `CAP` is at least 1 for every instantiation in the
    /// tree, so this cannot overflow.
    #[must_use]
    pub fn singleton(item: T) -> Self {
        Self { items: vec![item] }
    }

    /// Build from an iterator, deduplicating. Returns `None` when the distinct
    /// elements exceed `CAP` — the caller widens to its own top.
    #[must_use]
    pub fn from_iter_bounded(iter: impl IntoIterator<Item = T>) -> Option<Self> {
        let mut set = Self::new();
        for item in iter {
            if !set.insert(item) {
                return None;
            }
        }
        Some(set)
    }

    /// Insert `item` unless already present. Returns `false` — and leaves the
    /// set unchanged — when the element is new but the set is full; the caller
    /// widens to its top element.
    #[must_use = "a false return means the set is full and the caller must widen"]
    pub fn insert(&mut self, item: T) -> bool {
        if self.items.contains(&item) {
            return true;
        }
        if self.items.len() >= CAP {
            return false;
        }
        self.items.push(item);
        true
    }

    /// Whether `item` is a member.
    #[must_use]
    pub fn contains(&self, item: &T) -> bool {
        self.items.contains(item)
    }

    /// Number of elements.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate in insertion (or canonical, after [`Self::canonicalise`]) order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    /// The elements as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    /// Consume into the underlying `Vec`.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<T: Ord, const CAP: usize> BoundedSet<T, CAP> {
    /// Sort into canonical order so structurally-equal sets built in different
    /// insertion orders compare equal. Carriers that derive `PartialEq` on the
    /// whole set (the type lattice) must canonicalise after every mutation
    /// batch.
    pub fn canonicalise(&mut self) {
        self.items.sort_unstable();
    }
}

impl<'a, T, const CAP: usize> IntoIterator for &'a BoundedSet<T, CAP> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<T, const CAP: usize> IntoIterator for BoundedSet<T, CAP> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_dedups_and_caps() {
        let mut s: BoundedSet<u32, 2> = BoundedSet::new();
        assert!(s.insert(1));
        assert!(s.insert(1), "re-inserting a member is a no-op success");
        assert!(s.insert(2));
        assert!(!s.insert(3), "a new element past CAP must report overflow");
        assert_eq!(s.as_slice(), &[1, 2], "overflow leaves the set unchanged");
    }

    #[test]
    fn from_iter_bounded_widens_past_cap() {
        assert_eq!(
            BoundedSet::<u32, 2>::from_iter_bounded([1, 2, 1]).map(BoundedSet::into_vec),
            Some(vec![1, 2]),
            "duplicates do not count against the cap"
        );
        assert_eq!(BoundedSet::<u32, 2>::from_iter_bounded([1, 2, 3]), None);
    }

    #[test]
    fn canonicalise_makes_order_insensitive() {
        let mut a: BoundedSet<u32, 4> = BoundedSet::from_iter_bounded([3, 1]).unwrap();
        let mut b: BoundedSet<u32, 4> = BoundedSet::from_iter_bounded([1, 3]).unwrap();
        assert_ne!(a, b, "insertion order is observable before canonicalise");
        a.canonicalise();
        b.canonicalise();
        assert_eq!(a, b);
    }
}
