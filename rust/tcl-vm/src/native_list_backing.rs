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

//! Native List member owners are separate from borrowed memory snapshots.

use crate::Value;
use std::{
    cell::{Cell, RefCell},
    ops::{Deref, Range},
    rc::Rc,
};
use tcl_syntax::value::ValueError;

struct ListStorage {
    members: RefCell<Vec<Value>>,
    owns_members: bool,
    canonical: Rc<Cell<bool>>,
    headers: Cell<usize>,
    generation: Cell<u64>,
    capacity: Cell<Option<usize>>,
    first_used: Cell<usize>,
}

/// Original members and live canonical state retained without copying children.
/// A lifetime view is not a native header and grants no native ABI authority.
pub struct NativeListItems {
    storage: Rc<ListStorage>,
    snapshot: Rc<Vec<Value>>,
    window: Range<usize>,
    span: bool,
    generation: u64,
    header: bool,
}

impl NativeListItems {
    pub(crate) fn new(items: Vec<Value>, canonical: bool) -> Self {
        let members = items
            .into_iter()
            .map(Value::into_native_reference)
            .collect::<Vec<_>>();
        let length = members.len();
        let snapshot = Self::snapshot(&members);
        Self {
            storage: Rc::new(ListStorage {
                members: RefCell::new(members),
                owns_members: true,
                canonical: Rc::new(Cell::new(canonical)),
                headers: Cell::new(1),
                generation: Cell::new(0),
                capacity: Cell::new(Some(length.max(1))),
                first_used: Cell::new(0),
            }),
            snapshot,
            window: 0..length,
            span: false,
            generation: 0,
            header: true,
        }
    }

    fn snapshot(members: &[Value]) -> Rc<Vec<Value>> {
        Rc::new(
            members
                .iter()
                .map(|value| value.native_lifetime_lease().into_value())
                .collect(),
        )
    }

    pub(crate) fn invocation_view(items: Rc<Vec<Value>>) -> Self {
        let length = items.len();
        Self {
            storage: Rc::new(ListStorage {
                members: RefCell::new(Vec::new()),
                owns_members: false,
                canonical: Rc::new(Cell::new(false)),
                headers: Cell::new(1),
                generation: Cell::new(0),
                capacity: Cell::new(None),
                first_used: Cell::new(0),
            }),
            snapshot: items,
            window: 0..length,
            span: false,
            generation: 0,
            header: true,
        }
    }

    #[must_use]
    pub fn is_whole_backing(&self) -> bool {
        self.storage.owns_members
    }

    #[must_use]
    pub fn lifetime_view(&self) -> Self {
        Self {
            storage: Rc::clone(&self.storage),
            snapshot: Rc::clone(&self.snapshot),
            window: self.window.clone(),
            span: self.span,
            generation: self.generation,
            header: false,
        }
    }

    fn check_generation(&self) -> Result<(), ValueError> {
        if self.generation != self.storage.generation.get() {
            return Err(ValueError::CommandProtocolUnavailable(
                "changed native List member vector",
            ));
        }
        Ok(())
    }

    pub(crate) fn native_header(&self) -> Result<Self, ValueError> {
        self.check_generation()?;
        let count = self.storage.headers.get();
        if count == 0 {
            return Err(ValueError::CommandProtocolUnavailable(
                "retired native List backing",
            ));
        }
        self.storage
            .headers
            .set(count.checked_add(1).expect("native List owners exhausted"));
        let mut header = self.lifetime_view();
        header.header = true;
        Ok(header)
    }

    /// Borrow members only while this original vector and its headers are live.
    pub fn elements(&self) -> Result<&[Value], ValueError> {
        self.check_generation()?;
        for value in &**self {
            value.check_native_header()?;
        }
        Ok(&**self)
    }

