// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trace-shell withdrawal transfers existing owners out of storage borrows.

use super::{CellContents, FrameStack, RetiredVariableParts, Var, VarId, VarTable};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use tcl_syntax::value::ValueError;

/// Pure observation of the actual reached cell state. This acquires neither a
/// native variable role nor a strong cell owner that would delay withdrawal.
pub(crate) struct TraceShellState {
    contents: Weak<RefCell<CellContents>>,
    binding: Option<VarId>,
    array_epoch: u64,
    member: Option<(Vec<u8>, Option<VarId>)>,
}

impl TraceShellState {
    fn capture(contents: &Rc<RefCell<CellContents>>, key: Option<&[u8]>) -> Self {
        let cell = contents.borrow();
        Self {
            contents: Rc::downgrade(contents),
            binding: cell.binding_id,
            array_epoch: cell.rmw_array_epoch,
            member: key.map(|key| (key.to_vec(), cell.element_ids.get(key).copied())),
        }
    }

    pub(crate) fn check(&self, table: &VarTable, name: &[u8]) -> Result<(), ValueError> {
        let valid = table.contents(name).is_some_and(|contents| {
            if self.contents.strong_count() == 0
                || !Weak::ptr_eq(&self.contents, &Rc::downgrade(contents))
            {
                return false;
            }
            let cell = contents.borrow();
            cell.binding_id == self.binding
                && cell.rmw_array_epoch == self.array_epoch
                && self
                    .member
                    .as_ref()
                    .is_none_or(|(key, original)| cell.element_ids.get(key).copied() == *original)
        });
        if !valid {
            return Err(ValueError::CommandProtocolUnavailable(
                "original trace-shell cell",
            ));
        }
        table
            .contents(name)
            .expect("checked original trace shell")
            .borrow()
            .check_owned_value_role()
    }
}

impl VarTable {
    pub(crate) fn observe_trace_shell(
        &self,
        name: &[u8],
        key: Option<&[u8]>,
        binding: Option<VarId>,
    ) -> Option<TraceShellState> {
        let contents = self.contents(name)?;
        if contents.borrow().binding_id != binding {
            return None;
        }
        Some(TraceShellState::capture(contents, key))
    }

    /// The existing cleanup algorithm, with real value/key owners extracted.
    /// The caller releases the cell and its enclosing frame/namespace borrow
    /// before finishing the returned retirement under the original operation.
    pub(crate) fn prepare_trace_shell_cleanup(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        id: Option<VarId>,
    ) -> (Option<TraceShellState>, RetiredVariableParts) {
        let mut retired = RetiredVariableParts::default();
        let Some(contents) = self.contents(name) else {
            return (None, retired);
        };
        {
            let mut cell = contents.borrow_mut();
            if cell.binding_id != id {
                return (None, retired);
            }
            if let Some(key) = key {
                let defined =
                    matches!(cell.var.as_ref(), Some(Var::Array(map)) if map.contains_key(key));
                if !defined {
                    if !cell.element_operation_refs.contains_key(key)
                        && cell.rmw_refs == 0
                        && !cell.native_member_keys.has_aliases(key)
                    {
                        cell.member_order.remove(key);
                        retired.key(cell.native_member_keys.take_key(key));
                        cell.element_ids.remove(key);
                    } else {
                        cell.rmw_member_shells.insert(key.to_vec());
                    }
                }
            } else if cell.undefined_shell
                && matches!(cell.var.as_ref(), None | Some(Var::Link(_)))
                && !cell.namespace_declared
                && (cell.operation_refs != 0 || cell.rmw_refs != 0 || cell.native_alias_refs != 0)
            {
                retired.value(cell.take_value());
                cell.undefined_shell = false;
                if let Some(binding) = cell.binding_id {
                    if !self
                        .lookup_slot(name)
                        .is_some_and(|slot| self.declared_slots.contains(&slot))
                    {
                        cell.rmw_shell_entry = Some(super::rmw::RootShellEntry::new(
                            &self.entry_order,
                            &self.native_keys,
                            name,
                            binding,
                        ));
                    }
                }
            } else if cell.undefined_shell
                && matches!(cell.var.as_ref(), None | Some(Var::Link(_)))
                && !cell.namespace_declared
                && cell.operation_refs == 0
                && cell.rmw_refs == 0
                && Rc::strong_count(contents) == 1
            {
                retired.value(cell.take_value());
                cell.binding_id = None;
                cell.undefined_shell = false;
                retired.hash_key(self.take_native_entry(name));
            }
        }
        (
            self.contents(name)
                .map(|contents| TraceShellState::capture(contents, key)),
            retired,
        )
    }

    /// Existing neutral fixture port delegates the same withdrawal worker.
    #[cfg(test)]
    pub(crate) fn cleanup_trace_shell(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        id: Option<VarId>,
    ) {
        let (_, retired) = self.prepare_trace_shell_cleanup(name, key, id);
        let _: Result<(), core::convert::Infallible> = retired.finish(|| Ok(()));
    }
}

impl FrameStack {
    /// Withdraw from the already selected physical frame without selecting a
    /// container model or reaching conversion while its outer borrow is held.
    pub(crate) fn prepare_original_trace_shell_cleanup_at(
        &mut self,
        level: usize,
        name: &[u8],
        key: Option<&[u8]>,
        binding: Option<VarId>,
    ) -> Option<(Option<TraceShellState>, RetiredVariableParts)> {
        let index = self.index_of_level(level)?;
        Some(
            self.frames[index]
                .table
                .prepare_trace_shell_cleanup(name, key, binding),
        )
    }
}

#[cfg(test)]
mod tests;
