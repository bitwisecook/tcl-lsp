// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native hash keys belong to table entries, not retained Var cells.

use super::{FrameStack, VarTable};
use crate::obj::{Owned, TclObj};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

pub(super) struct NativeHashKey {
    _original: Owned,
    jim: bool,
}
pub(super) type NativeHashKeys = Rc<RefCell<BTreeMap<Vec<u8>, NativeHashKey>>>;

/// Original variable-table inventory and keys without table, cell or native
/// key owning references. Memory leases do not keep retired keys live.
pub(crate) struct OriginalJimVariableInventory {
    ledger: std::rc::Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
    revision: u64,
    keys: Vec<OriginalJimVariableInventoryKey>,
}
pub(crate) struct OriginalJimVariableInventoryKey {
    keys: std::rc::Weak<RefCell<BTreeMap<Vec<u8>, NativeHashKey>>>,
    cell: std::rc::Weak<RefCell<super::CellContents>>,
    binding: Option<tcl_runtime_api::VarId>,
    name: Vec<u8>,
    original: crate::obj::NativeObjectLifetime,
}
impl OriginalJimVariableInventory {
    pub(crate) fn check(&self) -> Result<(), tcl_syntax::value::ValueError> {
        let current = self
            .ledger
            .upgrade()
            .is_some_and(|ledger| ledger.borrow().revision() == Some(self.revision));
        if current {
            Ok(())
        } else {
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim inventory table changed",
            ))
        }
    }
    pub(crate) fn keys(&self) -> &[OriginalJimVariableInventoryKey] {
        &self.keys
    }
}
impl OriginalJimVariableInventoryKey {
    pub(crate) fn original(&self) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        let pointer = self.original.checked_ptr()?;
        let key_current = self.keys.upgrade().is_some_and(|keys| {
            keys.borrow()
                .get(&self.name)
                .is_some_and(|key| key.jim && key._original.as_ptr() == pointer)
        });
        let cell_current = self.cell.upgrade().is_some_and(|cell| {
            let cell = cell.borrow();
            cell.binding_id == self.binding
                && cell.var.is_some()
                && cell.rmw_retirement.is_none()
                && cell.check_owned_value_role().is_ok()
        });
        if key_current && cell_current {
            Ok(pointer)
        } else {
            Err(ValueError::CommandProtocolUnavailable(
                "retired original Jim inventory key",
            ))
        }
    }
}

