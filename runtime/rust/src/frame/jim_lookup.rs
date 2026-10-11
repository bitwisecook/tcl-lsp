// SPDX-License-Identifier: AGPL-3.0-or-later
//! Non-owning Jim variable-cell receipts and actual frame incarnations.

use super::{CellContents, FrameStack, Link, Var, VarError, VarHome, VarTable};
use crate::obj::{self, TclObj};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use tcl_syntax::value::ValueError;

/// Original VarVal identity without a cell, value, table or activation owner.
#[derive(Clone)]
pub(crate) struct WeakJimVariableCell {
    contents: Weak<RefCell<CellContents>>,
    ledger: Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
    name: Vec<u8>,
    home: Option<VarHome>,
}

pub(crate) enum JimVariableRead {
    Scalar(*mut TclObj),
    Link(Link),
}
/// The original binding and scalar header; no strong cell or native hold.
#[derive(Clone)]
pub(crate) struct CapturedJimDictionaryCell {
    cell: Weak<RefCell<CellContents>>,
    binding: Option<tcl_runtime_api::VarId>,
    epoch: u64,
    root: Rc<Cell<*mut TclObj>>,
}
impl CapturedJimDictionaryCell {
    pub(crate) fn root(&self) -> *mut TclObj {
        self.root.get()
    }
    pub(crate) fn check(&self) -> Result<(), ValueError> {
        let valid = self.cell.upgrade().is_some_and(|cell| {
            let cell = cell.borrow();
            cell.binding_id == self.binding && cell.rmw_array_epoch == self.epoch
                && cell.rmw_retirement.is_none()
                && cell.check_owned_value_role().is_ok()
                && matches!(cell.var.as_ref(), Some(Var::Scalar(value)) if *value == self.root.get())
        });
        if !valid {
            return Err(ValueError::CommandProtocolUnavailable(
                "captured original Jim dictionary cell",
            ));
        }
        obj::check_native_liveness(self.root.get())
    }
    pub(crate) fn publish(&self, value: *mut TclObj) -> Result<(), ValueError> {
        self.check()?;
        obj::check_native_liveness(value)?;
        let cell = self
            .cell
            .upgrade()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "retired original Jim dictionary cell",
            ))?;
        let retired = {
            let mut contents = cell.borrow_mut();
            if contents.constant {
                return Err(ValueError::CommandProtocolUnavailable(
                    "constant original Jim dictionary cell",
                ));
            }
            let retired = contents.publish_dictionary_root(value);
            // The same authorised cell adopts its new owner before old release.
            self.root.set(value);
            retired
        };
        drop(cell);
        drop(retired);
        self.check()
    }
}

