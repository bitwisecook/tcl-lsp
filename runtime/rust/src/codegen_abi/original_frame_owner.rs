// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained owners for paired compiler frame and activation operations.
//!
//! The ledger records actual entry, including failed void frame attempts. It
//! grants no name, command or value purpose. Cleanup retains the original
//! owner while callbacks run, without holding a ledger borrow across them.

use super::{CodegenOperation, Interp, TCL_INVOKE_ABI_HOST_REFUSED};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Frame,
    Activation,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Live,
    Cleaning,
    Completed,
}
struct Entry {
    kind: Kind,
    operation: Option<CodegenOperation>,
    interpreter: Option<Interp>,
    entered: bool,
    status: i32,
    phase: Cell<Phase>,
}
#[derive(Default)]
struct Ledger {
    entries: Vec<Rc<Entry>>,
    cleanup_refusal: Option<CleanupRefusal>,
}
struct CleanupRefusal {
    interpreter: crate::interp::WeakInterp,
    first: tcl_runtime_api::NativeExecutionError,
}
fn cleanup_failure() -> Option<tcl_runtime_api::NativeExecutionError> {
    with_ledger(|ledger| {
        ledger
            .borrow()
            .cleanup_refusal
            .as_ref()
            .map(|refusal| refusal.first.clone())
    })
}
#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static LEDGER: RefCell<Ledger> = RefCell::new(Ledger::default());
}
#[cfg(target_arch = "wasm32")]
static LEDGER: core::sync::atomic::AtomicPtr<RefCell<Ledger>> =
    core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());

fn with_ledger<T>(operation: impl FnOnce(&RefCell<Ledger>) -> T) -> T {
    #[cfg(not(target_arch = "wasm32"))]
    {
        LEDGER.with(operation)
    }
    #[cfg(target_arch = "wasm32")]
    {
        // The runtime's wasm32 target has one execution thread and no TLS
        // bootstrap. Allocate its module-global ledger on first actual use.
        let mut pointer = LEDGER.load(core::sync::atomic::Ordering::Relaxed);
        if pointer.is_null() {
            pointer = Box::into_raw(Box::new(RefCell::new(Ledger::default())));
            LEDGER.store(pointer, core::sync::atomic::Ordering::Relaxed);
        }
        // SAFETY: this single-threaded module owns the permanent allocation.
        operation(unsafe { &*pointer })
    }
}
fn installed() -> Option<Interp> {
    // SAFETY: the host retains its explicitly installed interpreter through entry.
    unsafe { super::current_interp().as_ref() }.cloned()
}
fn top() -> Option<Rc<Entry>> {
    with_ledger(|ledger| ledger.borrow().entries.last().cloned())
}
fn order_refusal(entry: &Entry) {
    if let Some(mut interpreter) = entry.interpreter.clone() {
        interpreter.refuse_native_access(NativeValueAccessRefusal::CommandProtocolUnavailable(
            "unmatched compiler ABI entry",
        ));
    }
}
/// Select the genuine outstanding entry before ambient bootstrap state.
/// A failed frame attempt is terminal until its matching pop consumes it.
pub(super) fn interpreter() -> Result<Option<Interp>, i32> {
    if cleanup_failure().is_some() {
        return Err(TCL_INVOKE_ABI_HOST_REFUSED);
    }
    if let Some(entry) = top() {
        if !entry.entered {
            return Err(entry.status);
        }
        let operation = entry
            .operation
            .as_ref()
            .expect("entered compiler operation");
        operation.ensure_current()?;
        return Ok(Some(operation.interpreter.clone()));
    }
    Ok(installed())
}
/// Decide only whether generated code should take its existing decline path.
/// Missing paired ownership leaves the old generated path unchanged.
pub(super) fn native_body_entry_status() -> i32 {
    use tcl_runtime_api::codegen_abi::{
        NATIVE_PROC_STATUS_DECLINED, NATIVE_PROC_STATUS_HOST_REFUSED, NATIVE_PROC_STATUS_RAN,
    };
    if cleanup_failure().is_some() {
        return NATIVE_PROC_STATUS_HOST_REFUSED;
    }
    // The Rc keeps the existing owner live after the ledger borrow ends; it
    // changes no Native object refcount and no ambient interpreter is used.
    let Some(entry) = top() else {
        return NATIVE_PROC_STATUS_RAN;
    };
    if !entry.entered {
        return NATIVE_PROC_STATUS_HOST_REFUSED;
    }
    let operation = entry
        .operation
        .as_ref()
        .expect("entered compiler operation");
    if operation.ensure_current().is_err() {
        return NATIVE_PROC_STATUS_HOST_REFUSED;
    }
    if operation.interpreter.codegen_body_has_invoke_headroom() {
        NATIVE_PROC_STATUS_RAN
    } else {
        NATIVE_PROC_STATUS_DECLINED
    }
}