    #[cfg(test)]
    pub(crate) fn native_header_reference_count(&self) -> usize {
        self.storage.headers.get()
    }
    #[must_use]
    pub fn native_is_shared(&self) -> bool {
        self.storage.headers.get() > 1
    }
    #[must_use]
    pub fn has_native_header(&self) -> bool {
        self.storage.headers.get() != 0
    }
    #[must_use]
    pub fn canonical(&self) -> bool {
        self.storage.canonical.get()
    }
    #[must_use]
    pub fn canonical_state(&self) -> Rc<Cell<bool>> {
        Rc::clone(&self.storage.canonical)
    }
    #[must_use]
    pub fn same_backing(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.storage, &other.storage)
    }

    pub(crate) fn capacity(&self) -> Option<usize> {
        self.storage.capacity.get()
    }
    pub(crate) fn set_capacity(&self, capacity: usize) {
        assert!(self.header && capacity >= self.storage.members.borrow().len());
        let mut members = self.storage.members.borrow_mut();
        let length = members.len();
        members.reserve_exact(capacity - length);
        self.storage.capacity.set(Some(capacity));
    }
    pub(crate) fn stored_length(&self) -> usize {
        self.storage.members.borrow().len()
    }
    pub(crate) fn has_span(&self) -> bool {
        self.span
    }

    pub(crate) fn first_used(&self) -> usize {
        self.storage.first_used.get()
    }
    #[cfg(test)]
    pub(crate) fn span_start(&self) -> Option<usize> {
        self.span.then_some(self.window.start)
    }

    /// Retire only members outside the selected window when the store has one
    /// real header. The allocation extent survives this garbage collection.
    pub(crate) fn collect_unreferenced(&mut self) -> Result<(), ValueError> {
        if !self.native_is_shared()
            && (self.window.start != self.first_used()
                || self.window.end != self.first_used() + self.stored_length())
        {
            let flag = self.canonical();
            self.select_window(0..self.len(), self.span)?;
            self.storage.canonical.set(flag);
        }
        Ok(())
    }

    /// Select an actual view and release unreachable slots only for an
    /// unshared store. Shared stores retain their complete member inventory.
    pub(crate) fn select_window(
        &mut self,
        range: Range<usize>,
        span: bool,
    ) -> Result<(), ValueError> {
        self.check_generation()?;
        assert!(self.header && self.storage.owns_members);
        assert!(range.start <= range.end && range.end <= self.len());
        let start = self.window.start - self.first_used() + range.start;
        let end = self.window.start - self.first_used() + range.end;
        if self.native_is_shared() {
            self.window = self.first_used() + start..self.first_used() + end;
            self.snapshot = Self::snapshot(&self.snapshot[range]);
        } else {
            let mut members = self.storage.members.borrow_mut();
            members.drain(end..);
            members.drain(..start);
            let generation = self
                .storage
                .generation
                .get()
                .checked_add(1)
                .expect("native List generation exhausted");
            self.storage.generation.set(generation);
            self.generation = generation;
            self.storage.first_used.set(if span {
                self.storage.first_used.get() + start
            } else {
                0
            });
            self.window = self.first_used()..self.first_used() + members.len();
            self.snapshot = Self::snapshot(&members);
        }
        self.span = span;
        if !span {
            self.storage.canonical.set(false);
        }
        Ok(())
    }

    /// Install an actual range header over the same member store.
    pub(crate) fn range_header(&self, range: Range<usize>) -> Result<Self, ValueError> {
        if range.start > range.end || range.end > self.len() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native List span bounds",
            ));
        }
        let mut result = self.native_header()?;
        result.window = self.window.start + range.start..self.window.start + range.end;
        result.span = self.span || range.start != 0 || range.end != self.len();
        result.snapshot = Self::snapshot(&self.snapshot[range]);
        Ok(result)
    }

    /// Mutate the original store under the selected physical C release.
    pub(crate) fn replace_native(
        &mut self,
        first: usize,
        delete: usize,
        insert: &[Value],
        version: tcl_dialect::TclVersion,
    ) -> Result<(), ValueError> {
        use tcl_cmd_core::native_list_storage::{NativeListReplaceStorage, replace_layout};
        self.check_generation()?;
        assert!(self.header && self.storage.owns_members);
        let first = first.min(self.len());
        let delete = delete.min(self.len() - first);
        if insert.is_empty() && delete == 0 {
            return Ok(());
        }
        if version >= tcl_dialect::TclVersion::V9_0
            && insert.is_empty()
            && (first == 0 || first + delete == self.len())
        {
            return self.delete_end_range(first, delete, version);
        }
        if !self.native_is_shared() {
            self.collect_unreferenced()?;
        }
        let layout =
            if version >= tcl_dialect::TclVersion::V9_0 {
                Some(replace_layout(
                    NativeListReplaceStorage {
                        length: self.len(),
                        used: self.stored_length(),
                        allocated: self.capacity().ok_or(
                            ValueError::CommandProtocolUnavailable("native List allocation extent"),
                        )?,
                        first_used: self.first_used(),
                        window_start: self.window.start,
                        has_span: self.span,
                        store_shared: self.native_is_shared(),
                    },
                    first,
                    delete,
                    insert.len(),
                ))
            } else {
                None
            };
        if let Some(layout) = layout.filter(|layout| layout.shared_prepend) {
            self.prepend_shared_members(insert, layout);
            return Ok(());
        }
        if let Some(layout) = layout.filter(|layout| layout.new_store) {
            let source = self.elements()?;
            let members = source[..first]
                .iter()
                .chain(insert)
                .chain(&source[first + delete..])
                .cloned()
                .collect();
            *self = Self::new(members, false);
            self.set_capacity(layout.allocated);
            self.storage.first_used.set(layout.first_used);
            self.window = layout.first_used..layout.first_used + self.len();
            self.span = layout.span;
            return Ok(());
        }
        if self.native_is_shared() {
            *self = Self::new(self.elements()?.to_vec(), self.canonical());
        }
        let mut members = self.storage.members.borrow_mut();
        let inserted = insert
            .iter()
            .cloned()
            .map(Value::into_native_reference)
            .collect::<Vec<_>>();
        members.splice(first..first + delete, inserted);
        let generation = self
            .storage
            .generation
            .get()
            .checked_add(1)
            .expect("native List generation exhausted");
        self.storage.generation.set(generation);
        self.generation = generation;
        self.snapshot = Self::snapshot(&members);
        if let Some(layout) = layout {
            let length = members.len();
            members.reserve_exact(layout.allocated - length);
            self.storage.capacity.set(Some(layout.allocated));
            self.storage.first_used.set(layout.first_used);
            self.span = layout.span;
        } else {
            self.storage.capacity.set(None);
            self.storage.first_used.set(0);
            self.span = false;
        }
        self.window = self.first_used()..self.first_used() + members.len();
        if version >= tcl_dialect::TclVersion::V9_0
            && !(delete == 0 && first + insert.len() == self.len())
        {
            self.storage.canonical.set(false);
        }
        Ok(())
    }

    fn prepend_shared_members(
        &mut self,
        insert: &[Value],
        layout: tcl_cmd_core::native_list_storage::NativeListReplaceLayout,
    ) {
        let inserted = insert
            .iter()
            .cloned()
            .map(Value::into_native_reference)
            .collect::<Vec<_>>();
        self.storage.members.borrow_mut().splice(0..0, inserted);
        self.storage.first_used.set(layout.first_used);
        self.window.start -= insert.len();
        self.snapshot = Self::snapshot(&self.storage.members.borrow()[..self.len()]);
        self.span = true;
    }

    fn delete_end_range(
        &mut self,
        first: usize,
        delete: usize,
        version: tcl_dialect::TclVersion,
    ) -> Result<(), ValueError> {
        use tcl_cmd_core::native_list_storage::{
            NativeListRangeAction, NativeListRangeStorage, range_storage_action,
        };
        if !self.native_is_shared() {
            self.collect_unreferenced()?;
        }
        let range = if first == 0 {
            delete..self.len()
        } else {
            0..first
        };
        if range.is_empty() {
            *self = Self::new(Vec::new(), false);
            return Ok(());
        }
        let action = range_storage_action(
            version,
            range,
            NativeListRangeStorage {
                length: self.len(),
                header_shared: false,
                store_shared: self.native_is_shared(),
                has_span: self.span,
                used: self.stored_length(),
                allocated: self.capacity(),
                string: tcl_cmd_core::native_list_storage::NativeListStringState::Nonempty,
            },
        )?;
        match action {
            NativeListRangeAction::Window { range, span } => self.select_window(range, span),
            NativeListRangeAction::FreshMembers(range) => {
                *self = Self::new(self.elements()?[range].to_vec(), false);
                Ok(())
            }
            NativeListRangeAction::Whole => Ok(()),
            _ => unreachable!("nonempty private List range"),
        }
    }
}

