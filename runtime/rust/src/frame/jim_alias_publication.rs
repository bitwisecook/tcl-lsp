// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim link detachment and old scalar retirement without new native holds.

use super::{
    CellContents, FrameLinkOrigin, Link, OriginalJimLinkTarget, RetiredVariableParts, Var, VarTable,
};
use crate::obj::{NativeObjectLifetime, Owned, TclObj};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use tcl_runtime_api::VarId;
use tcl_syntax::value::ValueError;

/// The cell still exposes its original pointer while its genuine +1 retires.
/// This ticket owns allocation safety only; cell displacement cannot retire
/// the already extracted reference again or make it a semantic value owner.
pub(super) struct JimScalarRetirement {
    original: NativeObjectLifetime,
    active: Cell<bool>,
}
impl JimScalarRetirement {
    pub(super) fn displace(&self, old: &Option<Var>) -> bool {
        self.active.set(false);
        matches!(old, Some(Var::Scalar(value)) if *value == self.original.as_ptr())
    }
}

enum OriginalShape {
    Missing,
    Link(Weak<OriginalJimLinkTarget>),
    Scalar(*mut TclObj),
    Retiring(Rc<JimScalarRetirement>),
}

/// A shallow, originally selected local. No alias is followed and no cell,
/// activation, table or native value reference is added by this observation.
pub(crate) struct PreparedJimAliasCell {
    contents: Weak<RefCell<CellContents>>,
    ledger: Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
    name: Vec<u8>,
    binding: Cell<Option<VarId>>,
    epoch: u64,
    shape: RefCell<OriginalShape>,
}
impl PreparedJimAliasCell {
    pub(super) fn capture(
        contents: &Rc<RefCell<CellContents>>,
        ledger: Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
        name: Vec<u8>,
    ) -> Result<Option<Self>, ValueError> {
        Self::capture_using(contents, ledger, name, false)
    }
    pub(super) fn capture_scalar(
        contents: &Rc<RefCell<CellContents>>,
        ledger: Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
        name: Vec<u8>,
    ) -> Result<Option<Self>, ValueError> {
        Self::capture_using(contents, ledger, name, true)
    }
    fn capture_using(
        contents: &Rc<RefCell<CellContents>>,
        ledger: Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
        name: Vec<u8>,
        scalar: bool,
    ) -> Result<Option<Self>, ValueError> {
        let cell = contents.borrow();
        cell.check_owned_value_role()?;
        let shape = match cell.var.as_ref() {
            Some(Var::Scalar(original)) if scalar => OriginalShape::Scalar(*original),
            Some(Var::Link(link)) if !scalar => {
                let original = link.original_jim_target.as_ref().ok_or(
                    ValueError::CommandProtocolUnavailable("Jim local link original stored role"),
                )?;
                original.original_name()?;
                OriginalShape::Link(Rc::downgrade(original))
            }
            Some(_) => return Ok(None),
            None if !scalar => OriginalShape::Missing,
            None => return Ok(None),
        };
        Ok(Some(Self {
            contents: Rc::downgrade(contents),
            ledger,
            name,
            binding: Cell::new(cell.binding_id),
            epoch: cell.rmw_array_epoch,
            shape: RefCell::new(shape),
        }))
    }
    pub(crate) fn matches_table(&self, table: &VarTable) -> bool {
        self.ledger.strong_count() != 0
            && Weak::ptr_eq(&self.ledger, &table.original_table_ledger())
            && table.contents(&self.name).is_some_and(|current| {
                self.contents.strong_count() != 0
                    && Weak::ptr_eq(&self.contents, &Rc::downgrade(current))
            })
    }
    pub(crate) fn name(&self) -> &[u8] {
        &self.name
    }
    pub(crate) fn was_missing(&self) -> bool {
        matches!(*self.shape.borrow(), OriginalShape::Missing)
    }
    pub(crate) fn weak_cell(&self, home: super::VarHome) -> super::WeakJimVariableCell {
        super::WeakJimVariableCell::selected(
            self.contents.clone(),
            self.ledger.clone(),
            self.name.clone(),
            home,
        )
    }
    pub(crate) fn check(&self) -> Result<(), ValueError> {
        let contents = self
            .contents
            .upgrade()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim local cell",
            ))?;
        let cell = contents.borrow();
        let shape = self.shape.borrow();
        let same = self.ledger.strong_count() != 0
            && cell.binding_id == self.binding.get()
            && cell.rmw_array_epoch == self.epoch
            && cell.rmw_retirement.is_none()
            && match (&*shape, cell.var.as_ref()) {
                (OriginalShape::Missing, None) => cell.jim_scalar_retirement.is_none(),
                (OriginalShape::Link(original), Some(Var::Link(current))) => {
                    current.original_jim_target.as_ref().is_some_and(|current| {
                        original.strong_count() != 0
                            && Weak::ptr_eq(original, &Rc::downgrade(current))
                            && current.original_name().is_ok()
                    })
                }
                (OriginalShape::Scalar(original), Some(Var::Scalar(current))) => {
                    *original == *current && cell.jim_scalar_retirement.is_none()
                }
                (OriginalShape::Retiring(original), Some(Var::Scalar(current))) => {
                    original.active.get()
                        && *current == original.original.as_ptr()
                        && cell
                            .jim_scalar_retirement
                            .as_ref()
                            .is_some_and(|current| Rc::ptr_eq(current, original))
                }
                _ => false,
            };
        if same {
            Ok(())
        } else {
            Err(ValueError::CommandProtocolUnavailable(
                "original Jim local publication cell",
            ))
        }
    }
    /// Clear only the genuine link-frame marker, transferring its existing
    /// stored target-name +1 into the same scalar cell before later getters.
    pub(crate) fn detach(&self) -> Result<RetiredVariableParts, ValueError> {
        self.check()?;
        if matches!(*self.shape.borrow(), OriginalShape::Missing) {
            return Ok(RetiredVariableParts::default());
        }
        let contents = self.contents.upgrade().expect("checked original Jim local");
        let mut cell = contents.borrow_mut();
        let Some(Var::Link(link)) = cell.var.as_ref() else {
            unreachable!("checked original link")
        };
        let name = link
            .original_jim_target
            .as_ref()
            .expect("checked original stored Jim role")
            .take_stored_name()?;
        let original = name.into_raw();
        let previous = cell.replace_value(Var::Scalar(original));
        cell.link_origin = FrameLinkOrigin::Ordinary;
        *self.shape.borrow_mut() = OriginalShape::Scalar(original);
        let mut retired = RetiredVariableParts::default();
        retired.value(previous);
        Ok(retired)
    }
    /// The caller has acquired the genuine incoming cell +1 independently of
    /// its target-name temporary. Keep the old pointer observable while its
    /// existing owning reference leaves in this retirement carrier.
    pub(crate) fn begin_assignment(&self) -> Result<RetiredVariableParts, ValueError> {
        self.check()?;
        let original = match *self.shape.borrow() {
            OriginalShape::Missing => return Ok(RetiredVariableParts::default()),
            OriginalShape::Scalar(original) => original,
            _ => {
                return Err(ValueError::CommandProtocolUnavailable(
                    "Jim detached scalar assignment",
                ))
            }
        };
        let ticket = Rc::new(JimScalarRetirement {
            original: NativeObjectLifetime::retain(original),
            active: Cell::new(true),
        });
        let contents = self.contents.upgrade().expect("checked original Jim local");
        contents.borrow_mut().jim_scalar_retirement = Some(Rc::clone(&ticket));
        *self.shape.borrow_mut() = OriginalShape::Retiring(ticket);
        let mut retired = RetiredVariableParts::default();
        // SAFETY: the selected cell transfers its one existing scalar +1.
        retired.key(Some(unsafe { Owned::from_raw(original) }));
        Ok(retired)
    }
    /// Assign only after old retirement and all original continuation checks.
    /// `incoming` transfers the independently acquired true cell reference.
    pub(crate) fn assign(&self, incoming: Owned) -> Result<(), (ValueError, Owned)> {
        if let Err(error) = self.check() {
            return Err((error, incoming));
        }
        let created = matches!(*self.shape.borrow(), OriginalShape::Missing);
        let contents = self.contents.upgrade().expect("checked original Jim local");
        let mut cell = contents.borrow_mut();
        let previous = cell.take_value();
        debug_assert!(
            previous.is_none(),
            "old Jim reference was already extracted"
        );
        let original = incoming.into_raw();
        cell.var = Some(Var::Scalar(original));
        cell.binding_id.get_or_insert_with(super::fresh_var_id);
        self.binding.set(cell.binding_id);
        cell.undefined_shell = false;
        cell.undefined_root_home = None;
        *self.shape.borrow_mut() = OriginalShape::Scalar(original);
        drop(cell);
        if created {
            self.ledger
                .upgrade()
                .expect("checked original table")
                .borrow_mut()
                .insert(&self.name);
        }
        Ok(())
    }
    /// Publish the link frame only after assignment, wrapping the same stored
    /// scalar +1; the caller's temporary remains a distinct actual role.
    pub(crate) fn mark_link(
        &self,
        mut link: Link,
        frame: tcl_runtime_api::jim_call_frame::JimCallFrameStorageReference,
    ) -> Result<(), (ValueError, Link)> {
        if let Err(error) = self.check() {
            return Err((error, link));
        }
        let original = match *self.shape.borrow() {
            OriginalShape::Scalar(original) => original,
            _ => {
                return Err((
                    ValueError::CommandProtocolUnavailable("Jim assigned scalar link frame"),
                    link,
                ))
            }
        };
        let contents = self.contents.upgrade().expect("checked original Jim local");
        let mut cell = contents.borrow_mut();
        let previous = cell.take_value();
        debug_assert!(matches!(previous, Some(Var::Scalar(value)) if value == original));
        // SAFETY: the exact scalar cell's owning +1 transfers into this role.
        let role = Rc::new(OriginalJimLinkTarget::new(
            unsafe { Owned::from_raw(original) },
            frame,
        ));
        link.original_jim_target = Some(Rc::clone(&role));
        cell.var = Some(Var::Link(link));
        *self.shape.borrow_mut() = OriginalShape::Link(Rc::downgrade(&role));
        Ok(())
    }
    /// Mandatory cleanup removes only this still-matching non-owning old
    /// pointer. Callback replacement is kept; no second old decrement occurs.
    pub(crate) fn clear_retiring(&self) {
        let OriginalShape::Retiring(original) = &*self.shape.borrow() else {
            return;
        };
        let Some(contents) = self.contents.upgrade() else {
            return;
        };
        let mut cell = contents.borrow_mut();
        if cell.binding_id == self.binding.get()
            && cell
                .jim_scalar_retirement
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, original))
        {
            let old = cell.take_value();
            debug_assert!(old.is_none());
        }
    }
}
impl VarTable {
    pub(crate) fn prepare_jim_alias_local(
        &mut self,
        name: &[u8],
    ) -> Result<Option<PreparedJimAliasCell>, ValueError> {
        self.slot_for(name);
        PreparedJimAliasCell::capture(
            self.contents(name).expect("selected original Jim local"),
            self.original_table_ledger(),
            name.to_vec(),
        )
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod scalar_store_tests;
