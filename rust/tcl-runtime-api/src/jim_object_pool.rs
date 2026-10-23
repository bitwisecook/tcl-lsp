// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Jim header reuse and exact incarnation are separate from Native object references.

use crate::native_object_incarnation::NativeObjectIncarnation;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JimObjectPoolUnavailable {
    Retired,
    ForeignAllocator,
    StaleIncarnation,
}

/// Header allocation cannot issue a birth when the physical constructor fails.
#[derive(Debug)]
pub enum JimObjectAllocationUnavailable<E> {
    Pool(JimObjectPoolUnavailable),
    Constructor(E),
}

struct PoolState {
    live: Cell<bool>,
}

struct HeaderSlot<Header> {
    owner: Weak<PoolState>,
    resident: RefCell<Option<NativeObjectIncarnation>>,
    header: Header,
}

/// One actual allocator's physical headers in original LIFO retirement order.
/// Header adapters retain allocation safety independently of Native references.
pub struct JimObjectPool<Header> {
    state: Rc<PoolState>,
    reusable: bool,
    free: RefCell<Vec<Rc<HeaderSlot<Header>>>>,
}

/// Exact constructor birth at an allocator-owned physical header.
/// This metadata carries no Native reference or object getter permission.
pub struct JimObjectPoolAllocation<Header> {
    slot: Rc<HeaderSlot<Header>>,
    incarnation: NativeObjectIncarnation,
}

impl<Header> Clone for JimObjectPoolAllocation<Header> {
    fn clone(&self) -> Self {
        Self {
            slot: Rc::clone(&self.slot),
            incarnation: self.incarnation.clone(),
        }
    }
}

/// Original cmdNameObj observes the current resident at one physical header.
/// Other original-object consumers must retain their exact allocation birth.
pub struct JimNeutralHeaderObservation<Header>(Rc<HeaderSlot<Header>>);

impl<Header> Clone for JimNeutralHeaderObservation<Header> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<Header> JimObjectPool<Header> {
    /// Construct bookkeeping for an explicitly supplied actual allocator mode.
    /// Native interpreter/currentness admission remains with the concrete adapter.
    #[must_use]
    pub fn new(reusable: bool) -> Self {
        Self {
            state: Rc::new(PoolState {
                live: Cell::new(true),
            }),
            reusable,
            free: RefCell::new(Vec::new()),
        }
    }

    /// Allocate the next header before issuing its fresh exact constructor birth.
    /// The real header constructor runs only when the actual free list is empty.
    ///
    /// # Errors
    /// Returns the allocator withdrawal or actual constructor failure.
    pub fn try_allocate<E>(
        &self,
        fresh: impl FnOnce() -> Result<Header, E>,
    ) -> Result<JimObjectPoolAllocation<Header>, JimObjectAllocationUnavailable<E>> {
        if !self.state.live.get() {
            return Err(JimObjectAllocationUnavailable::Pool(
                JimObjectPoolUnavailable::Retired,
            ));
        }
        let reused = self.free.borrow_mut().pop();
        let slot = match reused {
            Some(slot) => slot,
            None => Rc::new(HeaderSlot {
                owner: Rc::downgrade(&self.state),
                resident: RefCell::new(None),
                header: fresh().map_err(JimObjectAllocationUnavailable::Constructor)?,
            }),
        };
        if !self.state.live.get() {
            return Err(JimObjectAllocationUnavailable::Pool(
                JimObjectPoolUnavailable::Retired,
            ));
        }
        let incarnation = NativeObjectIncarnation::fresh();
        *slot.resident.borrow_mut() = Some(incarnation.clone());
        Ok(JimObjectPoolAllocation { slot, incarnation })
    }