impl Clone for NativeListItems {
    fn clone(&self) -> Self {
        if self.header {
            self.native_header().expect("live native List header")
        } else {
            self.lifetime_view()
        }
    }
}
impl Drop for NativeListItems {
    fn drop(&mut self) {
        if self.header {
            let count = self
                .storage
                .headers
                .get()
                .checked_sub(1)
                .expect("native List owner underflow");
            self.storage.headers.set(count);
            if count == 0 && self.storage.owns_members {
                let members = self.storage.members.replace(Vec::new());
                drop(members);
            }
        }
    }
}
impl Deref for NativeListItems {
    type Target = Vec<Value>;
    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}
impl AsRef<Vec<Value>> for NativeListItems {
    fn as_ref(&self) -> &Vec<Value> {
        &**self
    }
}
impl std::fmt::Debug for NativeListItems {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeListItems")
            .field("len", &self.len())
            .field("canonical", &self.canonical())
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::NativeListItems;
    use crate::Value;

    #[test]
    fn lifetime_views_do_not_own_native_headers_or_member_references() {
        let child = Value::string("member");
        let header = NativeListItems::new(vec![child.clone()], false);
        assert_eq!(child.native_object_reference_count(), 2);
        let view = header.lifetime_view();
        let repeated = view.clone();
        assert!(view.same_backing(&repeated));
        assert!(!view.native_is_shared());
        assert_eq!(child.native_object_reference_count(), 2);
        let duplicate = header.clone();
        assert!(view.native_is_shared());
        assert_eq!(child.native_object_reference_count(), 2);
        drop(header);
        assert!(!view.native_is_shared());
        drop(duplicate);
        assert!(!view.has_native_header());
        assert_eq!(child.native_object_reference_count(), 1);
        assert_eq!(
            view[0].native_object_identity(),
            child.native_object_identity()
        );
        drop(view);
        drop(repeated);
        assert_eq!(child.native_object_reference_count(), 1);
    }

