// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual element hash entries shared by their table and native alias roles.

use super::{CellContents, Var};
use crate::obj::{Owned, TclObj};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    fmt,
    rc::{Rc, Weak},
};
use tcl_runtime_api::VarId;

pub(super) struct NativeElementEntry {
    pub(super) id: VarId,
    key: Vec<u8>,
    original: RefCell<Option<Owned>>,
    aliases: Cell<usize>,
    retired: Cell<bool>,
    table_owned: Cell<bool>,
    trace_refs: Cell<usize>,
    retiring_definition: Cell<bool>,
}

/// A table owns one entry by its actual never-reused Var identity. C8.4 string
/// keys own bytes; later object keys own one original element header. Sharing the
/// entry with aliases does not acquire another object-header reference.
#[derive(Default)]
pub(super) struct NativeElementEntries(BTreeMap<VarId, Rc<NativeElementEntry>>);

impl NativeElementEntries {
    pub(super) fn any_aliases(&self) -> bool {
        self.0.values().any(|entry| entry.aliases.get() != 0)
    }
    pub(super) fn ensure(&mut self, id: VarId, key: &[u8], original: Option<*mut TclObj>) {
        let entry = self.0.entry(id).or_insert_with(|| {
            Rc::new(NativeElementEntry {
                id,
                key: key.to_vec(),
                original: RefCell::new(None),
                aliases: Cell::new(0),
                retired: Cell::new(false),
                table_owned: Cell::new(true),
                trace_refs: Cell::new(0),
                retiring_definition: Cell::new(false),
            })
        });
        if let Some(original) = original {
            let mut owner = entry.original.borrow_mut();
            if owner.is_none() {
                *owner = Some(Owned::retain(original));
            }
        }
    }
    pub(super) fn has_aliases(&self, key: &[u8]) -> bool {
        self.0
            .values()
            .any(|entry| entry.key == key && entry.aliases.get() != 0)
    }
    pub(super) fn remove(&mut self, key: &[u8]) {
        let id = self
            .0
            .iter()
            .find_map(|(id, entry)| (entry.key == key && entry.aliases.get() == 0).then_some(*id));
        if let Some(id) = id {
            if let Some(entry) = self.0.remove(&id) {
                entry.retired.set(true);
                entry.table_owned.set(false);
                let key = entry.original.borrow_mut().take();
                drop(key);
            }
        }
    }
    pub(super) fn retire(&mut self, key: &[u8]) {
        if let Some(entry) = self.0.values().find(|entry| entry.key == key) {
            // VarHashInvalidateEntry precedes the callback. The CPP key and
            // modern table Var role leave only at the final VarHashDeleteTable.
            entry.retired.set(true);
        }
    }
    pub(super) fn retirement_trace(
        &self,
        key: &[u8],
        preserve_definition: bool,
        defined: bool,
        traced: bool,
    ) -> Option<NativeElementRetirementTrace> {
        let entry = Rc::clone(self.0.values().find(|entry| entry.key == key)?);
        entry
            .retiring_definition
            .set(preserve_definition && defined);
        if traced {
            entry.trace_refs.set(
                entry
                    .trace_refs
                    .get()
                    .checked_add(1)
                    .expect("element trace overflow"),
            );
        }
        Some(NativeElementRetirementTrace { entry, traced })
    }
    pub(super) fn finish_member_retirement(&mut self, key: &[u8], object_table: bool) {
        if object_table {
            return;
        }
        // C84's string-key table has no Var ownership role. A node without an
        // alias or active trace dies after its own callback, before its sibling.
        let id = self.0.iter().find_map(|(id, entry)| {
            (entry.key == key && entry.aliases.get() == 0 && entry.trace_refs.get() == 0)
                .then_some(*id)
        });
        if let Some(id) = id {
            self.0.remove(&id);
        }
    }
    pub(super) fn clear(&mut self) {
        for entry in self.0.values() {
            entry.retired.set(true);
            entry.table_owned.set(false);
            let key = entry.original.borrow_mut().take();
            drop(key);
        }
        self.0.clear();
    }
    fn hold(
        &self,
        id: VarId,
        array: &Rc<RefCell<CellContents>>,
        epoch: u64,
    ) -> Option<Rc<NativeElementAliasEntry>> {
        let entry = Rc::clone(self.0.get(&id)?);
        entry.aliases.set(
            entry
                .aliases
                .get()
                .checked_add(1)
                .expect("native element alias overflow"),
        );
        Some(Rc::new(NativeElementAliasEntry {
            entry,
            array: Rc::downgrade(array),
            epoch,
        }))
    }
}
impl Drop for NativeElementEntries {
    fn drop(&mut self) {
        self.clear();
    }
}

