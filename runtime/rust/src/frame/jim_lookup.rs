// SPDX-License-Identifier: AGPL-3.0-or-later
//! Non-owning Jim variable-cell receipts and actual frame incarnations.

use super::{CellContents, FrameStack, Link, Var, VarError, VarHome, VarTable};
use crate::obj::{self, TclObj};
use std::{cell::RefCell, rc::Weak};

/// Original VarVal identity without a cell, value, table or activation owner.
#[derive(Clone)]
pub(crate) struct WeakJimVariableCell(Weak<RefCell<CellContents>>);

pub(crate) enum JimVariableRead {
    Scalar(*mut TclObj),
    Link(Link),
}
impl WeakJimVariableCell {
    pub(crate) fn read(&self) -> Option<JimVariableRead> {
        let cell = self.0.upgrade()?;
        let cell = cell.borrow();
        match cell.var.as_ref()? {
            Var::Scalar(value) => Some(JimVariableRead::Scalar(*value)),
            Var::Link(link) => Some(JimVariableRead::Link(link.clone())),
            Var::Array(_) => None,
        }
    }
    pub(crate) fn store(&self, value: *mut TclObj) -> Result<Option<Link>, VarError> {
        let cell = self.0.upgrade().ok_or(VarError::NameProtocolUnavailable)?;
        let mut cell = cell.borrow_mut();
        match cell.var.as_mut() {
            Some(Var::Scalar(old)) => {
                // SAFETY: this original live VarVal owns one value reference.
                unsafe {
                    obj::incr_ref_count(value);
                    obj::decr_ref_count(*old);
                }
                *old = value;
                Ok(None)
            }
            Some(Var::Link(link)) => Ok(Some(link.clone())),
            Some(Var::Array(_)) => Err(VarError::IsArray),
            None => Err(VarError::NameProtocolUnavailable),
        }
    }
}
impl VarTable {
    pub(crate) fn weak_jim_cell(&self, name: &[u8]) -> Option<WeakJimVariableCell> {
        let cell = self.contents(name)?;
        cell.borrow().var.as_ref()?;
        Some(WeakJimVariableCell(std::rc::Rc::downgrade(cell)))
    }
    /// The native table owns the actual original key object on its first birth.
    pub(crate) fn retain_jim_key(&self, name: &[u8], original: *mut TclObj) {
        self.retain_native_key(name, original, true);
    }
}
impl FrameStack {
    pub(crate) fn native_jim_frame_id(&self, global: bool) -> u64 {
        let index = if global {
            0
        } else {
            self.current_frame_index()
        };
        self.frames[index].jim_id
    }
    /// Successful root unset changes the actual selected frame's incarnation.
    /// Dictionary member deletion does not call this door.
    pub(crate) fn invalidate_jim_variable_frame(&mut self, home: VarHome) {
        if self.variable_lookup_policy != tcl_dialect::VariableLookupPolicy::Jim {
            return;
        }
        let index = match home {
            VarHome::Frame(level) => self.index_of_level(level),
            VarHome::Namespace(_) => Some(0),
        };
        if let Some(index) = index {
            self.frames[index].jim_id = self.next_jim_frame_id;
            self.next_jim_frame_id = self
                .next_jim_frame_id
                .checked_add(1)
                .expect("Jim frame identity exhausted");
        }
    }
}
