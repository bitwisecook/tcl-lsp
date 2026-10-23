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

//! Actual Jim allocator births and neutral cmdNameObj header observations.

use super::{
    NativeObjectLifetimeLease, NativeValueHandleOwnership, Obj, ObjectRepresentations, Value,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use tcl_runtime_api::jim_object_pool::{
    JimNeutralHeaderObservation, JimObjectPool, JimObjectPoolAllocation,
};
use tcl_runtime_api::native_object_incarnation::NativeObjectIncarnation;
use tcl_syntax::value::ValueError;

type Header = Rc<RefCell<Weak<Obj>>>;
pub(crate) struct JimPool(JimObjectPool<Header>);
impl JimPool {
    pub(super) fn new(reusable: bool) -> Self {
        Self(JimObjectPool::new(reusable))
    }
    pub(super) fn retire(&self) {
        self.0.retire();
    }
}

pub(super) struct ActualAllocation {
    stamp: JimObjectPoolAllocation<Header>,
    pool: Weak<JimPool>,
}
impl ActualAllocation {
    pub(super) fn release(self) {
        if let Some(pool) = self.pool.upgrade() {
            let _ = pool.0.release(self.stamp);
        }
    }
}
struct EnteredAllocator {
    pool: Option<Rc<JimPool>>,
    current: Rc<dyn Fn() -> bool>,
}
thread_local! {
    static ENTERED: RefCell<Vec<Rc<EnteredAllocator>>> = const { RefCell::new(Vec::new()) };
}
#[derive(Clone)]
pub(crate) struct JimAllocationScope {
    _owner: Rc<ScopeOwner>,
}
struct ScopeOwner(Rc<EnteredAllocator>);
impl JimAllocationScope {
    pub(crate) fn enter(pool: Option<Rc<JimPool>>, current: Rc<dyn Fn() -> bool>) -> Self {
        let entered = Rc::new(EnteredAllocator { pool, current });
        ENTERED.with(|stack| stack.borrow_mut().push(Rc::clone(&entered)));
        Self {
            _owner: Rc::new(ScopeOwner(entered)),
        }
    }
}
impl Drop for ScopeOwner {
    fn drop(&mut self) {
        // During thread exit the stack may already be destroyed; then there
        // is nothing left to unwind, and touching it would panic in a
        // destructor.
        let removed = ENTERED.try_with(|stack| {
            let mut stack = stack.borrow_mut();
            let index = stack
                .iter()
                .position(|entry| Rc::ptr_eq(entry, &self.0))
                .expect("entered allocator");
            stack.remove(index)
        });
        drop(removed);
    }
}

pub(super) fn construct(parts: ObjectRepresentations) -> Rc<Obj> {
    // No entered allocator survives the stack's own destruction at thread
    // exit: a value constructed then is an ordinary unpooled one.
    let entered = ENTERED
        .try_with(|stack| stack.borrow().last().cloned())
        .ok()
        .flatten();
    let allocation = entered.filter(|entry| (entry.current)()).and_then(|entry| {
        let pool = entry.pool.as_ref()?;
        pool.0
            .allocate(|| Rc::new(RefCell::new(Weak::new())))
            .ok()
            .map(|stamp| ActualAllocation {
                stamp,
                pool: Rc::downgrade(pool),
            })
    });
    let incarnation = allocation
        .as_ref()
        .map_or_else(NativeObjectIncarnation::fresh, |allocation| {
            allocation.stamp.incarnation().clone()
        });
    let object = Rc::new(Obj {
        jim_allocation: RefCell::new(allocation),
        incarnation,
        lifetime_pins: Cell::new(0),
        retired: Cell::new(false),
        string: parts.string,
        string_storage: parts.string_storage,
        intrep: parts.intrep,
        double_format: parts.double_format,
        source_location: parts.source_location,
        jim_context: parts.jim_context,
    });
    if let Some(allocation) = object.jim_allocation.borrow().as_ref() {
        *allocation
            .stamp
            .header()
            .expect("current constructor birth")
            .borrow_mut() = Rc::downgrade(&object);
    }
    object
}

pub(super) fn validate_allocator(value: &Value, actual: &Rc<JimPool>) -> Result<(), ValueError> {
    value.check_native_header()?;
    if value
        .0
        .jim_allocation
        .borrow()
        .as_ref()
        .is_some_and(|allocation| {
            !allocation
                .pool
                .upgrade()
                .is_some_and(|pool| Rc::ptr_eq(&pool, actual))
        })
    {
        return Err(ValueError::CommandProtocolUnavailable(
            "original Jim constructor allocator",
        ));
    }
    Ok(())
}

#[derive(Clone)]
pub(crate) struct JimHeaderObservation(JimNeutralHeaderObservation<Header>);

#[derive(Clone)]
pub(crate) enum JimCommandNameObservation {
    Exact(NativeObjectLifetimeLease),
    Resident(JimHeaderObservation),
}
impl JimCommandNameObservation {
    pub(crate) fn capture(original: &Value) -> Result<Self, ValueError> {
        original.check_native_header()?;
        let observed = original
            .0
            .jim_allocation
            .borrow()
            .as_ref()
            .map(|allocation| {
                allocation
                    .stamp
                    .observe_header()
                    .map(|header| Self::Resident(JimHeaderObservation(header)))
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable("original Jim observed header")
                    })
            })
            .transpose()?;
        Ok(observed.unwrap_or_else(|| Self::Exact(original.native_lifetime_lease())))
    }
    pub(crate) fn current(&self) -> Result<Value, ValueError> {
        match self {
            Self::Exact(original) => {
                original.value().check_native_header()?;
                if original.value().native_object_reference_count() == 0 {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original Jim observed name has no live Native role",
                    ));
                }
                Ok(original.value().clone())
            }
            Self::Resident(observed) => {
                let resident = observed.0.current_resident().map_err(|_| {
                    ValueError::CommandProtocolUnavailable("retired original Jim header resident")
                })?;
                let object = resident
                    .header()
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable("original Jim header resident")
                    })?
                    .borrow()
                    .upgrade()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original Jim resident allocation",
                    ))?;
                // The temporary Rc is the selected output reference; it cannot alone certify an old rc0 header.
                if object.retired.get()
                    || !object.incarnation.same_as(resident.incarnation())
                    || Rc::strong_count(&object) <= object.lifetime_pins.get() + 1
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original Jim resident has no live Native role",
                    ));
                }
                Ok(Value(object, NativeValueHandleOwnership::Reference))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::native_operation_currency::NativeOperationScope;

    fn interpreter() -> crate::Vm {
        crate::native_fixture::core(crate::environment::profile_for_dialect("jim"))
    }

    fn header(value: &Value) -> Header {
        Rc::clone(
            value
                .0
                .jim_allocation
                .borrow()
                .as_ref()
                .unwrap()
                .stamp
                .header()
                .unwrap(),
        )
    }

    #[test]
    fn actual_factory_reuses_only_the_header_and_keeps_exact_objects_retired() {
        let mut vm = interpreter();
        let _scope = NativeOperationScope::enter(&mut vm).unwrap();
        let original = Value::new_native_string_bytes(b"OLD".as_slice());
        let actual_header = header(&original);
        let exact = original.native_lifetime_lease();
        let observed = JimCommandNameObservation::capture(&original).unwrap();
        assert_eq!(original.native_object_reference_count(), 1);
        drop(original);
        assert!(!exact.value().native_object_is_live());
        assert!(observed.current().is_err());
        let resident = Value::int(42);
        assert!(Rc::ptr_eq(&header(&resident), &actual_header));
        assert_eq!(resident.native_object_reference_count(), 1);
        let selected = observed.current().unwrap();
        assert!(selected.is_same_object(&resident));
        assert_eq!(resident.native_object_reference_count(), 2);
        drop(selected);
        assert_eq!(resident.native_object_reference_count(), 1);
        assert!(!exact.value().native_object_is_live());
        let stale_clone = exact.clone();
        drop(exact);
        drop(stale_clone);
        assert_eq!(resident.native_object_reference_count(), 1);
        drop(resident);
        assert!(observed.current().is_err());
    }

    #[test]
    fn all_three_factory_constructors_and_lifo_release_use_the_actual_scope() {
        let mut vm = interpreter();
        let _scope = NativeOperationScope::enter(&mut vm).unwrap();
        let first = Value::new_native_string_bytes(b"FIRST".as_slice());
        let second = Value::new_native_string_bytes(b"SECOND".as_slice());
        let first_header = header(&first);
        let second_header = header(&second);
        drop(first);
        drop(second);
        let last = Value::int(1);
        let earlier = Value::int(2);
        assert!(Rc::ptr_eq(&header(&last), &second_header));
        assert!(Rc::ptr_eq(&header(&earlier), &first_header));
        let duplicated = last.duplicate_native_object();
        let duplicate_header = header(&duplicated);
        let duplicate_exact = duplicated.native_lifetime_lease();
        drop(duplicated);
        let supplied = earlier
            .clone()
            .with_resident_string_bytes(Rc::from(b"DATA".as_slice()));
        assert!(Rc::ptr_eq(&header(&supplied), &duplicate_header));
        assert!(!duplicate_exact.value().native_object_is_live());
        assert_eq!(supplied.resident_string_bytes().unwrap().as_ref(), b"DATA");
        assert_eq!(supplied.native_object_reference_count(), 1);
    }

    #[test]
    fn host_births_and_foreign_contexts_cannot_acquire_allocator_membership_later() {
        let host = Value::new_native_string_bytes(b"HOST".as_slice());
        let mut vm = interpreter();
        let _scope = NativeOperationScope::enter(&mut vm).unwrap();
        let context = vm.native_jim_object_context().unwrap();
        host.bind_native_jim_context(&context).unwrap();
        assert!(host.0.jim_allocation.borrow().is_none());
        assert!(matches!(
            JimCommandNameObservation::capture(&host).unwrap(),
            JimCommandNameObservation::Exact(_)
        ));
        let original = Value::new_native_string_bytes(b"OWNED".as_slice());
        let foreign =
            super::super::NativeJimObjectContext::new(tcl_registry::InvocationDialect::of_profile(
                crate::environment::profile_for_dialect("jim"),
            ))
            .unwrap();
        assert!(original.bind_native_jim_context(&foreign).is_err());
        assert!(original.0.jim_context.borrow().is_none());
        assert_eq!(original.native_object_reference_count(), 1);
    }
}
