// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual property reporting getters, cache owners and original operation checks.
use super::*;
use crate::interp::WeakInterp;
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static TARGET: Cell<OoId> = const { Cell::new(OoId(0)) };
    static UPDATES: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static FREES: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static REACHED: RefCell<Option<obj::Owned>> = const { RefCell::new(None) };
}
fn callback(first: bool) {
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    if first {
        match MODE.with(Cell::get) {
            1 => {
                interpreter.refuse_host_command("first property report callback");
            }
            2 => {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed
                    .packages
                    .push(("property-report".into(), "1.0".into()));
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
            3 => {
                interpreter
                    .native_property_method_created(TARGET.with(Cell::get), false)
                    .unwrap();
            }
            4 => {
                let class = interpreter.oo_resolve_object(b"C");
                let before = interpreter.oo.borrow().native_methods.mutation_stamp(class);
                assert_eq!(
                    interpreter.eval_str(b"oo::define C {private variable -set replacement}"),
                    Code::Ok
                );
                let after = interpreter.oo.borrow().native_methods.mutation_stamp(class);
                assert!(Rc::ptr_eq(&before.0, &after.0));
                assert_eq!((before.1, before.2), (after.1, after.2));
                interpreter.set_result_bytes(b"REACHED PRIVATE DECLARATION CALLBACK");
                REACHED.with(|result| {
                    *result.borrow_mut() = Some(obj::Owned::retain(interpreter.get_obj_result()))
                });
            }
            _ => {}
        }
    }
}
extern "C" fn update(original: *mut TclObj) {
    let role = obj::internal_rep(original);
    let first = UPDATES.with(|updates| {
        let mut updates = updates.borrow_mut();
        updates.push(role);
        updates.len() == 1
    });
    callback(first);
    unsafe { obj::set_string_rep(original, if role % 2 == 0 { b"-a" } else { b"-z" }) };
}
extern "C" fn free(original: *mut TclObj) {
    let role = obj::internal_rep(original);
    let first = FREES.with(|frees| {
        let mut frees = frees.borrow_mut();
        frees.push(role);
        frees.len() == 1
    });
    callback(first);
}
static MEMBER: obj::TclObjType = obj::TclObjType {
    name: c"originalPropertyReport".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn native() -> (Interp, OoId, OoId) {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(b"oo::class create ReportClass; ReportClass create ReportObject"),
        Code::Ok
    );
    let class = interpreter.oo_resolve_object(b"ReportClass");
    let target = interpreter.oo_resolve_object(b"ReportObject");
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    TARGET.with(|slot| slot.set(target));
    UPDATES.with(|updates| updates.borrow_mut().clear());
    FREES.with(|frees| frees.borrow_mut().clear());
    (interpreter, class, target)
}
fn members(interpreter: &mut Interp, target: OoId, class: bool) -> Vec<obj::Owned> {
    let members: Vec<_> = (0..2)
        .map(|role| {
            let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, role));
            unsafe { obj::set_string_rep(member.as_ptr(), if role == 0 { b"-a" } else { b"-z" }) };
            member
        })
        .collect();
    let originals: Vec<_> = members.iter().map(obj::Owned::as_ptr).collect();
    let input = obj::Owned::fresh(interpreter.new_list_object(&originals));
    native_properties::set_property_slot(interpreter, target, class, false, input.as_ptr())
        .unwrap();
    drop(input);
    members
}
fn first(interpreter: &Interp, mode: u8) {
    match mode {
        1 => assert!(
            matches!(interpreter.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) if first.reason == "first property report callback")
        ),
        2 => assert!(interpreter.native_execution_refusal().is_some()),
        _ => assert!(interpreter.native_execution_refusal().is_none()),
    }
}
#[test]
fn original_property_sort_stops_after_first_callback_without_cache_or_ref_inflation() {
    // Callback safety is a software owner control; native sort chronology is separate.
    for declared in [false, true] {
        for mode in 0..4 {
            let (mut interpreter, _, target) = native();
            let members = members(&mut interpreter, target, false);
            for member in &members {
                obj::invalidate_string(member.as_ptr());
                assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 2);
            }
            interpreter.set_result_bytes(b"PRIOR");
            let prior = obj::Owned::retain(interpreter.get_obj_result());
            MODE.with(|slot| slot.set(mode));
            let outcome = if declared {
                interpreter.native_declared_property_header(target, false, false)
            } else {
                interpreter.native_all_property_header(target, false, false)
            };
            assert_eq!(outcome.is_ok(), mode == 0);
            let updates = UPDATES.with(|updates| updates.borrow().clone());
            if mode == 0 {
                assert_eq!(updates.len(), 2);
                let header = outcome.unwrap();
                let strings = interpreter
                    .native_invocation_dialect()
                    .native_property_lookup_protocol()
                    .unwrap()
                    .strings();
                let list =
                    crate::list::list_elements_native_checked(header.header, strings).unwrap();
                assert_eq!(
                    list,
                    members.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()
                );
                for member in &members {
                    assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 3);
                }
                assert!(FREES.with(|frees| frees.borrow().is_empty()));
                drop(header);
            } else {
                assert_eq!(updates.len(), 1);
                assert!(!interpreter
                    .oo
                    .borrow()
                    .property_caches
                    .contains_key(&(target, false)));
                for member in &members {
                    assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 2);
                }
            }
            assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
            first(&interpreter, mode);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            MODE.with(|slot| slot.set(0));
            interpreter.retire_native_properties(target).ok();
            OWNER.with(|owner| *owner.borrow_mut() = None);
            // Caller holds retire after their authentic metadata roles have gone.
            drop(members);
        }
    }
}
#[test]
fn original_stale_property_cache_retirement_refuses_before_rebuilt_cache_adoption() {
    // Existing derived cache and real class slot mutation issue every role.
    for mode in 0..3 {
        let (mut interpreter, class, target) = native();
        let old = members(&mut interpreter, class, true);
        assert_eq!(
            interpreter.eval_str(b"info object properties ReportObject -all -readable"),
            Code::Ok
        );
        let old_header = interpreter.get_obj_result();
        let lease = obj::NativeObjectLifetime::retain(old_header);
        interpreter.reset_result();
        drop(old);
        let new = obj::Owned::fresh(obj::new_string_bytes(b"-new"));
        let input = obj::Owned::fresh(interpreter.new_list_object(&[new.as_ptr()]));
        native_properties::set_property_slot(&mut interpreter, class, true, false, input.as_ptr())
            .unwrap();
        drop(input);
        assert!(FREES.with(|frees| frees.borrow().is_empty()));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = obj::Owned::retain(interpreter.get_obj_result());
        MODE.with(|slot| slot.set(mode));
        let outcome = interpreter.native_all_property_header(target, false, false);
        assert_eq!(outcome.is_ok(), mode == 0);
        assert_eq!(FREES.with(|frees| frees.borrow().clone()), vec![0, 1]);
        assert!(!obj::allocation_is_live(old_header));
        if mode != 0 {
            assert!(!interpreter
                .oo
                .borrow()
                .property_caches
                .contains_key(&(target, false)));
            assert_eq!(unsafe { (*new.as_ptr()).ref_count }, 2);
        }
        drop(outcome);
        assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
        first(&interpreter, mode);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        drop(lease);
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}

struct ReportOutputHost {
    actual: Rc<dyn tcl_platform::Host>,
    stdout: RefCell<Vec<u8>>,
    stderr: RefCell<Vec<u8>>,
}
impl tcl_platform::StdIo for ReportOutputHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.stdout.borrow_mut().extend_from_slice(bytes);
    }
    fn write_stderr(&self, bytes: &[u8]) {
        self.stderr.borrow_mut().extend_from_slice(bytes);
    }
}
impl tcl_platform::Host for ReportOutputHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.actual.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.actual.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.actual.native_integer_formatter()
    }
    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.actual.system_encoding()
    }
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        self.actual.filesystem()
    }
    fn sockets(&self) -> Option<&dyn tcl_platform::Sockets> {
        self.actual.sockets()
    }
    fn process(&self) -> Option<&dyn tcl_platform::Process> {
        self.actual.process()
    }
}
#[test]
fn materialized_property_reports_preserve_native_snapshot_and_table_index_roles() {
    // Exact ordinary scripts measured independently on C9.0.4 and C9.1.0.
    // This physical adapter comparison is restricted to its selected C9.1 purpose.
    {
        let source = br#"set events {}
oo::configurable create C {
    property a -get {return A} -set {
        lappend ::events a
        oo::define C {property b -get {return B} -set {lappend ::events b}}
    }
    property z -get {return Z} -set {lappend ::events z}
}
C create o
set code [catch {o configure -a ONE -z TWO} result options]
puts [list code $code result $result events $events properties [info object properties o -all -writable]]
"#;
        let expected = b"code 0 result {} events {a b} properties {-a -b -z}\n";
        let host = Rc::new(ReportOutputHost {
            actual: crate::interp::default_host(),
            stdout: RefCell::new(Vec::new()),
            stderr: RefCell::new(Vec::new()),
        });
        let mut interpreter = Interp::with_native_core(
            host.clone(),
            crate::environment::profile_for_dialect("tcl9.1"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interpreter.eval_str(source), Code::Ok);
        assert_eq!(host.stdout.borrow().as_slice(), expected);
        assert!(host.stderr.borrow().is_empty());
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
    {
        let source = br#"set events {}
oo::configurable create C {
    property a -get {
        lappend ::events a
        oo::define C {
            property b -get {lappend ::events b; return B}
            method unrelated {} {return unchanged}
        }
        return A
    }
    property z -get {lappend ::events z; return Z}
}
C create o
set code [catch {o configure} result options]
puts [list code $code result $result events $events properties [info object properties o -all -readable]]
"#;
        let expected = b"code 0 result {-a A -z Z} events {a z} properties {-a -b -z}\n";
        let host = Rc::new(ReportOutputHost {
            actual: crate::interp::default_host(),
            stdout: RefCell::new(Vec::new()),
            stderr: RefCell::new(Vec::new()),
        });
        let mut interpreter = Interp::with_native_core(
            host.clone(),
            crate::environment::profile_for_dialect("tcl9.1"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interpreter.eval_str(source), Code::Ok);
        assert_eq!(host.stdout.borrow().as_slice(), expected);
        assert!(host.stderr.borrow().is_empty());
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
extern "C" fn update_variable(original: *mut TclObj) {
    let first = UPDATES.with(|updates| {
        let mut updates = updates.borrow_mut();
        updates.push(9);
        updates.len() == 1
    });
    callback(first);
    unsafe { obj::set_string_rep(original, b"a") };
}
static VARIABLE: obj::TclObjType = obj::TclObjType {
    name: c"originalPrivatePropertyVariable".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_variable),
    set_from_any_proc: None,
};
#[test]
fn private_property_comparison_rejects_changed_original_declaration_inventory() {
    // The real declaration slot does not change method-world mutation facts.
    for changed in [false, true] {
        let (mut interpreter, _, _) = native();
        assert_eq!(
            interpreter.eval_str(b"oo::configurable create C {property a; method writePrivate {value} {my configure -a $value}; method readPrivate {} {my configure -a}}; C create o"),
            Code::Ok
        );
        let declaration = obj::Owned::fresh(obj::alloc_typed(&VARIABLE, 0));
        unsafe { obj::set_string_rep(declaration.as_ptr(), b"a") };
        let words: Vec<_> = [b"oo::define".as_slice(), b"C", b"private", b"variable"]
            .into_iter()
            .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
            .collect();
        let mut argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
        argv.push(declaration.as_ptr());
        assert_eq!(interpreter.dispatch(&argv), Code::Ok);
        assert_eq!(interpreter.eval_str(b"o writePrivate VALUE"), Code::Ok);
        obj::invalidate_string(declaration.as_ptr());
        REACHED.with(|result| *result.borrow_mut() = None);
        UPDATES.with(|updates| updates.borrow_mut().clear());
        MODE.with(|mode| mode.set(if changed { 4 } else { 0 }));
        let code = interpreter.eval_str(b"o readPrivate");
        assert_eq!(UPDATES.with(|updates| updates.borrow().clone()), [9]);
        if changed {
            assert_eq!(code, Code::Error);
            assert!(interpreter.native_execution_refusal().is_some());
            REACHED.with(|result| {
                assert_eq!(
                    interpreter.get_obj_result(),
                    result.borrow().as_ref().unwrap().as_ptr()
                )
            });
            let class = interpreter.oo_resolve_object(b"C");
            assert!(interpreter.oo.borrow().classes[&class]
                .private_variables
                .iter()
                .any(|variable| variable.name == b"replacement"));
        } else {
            assert_eq!(code, Code::Ok);
            assert_eq!(interpreter.result_bytes(), b"VALUE");
            assert!(interpreter.native_execution_refusal().is_none());
        }
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        MODE.with(|mode| mode.set(0));
        OWNER.with(|owner| *owner.borrow_mut() = None);
        REACHED.with(|result| *result.borrow_mut() = None);
    }
}