    /// Infallible physical-header constructor for adapters whose allocator aborts on OOM.
    ///
    /// # Errors
    /// Returns an allocator withdrawal before constructing an object.
    pub fn allocate(
        &self,
        fresh: impl FnOnce() -> Header,
    ) -> Result<JimObjectPoolAllocation<Header>, JimObjectPoolUnavailable> {
        match self.try_allocate(|| Ok::<Header, core::convert::Infallible>(fresh())) {
            Ok(allocation) => Ok(allocation),
            Err(JimObjectAllocationUnavailable::Pool(error)) => Err(error),
            Err(JimObjectAllocationUnavailable::Constructor(impossible)) => match impossible {},
        }
    }

    /// Publish a header only after its old Native primary/string/children retire.
    /// A later birth cannot be retired through an old exact allocation stamp.
    ///
    /// # Errors
    /// Returns a retired/foreign allocator or stale original incarnation.
    pub fn release(
        &self,
        allocation: JimObjectPoolAllocation<Header>,
    ) -> Result<(), JimObjectPoolUnavailable> {
        let owner = allocation
            .slot
            .owner
            .upgrade()
            .ok_or(JimObjectPoolUnavailable::Retired)?;
        if !Rc::ptr_eq(&owner, &self.state) {
            return Err(JimObjectPoolUnavailable::ForeignAllocator);
        }
        allocation.check_current()?;
        allocation.slot.resident.borrow_mut().take();
        if self.reusable {
            self.free.borrow_mut().push(allocation.slot);
        }
        Ok(())
    }

    /// Withdraw this actual allocator without reviving outstanding observations.
    pub fn retire(&self) {
        self.state.live.set(false);
        let free = self.free.replace(Vec::new());
        drop(free);
    }
}

impl<Header> JimObjectPoolAllocation<Header> {
    /// Check metadata currentness without granting a Native reader.
    ///
    /// # Errors
    /// Returns a retired allocator or withdrawn exact object incarnation.
    pub fn check_current(&self) -> Result<(), JimObjectPoolUnavailable> {
        let owner = self
            .slot
            .owner
            .upgrade()
            .ok_or(JimObjectPoolUnavailable::Retired)?;
        if !owner.live.get() {
            return Err(JimObjectPoolUnavailable::Retired);
        }
        if !self
            .slot
            .resident
            .borrow()
            .as_ref()
            .is_some_and(|current| current.same_as(&self.incarnation))
        {
            return Err(JimObjectPoolUnavailable::StaleIncarnation);
        }
        Ok(())
    }

    /// Exact identity for original argv, primary and procedure-object guards.
    #[must_use]
    pub fn incarnation(&self) -> &NativeObjectIncarnation {
        &self.incarnation
    }

    /// Borrow physical backing; the adapter checks Native liveness separately.
    ///
    /// # Errors
    /// Returns a retired allocator or withdrawn exact object incarnation.
    pub fn header(&self) -> Result<&Header, JimObjectPoolUnavailable> {
        self.check_current()?;
        Ok(&self.slot.header)
    }

    /// Issue only the neutral physical observation made by actual cold lookup.
    ///
    /// # Errors
    /// Returns a retired allocator or withdrawn original incarnation.
    pub fn observe_header(
        &self,
    ) -> Result<JimNeutralHeaderObservation<Header>, JimObjectPoolUnavailable> {
        self.check_current()?;
        Ok(JimNeutralHeaderObservation(Rc::clone(&self.slot)))
    }
}

