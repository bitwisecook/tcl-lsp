// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reached metadata retirement and adoption on one original physical cell.

use super::{CellContents, FrameLinkOrigin, RetiredVariableParts, Var};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use tcl_runtime_api::VarId;
use tcl_syntax::value::ValueError;

/// Incoming ownership is the genuine stored role already acquired by its
/// worker. The weak cell observation cannot keep a native role alive or adopt
/// another cell after a callback. No name is resolved by this continuation.
pub(crate) struct PreparedVariableReplacement {
    contents: Weak<RefCell<CellContents>>,
    binding: VarId,
    retired: RetiredVariableParts,
    incoming: Option<Var>,
}

#[derive(Clone)]
pub(crate) struct VariablePublicationObservation {
    contents: Weak<RefCell<CellContents>>,
    binding: VarId,
}
impl VariablePublicationObservation {
    pub(super) fn matches(&self, current: &Rc<RefCell<CellContents>>) -> bool {
        self.contents.strong_count() != 0 && Weak::ptr_eq(&self.contents, &Rc::downgrade(current))
    }
    pub(crate) fn matches_root(&self, table: &super::VarTable, name: &[u8]) -> bool {
        table
            .contents(name)
            .is_some_and(|contents| self.matches(contents))
    }
    pub(crate) fn check_current(&self) -> Result<(), ValueError> {
        let Some(contents) = self.contents.upgrade() else {
            return Err(ValueError::CommandProtocolUnavailable(
                "original variable replacement cell",
            ));
        };
        let cell = contents.borrow();
        // Reentrant metadata retirement may publish a pending allocation in
        // this same cell. It still cannot issue an ordinary stored-value role.
        cell.check_owned_value_role()?;
        if cell.binding_id == Some(self.binding) && cell.rmw_retirement.is_none() {
            Ok(())
        } else {
            Err(ValueError::CommandProtocolUnavailable(
                "original variable replacement cell",
            ))
        }
    }

    pub(crate) fn check_array_current(&self, epoch: u64) -> Result<(), ValueError> {
        self.check_current()?;
        let contents = self.contents.upgrade().expect("checked original cell");
        let cell = contents.borrow();
        if cell.rmw_array_epoch == epoch && matches!(cell.var.as_ref(), Some(Var::Array(_))) {
            Ok(())
        } else {
            Err(ValueError::CommandProtocolUnavailable(
                "original materialised array",
            ))
        }
    }

    pub(crate) fn capture_array_receiver(
        &self,
        epoch: u64,
        member: VarId,
        element: Vec<u8>,
        root: Vec<u8>,
        retained_array: bool,
    ) -> Result<super::VariableReceiver, ValueError> {
        self.check_array_current(epoch)?;
        self.check_member_current(&element, member)?;
        let contents = self.contents.upgrade().expect("checked original array");
        // The old worker's genuine RMW role is taken only here, after the
        // selected retirement/adoption. The observation took no native role.
        Ok(if retained_array {
            super::RetainedArrayCell {
                contents,
                id: self.binding,
                epoch,
            }
            .capture_receiver(element)
        } else {
            super::VariableReceiver::capture_contents(contents, Some(element), Some(root))
        })
    }
    pub(crate) fn capture_array_cell(
        &self,
        epoch: u64,
        element: &[u8],
        member: VarId,
    ) -> Result<super::RetainedArrayCell, ValueError> {
        self.check_array_current(epoch)?;
        self.check_member_current(element, member)?;
        // A Rust allocation lease is not a variable-entry alias reference.
        // The existing link installer takes that role at its selected stage.
        Ok(super::RetainedArrayCell {
            contents: self.contents.upgrade().expect("checked original array"),
            id: self.binding,
            epoch,
        })
    }
    pub(crate) fn member_identity(&self, element: &[u8]) -> Option<VarId> {
        let contents = self.contents.upgrade()?;
        let cell = contents.borrow();
        (cell.binding_id == Some(self.binding))
            .then(|| cell.element_ids.get(element).copied())
            .flatten()
    }
    pub(crate) fn check_member_current(
        &self,
        element: &[u8],
        member: VarId,
    ) -> Result<(), ValueError> {
        self.check_current()?;
        if self.member_identity(element) == Some(member) {
            Ok(())
        } else {
            Err(ValueError::CommandProtocolUnavailable(
                "original materialised member",
            ))
        }
    }
}

