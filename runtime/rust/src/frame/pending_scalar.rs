// SPDX-License-Identifier: AGPL-3.0-or-later
//! A published allocation whose selected setter has not taken its cell reference.

use crate::obj::{self, NativeObjectLifetime, TclObj};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Published,
    Displaced,
    CellOwned,
    Released,
}
struct Role {
    allocation: NativeObjectLifetime,
    phase: Cell<Phase>,
}

/// A published scalar allocation with no native cell reference yet.
/// This state is distinct from `Var::Scalar`: ordinary checked reads cannot
/// borrow an owned scalar from it. Removing it retires only its pending
/// allocation, and never decrements a callback-created external native role.
pub struct PendingScalar(Rc<Role>);

pub(super) struct PendingScalarObservation(Rc<Role>);
impl PendingScalar {
    pub(super) fn new(original: *mut TclObj) -> Self {
        Self(Rc::new(Role {
            allocation: NativeObjectLifetime::retain(original),
            phase: Cell::new(Phase::Published),
        }))
    }
    pub(super) fn observe(&self) -> PendingScalarObservation {
        PendingScalarObservation(Rc::clone(&self.0))
    }
    pub(super) fn mark_displaced(&self) {
        if self.0.phase.get() == Phase::Published {
            self.0.phase.set(Phase::Displaced);
        }
    }
    pub(super) fn same_published(&self, original: &PendingScalarObservation) -> bool {
        Rc::ptr_eq(&self.0, &original.0) && self.0.phase.get() == Phase::Published
    }
    pub(super) fn settle_cell_reference(&self) -> *mut TclObj {
        assert!(self.0.phase.get() == Phase::Published);
        let original = self.0.allocation.as_ptr();
        // The caller has checked this exact live slot/ticket while borrowed.
        // This is the selected setter's genuine cell reference, not a guard.
        unsafe { obj::incr_ref_count(original) };
        self.0.phase.set(Phase::CellOwned);
        original
    }
    #[cfg(test)]
    pub(crate) fn raw_header(&self) -> *mut TclObj {
        self.0.allocation.as_ptr()
    }
}
impl PendingScalarObservation {
    pub(super) fn original(&self) -> *mut TclObj {
        self.0.allocation.as_ptr()
    }
    pub(super) fn is_published(&self) -> bool {
        self.0.phase.get() == Phase::Published
    }
}
impl Drop for PendingScalar {
    fn drop(&mut self) {
        if self.0.phase.get() == Phase::CellOwned {
            return;
        }
        self.0.phase.set(Phase::Released);
        let original = self.0.allocation.as_ptr();
        // The displaced cell never owned +1. A callback's actual external
        // references survive this mandatory allocation cleanup unchanged.
        if obj::allocation_is_live(original) && unsafe { (*original).ref_count == 0 } {
            // SAFETY: this pending allocation has no native owners. Observers
            // retain only memory safety leases and cannot use retired contents.
            unsafe { obj::free_obj(original) };
        }
    }
}

#[cfg(test)]
mod formal_tests {
    use super::*;
    use crate::{
        frame::{Var, VarError},
        interp::{native_operation_currency::NativeOperationScope, Interp, WeakInterp},
        namespace::GLOBAL,
    };
    use tcl_runtime_api::NativeExecutionError;
    use tcl_syntax::raw_string::NativeValueAccessRefusal;

