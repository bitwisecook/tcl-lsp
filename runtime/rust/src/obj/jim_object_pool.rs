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

//! Actual Jim allocation scopes and physical header memory, separate from exact births.

use super::{TclObj, auxiliary, deallocate_object};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use tcl_runtime_api::jim_object_pool::{
    JimNeutralHeaderObservation, JimObjectPool, JimObjectPoolAllocation,
};

pub(crate) struct JimHeaderMemory(*mut TclObj);
impl JimHeaderMemory {
    fn retain(pointer: *mut TclObj) -> Self {
        let state = auxiliary(pointer);
        state.lifetime_pins.set(
            state
                .lifetime_pins
                .get()
                .checked_add(1)
                .expect("header memory pins"),
        );
        Self(pointer)
    }
}
impl Drop for JimHeaderMemory {
    fn drop(&mut self) {
        let state = auxiliary(self.0);
        let count = state
            .lifetime_pins
            .get()
            .checked_sub(1)
            .expect("header memory pins");
        state.lifetime_pins.set(count);
        if count == 0 && !state.live.get() && !state.retiring.get() {
            // SAFETY: only this final physical memory pin remains; Native resources already retired.
            unsafe { deallocate_object(self.0) };
        }
    }
}

pub(crate) type JimPool = JimObjectPool<JimHeaderMemory>;
pub(crate) type JimAllocation = JimObjectPoolAllocation<JimHeaderMemory>;

struct EnteredAllocator {
    pool: Option<Rc<JimPool>>,
    current: Rc<dyn Fn() -> bool>,
}
thread_local! {
    static ENTERED: RefCell<Vec<Rc<EnteredAllocator>>> = const { RefCell::new(Vec::new()) };
}

pub(crate) struct JimAllocationScope(Rc<EnteredAllocator>);
impl JimAllocationScope {
    pub(crate) fn enter(pool: Option<Rc<JimPool>>, current: Rc<dyn Fn() -> bool>) -> Self {
        let entered = Rc::new(EnteredAllocator { pool, current });
        ENTERED.with(|stack| stack.borrow_mut().push(Rc::clone(&entered)));
        Self(entered)
    }
}
impl Drop for JimAllocationScope {
    fn drop(&mut self) {
        let removed = ENTERED.with(|stack| {
            let mut stack = stack.borrow_mut();
            let index = stack
                .iter()
                .position(|entry| Rc::ptr_eq(entry, &self.0))
                .expect("entered allocation scope");
            stack.remove(index)
        });
        drop(removed);
    }
}

pub(super) fn allocate() -> Result<Option<(JimAllocation, Weak<JimPool>)>, ()> {
    let current = ENTERED.with(|stack| stack.borrow().last().cloned());
    let Some(current) = current else {
        return Ok(None);
    };
    if !(current.current)() {
        return Ok(None);
    }
    let Some(pool) = &current.pool else {
        return Ok(None);
    };
    let allocation = pool
        .try_allocate(|| {
            let pointer = super::allocate_object_memory();
            if pointer.is_null() {
                return Err(());
            }
            Ok(JimHeaderMemory::retain(pointer))
        })
        .map_err(|_| ())?;
    Ok(Some((allocation, Rc::downgrade(pool))))
}

pub(super) fn pointer(allocation: &JimAllocation) -> *mut TclObj {
    allocation
        .header()
        .expect("current actual constructor birth")
        .0
}

pub(crate) fn validate_allocator(
    value: *mut TclObj,
    actual: &Rc<JimPool>,
) -> Result<(), tcl_syntax::value::ValueError> {
    super::check_native_liveness(value)?;
    if auxiliary(value)
        .jim_pool
        .borrow()
        .as_ref()
        .is_some_and(|owner| {
            !owner
                .upgrade()
                .is_some_and(|owner| Rc::ptr_eq(&owner, actual))
        })
    {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original Jim constructor allocator",
        ));
    }
    Ok(())
}

/// Only the original cmdNameObj role follows the live resident at this header.
#[derive(Clone)]
pub(crate) struct JimHeaderObservation {
    header: JimNeutralHeaderObservation<JimHeaderMemory>,
    #[cfg(test)]
    original: *mut TclObj,
}
impl JimHeaderObservation {
    pub(crate) fn capture(
        value: *mut TclObj,
    ) -> Result<Option<Self>, tcl_syntax::value::ValueError> {
        super::check_native_liveness(value)?;
        auxiliary(value)
            .jim_allocation
            .borrow()
            .as_ref()
            .map(|allocation| {
                allocation
                    .observe_header()
                    .map(|header| Self {
                        header,
                        #[cfg(test)]
                        original: value,
                    })
                    .map_err(|_| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original Jim header observation",
                        )
                    })
            })
            .transpose()
    }
    pub(crate) fn current(&self) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        let allocation = self.header.current_resident().map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired original Jim header resident",
            )
        })?;
        let pointer = allocation
            .header()
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original Jim header resident",
                )
            })?
            .0;
        super::check_native_liveness(pointer)?;
        if !allocation
            .incarnation()
            .same_as(&auxiliary(pointer).incarnation.borrow())
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim resident incarnation",
            ));
        }
        Ok(pointer)
    }
}