impl<Header> JimNeutralHeaderObservation<Header> {
    /// Observe a fresh live birth at the same actual allocator/header slot.
    /// A concrete selected frame still needs its own current Native value reader.
    ///
    /// # Errors
    /// Returns a retired allocator or header with no current resident birth.
    pub fn current_resident(
        &self,
    ) -> Result<JimObjectPoolAllocation<Header>, JimObjectPoolUnavailable> {
        let owner = self
            .0
            .owner
            .upgrade()
            .ok_or(JimObjectPoolUnavailable::Retired)?;
        if !owner.live.get() {
            return Err(JimObjectPoolUnavailable::Retired);
        }
        let incarnation = self
            .0
            .resident
            .borrow()
            .as_ref()
            .cloned()
            .ok_or(JimObjectPoolUnavailable::StaleIncarnation)?;
        Ok(JimObjectPoolAllocation {
            slot: Rc::clone(&self.0),
            incarnation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_withdrawal_cannot_publish_a_new_resident() {
        let pool = JimObjectPool::new(true);
        let created = Rc::new(Cell::new(0));
        let reached = pool.try_allocate(|| {
            created.set(created.get() + 1);
            pool.retire();
            Ok::<_, ()>(Rc::new(()))
        });
        assert!(matches!(
            reached,
            Err(JimObjectAllocationUnavailable::Pool(
                JimObjectPoolUnavailable::Retired
            ))
        ));
        assert_eq!(created.get(), 1);
        assert!(matches!(
            pool.allocate(|| Rc::new(())),
            Err(JimObjectPoolUnavailable::Retired)
        ));
    }

    #[test]
    fn neutral_header_follows_a_real_new_birth_while_exact_views_stay_retired() {
        let pool = JimObjectPool::new(true);
        let first = pool.allocate(|| Rc::new(())).unwrap();
        let physical = Rc::clone(first.header().unwrap());
        let exact = first.clone();
        let observed = first.observe_header().unwrap();
        pool.release(first).unwrap();
        assert!(matches!(
            exact.check_current(),
            Err(JimObjectPoolUnavailable::StaleIncarnation)
        ));
        assert!(observed.current_resident().is_err());
        let second = pool
            .allocate(|| panic!("original free header is reused"))
            .unwrap();
        assert!(Rc::ptr_eq(second.header().unwrap(), &physical));
        assert!(!exact.incarnation().same_as(second.incarnation()));
        assert!(matches!(
            exact.observe_header(),
            Err(JimObjectPoolUnavailable::StaleIncarnation)
        ));
        let resident = observed.current_resident().unwrap();
        assert!(resident.incarnation().same_as(second.incarnation()));
        assert!(matches!(
            pool.release(exact),
            Err(JimObjectPoolUnavailable::StaleIncarnation)
        ));
        assert!(second.check_current().is_ok());
        pool.retire();
        assert!(matches!(
            observed.current_resident(),
            Err(JimObjectPoolUnavailable::Retired)
        ));
        assert!(matches!(
            second.check_current(),
            Err(JimObjectPoolUnavailable::Retired)
        ));
    }

    #[test]
    fn constructor_failure_issues_no_resident_and_keeps_the_next_real_birth_available() {
        let pool = JimObjectPool::<usize>::new(true);
        assert!(matches!(
            pool.try_allocate(|| Err("OOM")),
            Err(JimObjectAllocationUnavailable::Constructor("OOM"))
        ));
        let allocated = pool.try_allocate(|| Ok::<_, &str>(7)).unwrap();
        assert_eq!(*allocated.header().unwrap(), 7);
        assert!(allocated.check_current().is_ok());
    }

    #[test]
    fn allocator_identity_lifo_and_actual_nonreuse_mode_remain_independent() {
        let pool = JimObjectPool::new(true);
        let foreign = JimObjectPool::new(true);
        let first = pool.allocate(|| 1).unwrap();
        let second = pool.allocate(|| 2).unwrap();
        let exact = first.clone();
        assert!(matches!(
            foreign.release(exact),
            Err(JimObjectPoolUnavailable::ForeignAllocator)
        ));
        assert!(first.check_current().is_ok());
        pool.release(first).unwrap();
        pool.release(second).unwrap();
        assert_eq!(*pool.allocate(|| 3).unwrap().header().unwrap(), 2);
        assert_eq!(*pool.allocate(|| 4).unwrap().header().unwrap(), 1);
        let unpooled = JimObjectPool::new(false);
        let first = unpooled.allocate(|| 5).unwrap();
        let observed = first.observe_header().unwrap();
        unpooled.release(first).unwrap();
        assert_eq!(*unpooled.allocate(|| 6).unwrap().header().unwrap(), 6);
        assert!(observed.current_resident().is_err());
    }
}