    #[test]
    fn range_headers_retain_one_store_and_original_member_owners() {
        let first = Value::string("first");
        let middle = Value::string("middle");
        let last = Value::string("last");
        let mut original =
            NativeListItems::new(vec![first.clone(), middle.clone(), last.clone()], false);
        let view = original.lifetime_view();
        let range = original.range_header(1..2).unwrap();
        assert!(original.same_backing(&range));
        assert_eq!(range.span_start(), Some(1));
        assert_eq!(first.native_object_reference_count(), 2);
        assert_eq!(middle.native_object_reference_count(), 2);
        drop(range);
        original.select_window(1..2, true).unwrap();
        assert_eq!(original.first_used(), 1);
        assert_eq!(original.capacity(), Some(3));
        assert_eq!(original.stored_length(), 1);
        assert_eq!(first.native_object_reference_count(), 1);
        assert_eq!(last.native_object_reference_count(), 1);
        assert_eq!(middle.native_object_reference_count(), 2);
        assert!(view.elements().is_err());
        assert!(view.same_backing(&original));
    }

    #[test]
    fn replacing_a_shared_span_copies_only_selected_original_members() {
        let first = Value::string("first");
        let middle = Value::string("middle");
        let last = Value::string("last");
        let inserted = Value::string("inserted");
        let original =
            NativeListItems::new(vec![first.clone(), middle.clone(), last.clone()], false);
        let mut range = original.range_header(1..2).unwrap();
        range
            .replace_native(
                1,
                0,
                std::slice::from_ref(&inserted),
                tcl_dialect::TclVersion::V9_0,
            )
            .unwrap();
        assert!(!range.same_backing(&original));
        assert_eq!(range.len(), 2);
        assert_eq!(
            range[0].native_object_identity(),
            middle.native_object_identity()
        );
        assert_eq!(first.native_object_reference_count(), 2);
        assert_eq!(middle.native_object_reference_count(), 3);
        drop(range);
        assert_eq!(middle.native_object_reference_count(), 2);
        assert_eq!(inserted.native_object_reference_count(), 1);
    }

    #[test]
    fn repeated_member_slots_each_own_one_native_reference() {
        let child = Value::string("member");
        let header = NativeListItems::new(vec![child.clone(), child.clone()], false);
        assert_eq!(child.native_object_reference_count(), 3);
        let view = header.lifetime_view();
        drop(header);
        assert_eq!(child.native_object_reference_count(), 1);
        drop(view);
        assert_eq!(child.native_object_reference_count(), 1);
    }
}