impl VarTable {
    /// Capture the actual selected primary hash table and its original key
    /// objects. Static fallback and compiled C declaration arrays are separate.
    pub(crate) fn capture_original_jim_inventory(
        &self,
        include_links: bool,
        lookup: tcl_syntax::native_jim_lookup::NativeJimVariableInventoryLookup<'_>,
    ) -> Result<OriginalJimVariableInventory, tcl_syntax::value::ValueError> {
        use tcl_syntax::{
            native_jim_lookup::NativeJimVariableInventoryLookup as Lookup, value::ValueError,
        };
        let ledger = self.entry_order.borrow();
        let revision = ledger
            .revision()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim inventory table arithmetic",
            ))?;
        let names: Vec<&[u8]> = match lookup {
            Lookup::Exact(pattern) => self
                .slots
                .get_key_value(pattern)
                .map(|(name, _)| vec![name.as_slice()])
                .unwrap_or_default(),
            Lookup::All | Lookup::Scan(_) => ledger.keys().ok_or(
                ValueError::CommandProtocolUnavailable("original Jim inventory table order"),
            )?,
        };
        let held_keys = self.native_keys.borrow();
        let mut keys = Vec::new();
        for name in names {
            let slot = *self
                .slots
                .get(name)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original Jim inventory slot",
                ))?;
            let cell = self.cells[slot].contents.borrow();
            match cell.var.as_ref() {
                Some(super::Var::Link(_)) if !include_links => continue,
                None => continue,
                Some(super::Var::PendingScalar(_) | super::Var::Array(_)) => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original Jim inventory variable kind",
                    ))
                }
                Some(super::Var::Scalar(_) | super::Var::Link(_)) => {}
            }
            cell.check_owned_value_role()?;
            let key = held_keys.get(name).filter(|key| key.jim).ok_or(
                ValueError::CommandProtocolUnavailable("original Jim inventory key owner"),
            )?;
            crate::obj::check_native_liveness(key._original.as_ptr())?;
            keys.push(OriginalJimVariableInventoryKey {
                keys: Rc::downgrade(&self.native_keys),
                cell: Rc::downgrade(&self.cells[slot].contents),
                binding: cell.binding_id,
                name: name.to_vec(),
                original: crate::obj::NativeObjectLifetime::retain(key._original.as_ptr()),
            });
        }
        Ok(OriginalJimVariableInventory {
            ledger: Rc::downgrade(&self.entry_order),
            revision,
            keys,
        })
    }
    pub(crate) fn prepare_original_array_member(
        &mut self,
        name: &[u8],
        element: Option<&[u8]>,
        original: Option<*mut TclObj>,
    ) -> Result<super::PreparedArrayMember, super::JimLinkStoreError> {
        super::PreparedArrayMember::table(self, name, element, original)
    }
    #[cfg(test)]
    pub(crate) fn original_native_key_is(&self, name: &[u8], original: *mut TclObj) -> bool {
        self.native_keys
            .borrow()
            .get(name)
            .is_some_and(|key| key._original.as_ptr() == original)
    }
    #[cfg(test)]
    pub(crate) fn native_name_cell_identity(&self, name: &[u8]) -> Option<tcl_runtime_api::VarId> {
        self.contents(name)?.borrow().binding_id
    }
    #[cfg(test)]
    pub(crate) fn prepare_original_native_element(
        &mut self,
        name: &[u8],
        element: &[u8],
        original: Option<*mut TclObj>,
    ) -> Result<(), super::VarError> {
        self.ensure_array(name)?;
        let slot = self.lookup_slot(name).expect("selected array");
        self.prepare_original_native_element_at(slot, element, original)
    }
    #[cfg(test)]
    pub(super) fn prepare_original_native_element_at(
        &mut self,
        slot: usize,
        element: &[u8],
        original: Option<*mut TclObj>,
    ) -> Result<(), super::VarError> {
        let missing = match self.cells[slot].contents.borrow().var.as_ref() {
            Some(super::Var::Array(_)) => false,
            Some(super::Var::Scalar(_) | super::Var::PendingScalar(_)) => {
                return Err(super::VarError::IsScalar)
            }
            Some(super::Var::Link(_)) => return Err(super::VarError::NameProtocolUnavailable),
            None => true,
        };
        if missing {
            if let Some(old) = self.put_at(slot, super::Var::Array(Default::default())) {
                old.release();
            }
        }
        let mut cell = self.cells[slot].contents.borrow_mut();
        cell.insert_member_entry(element);
        cell.element_ids
            .entry(element.to_vec())
            .or_insert_with(super::fresh_var_id);
        if !matches!(cell.var.as_ref(), Some(super::Var::Array(values)) if values.contains_key(element))
        {
            cell.rmw_member_shells.insert(element.to_vec());
        }
        let id = cell.element_ids[element];
        cell.native_member_keys.ensure(id, element, original);
        Ok(())
    }
    pub(crate) fn retain_native_key(&self, name: &[u8], original: *mut TclObj, jim: bool) {
        if self
            .lookup_slot(name)
            .is_some_and(|slot| !self.declared_slots.contains(&slot))
        {
            self.native_keys
                .borrow_mut()
                .entry(name.to_vec())
                .or_insert_with(|| NativeHashKey {
                    _original: Owned::retain(original),
                    jim,
                });
        }
    }
    pub(super) fn take_native_entry(&self, name: &[u8]) -> Option<NativeHashKey> {
        self.entry_order.borrow_mut().remove(name);
        self.native_keys.borrow_mut().remove(name)
    }
    pub(super) fn take_jim_key(&self, name: &[u8]) -> Option<NativeHashKey> {
        let jim = self
            .native_keys
            .borrow()
            .get(name)
            .is_some_and(|key| key.jim);
        if jim {
            self.native_keys.borrow_mut().remove(name)
        } else {
            None
        }
    }
    /// Native lookup succeeds on an undefined compiled Var as well as a hash
    /// shell. Creation precedes observers and gives the key its real owner.
    pub(crate) fn prepare_native_name_cell(&mut self, name: &[u8], create: bool) -> bool {
        if self.has_native_name_cell(name) {
            let slot = self.lookup_slot(name).expect("selected original name cell");
            self.cells[slot]
                .contents
                .borrow_mut()
                .binding_id
                .get_or_insert_with(super::fresh_var_id);
            return true;
        }
        if !create {
            return false;
        }
        let slot = self.slot_for(name);
        let mut contents = self.cells[slot].contents.borrow_mut();
        contents.binding_id.get_or_insert_with(super::fresh_var_id);
        contents.undefined_shell = contents.var.is_none();
        if !self.declared_slots.contains(&slot) {
            self.entry_order.borrow_mut().insert(name);
        }
        true
    }
    pub(crate) fn has_native_name_cell(&self, name: &[u8]) -> bool {
        if let Some(slot) = self.lookup_slot(name) {
            if self.declared_slots.contains(&slot)
                || self.cells[slot].contents.borrow().binding_id.is_some()
            {
                return true;
            }
        }
        false
    }
    pub(crate) fn native_compiled_cell(&self, name: &[u8]) -> Option<usize> {
        let slot = self.lookup_slot(name)?;
        self.declared_slots.contains(&slot).then_some(slot)
    }
}
impl FrameStack {
    /// Inspection-only physical cell identity of an actual installed native slot.
    #[cfg(test)]
    pub(crate) fn native_compiled_cell_identity(&self, slot: usize) -> Option<usize> {
        let (frame, cell) = self.compiled_cell(slot)?;
        Some(Rc::as_ptr(&self.frames[frame].table.cells.get(cell)?.contents) as usize)
    }

