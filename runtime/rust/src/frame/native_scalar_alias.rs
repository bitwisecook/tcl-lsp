// SPDX-License-Identifier: AGPL-3.0-or-later
//! C scalar aliases retain their actual variable entry until the alias is freed.

use super::{CellContents, VarTable};
use std::{cell::RefCell, rc::Rc};
use tcl_runtime_api::VarId;

/// One actual alias role plus independently retained Rust entry metadata.
pub(crate) struct NativeScalarAliasEntry {
    pub(super) contents: Rc<RefCell<CellContents>>,
    pub(super) binding: VarId,
    target_reference: bool,
    role_active: std::cell::Cell<bool>,
    role_unissued: std::cell::Cell<bool>,
    /// Exact retained root slot; it is not resolved again after alias creation.
    name: Vec<u8>,
}
impl std::fmt::Debug for NativeScalarAliasEntry {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_tuple("NativeScalarAliasEntry")
            .field(&self.binding)
            .finish()
    }
}
impl PartialEq for NativeScalarAliasEntry {
    fn eq(&self, other: &Self) -> bool {
        self.binding == other.binding && Rc::ptr_eq(&self.contents, &other.contents)
    }
}
impl Eq for NativeScalarAliasEntry {}
impl NativeScalarAliasEntry {
    pub(crate) fn check_owned_value_role(&self) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_stored_role()?;
        let cell = self.contents.borrow();
        if cell.binding_id != Some(self.binding) || cell.rmw_retirement.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired original scalar alias entry",
            ));
        }
        cell.check_owned_value_role()
    }
    pub(crate) fn check_trace_root(
        &self,
        table: &VarTable,
        name: &[u8],
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_owned_value_role()?;
        if table
            .contents(name)
            .is_some_and(|contents| Rc::ptr_eq(contents, &self.contents))
        {
            Ok(())
        } else {
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original scalar trace root table",
            ))
        }
    }
    pub(crate) fn identity(&self) -> VarId {
        self.binding
    }
    pub(crate) fn check_stored_role(&self) -> Result<(), tcl_syntax::value::ValueError> {
        if self.role_active.get() {
            Ok(())
        } else {
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "withdrawn original scalar alias role",
            ))
        }
    }
    pub(crate) fn check_selected_target(&self) -> Result<(), tcl_syntax::value::ValueError> {
        if !self.role_active.get() && !self.role_unissued.get() {
            return self.check_stored_role();
        }
        self.check_target_identity()
    }
    pub(crate) fn check_target_identity(&self) -> Result<(), tcl_syntax::value::ValueError> {
        let cell = self.contents.borrow();
        if cell.binding_id != Some(self.binding) || cell.rmw_retirement.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original scalar alias selection",
            ));
        }
        cell.check_owned_value_role()
    }
    pub(crate) fn select_new_binding(&self) -> Result<Rc<Self>, tcl_syntax::value::ValueError> {
        self.check_stored_role()?;
        self.check_selected_target()?;
        Ok(Rc::new(Self {
            contents: Rc::clone(&self.contents),
            binding: self.binding,
            target_reference: self.target_reference,
            role_active: std::cell::Cell::new(false),
            role_unissued: std::cell::Cell::new(true),
            name: self.name.clone(),
        }))
    }
    pub(crate) fn acquire_selected_role(&self) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_selected_target()?;
        if self.role_unissued.replace(false) {
            let mut cell = self.contents.borrow_mut();
            if self.target_reference {
                cell.native_alias_refs = cell
                    .native_alias_refs
                    .checked_add(1)
                    .expect("native alias holds exhausted");
            }
            self.role_active.set(true);
        }
        self.check_stored_role()
    }
    pub(crate) fn try_new_binding(&self) -> Result<Rc<Self>, tcl_syntax::value::ValueError> {
        self.check_stored_role()?;
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.binding) || cell.rmw_retirement.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original scalar alias mint cell",
            ));
        }
        cell.check_owned_value_role()?;
        if self.target_reference {
            cell.native_alias_refs = cell
                .native_alias_refs
                .checked_add(1)
                .expect("native alias holds exhausted");
        }
        drop(cell);
        Ok(Rc::new(Self {
            contents: Rc::clone(&self.contents),
            binding: self.binding,
            target_reference: self.target_reference,
            role_active: std::cell::Cell::new(true),
            role_unissued: std::cell::Cell::new(false),
            name: self.name.clone(),
        }))
    }
    /// Neutral direct-holder construction keeps its existing role schedule.
    pub(crate) fn new_binding(&self) -> Rc<Self> {
        self.try_new_binding().expect("current fixture alias role")
    }
    pub(crate) fn withdraw_stored_role(&self) -> super::RetiredVariableParts {
        let mut retired = super::RetiredVariableParts::default();
        self.role_unissued.set(false);
        if !self.role_active.replace(false) {
            return retired;
        }
        if !self.target_reference {
            return retired;
        }
        let mut cell = self.contents.borrow_mut();
        cell.native_alias_refs = cell
            .native_alias_refs
            .checked_sub(1)
            .expect("actual C alias hold");
        if cell.native_alias_refs != 0
            || cell.binding_id != Some(self.binding)
            || cell.operation_refs != 0
            || cell.rmw_refs != 0
            || cell.namespace_declared
            || (!cell.undefined_shell && cell.var.is_some())
        {
            return retired;
        }
        let old = cell.take_value();
        cell.undefined_shell = false;
        let key = cell.retire_rmw_shell();
        if !cell.compiled_declaration {
            cell.binding_id = None;
        }
        // The existing scalar role retires its hash key before old contents.
        retired.hash_key(key);
        retired.value(old);
        retired
    }
    pub(crate) fn ensure_array(&self) -> Result<(), super::VarError> {
        self.check_stored_role()
            .map_err(|_| super::VarError::NameProtocolUnavailable)?;
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.binding) {
            return Err(super::VarError::DeletedNamespace);
        }
        if let Some(error) = cell.rmw_retirement {
            return Err(error);
        }
        let retired = cell.publish_selected_array(Some(&self.name))?;
        drop(cell);
        if let Some(retired) = retired {
            retired.release();
        }
        Ok(())
    }
    pub(crate) fn array_names(&self) -> Result<Option<Vec<Vec<u8>>>, super::VarError> {
        self.check_stored_role()
            .map_err(|_| super::VarError::NameProtocolUnavailable)?;
        let cell = self.contents.borrow();
        if cell.binding_id != Some(self.binding) {
            return Err(super::VarError::DeletedNamespace);
        }
        if let Some(error) = cell.rmw_retirement {
            return Err(error);
        }
        Ok(match cell.var.as_ref() {
            Some(super::Var::Array(values)) => Some(match cell.member_order.keys() {
                Some(keys) => keys
                    .into_iter()
                    .filter(|key| values.contains_key(*key))
                    .map(<[u8]>::to_vec)
                    .collect(),
                None => values.keys().cloned().collect(),
            }),
            _ => None,
        })
    }
    pub(crate) fn trace_member_identity(&self, key: &[u8]) -> Option<tcl_runtime_api::VarId> {
        if !self.role_active.get() {
            return None;
        }
        let cell = self.contents.borrow();
        (cell.binding_id == Some(self.binding) && cell.rmw_retirement.is_none())
            .then(|| cell.element_ids.get(key).copied())
            .flatten()
    }
    pub(crate) fn prepare_original_array_member(
        &self,
        element: Option<&[u8]>,
        original: Option<*mut crate::obj::TclObj>,
    ) -> Result<super::PreparedArrayMember, super::JimLinkStoreError> {
        self.check_stored_role()?;
        {
            let cell = self.contents.borrow();
            if cell.binding_id != Some(self.binding) || cell.rmw_retirement.is_some() {
                return Err(super::VarError::DeletedNamespace.into());
            }
        }
        super::PreparedArrayMember::selected(&self.contents, Some(&self.name), element, original)
    }

    #[cfg(test)]
    pub(crate) fn prepare_original_element(
        &self,
        key: &[u8],
        original: Option<*mut crate::obj::TclObj>,
    ) -> Result<(), super::VarError> {
        self.check_stored_role()
            .map_err(|_| super::VarError::NameProtocolUnavailable)?;
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.binding) {
            return Err(super::VarError::DeletedNamespace);
        }
        if let Some(error) = cell.rmw_retirement {
            return Err(error);
        }
        let old = cell.publish_selected_array(Some(&self.name))?;
        drop(cell);
        if let Some(old) = old {
            old.release();
        }
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.binding) || cell.rmw_retirement.is_some() {
            return Err(super::VarError::DeletedNamespace);
        }
        cell.insert_member_entry(key);
        let id = cell.element_ids[key];
        cell.native_member_keys.ensure(id, key, original);
        Ok(())
    }
    pub(crate) fn capture_receiver(
        &self,
        element: Option<Vec<u8>>,
        create: bool,
    ) -> Result<Option<super::VariableReceiver>, super::VarError> {
        self.check_stored_role()
            .map_err(|_| super::VarError::NameProtocolUnavailable)?;
        self.capture_selected_receiver(element, create)
    }
    pub(crate) fn capture_selected_receiver(
        &self,
        element: Option<Vec<u8>>,
        create: bool,
    ) -> Result<Option<super::VariableReceiver>, super::VarError> {
        self.check_selected_target()
            .map_err(|_| super::VarError::NameProtocolUnavailable)?;
        let mut cell = self.contents.borrow_mut();
        let mut old = None;
        if cell.binding_id != Some(self.binding) {
            return Err(super::VarError::DeletedNamespace);
        }
        if let Some(error) = cell.rmw_retirement {
            return Err(error);
        }
        if element.is_some() && cell.array_materialisation_needed(Some(&self.name))? {
            if !create {
                return Ok(None);
            }
            old = cell.publish_selected_array(Some(&self.name))?;
        }
        drop(cell);
        if let Some(old) = old {
            old.release();
        }
        {
            let cell = self.contents.borrow();
            if cell.binding_id != Some(self.binding) || cell.rmw_retirement.is_some() {
                return Err(super::VarError::DeletedNamespace);
            }
        }
        Ok(Some(super::VariableReceiver::capture_contents(
            Rc::clone(&self.contents),
            element,
            Some(self.name.clone()),
        )))
    }
}
impl Drop for NativeScalarAliasEntry {
    fn drop(&mut self) {
        // Neutral fallback withdraws only an actual still-owned role.
        drop(self.withdraw_stored_role());
    }
}
impl VarTable {
    pub(crate) fn retain_native_scalar_alias(&mut self, name: &[u8]) -> Rc<NativeScalarAliasEntry> {
        self.prepare_native_name_cell(name, true);
        let slot = self.lookup_slot(name).expect("actual native alias target");
        self.retain_native_scalar_alias_at(slot)
    }
    pub(super) fn retain_native_scalar_alias_at(
        &mut self,
        slot: usize,
    ) -> Rc<NativeScalarAliasEntry> {
        self.select_native_scalar_alias_at(slot, true)
    }
    pub(crate) fn select_native_scalar_alias(&mut self, name: &[u8]) -> Rc<NativeScalarAliasEntry> {
        self.prepare_native_name_cell(name, true);
        let slot = self
            .lookup_slot(name)
            .expect("actual selected alias target");
        self.select_native_scalar_alias_at(slot, false)
    }
    pub(super) fn select_native_scalar_alias_at(
        &mut self,
        slot: usize,
        acquire: bool,
    ) -> Rc<NativeScalarAliasEntry> {
        let name = self.cells[slot].name.clone();
        let contents = Rc::clone(&self.cells[slot].contents);
        let target_reference = self
            .compiled_protocol
            .and_then(|protocol| match protocol.recipe() {
                tcl_syntax::naming::NativeCompiledVariableRecipe::C(version) => Some(
                    tcl_syntax::native_variable_name::NativeVariableNameProtocol::for_tcl_version(
                        version,
                    )
                    .alias_target_owns_reference(!self.declared_slots.contains(&slot)),
                ),
                tcl_syntax::naming::NativeCompiledVariableRecipe::Jim084 => None,
            })
            .unwrap_or(true); // Existing neutral software-holder contract.
        let binding = {
            let mut cell = contents.borrow_mut();
            let binding = cell.binding_id.expect("actual native target entry");
            if acquire && target_reference {
                cell.native_alias_refs = cell
                    .native_alias_refs
                    .checked_add(1)
                    .expect("native alias holds exhausted");
            }
            if !self.declared_slots.contains(&slot) && cell.rmw_shell_entry.is_none() {
                cell.rmw_shell_entry = Some(super::rmw::RootShellEntry::new(
                    &self.entry_order,
                    &self.native_keys,
                    &name,
                    binding,
                ));
            }
            binding
        };
        Rc::new(NativeScalarAliasEntry {
            contents,
            binding,
            target_reference,
            role_active: std::cell::Cell::new(acquire),
            role_unissued: std::cell::Cell::new(!acquire),
            name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::{self, Owned};

    #[test]
    fn alias_hold_retains_exact_key_through_unset_and_recreate() {
        let mut table = VarTable::default();
        let name = Owned::fresh(obj::new_string_bytes(b"k"));
        let value = Owned::fresh(obj::new_string_bytes(b"ONE"));
        table.store_scalar(b"k", value.as_ptr()).unwrap();
        table.retain_native_key(b"k", name.as_ptr(), false);
        let hold = table.retain_native_scalar_alias(b"k");
        let original = table.contents(b"k").unwrap().clone();
        let binding = original.borrow().binding_id;
        let borrowed = Rc::clone(&hold);
        assert_eq!(original.borrow().native_alias_refs, 1);
        assert!(table.remove(b"k"));
        assert_eq!(original.borrow().binding_id, binding);
        assert!(table.native_keys.borrow().contains_key(b"k".as_slice()));
        table.store_scalar(b"k", value.as_ptr()).unwrap();
        assert!(Rc::ptr_eq(table.contents(b"k").unwrap(), &original));
        assert_eq!(original.borrow().binding_id, binding);
        assert!(table.remove(b"k"));
        drop(hold);
        assert_eq!(original.borrow().native_alias_refs, 1);
        drop(borrowed);
        assert_eq!(original.borrow().native_alias_refs, 0);
        assert_eq!(original.borrow().binding_id, None);
        assert!(!table.native_keys.borrow().contains_key(b"k".as_slice()));
        // SAFETY: the test owns this live original header throughout.
        assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
    }

    #[test]
    fn retained_alias_materialises_only_its_original_undefined_root() {
        // naming.variable.original-traced-array-root-materialisation

        // Rust receiver correspondence only; native public values are independent.
        let mut table = VarTable::default();
        table.insert_link(
            b"x",
            super::super::Link {
                original_jim_target: None,
                native_scalar_entry: None,
                native_element_entry: None,
                array_identity: None,
                array_cell: None,
                home: super::super::VarHome::Frame(0),
                name: b"x".to_vec(),
                elem: None,
            },
        );
        table.mark_trace_shell(b"x", super::super::VarHome::Frame(0));
        let alias = table.retain_native_scalar_alias(b"x");
        let original = alias.identity();
        alias.ensure_array().unwrap();
        assert_eq!(table.binding_id(b"x"), Some(original));
        let receiver = alias
            .capture_receiver(Some(b"k".to_vec()), true)
            .unwrap()
            .unwrap();
        assert_eq!(receiver.binding_id(), Some(original));
        assert!(receiver.is_element());
        let value = Owned::fresh(obj::new_string_bytes(b"SCALAR"));
        table.store_scalar(b"other", value.as_ptr()).unwrap();
        let scalar = table.capture_receiver(b"other", None).unwrap();
        assert_eq!(scalar.ensure_array(), Err(super::super::VarError::IsScalar));
        assert_eq!(table.load_scalar(b"other"), Some(value.as_ptr()));
    }

    #[test]
    fn same_spelled_foreign_link_cannot_issue_an_undefined_root_marker() {
        // naming.variable.original-traced-array-root-materialisation

        // Rust receiver correspondence only; native public values are independent.
        let mut table = VarTable::default();
        table.insert_link(
            b"x",
            super::super::Link {
                original_jim_target: None,
                native_scalar_entry: None,
                native_element_entry: None,
                array_identity: None,
                array_cell: None,
                home: super::super::VarHome::Frame(1),
                name: b"x".to_vec(),
                elem: None,
            },
        );
        assert!(!table.mark_undefined_root(b"x", super::super::VarHome::Frame(0)));
        table.mark_trace_shell(b"x", super::super::VarHome::Frame(0));
        let receiver = table.capture_receiver(b"x", None).unwrap();
        assert_eq!(
            receiver.ensure_array(),
            Err(super::super::VarError::IsScalar)
        );
        let alias = table.retain_native_scalar_alias(b"x");
        assert_eq!(alias.ensure_array(), Err(super::super::VarError::IsScalar));
        assert!(
            matches!(table.cell(b"x").as_deref(), Some(super::super::Var::Link(link))
            if link.home == super::super::VarHome::Frame(1))
        );
    }

    #[test]
    fn namespace_table_retirement_does_not_remain_owned_by_alias() {
        let mut table = VarTable::default();
        let name = Owned::fresh(obj::new_string_bytes(b"k"));
        table.prepare_native_name_cell(b"k", true);
        table.retain_native_key(b"k", name.as_ptr(), false);
        let hold = table.retain_native_scalar_alias(b"k");
        drop(table);
        // SAFETY: table retirement releases its original key despite the cell hold.
        assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
        drop(hold);
    }
}