/// The actual active unset trace holds its selected Var independently of the
/// CPP table owner. C84's pending undefined flag retires when that trace leaves.
pub(crate) struct NativeElementRetirementTrace {
    entry: Rc<NativeElementEntry>,
    traced: bool,
}
impl Drop for NativeElementRetirementTrace {
    fn drop(&mut self) {
        self.entry.retiring_definition.set(false);
        if self.traced {
            self.entry.trace_refs.set(
                self.entry
                    .trace_refs
                    .get()
                    .checked_sub(1)
                    .expect("element trace underflow"),
            );
        }
    }
}

/// One newly installed native alias owns one actual element-entry role.
/// Borrowed Link copies share this role; a new alias mints a separate hold.
pub(crate) struct NativeElementAliasEntry {
    entry: Rc<NativeElementEntry>,
    array: Weak<RefCell<CellContents>>,
    epoch: u64,
}
impl NativeElementAliasEntry {
    pub(crate) fn new_binding(&self) -> Rc<Self> {
        self.entry.aliases.set(
            self.entry
                .aliases
                .get()
                .checked_add(1)
                .expect("native element alias overflow"),
        );
        Rc::new(Self {
            entry: Rc::clone(&self.entry),
            array: self.array.clone(),
            epoch: self.epoch,
        })
    }
    /// The active retirement callback retains this original entry for trace
    /// list changes, even after value access has become invalid.
    pub(crate) fn permits_trace_registration(&self) -> bool {
        self.is_live()
            || (self.entry.trace_refs.get() != 0
                && self.entry.table_owned.get()
                && self.array.upgrade().is_some_and(|array| {
                    array.borrow().detached_arrays.values().any(|old| {
                        old.native_keys
                            .0
                            .get(&self.entry.id)
                            .is_some_and(|entry| Rc::ptr_eq(entry, &self.entry))
                    })
                }))
    }