    /// Keep the original invocation allocation safe until frame pop, without
    /// acquiring native references or reading its argument Strings.
    pub(crate) fn install_original_error_stack_argv(
        &mut self,
        original: &[*mut TclObj],
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let original = original
            .iter()
            .map(|&original| {
                crate::obj::check_native_liveness(original)?;
                Ok(crate::obj::NativeObjectLifetime::retain(original))
            })
            .collect::<Result<Vec<_>, tcl_syntax::value::ValueError>>()?;
        let index = self.current_frame_index();
        self.frames[index].original_error_stack_argv = Some(original);
        Ok(())
    }
    pub(crate) fn original_error_stack_argv(&self) -> Option<Vec<*mut TclObj>> {
        self.frames[self.current_frame_index()]
            .original_error_stack_argv
            .as_ref()
            .map(|original| {
                original
                    .iter()
                    .map(crate::obj::NativeObjectLifetime::as_ptr)
                    .collect()
            })
    }
    /// Retain allocation safety outside the frame borrow while a reporter
    /// selects its own original header/List/String purpose. No native +1.
    pub(crate) fn original_error_stack_argv_owner_at(
        &self,
        level: usize,
    ) -> Option<Vec<crate::obj::NativeObjectLifetime>> {
        self.frames[self.index_of_level(level)?]
            .original_error_stack_argv
            .clone()
    }
    pub(crate) fn prepare_native_compiled_name_cell(&mut self, index: usize) -> bool {
        let Some((frame, cell)) = self.compiled_cell(index) else {
            return false;
        };
        self.frames[frame].table.cells[cell]
            .contents
            .borrow_mut()
            .binding_id
            .get_or_insert_with(super::fresh_var_id);
        true
    }
    /// Observe the already selected ordinal cell without a new variable role.
    pub(crate) fn original_compiled_name_observation(
        &self,
        index: usize,
    ) -> Option<(
        usize,
        Vec<u8>,
        super::VariablePublicationObservation,
        Option<super::OriginalVariableLinkObservation>,
    )> {
        let (frame, cell) = self.compiled_cell(index)?;
        let table = &self.frames[frame].table;
        let (observation, link) =
            super::VarTable::observe_original_name_contents(&table.cells[cell].contents).ok()?;
        Some((
            self.frames[frame].level,
            table.slot_name(cell)?.to_vec(),
            observation,
            link,
        ))
    }
    pub(crate) fn original_compiled_cell_at_is_current(
        &self,
        level: usize,
        slot: usize,
        original: &super::VariablePublicationObservation,
    ) -> bool {
        self.index_of_level(level)
            .and_then(|frame| {
                let frame = &self.frames[frame];
                let cell = frame.compiled_slots.get(slot).copied().flatten()?;
                Some(original.matches(&frame.table.cells.get(cell)?.contents))
            })
            .unwrap_or(false)
    }