/// A non-owning observation of an actual C alias issuer and its source cell.
/// It cannot postpone the original alias role's retirement or resolve a name.
pub(crate) struct OriginalVariableLinkObservation {
    contents: Weak<RefCell<CellContents>>,
    binding: VarId,
    scalar: Option<Weak<super::NativeScalarAliasEntry>>,
    element: Option<Weak<super::NativeElementAliasEntry>>,
    home: super::VarHome,
    name: Vec<u8>,
    key: Option<Vec<u8>>,
}
impl OriginalVariableLinkObservation {
    fn capture(contents: &Rc<RefCell<CellContents>>) -> Result<Self, super::VarError> {
        let cell = contents.borrow();
        let Some(Var::Link(link)) = cell.var.as_ref() else {
            return Err(super::VarError::NameProtocolUnavailable);
        };
        if link.native_scalar_entry.is_none() && link.native_element_entry.is_none() {
            // An authored byte-only link supplies no actual C alias issuer.
            return Err(super::VarError::NameProtocolUnavailable);
        }
        Ok(Self {
            contents: Rc::downgrade(contents),
            binding: cell
                .binding_id
                .ok_or(super::VarError::NameProtocolUnavailable)?,
            scalar: link.native_scalar_entry.as_ref().map(Rc::downgrade),
            element: link.native_element_entry.as_ref().map(Rc::downgrade),
            home: link.home,
            name: link.name.clone(),
            key: link.elem.clone(),
        })
    }
    pub(crate) fn check_current(&self) -> Result<(), ValueError> {
        let valid = self.contents.upgrade().is_some_and(|contents| {
            let cell = contents.borrow();
            let Some(Var::Link(link)) = cell.var.as_ref() else {
                return false;
            };
            cell.binding_id == Some(self.binding)
                && cell.rmw_retirement.is_none()
                && link.home == self.home
                && link.name == self.name
                && link.elem == self.key
                && match (&self.scalar, &link.native_scalar_entry) {
                    (Some(original), Some(current)) => {
                        original.strong_count() != 0
                            && Weak::ptr_eq(original, &Rc::downgrade(current))
                    }
                    (None, None) => true,
                    _ => false,
                }
                && match (&self.element, &link.native_element_entry) {
                    (Some(original), Some(current)) => {
                        original.strong_count() != 0
                            && Weak::ptr_eq(original, &Rc::downgrade(current))
                    }
                    (None, None) => true,
                    _ => false,
                }
        });
        if valid {
            Ok(())
        } else {
            Err(ValueError::CommandProtocolUnavailable(
                "original C alias selection",
            ))
        }
    }
}
impl super::VarTable {
    pub(crate) fn observe_original_c_link(
        &self,
        name: &[u8],
    ) -> Result<OriginalVariableLinkObservation, super::VarError> {
        OriginalVariableLinkObservation::capture(
            self.contents(name)
                .ok_or(super::VarError::NameProtocolUnavailable)?,
        )
    }
}

impl PreparedVariableReplacement {
    pub(super) fn capture(
        contents: &Rc<RefCell<CellContents>>,
        incoming: Var,
        recipe: Option<tcl_core_types::NativeHashRecipe>,
    ) -> Self {
        let mut cell = contents.borrow_mut();
        cell.undefined_shell = false;
        cell.undefined_root_home = None;
        cell.rmw_shell_entry = None;
        cell.link_origin = FrameLinkOrigin::Ordinary;
        let binding = *cell.binding_id.get_or_insert_with(super::fresh_var_id);
        cell.element_ids.clear();
        let mut retired = RetiredVariableParts::default();
        retired.elements(std::mem::take(&mut cell.native_member_keys));
        cell.rmw_member_shells.clear();
        cell.element_operation_refs.clear();
        cell.array_search_epoch = cell.array_search_epoch.wrapping_add(1);
        retired.searches(&mut cell.array_searches);
        cell.member_order = tcl_core_types::NativeEntryLedger::default();
        cell.member_order.select_recipe(recipe);
        Self {
            contents: Rc::downgrade(contents),
            binding,
            retired,
            incoming: Some(incoming),
        }
    }

    pub(crate) fn observation(&self) -> VariablePublicationObservation {
        VariablePublicationObservation {
            contents: self.contents.clone(),
            binding: self.binding,
        }
    }
    fn check<E: From<ValueError>>(&self, current: &impl Fn() -> Result<(), E>) -> Result<(), E> {
        current()?;
        self.observation().check_current().map_err(Into::into)
    }