impl WeakJimVariableCell {
    pub(super) fn selected(
        contents: Weak<RefCell<CellContents>>,
        ledger: Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
        name: Vec<u8>,
        home: VarHome,
    ) -> Self {
        Self {
            contents,
            ledger,
            name,
            home: Some(home),
        }
    }
    pub(crate) fn with_home(mut self, home: VarHome) -> Self {
        self.home = Some(home);
        self
    }
    pub(crate) fn home(&self) -> Option<VarHome> {
        self.home
    }
    pub(crate) fn prepare_alias_local(
        &self,
    ) -> Result<Option<super::PreparedJimAliasCell>, ValueError> {
        let cell = self
            .contents
            .upgrade()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim alias local cell",
            ))?;
        super::PreparedJimAliasCell::capture(&cell, self.ledger.clone(), self.name.clone())
    }

    pub(crate) fn capture_dictionary_cell(&self) -> Result<CapturedJimDictionaryCell, ValueError> {
        let owner = self
            .contents
            .upgrade()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "retired original Jim dictionary binding",
            ))?;
        let contents = owner.borrow();
        contents.check_owned_value_role()?;
        let Some(Var::Scalar(root)) = contents.var.as_ref() else {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim dictionary scalar binding",
            ));
        };
        let captured = CapturedJimDictionaryCell {
            cell: self.contents.clone(),
            binding: contents.binding_id,
            epoch: contents.rmw_array_epoch,
            root: Rc::new(Cell::new(*root)),
        };
        drop(contents);
        captured.check()?;
        Ok(captured)
    }
    /// Observe the actual cached scalar or retained link around primary release.
    /// This never resolves a spelling or grants a dictionary worker purpose.
    pub(crate) fn original_stage_check(
        &self,
    ) -> Result<crate::dict::NativeDictionaryStageCheck, ValueError> {
        let owner = self
            .contents
            .upgrade()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "retired original Jim variable binding",
            ))?;
        let contents = owner.borrow();
        contents.check_owned_value_role()?;
        let binding = contents.binding_id;
        let epoch = contents.rmw_array_epoch;
        let scalar = match contents.var.as_ref() {
            Some(Var::Scalar(value)) => Some(*value),
            _ => None,
        };
        if let Some(Var::Link(link)) = contents.var.as_ref() {
            link.check_stored_roles()?;
        }
        let original_link = match contents.var.as_ref() {
            Some(Var::Link(link)) => link.original_jim_target.as_ref().map(Rc::downgrade),
            _ => None,
        };
        let compatibility_link = match contents.var.as_ref() {
            Some(Var::Link(link)) if link.original_jim_target.is_none() => Some(link.clone()),
            _ => None,
        };
        if scalar.is_none() && original_link.is_none() && compatibility_link.is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim retained variable target",
            ));
        }
        let cell = self.contents.clone();
        drop(contents);
        Ok(Rc::new(move || {
            let valid = cell.upgrade().is_some_and(|cell| {
                let contents = cell.borrow();
                if contents.binding_id != binding
                    || contents.rmw_array_epoch != epoch
                    || contents.rmw_retirement.is_some()
                    || contents.check_owned_value_role().is_err()
                {
                    return false;
                }
                match contents.var.as_ref() {
                    Some(Var::Scalar(value)) => scalar == Some(*value),
                    Some(Var::Link(current)) if current.check_stored_roles().is_ok() => {
                        match original_link.as_ref() {
                            Some(original) => {
                                current.original_jim_target.as_ref().is_some_and(|target| {
                                    Weak::ptr_eq(original, &Rc::downgrade(target))
                                })
                            }
                            None => compatibility_link.as_ref() == Some(current),
                        }
                    }
                    _ => false,
                }
            });
            if valid {
                Ok(())
            } else {
                Err(ValueError::CommandProtocolUnavailable(
                    "captured original Jim variable target",
                ))
            }
        }))
    }

    pub(crate) fn read_checked(&self) -> Result<Option<JimVariableRead>, ValueError> {
        let Some(cell) = self.contents.upgrade() else {
            return Ok(None);
        };
        cell.borrow().check_owned_value_role()?;
        Ok(self.read())
    }
    fn read(&self) -> Option<JimVariableRead> {
        let cell = self.contents.upgrade()?;
        let cell = cell.borrow();
        if cell.check_owned_value_role().is_err() {
            return None;
        }
        match cell.var.as_ref()? {
            Var::Scalar(value) => Some(JimVariableRead::Scalar(*value)),
            Var::Link(link) => Some(JimVariableRead::Link(link.clone())),
            Var::PendingScalar(_) | Var::Array(_) => None,
        }
    }
    pub(crate) fn store(&self, value: *mut TclObj) -> Result<Option<Link>, VarError> {
        let cell = self
            .contents
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        let mut cell = cell.borrow_mut();
        match cell.var.as_mut() {
            Some(Var::Scalar(_)) => {
                // SAFETY: the actual VarVal takes its new owning reference first.
                unsafe { obj::incr_ref_count(value) };
                let retired = cell.replace_value(Var::Scalar(value));
                drop(cell);
                // SAFETY: the replaced hold retires outside the selected cell borrow.
                if let Some(retired) = retired {
                    retired.release();
                }
                Ok(None)
            }
            Some(Var::PendingScalar(_)) => Err(VarError::NameProtocolUnavailable),
            Some(Var::Link(link)) => Ok(Some(link.clone())),
            Some(Var::Array(_)) => Err(VarError::IsArray),
            None => Err(VarError::NameProtocolUnavailable),
        }
    }
}
impl VarTable {
    /// Jim_UnsetVariable removes a frame's primary table, never its static fallback.
    pub(crate) fn has_jim_primary(&self, name: &[u8]) -> bool {
        self.slots.contains_key(name)
    }

    pub(crate) fn weak_jim_cell(&self, name: &[u8]) -> Option<WeakJimVariableCell> {
        let cell = self.contents(name)?;
        cell.borrow().var.as_ref()?;
        Some(WeakJimVariableCell {
            contents: Rc::downgrade(cell),
            ledger: self.original_table_ledger(),
            name: name.to_vec(),
            home: None,
        })
    }
    /// The native table owns the actual original key object on its first birth.
    pub(crate) fn retain_jim_key(&self, name: &[u8], original: *mut TclObj) {
        self.retain_native_key(name, original, true);
    }
}
impl FrameStack {
    pub(crate) fn original_jim_frame_incarnation(&self, global: bool) -> Weak<()> {
        let index = if global {
            0
        } else {
            self.current_frame_index()
        };
        Rc::downgrade(&self.frames[index].jim_link_birth)
    }
    pub(crate) fn owns_original_jim_frame_incarnation(&self, original: &Weak<()>) -> bool {
        original.strong_count() != 0
            && self
                .frames
                .iter()
                .any(|frame| Weak::ptr_eq(&Rc::downgrade(&frame.jim_link_birth), original))
    }

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