    pub(crate) fn select_original_compiled_alias(
        &mut self,
        index: usize,
        acquire: bool,
    ) -> Option<Rc<super::NativeScalarAliasEntry>> {
        let (frame, cell) = self.compiled_cell(index)?;
        Some(
            self.frames[frame]
                .table
                .select_native_scalar_alias_at(cell, acquire),
        )
    }
    pub(crate) fn original_compiled_link(&self, index: usize) -> Option<super::Link> {
        let (frame, cell) = self.compiled_cell(index)?;
        let contents = self.frames[frame].table.cells[cell].contents.borrow();
        match contents.var.as_ref() {
            Some(super::Var::Link(link)) => Some(link.clone()),
            _ => None,
        }
    }
    pub(crate) fn capture_original_compiled_receiver(
        &mut self,
        index: usize,
        element: Option<Vec<u8>>,
        create: bool,
    ) -> Result<Option<super::VariableReceiver>, super::VarError> {
        let (frame, cell) = self
            .compiled_cell(index)
            .ok_or(super::VarError::NameProtocolUnavailable)?;
        self.frames[frame]
            .table
            .capture_receiver_at(cell, element, create)
    }
    /// Install the admitted program's actual indexed layout before binding
    /// formal values. Existing declaration cells remain at their original indices.
    pub(crate) fn install_native_compiled_local_layout(
        &mut self,
        layout: &tcl_runtime_api::native_compilation::NativeCompiledLocalLayout,
    ) -> Result<(), super::VarError> {
        let index = self.current_frame_index();
        let frame = &mut self.frames[index];
        if layout.names.len() < frame.compiled_slots.len() {
            return Err(super::VarError::NameProtocolUnavailable);
        }
        for (slot, existing) in frame.compiled_slots.iter().enumerate() {
            let actual = existing.and_then(|cell| frame.table.slot_name(cell));
            let supplied = layout.names.get(slot).and_then(Option::as_ref);
            if actual != supplied.map(tcl_core_types::NameBytes::as_bytes) {
                return Err(super::VarError::NameProtocolUnavailable);
            }
        }
        for name in &layout.names[frame.compiled_slots.len()..] {
            let cell = match name {
                Some(name) => frame.table.declare_compiled_cell(name.as_bytes()),
                None => frame.table.declare_compiled_temporary(),
            };
            frame.compiled_slots.push(Some(cell));
        }
        Ok(())
    }
    /// Store into an installed unnamed native local without name lookup or traces.
    /// The real frame cell retains the value until replacement or frame teardown.
    #[cfg(test)]
    pub(crate) fn store_native_compiled_temporary(
        &mut self,
        slot: usize,
        value: *mut crate::obj::TclObj,
    ) -> Result<(), super::VarError> {
        crate::obj::check_native_liveness(value)
            .map_err(|_| super::VarError::NameProtocolUnavailable)?;
        let receiver = self.capture_native_compiled_temporary_receiver(slot)?;
        receiver.store(value)
    }

    pub(crate) fn capture_native_compiled_temporary_receiver(
        &mut self,
        slot: usize,
    ) -> Result<super::VariableReceiver, super::VarError> {
        self.capture_native_compiled_temporary_receiver_at(slot, true)?
            .ok_or(super::VarError::NameProtocolUnavailable)
    }

