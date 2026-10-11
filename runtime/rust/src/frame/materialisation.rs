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
