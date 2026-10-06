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

impl VarTable {
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
    pub(super) fn prepare_original_native_element_at(
        &mut self,
        slot: usize,
        element: &[u8],
        original: Option<*mut TclObj>,
    ) -> Result<(), super::VarError> {
        let missing = match self.cells[slot].contents.borrow().var.as_ref() {
            Some(super::Var::Array(_)) => false,
            Some(super::Var::Scalar(_)) => return Err(super::VarError::IsScalar),
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
    pub(super) fn retire_native_entry(&self, name: &[u8]) {
        self.entry_order.borrow_mut().remove(name);
        let retired = self.native_keys.borrow_mut().remove(name);
        drop(retired);
    }
    pub(super) fn retire_jim_key(&self, name: &[u8]) {
        let jim = self
            .native_keys
            .borrow()
            .get(name)
            .is_some_and(|key| key.jim);
        if jim {
            let retired = self.native_keys.borrow_mut().remove(name);
            drop(retired);
        }
    }
    /// Native lookup succeeds on an undefined compiled Var as well as a hash
    /// shell. Creation precedes observers and gives the key its real owner.
    pub(crate) fn prepare_native_name_cell(&mut self, name: &[u8], create: bool) -> bool {
        if self.has_native_name_cell(name) {
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

    /// Borrow the original argument vector held by this active C invocation.
    /// The admitting caller keeps its original headers pinned until frame pop.
    pub(crate) fn install_original_error_stack_argv(&mut self, original: &[*mut TclObj]) {
        let index = self.current_frame_index();
        self.frames[index].original_error_stack_argv = Some(original.to_vec());
    }
    pub(crate) fn original_error_stack_argv(&self) -> Option<Vec<*mut TclObj>> {
        self.frames[self.current_frame_index()]
            .original_error_stack_argv
            .clone()
    }
    /// Borrow the actual invocation transport at an absolute call level.
    /// Copying these pointer values acquires no native object references.
    pub(crate) fn original_error_stack_argv_at(&self, level: usize) -> Option<Vec<*mut TclObj>> {
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
    pub(crate) fn retain_original_compiled_alias(
        &mut self,
        index: usize,
    ) -> Option<Rc<super::NativeScalarAliasEntry>> {
        let (frame, cell) = self.compiled_cell(index)?;
        Some(self.frames[frame].table.retain_native_scalar_alias_at(cell))
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
    pub(crate) fn store_native_compiled_temporary(
        &mut self,
        slot: usize,
        value: *mut crate::obj::TclObj,
    ) -> Result<(), super::VarError> {
        let (frame, cell) = self
            .compiled_cell(slot)
            .ok_or(super::VarError::NameProtocolUnavailable)?;
        if !self.frames[frame].table.cells[cell].anonymous {
            return Err(super::VarError::NameProtocolUnavailable);
        }
        crate::obj::check_native_liveness(value)
            .map_err(|_| super::VarError::NameProtocolUnavailable)?;
        let receiver = self.frames[frame]
            .table
            .capture_receiver_at(cell, None, true)?
            .ok_or(super::VarError::NameProtocolUnavailable)?;
        receiver.store(value)
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