    struct MetadataRetirement {
        interpreter: WeakInterp,
        pending: *mut TclObj,
        calls: Rc<Cell<usize>>,
        arm: Rc<Cell<bool>>,
    }
    extern "C" fn retire(value: *mut TclObj) {
        let state =
            unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut MetadataRetirement) };
        if !state.arm.get() {
            return;
        }
        state.calls.set(state.calls.get() + 1);
        let interpreter = state.interpreter.upgrade().unwrap();
        assert!(interpreter.frames.try_borrow_mut().is_ok());
        let contents = {
            let frames = interpreter.frames.borrow();
            let (frame, slot) = frames.compiled_cell(0).unwrap();
            Rc::clone(&frames.frames[frame].table.cells[slot].contents)
        };
        let old = contents
            .borrow_mut()
            .replace_value(Var::PendingScalar(PendingScalar::new(state.pending)));
        if let Some(old) = old {
            old.release();
        }
    }
    static METADATA: obj::TclObjType = obj::TclObjType {
        name: c"originalPendingFormalMetadata".as_ptr(),
        free_int_rep_proc: Some(retire),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };

    #[test]
    fn original_formal_store_refuses_pending_role_before_hold_and_after_metadata_retirement() {

        // Software storage/transport boundary only: the private fixture installs
        // the actual pending allocation role, without a C compiler chronology claim.
        for pending_at_entry in [true, false] {
            crate::counters::reset();
            {
                let mut interpreter = Interp::with_native_core(
                    crate::interp::default_host(),
                    crate::environment::profile_for_dialect("tcl8.4"),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                let protocol = interpreter
                    .native_invocation_dialect()
                    .native_compiled_variable_protocol()
                    .unwrap();
                interpreter.frames.borrow_mut().push(GLOBAL);
                interpreter
                    .frames
                    .borrow_mut()
                    .install_formal_cells(&[b"formal".to_vec()], protocol);
                let pending = obj::new_string_bytes(b"PENDING ALLOCATION");
                let allocation = NativeObjectLifetime::retain(pending);
                let calls = Rc::new(Cell::new(0));
                let arm = Rc::new(Cell::new(true));
                let key = obj::Owned::fresh(obj::alloc_typed(
                    &METADATA,
                    Box::into_raw(Box::new(MetadataRetirement {
                        interpreter: interpreter.downgrade(),
                        pending,
                        calls: calls.clone(),
                        arm: arm.clone(),
                    })) as usize as u64,
                ));
                let old_value = obj::Owned::fresh(obj::new_string_bytes(b"OLD MEMBER"));
                let contents = {
                    let mut frames = interpreter.frames.borrow_mut();
                    let (frame, slot) = frames.compiled_cell(0).unwrap();
                    let table = &mut frames.frames[frame].table;
                    table
                        .prepare_original_native_element_at(slot, b"k", Some(key.as_ptr()))
                        .unwrap();
                    table
                        .store_elem(b"formal", b"k", old_value.as_ptr())
                        .unwrap();
                    Rc::clone(&table.cells[slot].contents)
                };
                drop(key);
                if pending_at_entry {
                    let old = contents
                        .borrow_mut()
                        .replace_value(Var::PendingScalar(PendingScalar::new(pending)));
                    if let Some(old) = old {
                        old.release();
                    }
                }
                let incoming = obj::Owned::fresh(obj::new_string_bytes(b"INCOMING"));
                interpreter.set_result_bytes(b"PRIOR RESULT");
                let prior = interpreter.result_obj();
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                assert_eq!(
                    interpreter.store_native_formal_cell_checked(0, incoming.as_ptr()),
                    Err(VarError::NameProtocolUnavailable)
                );
                assert_eq!(calls.get(), usize::from(!pending_at_entry));
                assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 1);
                assert_eq!(unsafe { (*pending).ref_count }, 0);
                assert!(matches!(contents.borrow().var, Some(Var::PendingScalar(_))));
                assert_eq!(interpreter.result_obj(), prior);
                assert_eq!(interpreter.entered_native_operation_count(), 1);
                let first = NativeExecutionError::ValueAccessRefusal(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "unsettled scalar native reference role",
                    ),
                );
                assert_eq!(interpreter.native_execution_refusal(), Some(first.clone()));
                assert_eq!(scope.currency().ensure_current_or_refuse(), Err(first));
                arm.set(false);
                // Withdraw the actual pending ticket while its observations
                // remain alive; mandatory disposal must not decrement a cell +1.
                let old = contents.borrow_mut().take_value();
                if let Some(old) = old {
                    old.release();
                }
                assert!(obj::check_native_liveness(allocation.as_ptr()).is_err());
                // This optional metadata owner was never reached on the entry
                // refusal path. Disable its fixture reentry before teardown.
                if pending_at_entry {
                    let retired = std::mem::take(&mut contents.borrow_mut().native_member_keys);
                    drop(retired);
                }
                drop(scope);
                assert_eq!(interpreter.entered_native_operation_count(), 0);
                drop(allocation);
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }
}
