// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual worker retirement controls; no Native callback timing claim.
use super::*;
use crate::interp::{ObjCommand, WeakInterp};
use std::{
    cell::{Cell, RefCell},
    ffi::{c_int, c_void},
};

struct Probe {
    interpreter: WeakInterp,
    current: Cell<*mut Record>,
    generation: Cell<u64>,
    calls: Cell<usize>,
    retired: RefCell<Vec<usize>>,
    active_free: Cell<bool>,
    namespace_free: Cell<bool>,
    entered: Cell<bool>,
    nested: Cell<usize>,
    nested_ok: Cell<bool>,
    deleted: Cell<bool>,
    result: Cell<*mut TclObj>,
    guest: c_int,
    mode: usize,
    fail_role: usize,
    restored: Cell<bool>,
}
struct Record {
    probe: *mut Probe,
    role: usize,
}
struct Fixture {
    interpreter: Option<Interp>,
    probe: Box<Probe>,
    records: Vec<Box<Record>>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Actual callback client data survives teardown, including assertion unwind.
        drop(self.interpreter.take());
    }
}
fn text(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
unsafe extern "C" fn invoke(
    data: *mut c_void,
    interpreter: *mut Interp,
    _argc: c_int,
    _argv: *const *mut TclObj,
) -> c_int {
    // SAFETY: Fixture retains all records through actual command retirement.
    let record = unsafe { &*data.cast::<Record>() };
    let probe = unsafe { &*record.probe };
    let interpreter = unsafe { &mut *interpreter };
    if record.role == 2 {
        let Some(generation) = interpreter.create_obj_command(
            b"active",
            ObjCommand::new(invoke, probe.current.get().cast(), Some(retire)),
        ) else {
            return 1;
        };
        probe.generation.set(generation);
        interpreter.set_result_bytes(b"active");
        return 0;
    }
    if record.role == 3 {
        probe.nested.set(probe.nested.get() + 1);
        return 0;
    }
    probe.calls.set(probe.calls.get() + 1);
    probe.deleted.set(interpreter.delete_command(b"active"));
    interpreter.set_result_bytes(b"WORKER REACHED");
    probe.guest
}
unsafe extern "C" fn retire(data: *mut c_void) {
    // SAFETY: Fixture's record storage outlives its actual interpreter commands.
    let record = unsafe { &*data.cast::<Record>() };
    let probe = unsafe { &*record.probe };
    probe.retired.borrow_mut().push(record.role);
    let Some(mut interpreter) = probe.interpreter.upgrade() else {
        return;
    };
    let active = interpreter.jim_active_command_workers.try_borrow_mut();
    let active_free = active.is_ok();
    drop(active);
    let namespace = interpreter.namespaces.try_borrow_mut();
    let namespace_free = namespace.is_ok();
    drop(namespace);
    probe
        .active_free
        .set(probe.active_free.get() && active_free);
    probe
        .namespace_free
        .set(probe.namespace_free.get() && namespace_free);
    probe
        .entered
        .set(probe.entered.get() && interpreter.entered_native_operation_count() > 0);
    // Observe broken borrows outside extern callbacks instead of a process abort.
    if !active_free || !namespace_free || interpreter.check_entered_native_operation().is_err() {
        return;
    }
    let head = text(b"retirement_observer");
    let code = interpreter.dispatch(&[head.as_ptr()]);
    probe
        .nested_ok
        .set(probe.nested_ok.get() && code == Code::Ok);
    interpreter.set_result_bytes(b"RETIREMENT REACHED");
    probe.result.set(interpreter.get_obj_result());
    if record.role == probe.fail_role {
        if probe.mode == 1 {
            interpreter.refuse_host_command("first active worker retirement");
        } else if probe.mode == 2 {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("original-retirement-world".into(), "1.0".into())];
            let a = interpreter.pin_context(&changed);
            let b = interpreter.pin_context(&original);
            probe
                .restored
                .set(a.is_ok() && b.is_ok() && interpreter.runtime_context() == original);
        }
    }
}
impl Fixture {
    fn new(previous: bool, guest: c_int, mode: usize, fail_role: usize) -> Self {
        let interpreter = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let probe = Box::new(Probe {
            interpreter: interpreter.downgrade(),
            current: Cell::new(std::ptr::null_mut()),
            generation: Cell::new(0),
            calls: Cell::new(0),
            retired: RefCell::new(Vec::new()),
            active_free: Cell::new(true),
            namespace_free: Cell::new(true),
            entered: Cell::new(true),
            nested: Cell::new(0),
            nested_ok: Cell::new(true),
            deleted: Cell::new(false),
            result: Cell::new(std::ptr::null_mut()),
            guest,
            mode,
            fail_role,
            restored: Cell::new(false),
        });
        let mut fixture = Self {
            interpreter: Some(interpreter),
            probe,
            records: Vec::new(),
        };
        for role in 0..4 {
            fixture.records.push(Box::new(Record {
                probe: &mut *fixture.probe,
                role,
            }));
        }
        let current = std::ptr::from_mut(&mut *fixture.records[0]);
        fixture.probe.current.set(current);
        let observer = std::ptr::from_mut(&mut *fixture.records[3]);
        fixture
            .interpreter
            .as_mut()
            .unwrap()
            .create_obj_command(
                b"retirement_observer",
                ObjCommand::new(invoke, observer.cast(), None),
            )
            .expect("original Jim observer registration");
        if previous {
            let old = std::ptr::from_mut(&mut *fixture.records[1]);
            fixture
                .interpreter
                .as_mut()
                .unwrap()
                .create_obj_command(b"active", ObjCommand::new(invoke, old.cast(), Some(retire)))
                .expect("original previous command");
            let installer = std::ptr::from_mut(&mut *fixture.records[2]);
            fixture
                .interpreter
                .as_mut()
                .unwrap()
                .create_obj_command(
                    b"install_current",
                    ObjCommand::new(invoke, installer.cast(), None),
                )
                .expect("actual local installer");
            let local = text(b"local");
            let install = text(b"install_current");
            assert_eq!(
                fixture
                    .interpreter
                    .as_mut()
                    .unwrap()
                    .dispatch(&[local.as_ptr(), install.as_ptr()]),
                Code::Ok
            );
            assert!(fixture
                .interpreter
                .as_ref()
                .unwrap()
                .namespaces
                .borrow()
                .jim_has_previous(fixture.probe.generation.get()));
        } else {
            let generation = fixture
                .interpreter
                .as_mut()
                .unwrap()
                .create_obj_command(
                    b"active",
                    ObjCommand::new(invoke, current.cast(), Some(retire)),
                )
                .expect("original current command");
            fixture.probe.generation.set(generation);
        }
        fixture
    }
    fn run(&mut self) -> Code {
        let head = text(b"active");
        let interpreter = self.interpreter.as_mut().unwrap();
        let (command, generation) = interpreter
            .resolve_original_command(head.as_ptr())
            .unwrap()
            .expect("original active lookup");
        assert_eq!(generation, Some(self.probe.generation.get()));
        interpreter.invoke_bound(command, generation, &[head.as_ptr()])
    }
    fn assert_cleanup(&self, previous: bool) {
        assert_eq!(self.probe.calls.get(), 1);
        assert!(self.probe.deleted.get());
        assert!(self.probe.active_free.get());
        assert!(self.probe.namespace_free.get());
        assert!(self.probe.entered.get());
        assert!(self.probe.nested_ok.get());
        assert_eq!(
            *self.probe.retired.borrow(),
            if previous { vec![0, 1] } else { vec![0] }
        );
        let interpreter = self.interpreter.as_ref().unwrap();
        assert!(interpreter.jim_active_command_workers.borrow().is_empty());
        assert!(!interpreter
            .namespaces
            .borrow()
            .jim_has_previous(self.probe.generation.get()));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert_eq!(interpreter.current_level(), 0);
        assert!(!self.probe.result.get().is_null());
        assert_eq!(interpreter.get_obj_result(), self.probe.result.get());
        assert_eq!(
            obj::bytes_of(interpreter.get_obj_result()),
            b"RETIREMENT REACHED"
        );
    }
}
#[test]
fn original_jim_active_worker_retirement_reenters_without_borrows_and_keeps_guest_codes() {
    // naming.embedding.original-host-publication-and-fact-transport

    for code in [0, 1, 2, 3, 4, 7] {
        let mut fixture = Fixture::new(false, code, 0, 0);
        assert_eq!(fixture.run(), Code::from_int(code));
        fixture.assert_cleanup(false);
        assert_eq!(fixture.probe.nested.get(), 1);
        assert!(fixture
            .interpreter
            .as_ref()
            .unwrap()
            .native_execution_refusal()
            .is_none());
    }
}
#[test]
fn original_jim_previous_workers_retire_after_current_outside_namespace_borrows() {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut fixture = Fixture::new(true, 0, 0, 0);
    assert_eq!(fixture.run(), Code::Ok);
    fixture.assert_cleanup(true);
    assert_eq!(fixture.probe.nested.get(), 2);
    assert!(fixture
        .interpreter
        .as_ref()
        .unwrap()
        .native_execution_refusal()
        .is_none());
}
#[test]
fn original_jim_final_retirement_preserves_first_host_and_restored_context_refusals() {
    // naming.embedding.original-host-publication-and-fact-transport

    for fail_role in [0, 1] {
        for mode in [1, 2] {
            for guest in [0, 1, 2, 7] {
                let mut fixture = Fixture::new(true, guest, mode, fail_role);
                assert_eq!(fixture.run(), Code::Error);
                fixture.assert_cleanup(true);
                assert_eq!(fixture.probe.nested.get(), fail_role + 1);
                let first = fixture
                    .interpreter
                    .as_ref()
                    .unwrap()
                    .native_execution_refusal()
                    .expect("actual retirement cause");
                if mode == 1 {
                    let tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first) = first
                    else {
                        panic!("original first Host channel lost");
                    };
                    assert_eq!(first.reason, "first active worker retirement");
                } else {
                    assert!(fixture.probe.restored.get());
                    assert_eq!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation")));
                }
            }
        }
    }
}