    pub(crate) fn is_live(&self) -> bool {
        !self.entry.retired.get()
            && self.array.upgrade().is_some_and(|array| {
                let contents = array.borrow();
                contents.rmw_retirement.is_none()
                    && ((contents.rmw_array_epoch == self.epoch
                        && contents.element_ids.get(self.entry.key.as_slice())
                            == Some(&self.entry.id))
                        || contents.detached_arrays.values().any(|old| {
                            old.native_keys
                                .0
                                .get(&self.entry.id)
                                .is_some_and(|entry| Rc::ptr_eq(entry, &self.entry))
                        }))
            })
    }
}
impl fmt::Debug for NativeElementAliasEntry {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_tuple("NativeElementAliasEntry")
            .field(&self.entry.id)
            .finish()
    }
}
impl PartialEq for NativeElementAliasEntry {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.entry, &other.entry)
    }
}
impl Eq for NativeElementAliasEntry {}
impl Drop for NativeElementAliasEntry {
    fn drop(&mut self) {
        let count = self.entry.aliases.get();
        assert!(count != 0, "native element alias underflow");
        self.entry.aliases.set(count - 1);
        if count != 1 {
            return;
        }
        let Some(array) = self.array.upgrade() else {
            return;
        };
        let mut contents = array.borrow_mut();
        let key = &self.entry.key;
        if contents.rmw_array_epoch == self.epoch
            && contents.element_ids.get(key.as_slice()) == Some(&self.entry.id)
            && !contents.element_operation_refs.contains_key(key.as_slice())
            && matches!(contents.var.as_ref(), Some(Var::Array(values)) if !values.contains_key(key.as_slice()))
        {
            contents.member_order.remove(key);
            contents.element_ids.remove(key.as_slice());
            contents.rmw_member_shells.remove(key.as_slice());
            contents.native_member_keys.remove(key);
        }
    }
}
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct NativeElementEntryObserver {
    entry: Weak<NativeElementEntry>,
    array: Weak<RefCell<CellContents>>,
    epoch: u64,
    key: Vec<u8>,
}
#[cfg(test)]
impl NativeElementEntryObserver {
    pub(crate) fn key(&self) -> Option<*mut TclObj> {
        self.entry
            .upgrade()?
            .original
            .borrow()
            .as_ref()
            .map(Owned::as_ptr)
    }
    pub(crate) fn observe(
        &self,
        current: Option<&super::RetainedArrayCell>,
        object_table: bool,
    ) -> (i32, i32, i32, i32, i32) {
        let present = current.is_some_and(|array| {
            array
                .contents
                .borrow()
                .element_ids
                .contains_key(self.key.as_slice())
        });
        let Some(entry) = self.entry.upgrade() else {
            return (i32::from(present), 0, -1, -1, -1);
        };
        let same = current.is_some_and(|array| {
            array.contents.borrow().element_ids.get(self.key.as_slice()) == Some(&entry.id)
        });
        let defined = self.array.upgrade().is_some_and(|array| {
            let cell = array.borrow();
            (cell.rmw_array_epoch == self.epoch && matches!(cell.var.as_ref(), Some(Var::Array(values)) if values.contains_key(self.key.as_slice())))
                || cell.detached_arrays.values().any(|old| old.native_keys.0.get(&entry.id).is_some_and(|old_entry| Rc::ptr_eq(old_entry, &entry)) && old.values.contains_key(self.key.as_slice()))
        });
        let dead = entry.retired.get();
        let defined = defined || entry.retiring_definition.get();
        let references = entry.aliases.get()
            + entry.trace_refs.get()
            + usize::from(object_table && entry.table_owned.get());
        (
            i32::from(present),
            i32::from(same),
            i32::from(defined),
            i32::from(dead),
            i32::try_from(references).unwrap(),
        )
    }
}