#[derive(Clone)]
pub(crate) enum JimCommandNameObservation {
    Exact(super::NativeObjectLifetime),
    Resident(JimHeaderObservation),
}
impl JimCommandNameObservation {
    #[cfg(test)]
    pub(crate) fn as_ptr(&self) -> *mut TclObj {
        match self {
            Self::Exact(original) => original.as_ptr(),
            Self::Resident(observed) => observed.original,
        }
    }
    pub(crate) fn capture(pointer: *mut TclObj) -> Result<Self, tcl_syntax::value::ValueError> {
        match JimHeaderObservation::capture(pointer)? {
            Some(resident) => Ok(Self::Resident(resident)),
            None => Ok(Self::Exact(super::NativeObjectLifetime::retain(pointer))),
        }
    }
    pub(crate) fn current_ptr(&self) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        let pointer = match self {
            Self::Exact(original) => original.checked_ptr()?,
            Self::Resident(observed) => observed.current()?,
        };
        if unsafe { (*pointer).ref_count } <= 0 {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim observed name has no live Native role",
            ));
        }
        Ok(pointer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::{NativeObjectLifetime, Owned, new_string_bytes, ref_count};

    fn interpreter() -> crate::Interp {
        crate::Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn actual_allocator_reuses_a_header_without_reviving_exact_births_or_donating_refs() {
        let interpreter = interpreter();
        let _scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&interpreter)
                .unwrap();
        let original = Owned::fresh(new_string_bytes(b"OLD"));
        let pointer = original.as_ptr();
        let exact = NativeObjectLifetime::retain(pointer);
        let neutral = JimCommandNameObservation::capture(pointer).unwrap();
        assert_eq!(ref_count(pointer), 1);
        drop(original);
        assert!(exact.checked_ptr().is_err());
        assert!(neutral.current_ptr().is_err());
        let resident = Owned::fresh(new_string_bytes(b"NEW"));
        assert_eq!(resident.as_ptr(), pointer);
        assert_eq!(neutral.current_ptr().unwrap(), pointer);
        assert_eq!(crate::obj::bytes_of(pointer), b"NEW");
        assert!(exact.checked_ptr().is_err());
        let stale_clone = exact.clone();
        drop(exact);
        drop(stale_clone);
        assert_eq!(ref_count(pointer), 1);
        drop(resident);
        assert!(neutral.current_ptr().is_err());
    }

    #[test]
    fn actual_allocator_keeps_lifo_retirement_and_host_births_separate() {
        let host = Owned::fresh(new_string_bytes(b"HOST"));
        let host_observed = JimCommandNameObservation::capture(host.as_ptr()).unwrap();
        assert!(matches!(host_observed, JimCommandNameObservation::Exact(_)));
        let interpreter = interpreter();
        let _scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&interpreter)
                .unwrap();
        let first = Owned::fresh(new_string_bytes(b"FIRST"));
        let second = Owned::fresh(new_string_bytes(b"SECOND"));
        let first_pointer = first.as_ptr();
        let second_pointer = second.as_ptr();
        drop(first);
        drop(second);
        let last = Owned::fresh(new_string_bytes(b"LAST"));
        let earlier = Owned::fresh(new_string_bytes(b"EARLIER"));
        assert_eq!(last.as_ptr(), second_pointer);
        assert_eq!(earlier.as_ptr(), first_pointer);
        assert_eq!(host_observed.current_ptr().unwrap(), host.as_ptr());
    }

    #[test]
    fn constructor_scope_is_withdrawn_by_actual_host_change_and_suppressed_for_c_entry() {
        let interpreter = interpreter();
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&interpreter)
                .unwrap();
        let original = Owned::fresh(new_string_bytes(b"OLD"));
        let neutral = JimCommandNameObservation::capture(original.as_ptr()).unwrap();
        drop(original);
        let c_interpreter = crate::Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        {
            let _c_scope = crate::interp::native_operation_currency::NativeOperationScope::enter(
                &c_interpreter,
            )
            .unwrap();
            let c_value = Owned::fresh(new_string_bytes(b"C"));
            assert!(matches!(
                JimCommandNameObservation::capture(c_value.as_ptr()).unwrap(),
                JimCommandNameObservation::Exact(_)
            ));
            assert!(neutral.current_ptr().is_err());
        }
        interpreter.set_host(crate::interp::default_host());
        assert!(scope.currency().ensure_current_or_refuse().is_err());
        let withdrawn = Owned::fresh(new_string_bytes(b"WITHDRAWN"));
        assert!(matches!(
            JimCommandNameObservation::capture(withdrawn.as_ptr()).unwrap(),
            JimCommandNameObservation::Exact(_)
        ));
        assert!(neutral.current_ptr().is_err());
    }
}