/// Observe reached refusal on its actual owner, including cleanup after exit.
pub(super) fn refusal_pending() -> bool {
    if cleanup_failure().is_some() {
        return true;
    }
    if let Some(entry) = top() {
        return !entry.entered
            || entry
                .operation
                .as_ref()
                .is_some_and(|operation| operation.ensure_current().is_err());
    }
    installed().is_some_and(|interpreter| interpreter.host_refusal_pending())
}
/// Mandatory token/frame cleanup retains its original owner under refusal.
pub(super) fn cleanup_interpreter() -> Option<Interp> {
    if let Some(entry) = top() {
        return entry.interpreter.clone();
    }
    let original = with_ledger(|ledger| {
        ledger
            .borrow()
            .cleanup_refusal
            .as_ref()
            .map(|refusal| refusal.interpreter.clone())
    });
    original.map_or_else(installed, |original| original.upgrade())
}

/// A new explicit bootstrap outside paired work starts an independent
/// transport boundary. Pointer retargeting inside a held entry changes none
/// of its original currency, cause or receiver selection.
pub(super) fn explicit_bootstrap() {
    let previous = with_ledger(|ledger| {
        let mut ledger = ledger.borrow_mut();
        if ledger.entries.is_empty() {
            ledger.cleanup_refusal.take()
        } else {
            None
        }
    });
    drop(previous);
}
fn explicit_entry() -> Result<CodegenOperation, i32> {
    // Only explicit entry starts a separate installed-interpreter operation.
    // Its own retained first Host cause still prevents entry on that owner.
    with_ledger(|ledger| ledger.borrow_mut().cleanup_refusal.take());
    CodegenOperation::enter_installed()
}
pub(super) fn frame_push() {
    let interpreter = installed();
    let (operation, entered, status) = match explicit_entry() {
        Ok(mut operation) => {
            operation.interpreter.codegen_frame_push();
            let status = operation.ensure_current().err().unwrap_or(0);
            // The actual frame was pushed before the checked Jim-retain stage.
            (Some(operation), true, status)
        }
        Err(status) => (None, false, status),
    };
    let entry = Rc::new(Entry {
        kind: Kind::Frame,
        operation,
        interpreter,
        entered,
        status,
        phase: Cell::new(Phase::Live),
    });
    with_ledger(|ledger| ledger.borrow_mut().entries.push(entry));
}
pub(super) fn activation_enter() -> i32 {
    let mut operation = match explicit_entry() {
        Ok(operation) => operation,
        Err(status) => return status,
    };
    let entered = operation.interpreter.codegen_activation_enter();
    let current = operation.ensure_current();
    if current.is_err() || !entered {
        if entered {
            operation
                .interpreter
                .codegen_activation_leave(crate::interp::Code::Error);
        }
        return current.err().unwrap_or(1);
    }
    let interpreter = Some(operation.interpreter.clone());
    let entry = Rc::new(Entry {
        kind: Kind::Activation,
        operation: Some(operation),
        interpreter,
        entered: true,
        status: 0,
        phase: Cell::new(Phase::Live),
    });
    with_ledger(|ledger| ledger.borrow_mut().entries.push(entry));
    0
}
fn remember_cleanup(entry: &Entry) {
    let refusal = entry.interpreter.as_ref().and_then(|interpreter| {
        interpreter
            .native_execution_refusal()
            .map(|first| CleanupRefusal {
                interpreter: interpreter.downgrade(),
                first,
            })
    });
    if let Some(refusal) = refusal {
        with_ledger(|ledger| {
            let mut ledger = ledger.borrow_mut();
            if ledger.cleanup_refusal.is_none() {
                ledger.cleanup_refusal = Some(refusal);
            }
        });
    }
}
fn drain_completed() {
    loop {
        let departed = with_ledger(|ledger| {
            let mut ledger = ledger.borrow_mut();
            if ledger
                .entries
                .last()
                .is_some_and(|entry| entry.phase.get() == Phase::Completed)
            {
                ledger.entries.pop()
            } else {
                None
            }
        });
        let Some(departed) = departed else { return };
        // Retire its actual Scope outside the ledger borrow. A completed
        // record never repeats physical frame or activation cleanup.
        drop(departed);
    }
}
fn leave(kind: Kind, code: crate::interp::Code) {
    let Some(entry) = top() else { return };
    if entry.kind != kind || entry.phase.get() == Phase::Cleaning {
        order_refusal(&entry);
        return;
    }
    if entry.phase.get() == Phase::Live {
        entry.phase.set(Phase::Cleaning);
        if entry.entered {
            // The actual owner remains selected through reentrant retirement.
            let mut interpreter = entry
                .interpreter
                .clone()
                .expect("entered compiler interpreter");
            match kind {
                Kind::Frame => interpreter.codegen_frame_pop(),
                Kind::Activation => interpreter.codegen_activation_leave(code),
            }
            if let Some(operation) = &entry.operation {
                let _ = operation.ensure_current();
            }
        }
        entry.phase.set(Phase::Completed);
    }
    let remains_top = with_ledger(|ledger| {
        ledger
            .borrow()
            .entries
            .last()
            .is_some_and(|current| Rc::ptr_eq(current, &entry))
    });
    if !remains_top {
        // A callback left a separate entry open. Retain this completed Scope
        // below it, and record the original order refusal before returning.
        order_refusal(&entry);
    }
    remember_cleanup(&entry);
    // Release the local Rc before draining lower completed records. On the
    // same interpreter its Scope must retire after every newer Scope.
    drop(entry);
    drain_completed();
}
pub(super) fn frame_pop() {
    leave(Kind::Frame, crate::interp::Code::Ok);
}
pub(super) fn activation_leave(code: i32) {
    leave(Kind::Activation, crate::interp::Code::from_int(code));
}
#[cfg(test)]
fn outstanding() -> usize {
    with_ledger(|ledger| ledger.borrow().entries.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{codegen_abi::*, interp::Code, obj};
    use core::ptr;
    thread_local! {
        static FREES: Cell<usize> = const { Cell::new(0) };
        static ORIGINAL: Cell<*mut Interp> = const { Cell::new(ptr::null_mut()) };
        static RETARGET: Cell<*mut Interp> = const { Cell::new(ptr::null_mut()) };
        static REENTER: Cell<bool> = const { Cell::new(false) };
        static LEAVE_NESTED: Cell<bool> = const { Cell::new(false) };
        static FIRST: RefCell<Option<tcl_runtime_api::NativeExecutionError>> = const { RefCell::new(None) };
    }
    fn original() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    fn set(name: &[u8], bytes: &[u8]) -> i32 {
        let owned = obj::Owned::fresh(obj::new_string_bytes(bytes));
        unsafe {
            tcl_codegen_var_set(
                name.as_ptr(),
                i32::try_from(name.len()).unwrap(),
                owned.into_raw(),
            )
        }
    }
    fn get(name: &[u8]) -> Vec<u8> {
        let value =
            unsafe { tcl_codegen_var_get(name.as_ptr(), i32::try_from(name.len()).unwrap()) };
        assert!(!value.is_null());
        let owned = unsafe { obj::Owned::from_raw(value) };
        obj::bytes_of(owned.as_ptr()).to_vec()
    }
    fn reset_transport(interpreter: &mut Interp) {
        tcl_runtime_set_current_interp(interpreter);
        assert_eq!(tcl_codegen_activation_enter(), 0);
        tcl_codegen_activation_leave(0);
        assert_eq!(outstanding(), 0);
        tcl_runtime_set_current_interp(ptr::null_mut());
    }
    #[test]
    fn generated_entry_probe_uses_held_owner_and_keeps_refusal_terminal() {
        use tcl_runtime_api::codegen_abi::{
            NATIVE_PROC_STATUS_DECLINED, NATIVE_PROC_STATUS_HOST_REFUSED, NATIVE_PROC_STATUS_RAN,
        };
        let mut first = original();
        let mut second = original();
        reset_transport(&mut first);
        tcl_runtime_set_current_interp(&mut first);
        assert_eq!(
            tcl_codegen_native_body_entry_status(),
            NATIVE_PROC_STATUS_RAN
        );
        for _ in 0..128 {
            assert_eq!(tcl_codegen_activation_enter(), 0);
        }
        assert_eq!(
            tcl_codegen_native_body_entry_status(),
            NATIVE_PROC_STATUS_DECLINED
        );
        tcl_runtime_set_current_interp(&mut second);
        assert_eq!(
            tcl_codegen_native_body_entry_status(),
            NATIVE_PROC_STATUS_DECLINED
        );
        assert!(first.native_execution_refusal().is_none());
        tcl_codegen_activation_leave(0);
        assert_eq!(
            tcl_codegen_native_body_entry_status(),
            NATIVE_PROC_STATUS_RAN
        );
        first.refuse_native_access(NativeValueAccessRefusal::CommandProtocolUnavailable(
            "generated entry probe test",
        ));
        let cause = first.native_execution_refusal();
        assert_eq!(
            tcl_codegen_native_body_entry_status(),
            NATIVE_PROC_STATUS_HOST_REFUSED
        );
        assert_eq!(first.native_execution_refusal(), cause);
        for _ in 0..127 {
            tcl_codegen_activation_leave(0);
        }
        assert_eq!(outstanding(), 0);
        reset_transport(&mut second);
    }

    #[test]
    fn paired_entries_keep_two_original_interpreters_and_nested_lifo_through_tls_retarget() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Genuine entered frames and independent interpreter scopes; no external Tcl chronology claim.
        let mut first = original();
        let mut second = original();
        assert_eq!(outstanding(), 0);
        tcl_runtime_set_current_interp(&mut first);
        assert_eq!(tcl_codegen_activation_enter(), 0);
        tcl_codegen_frame_push();
        assert_eq!(set(b"x", b"FIRST\0\xff"), 0);
        assert_eq!(first.current_level(), 1);
        tcl_runtime_set_current_interp(&mut second);
        assert_eq!(get(b"x"), b"FIRST\0\xff");
        assert_eq!(second.current_level(), 0);
        assert_eq!(tcl_codegen_activation_enter(), 0);
        tcl_codegen_frame_push();
        assert_eq!(set(b"x", b"SECOND"), 0);
        tcl_runtime_set_current_interp(&mut first);
        assert_eq!(get(b"x"), b"SECOND");
        assert_eq!(outstanding(), 4);
        tcl_codegen_frame_pop();
        assert_eq!(second.current_level(), 0);
        assert_eq!(first.current_level(), 1);
        tcl_codegen_activation_leave(0);
        tcl_runtime_set_current_interp(&mut second);
        assert_eq!(get(b"x"), b"FIRST\0\xff");
        assert_eq!(tcl_codegen_host_refusal_pending(), 0);
        tcl_codegen_frame_pop();
        tcl_codegen_activation_leave(0);
        assert_eq!(first.current_level(), 0);
        assert_eq!(outstanding(), 0);
        assert!(first.native_execution_refusal().is_none());
        assert!(second.native_execution_refusal().is_none());
        tcl_runtime_set_current_interp(ptr::null_mut());
    }
    extern "C" fn retire(_: *mut obj::TclObj) {
        FREES.with(|calls| calls.set(calls.get() + 1));
        if LEAVE_NESTED.with(Cell::get) {
            // A genuine object destructor enters another installed owner and
            // leaves its frame and activation for their own later cleanup.
            tcl_runtime_set_current_interp(RETARGET.with(Cell::get));
            assert_eq!(tcl_codegen_activation_enter(), 0);
            tcl_codegen_frame_push();
            assert_eq!(set(b"inner", b"INNER\0\xff"), 0);
            return;
        }
        if !REENTER.with(Cell::get) {
            return;
        }
        tcl_runtime_set_current_interp(RETARGET.with(Cell::get));
        // The test's real host retains this original interpreter through its
        // extension callback. The other installed interpreter stays healthy.
        let mut original = unsafe { ORIGINAL.with(Cell::get).as_ref() }
            .unwrap()
            .clone();
        original.refuse_host_command("first original frame retirement refusal");
        FIRST.with(|first| *first.borrow_mut() = original.native_execution_refusal());
        tcl_codegen_frame_pop();
        assert_eq!(set(b"::later", b"LATER"), TCL_INVOKE_ABI_HOST_REFUSED);
        assert_eq!(tcl_codegen_host_refusal_pending(), 1);
    }
    static RETIRE: obj::TclObjType = obj::TclObjType {
        name: c"originalFrameOwnerRetirement".as_ptr(),
        free_int_rep_proc: Some(retire),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };
    #[test]
    fn refused_void_frame_attempt_consumes_only_its_matching_marker_and_owned_input() {
        // naming.embedding.original-host-publication-and-fact-transport

        // An actual first Host cause prevents nested entry; no frame is fabricated on that owner.
        let mut first = original();
        let mut second = original();
        let mut fresh = original();
        tcl_runtime_set_current_interp(&mut first);
        tcl_codegen_frame_push();
        assert_eq!(set(b"x", b"OUTER"), 0);
        second.refuse_host_command("first failed frame owner");
        let cause = second.native_execution_refusal().unwrap();
        tcl_runtime_set_current_interp(&mut second);
        tcl_codegen_frame_push();
        assert_eq!(outstanding(), 2);
        assert_eq!(first.current_level(), 1);
        assert_eq!(second.current_level(), 0);
        REENTER.with(|reenter| reenter.set(false));
        FREES.with(|calls| calls.set(0));
        let value = obj::Owned::fresh(obj::alloc_typed(&RETIRE, 0));
        assert_eq!(
            unsafe { tcl_codegen_var_set(b"x".as_ptr(), 1, value.into_raw()) },
            TCL_INVOKE_ABI_HOST_REFUSED
        );
        assert_eq!(FREES.with(Cell::get), 1);
        tcl_codegen_frame_pop();
        assert_eq!(outstanding(), 1);
        assert_eq!(first.current_level(), 1);
        assert_eq!(second.current_level(), 0);
        assert_eq!(second.native_execution_refusal(), Some(cause));
        assert!(first.native_execution_refusal().is_none());
        assert_eq!(tcl_codegen_host_refusal_pending(), 1);
        tcl_codegen_frame_pop();
        assert_eq!(first.current_level(), 0);
        assert_eq!(outstanding(), 0);
        reset_transport(&mut fresh);
    }
    #[test]
    fn real_frame_retirement_runs_on_original_owner_and_keeps_first_cause_during_reentrant_exit() {
        // naming.embedding.original-host-publication-and-fact-transport

        // A genuine frame-owned extension object reaches freeIntRep; subsequent operations refuse.
        let mut first = original();
        let mut second = original();
        tcl_runtime_set_current_interp(&mut first);
        tcl_codegen_frame_push();
        FREES.with(|calls| calls.set(0));
        REENTER.with(|reenter| reenter.set(true));
        ORIGINAL.with(|original| original.set(&mut first));
        RETARGET.with(|retarget| retarget.set(&mut second));
        let value = obj::Owned::fresh(obj::alloc_typed(&RETIRE, 0));
        assert_eq!(
            unsafe { tcl_codegen_local_bind(0, b"x".as_ptr(), 1, value.into_raw()) },
            0
        );
        tcl_codegen_frame_pop();
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(outstanding(), 0);
        assert_eq!(first.current_level(), 0);
        assert_eq!(second.current_level(), 0);
        let cause = FIRST.with(|first| first.borrow_mut().take()).unwrap();
        assert!(
            matches!(&cause, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(cause) if cause.reason == "first original frame retirement refusal")
        );
        assert_eq!(first.native_execution_refusal(), Some(cause.clone()));
        assert_eq!(cleanup_failure(), Some(cause));
        assert!(second.native_execution_refusal().is_none());
        assert!(second.var_get(b"::later").is_none());
        assert_eq!(tcl_codegen_host_refusal_pending(), 1);
        REENTER.with(|reenter| reenter.set(false));
        ORIGINAL.with(|original| original.set(ptr::null_mut()));
        RETARGET.with(|retarget| retarget.set(ptr::null_mut()));
        reset_transport(&mut second);
    }
    #[test]
    fn mismatched_entry_kind_refuses_without_removing_an_actual_frame_or_activation() {
        // naming.embedding.original-host-publication-and-fact-transport

        // The malformed caller order is a software API control, not a native Tcl test.
        let mut first = original();
        let mut second = original();
        tcl_runtime_set_current_interp(&mut first);
        assert_eq!(tcl_codegen_activation_enter(), 0);
        tcl_codegen_frame_push();
        tcl_codegen_activation_leave(0);
        assert_eq!(outstanding(), 2);
        assert_eq!(first.current_level(), 1);
        let cause = first.native_execution_refusal().unwrap();
        assert_eq!(
            cause,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "unmatched compiler ABI entry"
                )
            )
        );
        tcl_codegen_frame_pop();
        tcl_codegen_activation_leave(0);
        assert_eq!(first.current_level(), 0);
        assert_eq!(outstanding(), 0);
        assert_eq!(first.native_execution_refusal(), Some(cause));
        reset_transport(&mut second);
        assert!(second.native_execution_refusal().is_none());
        assert_eq!(second.eval_str(b"set x GOOD"), Code::Ok);
    }
    #[test]
    fn callback_left_nested_entry_retires_completed_original_scope_once_after_its_matched_exit() {
        // naming.embedding.original-host-publication-and-fact-transport

        // An actual freeIntRep leaves a second owner's genuine entry open; no external Tcl chronology claim.
        let mut first = original();
        let mut second = original();
        tcl_runtime_set_current_interp(&mut first);
        tcl_codegen_frame_push();
        FREES.with(|calls| calls.set(0));
        REENTER.with(|reenter| reenter.set(false));
        LEAVE_NESTED.with(|nested| nested.set(true));
        RETARGET.with(|retarget| retarget.set(&mut second));
        let value = obj::Owned::fresh(obj::alloc_typed(&RETIRE, 0));
        assert_eq!(
            unsafe { tcl_codegen_local_bind(0, b"x".as_ptr(), 1, value.into_raw()) },
            0
        );
        tcl_codegen_frame_pop();
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(outstanding(), 3);
        assert_eq!(first.current_level(), 0);
        assert_eq!(second.current_level(), 1);
        assert_eq!(first.entered_native_operation_count(), 1);
        assert_eq!(second.entered_native_operation_count(), 2);
        let cause = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("unmatched compiler ABI entry"),
        );
        assert_eq!(first.native_execution_refusal(), Some(cause.clone()));
        assert_eq!(cleanup_failure(), Some(cause.clone()));
        assert!(second.native_execution_refusal().is_none());
        assert_eq!(tcl_codegen_host_refusal_pending(), 1);
        // Ordinary work cannot hide the original cause behind the new owner.
        assert_eq!(set(b"::later", b"LATER"), TCL_INVOKE_ABI_HOST_REFUSED);
        assert!(second.var_get(b"::later").is_none());
        LEAVE_NESTED.with(|nested| nested.set(false));
        tcl_codegen_frame_pop();
        assert_eq!(outstanding(), 2);
        assert_eq!(second.current_level(), 0);
        assert_eq!(first.entered_native_operation_count(), 1);
        assert_eq!(second.entered_native_operation_count(), 1);
        tcl_codegen_activation_leave(0);
        assert_eq!(outstanding(), 0);
        assert!(first.entered_native_operation_count() == 0);
        assert!(second.entered_native_operation_count() == 0);
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(first.native_execution_refusal(), Some(cause.clone()));
        assert_eq!(cleanup_failure(), Some(cause));
        assert!(second.native_execution_refusal().is_none());
        assert_eq!(tcl_codegen_host_refusal_pending(), 1);
        RETARGET.with(|retarget| retarget.set(ptr::null_mut()));
        reset_transport(&mut second);
    }
    #[test]
    fn callback_left_same_interpreter_entry_retires_actual_scopes_in_lifo_order() {
        // naming.embedding.original-host-publication-and-fact-transport

        // The real callback inherits one interpreter's original scope stack; no native provider equivalence is asserted.
        let mut interpreter = original();
        let mut healthy = original();
        tcl_runtime_set_current_interp(&mut interpreter);
        tcl_codegen_frame_push();
        FREES.with(|calls| calls.set(0));
        REENTER.with(|reenter| reenter.set(false));
        LEAVE_NESTED.with(|nested| nested.set(true));
        RETARGET.with(|retarget| retarget.set(&mut interpreter));
        let value = obj::Owned::fresh(obj::alloc_typed(&RETIRE, 0));
        assert_eq!(
            unsafe { tcl_codegen_local_bind(0, b"x".as_ptr(), 1, value.into_raw()) },
            0
        );
        tcl_codegen_frame_pop();
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(outstanding(), 3);
        assert_eq!(interpreter.current_level(), 1);
        assert_eq!(interpreter.entered_native_operation_count(), 3);
        let cause = interpreter.native_execution_refusal().unwrap();
        assert_eq!(
            cause,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "unmatched compiler ABI entry"
                ),
            )
        );
        LEAVE_NESTED.with(|nested| nested.set(false));
        tcl_codegen_frame_pop();
        assert_eq!(outstanding(), 2);
        assert_eq!(interpreter.current_level(), 0);
        assert_eq!(interpreter.entered_native_operation_count(), 2);
        tcl_codegen_activation_leave(0);
        assert_eq!(outstanding(), 0);
        assert!(interpreter.entered_native_operation_count() == 0);
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(interpreter.native_execution_refusal(), Some(cause.clone()));
        assert_eq!(cleanup_failure(), Some(cause));
        assert_eq!(tcl_codegen_host_refusal_pending(), 1);
        RETARGET.with(|retarget| retarget.set(ptr::null_mut()));
        reset_transport(&mut healthy);
    }
}