    pub(crate) fn capture_native_compiled_temporary_receiver_for_unset(
        &mut self,
        slot: usize,
    ) -> Result<Option<super::VariableReceiver>, super::VarError> {
        self.capture_native_compiled_temporary_receiver_at(slot, false)
    }

    fn capture_native_compiled_temporary_receiver_at(
        &mut self,
        slot: usize,
        create: bool,
    ) -> Result<Option<super::VariableReceiver>, super::VarError> {
        let (frame, cell) = self
            .compiled_cell(slot)
            .ok_or(super::VarError::NameProtocolUnavailable)?;
        if !self.frames[frame].table.cells[cell].anonymous {
            return Err(super::VarError::NameProtocolUnavailable);
        }
        self.frames[frame]
            .table
            .capture_receiver_at(cell, None, create)
    }

    /// Borrow the actual header held by an installed unnamed compiler local.
    pub(crate) fn native_compiled_temporary(
        &self,
        slot: usize,
    ) -> Result<Option<*mut crate::obj::TclObj>, super::VarError> {
        let (frame, cell) = self
            .compiled_cell(slot)
            .ok_or(super::VarError::NameProtocolUnavailable)?;
        let table = &self.frames[frame].table;
        if !table.cells[cell].anonymous {
            return Err(super::VarError::NameProtocolUnavailable);
        }
        let contents = table.cells[cell].contents.borrow();
        contents
            .check_owned_value_role()
            .map_err(|_| super::VarError::NameProtocolUnavailable)?;
        match contents.var.as_ref() {
            Some(super::Var::Scalar(value)) => Ok(Some(*value)),
            None => Ok(None),
            _ => Err(super::VarError::NameProtocolUnavailable),
        }
    }

    pub(crate) fn native_local_name_table(
        &self,
    ) -> Option<&Rc<tcl_runtime_api::native_literal::NativeLocalNameTable<Owned>>> {
        self.frames
            .get(self.current_frame_index())?
            .native_local_names
            .as_ref()
    }
    pub(crate) fn take_native_local_name_table(
        &mut self,
    ) -> Option<Rc<tcl_runtime_api::native_literal::NativeLocalNameTable<Owned>>> {
        let index = self.current_frame_index();
        self.frames[index].native_local_names.take()
    }