    /// Retire the old member/search roles before adopting incoming contents,
    /// with no cell/table/frame/namespace borrow held across any decrement.
    /// Return the actual displaced value for the selected post-adoption stage.
    pub(crate) fn finish<E: From<ValueError>>(
        mut self,
        current: impl Fn() -> Result<(), E>,
    ) -> Result<RetiredVariableParts, E> {
        let old = std::mem::take(&mut self.retired);
        let mut reached = old.finish(|| self.check(&current));
        let mut displaced = RetiredVariableParts::default();
        if reached.is_ok() {
            reached = self.check(&current);
        }
        if reached.is_ok() {
            // The preceding same-cell check and this borrow contain no callback.
            let contents = self.contents.upgrade().expect("checked original cell");
            let mut cell = contents.borrow_mut();
            let incoming = self
                .incoming
                .take()
                .expect("one genuine incoming stored role");
            if let Var::Array(elements) = &incoming {
                for key in elements.keys() {
                    cell.member_order.insert(key);
                    let id = super::fresh_var_id();
                    cell.element_ids.insert(key.clone(), id);
                    cell.native_member_keys.ensure(id, key, None);
                }
                cell.element_ids.extend(
                    elements
                        .keys()
                        .cloned()
                        .map(|key| (key, super::fresh_var_id())),
                );
            }
            displaced.value(cell.replace_value(incoming));
        }
        // A declined adoption still releases the genuine incoming owning role.
        // It is mandatory cleanup, never a successful publication or fresh entry.
        if let Some(incoming) = self.incoming.take() {
            incoming.release();
        }
        let after_cleanup = current();
        reached?;
        after_cleanup?;
        Ok(displaced)
    }
}

impl Drop for PreparedVariableReplacement {
    fn drop(&mut self) {
        if let Some(incoming) = self.incoming.take() {
            incoming.release();
        }
    }
}

/// The selected array publication and its later member-entry stage. The key
/// lease protects allocation only; its table reference is acquired at adoption.
pub(crate) struct PreparedArrayMember {
    contents: Weak<RefCell<CellContents>>,
    binding: VarId,
    epoch: u64,
    replacement: Option<PreparedVariableReplacement>,
    retired_root: RetiredVariableParts,
    element: Option<Vec<u8>>,
    original_key: Option<crate::obj::NativeObjectLifetime>,
    mark_shell: bool,
}

impl PreparedArrayMember {
    pub(crate) fn array_epoch(&self) -> u64 {
        self.epoch
    }
    fn capture(
        contents: &Rc<RefCell<CellContents>>,
        replacement: Option<PreparedVariableReplacement>,
        retired_root: RetiredVariableParts,
        element: Option<&[u8]>,
        original: Option<*mut crate::obj::TclObj>,
        mark_shell: bool,
    ) -> Self {
        let mut cell = contents.borrow_mut();
        let binding = *cell.binding_id.get_or_insert_with(super::fresh_var_id);
        Self {
            contents: Rc::downgrade(contents),
            binding,
            epoch: cell.rmw_array_epoch,
            replacement,
            retired_root,
            element: element.map(<[u8]>::to_vec),
            original_key: original.map(crate::obj::NativeObjectLifetime::retain),
            mark_shell,
        }
    }

    pub(super) fn selected(
        contents: &Rc<RefCell<CellContents>>,
        root: Option<&[u8]>,
        element: Option<&[u8]>,
        original: Option<*mut crate::obj::TclObj>,
    ) -> Result<Self, super::JimLinkStoreError> {
        let mut retired = RetiredVariableParts::default();
        {
            let mut cell = contents.borrow_mut();
            if let Some(error) = cell.rmw_retirement {
                return Err(error.into());
            }
            cell.check_owned_value_role()?;
            // This selected alias worker publishes the array before releasing
            // its old undefined-link role. Table replacement has its own lane.
            retired.value(cell.publish_selected_array(root)?);
        }
        Ok(Self::capture(
            contents, None, retired, element, original, false,
        ))
    }

    pub(super) fn table(
        table: &mut super::VarTable,
        name: &[u8],
        element: Option<&[u8]>,
        original: Option<*mut crate::obj::TclObj>,
    ) -> Result<Self, super::JimLinkStoreError> {
        if table.dictionary_variables() {
            return Err(super::VarError::NameProtocolUnavailable.into());
        }
        let slot = table.slot_for(name);
        let contents = Rc::clone(&table.cells[slot].contents);
        let needed = {
            let cell = contents.borrow();
            cell.check_owned_value_role()?;
            cell.array_materialisation_needed(Some(name))?
        };
        let replacement = needed.then(|| {
            if !table.declared_slots.contains(&slot) {
                table.entry_order.borrow_mut().insert(name);
            }
            PreparedVariableReplacement::capture(
                &contents,
                Var::Array(Default::default()),
                table.hash_recipe.get(),
            )
        });
        Ok(Self::capture(
            &contents,
            replacement,
            RetiredVariableParts::default(),
            element,
            original,
            true,
        ))
    }