impl super::RetainedArrayCell {
    #[cfg(test)]
    pub(crate) fn observe_element_entry(&self, key: &[u8]) -> Option<NativeElementEntryObserver> {
        let cell = self.contents.borrow();
        let id = cell.element_ids.get(key)?;
        Some(NativeElementEntryObserver {
            entry: Rc::downgrade(cell.native_member_keys.0.get(id)?),
            array: Rc::downgrade(&self.contents),
            epoch: self.epoch,
            key: key.to_vec(),
        })
    }
    pub(crate) fn retain_element_alias(&self, key: &[u8]) -> Option<Rc<NativeElementAliasEntry>> {
        let mut contents = self.contents.borrow_mut();
        if contents.binding_id != Some(self.id)
            || contents.rmw_array_epoch != self.epoch
            || !matches!(contents.var.as_ref(), Some(Var::Array(_)))
        {
            return None;
        }
        contents.insert_member_entry(key);
        let id = *contents.element_ids.get(key)?;
        contents
            .native_member_keys
            .hold(id, &self.contents, self.epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{VarError, VarTable};
    use crate::obj;

    #[test]
    fn element_aliases_share_one_original_key_and_refill_the_same_entry() {
        let mut table = VarTable::default();
        let key = Owned::fresh(obj::new_string_bytes(b"k\0z"));
        let value = Owned::fresh(obj::new_string_bytes(b"one"));
        table
            .prepare_original_native_element(b"a", b"k\0z", Some(key.as_ptr()))
            .unwrap();
        table.store_elem(b"a", b"k\0z", value.as_ptr()).unwrap();
        let array = table.capture_array_cell(b"a").unwrap();
        let first = array.retain_element_alias(b"k\0z").unwrap();
        let view = Rc::clone(&first);
        let second = first.new_binding();
        let identity = first.entry.id;
        assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 2);
        assert!(table.remove_elem(b"a", b"k\0z"));
        assert!(first.is_live());
        assert_eq!(
            array.contents.borrow().element_ids[b"k\0z".as_slice()],
            identity
        );
        table.store_elem(b"a", b"k\0z", value.as_ptr()).unwrap();
        assert_eq!(table.element_id(b"a", b"k\0z"), Some(identity));
        assert!(table.remove_elem(b"a", b"k\0z"));
        drop(first);
        drop(view);
        assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 2);
        drop(second);
        assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 1);
        assert!(!array
            .contents
            .borrow()
            .element_ids
            .contains_key(b"k\0z".as_slice()));
    }

    #[test]
    fn retired_generation_releases_its_key_before_the_final_real_alias() {
        let mut table = VarTable::default();
        let key = Owned::fresh(obj::new_string_bytes(b"k"));
        let replacement = Owned::fresh(obj::new_string_bytes(b"k"));
        let value = Owned::fresh(obj::new_string_bytes(b"one"));
        table
            .prepare_original_native_element(b"a", b"k", Some(key.as_ptr()))
            .unwrap();
        table.store_elem(b"a", b"k", value.as_ptr()).unwrap();
        let original = table.capture_array_cell(b"a").unwrap();
        let alias = original.retain_element_alias(b"k").unwrap();
        let next_alias = alias.new_binding();
        let retired = table.begin_array_destruction(b"a").unwrap();
        retired.retire_member(b"k");
        retired.finish_destruction();
        assert!(!alias.is_live());
        assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 1);
        table
            .prepare_original_native_element(b"a", b"k", Some(replacement.as_ptr()))
            .unwrap();
        table.store_elem(b"a", b"k", value.as_ptr()).unwrap();
        assert_eq!(
            original.store(b"k", value.as_ptr()),
            Err(VarError::DeletedArray)
        );
        drop(alias);
        assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 1);
        drop(next_alias);
        assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*replacement.as_ptr()).ref_count }, 2);
        assert!(table.load_elem(b"a", b"k").is_some());
    }

    #[test]
    fn active_retirement_keeps_trace_registration_separate_from_value_access() {
        // naming.variable.original-array-member-trace-retirement-horizon
        // docs/design/analysis/name-resolution-proofs/variable-original-array-member-trace-retirement-horizon.md
        // Software allocation control; the native proof observes callback logs.
        let mut table = VarTable::default();
        let value = Owned::fresh(obj::new_string_bytes(b"one"));
        table
            .prepare_original_native_element(b"a", b"k", None)
            .unwrap();
        table.store_elem(b"a", b"k", value.as_ptr()).unwrap();
        let original = table.capture_array_cell(b"a").unwrap();
        let alias = original.retain_element_alias(b"k").unwrap();
        let retired = table.begin_array_destruction(b"a").unwrap();
        let trace = retired.begin_member_retirement(b"k", true, true).unwrap();
        retired.retire_member(b"k");
        assert!(!alias.is_live());
        assert!(alias.permits_trace_registration());
        drop(trace);
        assert!(!alias.permits_trace_registration());
        retired.finish_destruction();
        assert!(!alias.permits_trace_registration());
    }

    #[test]
    fn c84_element_entries_own_bytes_without_an_object_key() {
        let mut table = VarTable::default();
        table
            .prepare_original_native_element(b"a", b"k", None)
            .unwrap();
        let array = table.capture_array_cell(b"a").unwrap();
        let alias = array.retain_element_alias(b"k").unwrap();
        assert!(alias.entry.original.borrow().is_none());
        drop(table);
        assert!(!alias.is_live());
        assert!(array.read(b"k").is_none());
    }
}