    pub(crate) fn install_native_local_name_table(
        &mut self,
        table: Rc<tcl_runtime_api::native_literal::NativeLocalNameTable<Owned>>,
    ) {
        let index = self.current_frame_index();
        self.frames[index].native_local_names = Some(table);
    }
    pub(crate) fn native_compiled_name_index(&self, level: usize, name: &[u8]) -> Option<usize> {
        let frame = &self.frames[self.index_of_level(level)?];
        let cell = frame.table.native_compiled_cell(name)?;
        frame
            .compiled_slots
            .iter()
            .position(|slot| *slot == Some(cell))
    }
    pub(crate) fn native_compiled_names(&self) -> Vec<Option<tcl_core_types::NameBytes>> {
        let frame = &self.frames[self.current_frame_index()];
        frame
            .compiled_slots
            .iter()
            .map(|slot| {
                Some(tcl_core_types::NameBytes::from(
                    frame.table.slot_name((*slot)?)?,
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod indexed_receiver_tests {
    use super::*;
    use crate::{
        namespace::{Namespaces, GLOBAL},
        obj,
    };
    use tcl_dialect::TclVersion;

    #[test]
    fn name_lookup_observes_only_the_selected_existing_compiled_cell() {
        for version in TclVersion::ALL {
            let protocol = tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(
                tcl_registry::InvocationDialect::for_version(version)
                    .execution_point()
                    .unwrap(),
            )
            .unwrap();
            let mut frames = FrameStack::new();
            frames.push(GLOBAL);
            frames.install_formal_cells(&[b"same".to_vec(), b"same".to_vec()], protocol);
            let frame = frames.current_frame_index();
            let table = &mut frames.frames[frame].table;
            assert!(!table.prepare_native_name_cell(b"missing", false));
            assert_eq!(table.cells.len(), 2);
            assert!(table.prepare_native_name_cell(b"same", false));
            let (observed, _) = table.observe_original_name_cell(b"same").unwrap();
            assert!(table.cells[0].contents.borrow().binding_id.is_some());
            assert!(table.cells[1].contents.borrow().binding_id.is_none());
            assert!(table.cells[0].contents.borrow().var.is_none());
            assert!(table.entry_order.borrow().physical_keys().is_empty());
            assert!(table.native_keys.borrow().is_empty());
            assert!(frames.original_compiled_cell_at_is_current(1, 0, &observed));
            assert!(!frames.original_compiled_cell_at_is_current(1, 1, &observed));
        }
    }

    #[test]
    fn unnamed_native_locals_retain_original_values_without_dynamic_entries() {
        use tcl_runtime_api::native_compilation::{
            NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind, NativeInterpreterIdentity,
        };
        let mut frames = FrameStack::new();
        frames.push(GLOBAL);
        let layout = NativeCompiledLocalLayout {
            owner: NativeInterpreterIdentity {
                owner: NativeInterpreterIdentity::fresh_owner(),
                interpreter: 0,
            },
            token: 1,
            epoch: 0,
            kind: NativeCompiledLocalLayoutKind::Procedure,
            names: vec![
                Some(tcl_core_types::NameBytes::from(b"named".as_slice())),
                None,
                None,
            ],
        };
        frames
            .install_native_compiled_local_layout(&layout)
            .unwrap();
        let first = Owned::fresh(obj::new_string_bytes(b"original"));
        let replacement = Owned::fresh(obj::new_string_bytes(b"replacement"));
        frames
            .store_native_compiled_temporary(1, first.as_ptr())
            .unwrap();
        assert_eq!(
            frames.native_compiled_temporary(1).unwrap(),
            Some(first.as_ptr())
        );
        assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 2);
        assert!(frames.local_names().is_empty());
        assert!(frames
            .store_native_compiled_temporary(0, first.as_ptr())
            .is_err());
        frames
            .install_native_compiled_local_layout(&layout)
            .unwrap();
        assert_eq!(
            frames.native_compiled_temporary(1).unwrap(),
            Some(first.as_ptr())
        );
        frames
            .store_native_compiled_temporary(1, replacement.as_ptr())
            .unwrap();
        assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*replacement.as_ptr()).ref_count }, 2);
        frames.pop();
        assert_eq!(unsafe { (*replacement.as_ptr()).ref_count }, 1);
    }

    #[test]
    fn duplicate_formal_indices_keep_distinct_read_write_unset_and_alias_receivers() {
        let protocol = tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(
            tcl_registry::InvocationDialect::for_version(TclVersion::V9_0)
                .execution_point()
                .unwrap(),
        )
        .unwrap();
        let mut frames = FrameStack::new();
        let mut namespaces = Namespaces::new();
        frames.push(GLOBAL);
        frames.install_formal_cells(&[b"x".to_vec(), b"x".to_vec()], protocol);
        let one = Owned::fresh(obj::new_string_bytes(b"one"));
        let two = Owned::fresh(obj::new_string_bytes(b"two"));
        frames.store_formal_cell(0, one.as_ptr()).unwrap();
        frames.store_formal_cell(1, two.as_ptr()).unwrap();
        let (first, _) = crate::vars::capture_original_indexed_receiver(
            &mut frames,
            &mut namespaces,
            0,
            None,
            false,
        )
        .unwrap()
        .unwrap();
        let (second, _) = crate::vars::capture_original_indexed_receiver(
            &mut frames,
            &mut namespaces,
            1,
            None,
            false,
        )
        .unwrap()
        .unwrap();
        assert_ne!(first.binding_id(), second.binding_id());
        assert_eq!(first.read().unwrap(), Some(one.as_ptr()));
        assert_eq!(second.read().unwrap(), Some(two.as_ptr()));
        let link = crate::vars::original_indexed_link_target(&mut frames, &mut namespaces, 1, None)
            .unwrap();
        let alias = link
            .native_scalar_entry
            .as_ref()
            .unwrap()
            .capture_receiver(None, false)
            .unwrap()
            .unwrap();
        assert_eq!(alias.read().unwrap(), Some(two.as_ptr()));
        alias.store(one.as_ptr()).unwrap();
        assert_eq!(second.read().unwrap(), Some(one.as_ptr()));
        second.unset().unwrap();
        assert_eq!(alias.read().unwrap(), None);
        assert_eq!(first.read().unwrap(), Some(one.as_ptr()));
        alias.store(two.as_ptr()).unwrap();
        assert_eq!(second.read().unwrap(), Some(two.as_ptr()));
        assert_eq!(first.read().unwrap(), Some(one.as_ptr()));
        frames.table_mut(1).unwrap().insert_link(b"alias", link);
        assert_eq!(
            crate::vars::get(&frames, &namespaces, GLOBAL, b"alias"),
            Some(two.as_ptr())
        );
        crate::vars::set(&mut frames, &mut namespaces, GLOBAL, b"alias", one.as_ptr()).unwrap();
        assert_eq!(second.read().unwrap(), Some(one.as_ptr()));
        assert!(crate::vars::unset(
            &mut frames,
            &mut namespaces,
            GLOBAL,
            b"alias"
        ));
        assert_eq!(second.read().unwrap(), None);
        assert_eq!(first.read().unwrap(), Some(one.as_ptr()));
        crate::vars::set(&mut frames, &mut namespaces, GLOBAL, b"alias", two.as_ptr()).unwrap();
        assert_eq!(second.read().unwrap(), Some(two.as_ptr()));
    }
}
#[cfg(test)]
mod original_frame_index_tests {
    use super::FrameStack;
    use crate::{
        namespace::GLOBAL,
        obj::{self, Owned},
    };

    #[test]
    fn indexed_original_argv_keeps_level_selection_and_retained_allocation_roles() {
        // Software indexing and allocation ownership; no native frame admission.
        let first = Owned::fresh(obj::new_string_bytes(b"first"));
        let second = Owned::fresh(obj::new_string_bytes(b"second"));
        let newest = Owned::fresh(obj::new_string_bytes(b"same-level"));
        let mut frames = FrameStack::new();
        assert!(frames.original_error_stack_argv_owner_at(0).is_none());
        frames.push(GLOBAL);
        frames
            .install_original_error_stack_argv(&[first.as_ptr()])
            .unwrap();
        frames.push(GLOBAL);
        frames
            .install_original_error_stack_argv(&[second.as_ptr()])
            .unwrap();
        let saved = frames.set_active_level(1);
        let selected = frames.original_error_stack_argv_owner_at(2).unwrap();
        assert_eq!(selected[0].as_ptr(), second.as_ptr());
        assert_eq!(
            frames.original_error_stack_argv().unwrap(),
            vec![first.as_ptr()]
        );
        assert_eq!(
            frames.original_error_stack_argv_owner_at(1).unwrap()[0].as_ptr(),
            first.as_ptr()
        );
        assert!(frames.original_error_stack_argv_owner_at(3).is_none());
        frames.set_active_level(saved);
        frames.push_same_level(GLOBAL);
        frames
            .install_original_error_stack_argv(&[newest.as_ptr()])
            .unwrap();
        assert_eq!(
            frames.original_error_stack_argv_owner_at(2).unwrap()[0].as_ptr(),
            newest.as_ptr()
        );
        frames.pop();
        assert_eq!(
            frames.original_error_stack_argv_owner_at(2).unwrap()[0].as_ptr(),
            second.as_ptr()
        );
        frames.pop();
        assert!(frames.original_error_stack_argv_owner_at(2).is_none());
        assert_eq!(selected[0].as_ptr(), second.as_ptr());
        assert_eq!(unsafe { (*second.as_ptr()).ref_count }, 1);
        let original = second.as_ptr();
        drop(second);
        assert_eq!(selected[0].as_ptr(), original);
        assert!(obj::check_native_liveness(original).is_err());
        drop(selected);
    }
}