    fn check<E: From<ValueError>>(
        &self,
        current: &impl Fn() -> Result<(), E>,
        array_required: bool,
        refuse: &impl Fn(ValueError) -> E,
    ) -> Result<(), E> {
        current()?;
        let physical = self
            .contents
            .upgrade()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original array member preparation",
            ))
            .and_then(|contents| {
                let cell = contents.borrow();
                cell.check_owned_value_role()?;
                if cell.binding_id == Some(self.binding)
                    && cell.rmw_array_epoch == self.epoch
                    && cell.rmw_retirement.is_none()
                    && (!array_required || matches!(cell.var.as_ref(), Some(Var::Array(_))))
                {
                    Ok(())
                } else {
                    Err(ValueError::CommandProtocolUnavailable(
                        "original array member preparation",
                    ))
                }
            });
        physical.map_err(refuse)
    }

    pub(crate) fn observation(&self) -> VariablePublicationObservation {
        VariablePublicationObservation {
            contents: self.contents.clone(),
            binding: self.binding,
        }
    }

    pub(crate) fn finish<E: From<ValueError>>(
        mut self,
        current: impl Fn() -> Result<(), E>,
        refuse: impl Fn(ValueError) -> E,
    ) -> Result<(), E> {
        let root = std::mem::take(&mut self.retired_root);
        let mut reached = root.finish(|| self.check(&current, false, &refuse));
        if let Some(replacement) = self.replacement.take() {
            // The incoming array owns no object references. On refusal the
            // replacement still performs its mandatory old/incoming cleanup.
            let published = replacement.finish(|| {
                reached.as_ref().map_err(|_| {
                    refuse(ValueError::CommandProtocolUnavailable(
                        "refused original array preparation",
                    ))
                })?;
                self.check(&current, false, &refuse)
            });
            match published {
                Ok(old) => {
                    if let Err(error) = old.finish(|| self.check(&current, true, &refuse)) {
                        if reached.is_ok() {
                            reached = Err(error);
                        }
                    }
                }
                Err(error) => {
                    if reached.is_ok() {
                        reached = Err(error);
                    }
                }
            }
        }
        if reached.is_ok() {
            reached = self.check(&current, true, &refuse);
        }
        if reached.is_ok() {
            if let Some(element) = &self.element {
                if let Some(original) = &self.original_key {
                    reached = crate::obj::check_native_liveness(original.as_ptr()).map_err(&refuse);
                }
                if reached.is_ok() {
                    // C85+ TclLookupArrayElement creates the actual entry/key before
                    // DeleteSearches. Its ArrayElement bookkeeping follows the
                    // reached releases. Generic insertion has a separate order.
                    let (member, searches) = {
                        let contents = self.contents.upgrade().expect("checked original array");
                        let mut cell = contents.borrow_mut();
                        let new = !cell.member_order.contains_key(element);
                        cell.adopt_member_entry(element);
                        let member = cell.element_ids[element.as_slice()];
                        cell.native_member_keys.ensure(
                            member,
                            element,
                            self.original_key
                                .as_ref()
                                .map(crate::obj::NativeObjectLifetime::as_ptr),
                        );
                        let mut old = RetiredVariableParts::default();
                        if new {
                            cell.array_search_epoch = cell.array_search_epoch.wrapping_add(1);
                            old.searches(&mut cell.array_searches);
                        }
                        (member, old)
                    };
                    let check_member = || {
                        self.check(&current, true, &refuse)?;
                        self.observation()
                            .check_member_current(element, member)
                            .map_err(&refuse)
                    };
                    reached = searches.finish(|| check_member());
                    if reached.is_ok() {
                        reached = check_member();
                    }
                    if reached.is_ok() && self.mark_shell {
                        let contents = self.contents.upgrade().expect("checked original array");
                        let mut cell = contents.borrow_mut();
                        if !matches!(cell.var.as_ref(), Some(Var::Array(values)) if values.contains_key(element))
                        {
                            cell.rmw_member_shells.insert(element.clone());
                        }
                    }
                }
            }
        }
        drop(self.original_key.take());
        let after_cleanup = self.check(&current, reached.is_ok(), &refuse);
        reached?;
        after_cleanup
    }
}
