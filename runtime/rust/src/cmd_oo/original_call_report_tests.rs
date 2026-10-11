// SPDX-License-Identifier: AGPL-3.0-or-later
//! Entered record reports and actual producer flags; source/API facts only.
use super::*;
use crate::interp::WeakInterp;
use std::cell::Cell;

fn native(version: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn source(interpreter: &mut Interp, script: &[u8]) {
    assert_eq!(interpreter.eval_str(script), Code::Ok);
    assert!(interpreter.native_execution_refusal().is_none());
}
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn elements(interpreter: &Interp, original: *mut TclObj) -> Vec<*mut TclObj> {
    interpreter.list_elements(&original).unwrap()
}
fn fields(interpreter: &Interp, original: *mut TclObj) -> Vec<Vec<u8>> {
    elements(interpreter, original)
        .into_iter()
        .map(|member| {
            interpreter
                .native_object_string_bytes(member)
                .unwrap()
                .to_vec()
        })
        .collect()
}
fn info_report(
    interpreter: &mut Interp,
    receiver: &[u8],
    selector: &[u8],
    class: bool,
) -> obj::Owned {
    let words = [
        word(b"info"),
        word(if class { b"class" } else { b"object" }),
        word(b"call"),
        word(receiver),
        word(selector),
    ];
    let pointers: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
    assert_eq!(
        if class {
            info_class(interpreter, &pointers)
        } else {
            info_object(interpreter, &pointers)
        },
        Code::Ok
    );
    obj::Owned::retain(interpreter.get_obj_result())
}
#[test]
fn original_call_report_uses_payload_descriptors_before_literal_names_and_stock_fallbacks() {
    // naming.tcloo.original-entered-call-chain-report-record-and-flags
    // docs/design/analysis/name-resolution-proofs/tcloo-original-entered-call-chain-report-record-and-flags.md
    for version in ["tcl9.0", "tcl9.1"] {
        let mut interpreter = native(version);
        source(&mut interpreter, b"oo::class create TypeClass {forward <constructor> list C; forward <destructor> list D; forward {} list EMPTY; export <constructor> <destructor> {}}; TypeClass create TypeObject; oo::define ::oo::object forward destroy list OVERRIDE");
        for name in [b"<constructor>".as_slice(), b"<destructor>", b""] {
            let report = info_report(&mut interpreter, b"TypeObject", name, false);
            let entries = elements(&interpreter, report.as_ptr());
            let row = fields(&interpreter, entries[0]);
            assert_eq!(
                row,
                [
                    b"method".to_vec(),
                    name.to_vec(),
                    b"::TypeClass".to_vec(),
                    b"forward".to_vec()
                ]
            );
            source(
                &mut interpreter,
                &[
                    b"info class methodtype TypeClass ".as_slice(),
                    if name.is_empty() { b"{}" } else { name },
                ]
                .concat(),
            );
            assert_eq!(interpreter.result_bytes(), b"forward");
        }
        let hidden = info_report(&mut interpreter, b"TypeObject", b"eval", false);
        assert_eq!(
            fields(&interpreter, elements(&interpreter, hidden.as_ptr())[0])[0],
            b"unknown"
        );
        source(&mut interpreter, b"oo::define TypeClass export eval");
        let visible = info_report(&mut interpreter, b"TypeObject", b"eval", false);
        assert_eq!(
            fields(
                &interpreter,
                *elements(&interpreter, visible.as_ptr()).last().unwrap()
            ),
            [
                b"method".to_vec(),
                b"eval".to_vec(),
                b"::oo::object".to_vec(),
                b"core method: \"eval\"".to_vec()
            ]
        );
        let report = info_report(&mut interpreter, b"TypeObject", b"destroy", false);
        assert_eq!(
            fields(&interpreter, elements(&interpreter, report.as_ptr())[0])[3],
            b"forward"
        );
        source(
            &mut interpreter,
            b"info class methodtype ::oo::Slot -append",
        );
        assert_eq!(interpreter.result_bytes(), b"core method: -append slot");
        source(
            &mut interpreter,
            b"info object methodtype ::oo::define::variable Get",
        );
        assert_eq!(
            interpreter.result_bytes(),
            b"core method: define::variable Getter"
        );
        source(
            &mut interpreter,
            b"info class methodtype ::oo::Slot Resolve",
        );
        assert_eq!(interpreter.result_bytes(), b"core method: Resolve slot");
        if version == "tcl9.1" {
            source(
                &mut interpreter,
                b"oo::configurable create PropertyType {property p}",
            );
            let report = info_report(&mut interpreter, b"PropertyType", b"<ReadProp-p>", true);
            assert_eq!(
                fields(&interpreter, elements(&interpreter, report.as_ptr())[0])[3],
                b"PropertyGetter"
            );
            source(
                &mut interpreter,
                b"info class methodtype PropertyType <WriteProp-p>",
            );
            assert_eq!(interpreter.result_bytes(), b"PropertySetter");
        }
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_call_report_keeps_filter_unknown_and_declaring_facet_flags_independent() {
    // naming.tcloo.original-entered-call-chain-report-record-and-flags
    // docs/design/analysis/name-resolution-proofs/tcloo-original-entered-call-chain-report-record-and-flags.md
    for version in ["tcl9.0", "tcl9.1"] {
        let mut interpreter = native(version);
        source(&mut interpreter, b"oo::class create FlagsClass {method m {} {if {[lindex [self call] 1] == 0} {return [next]}; return [self call]}; filter m}; FlagsClass create FlagsObject; FlagsObject m");
        let result = obj::Owned::retain(interpreter.get_obj_result());
        let outer = elements(&interpreter, result.as_ptr());
        assert_eq!(
            interpreter
                .native_object_string_bytes(outer[1])
                .unwrap()
                .as_ref(),
            b"1"
        );
        let chain = elements(&interpreter, outer[0]);
        assert_eq!(fields(&interpreter, chain[0])[0], b"filter");
        assert_eq!(fields(&interpreter, chain[1])[0], b"method");
        assert_eq!(
            elements(&interpreter, chain[0])[1],
            elements(&interpreter, chain[1])[1]
        );
        source(&mut interpreter, b"oo::define FlagsClass filter {}; oo::define FlagsClass method unknown args {return [self call]}; oo::define FlagsClass export unknown; FlagsObject missing");
        let missing = obj::Owned::retain(interpreter.get_obj_result());
        let missing_chain = elements(&interpreter, elements(&interpreter, missing.as_ptr())[0]);
        assert!(missing_chain
            .iter()
            .all(|row| fields(&interpreter, *row)[0] == b"unknown"));
        let unknown_header = interpreter
            .oo
            .borrow()
            .method_reports
            .as_ref()
            .unwrap()
            .unknown
            .as_ptr();
        assert!(missing_chain
            .iter()
            .all(|row| elements(&interpreter, *row)[0] == unknown_header));
        source(&mut interpreter, b"FlagsObject unknown literal");
        let direct = obj::Owned::retain(interpreter.get_obj_result());
        let direct_chain = elements(&interpreter, elements(&interpreter, direct.as_ptr())[0]);
        assert!(direct_chain
            .iter()
            .all(|row| fields(&interpreter, *row)[0] == b"method"));
        source(&mut interpreter, b"oo::class create BuiltinFlags {method f args {return [self call]}; filter f}; BuiltinFlags create BuiltinFlagsObject; BuiltinFlagsObject destroy");
        let builtin = obj::Owned::retain(interpreter.get_obj_result());
        let builtin_chain = elements(&interpreter, elements(&interpreter, builtin.as_ptr())[0]);
        assert_eq!(
            fields(&interpreter, *builtin_chain.last().unwrap()),
            [
                b"method".to_vec(),
                b"destroy".to_vec(),
                b"::oo::object".to_vec(),
                b"core method: \"destroy\"".to_vec()
            ]
        );
        source(
            &mut interpreter,
            b"oo::objdefine FlagsObject method m {} {return [self call]}; FlagsObject m",
        );
        let object = obj::Owned::retain(interpreter.get_obj_result());
        let row = elements(&interpreter, elements(&interpreter, object.as_ptr())[0])[0];
        assert_eq!(fields(&interpreter, row)[2], b"object");
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_call_report_keeps_exact_entered_record_through_deletion_rename_and_payload_replacement()
{
    // naming.tcloo.original-entered-call-chain-report-record-and-flags
    // docs/design/analysis/name-resolution-proofs/tcloo-original-entered-call-chain-report-record-and-flags.md
    for version in ["tcl9.0", "tcl9.1"] {
        let mut interpreter = native(version);
        source(&mut interpreter, b"oo::class create RecordClass {method m {} {oo::define RecordClass deletemethod m; oo::define RecordClass forward m list REBORN; return [self call]}}; RecordClass create RecordObject");
        let class = interpreter.oo_resolve_object(b"RecordClass");
        let (original, record) = {
            let state = interpreter.oo.borrow();
            let record = state.classes[&class].methods.owner(b"m").unwrap();
            (
                obj::NativeObjectLifetime::retain(record.original_name().unwrap()),
                Rc::downgrade(&record),
            )
        };
        source(&mut interpreter, b"RecordObject m");
        let report = obj::Owned::retain(interpreter.get_obj_result());
        let row = elements(&interpreter, elements(&interpreter, report.as_ptr())[0])[0];
        assert_eq!(elements(&interpreter, row)[1], original.as_ptr());
        assert_eq!(fields(&interpreter, row)[3], b"method");
        assert!(record.upgrade().is_none());
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        let current = interpreter.oo.borrow().classes[&class]
            .methods
            .owner(b"m")
            .unwrap();
        assert_ne!(current.original_name(), Some(original.as_ptr()));
        assert_eq!(
            current.borrow().report_type(),
            NativeTclooMethodPayloadType::Forward
        );
        drop(current);
        source(&mut interpreter, b"oo::define RecordClass method m {} {oo::define RecordClass renamemethod m moved; oo::define RecordClass forward moved list REPLACED; oo::define RecordClass method m {} {return NEW}; return [self call]}");
        let before = interpreter.oo.borrow().classes[&class]
            .methods
            .owner(b"m")
            .unwrap();
        let before = Rc::downgrade(&before);
        source(&mut interpreter, b"RecordObject m");
        let report = obj::Owned::retain(interpreter.get_obj_result());
        let row = elements(&interpreter, elements(&interpreter, report.as_ptr())[0])[0];
        assert_eq!(
            fields(&interpreter, row),
            [
                b"method".to_vec(),
                b"moved".to_vec(),
                b"::RecordClass".to_vec(),
                b"forward".to_vec()
            ]
        );
        let retained = before.upgrade().unwrap();
        let moved = interpreter.oo.borrow().classes[&class]
            .methods
            .owner(b"moved")
            .unwrap();
        assert!(Rc::ptr_eq(&retained, &moved));
        assert_eq!(
            elements(&interpreter, row)[1],
            moved.original_name().unwrap()
        );
        source(&mut interpreter, b"oo::define RecordClass method m {} {oo::define RecordClass private method m {} {return PRIVATE}; return [self call]}; RecordObject m");
        let report = obj::Owned::retain(interpreter.get_obj_result());
        let row = elements(&interpreter, elements(&interpreter, report.as_ptr())[0])[0];
        assert_eq!(fields(&interpreter, row)[0], b"private");
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static UPDATES: Cell<usize> = const { Cell::new(0) };
    static MODE: Cell<u8> = const { Cell::new(0) };
}
extern "C" fn report_name_update(original: *mut TclObj) {
    UPDATES.with(|count| count.set(count.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original call report presentation");
        }
        2 => {
            let context = interpreter.runtime_context();
            let mut changed = context.clone();
            changed
                .packages
                .push(("call-report-presentation".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&context).unwrap();
        }
        _ => {}
    }
    unsafe { obj::set_native_updater_string_rep(original, b"lazyReportName", false) };
}
static REPORT_NAME: obj::TclObjType = obj::TclObjType {
    name: c"originalCallReportName".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(report_name_update),
    set_from_any_proc: None,
};
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}
#[test]
fn original_call_report_retains_original_name_until_checked_presentation_and_first_cause() {
    // naming.tcloo.original-entered-call-chain-report-record-and-flags
    // docs/design/analysis/name-resolution-proofs/tcloo-original-entered-call-chain-report-record-and-flags.md
    for version in ["tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2] {
            let mut interpreter = native(version);
            source(
                &mut interpreter,
                b"oo::class create LazyClass; LazyClass create LazyObject",
            );
            OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
            let _cleanup = Cleanup;
            UPDATES.with(|count| count.set(0));
            MODE.with(|selected| selected.set(0));
            let original = obj::Owned::fresh(obj::alloc_typed(&REPORT_NAME, 0));
            let definition = [word(b"oo::define"), word(b"LazyClass"), word(b"")];
            let tape: Vec<_> = definition.iter().map(obj::Owned::as_ptr).collect();
            let class = interpreter.oo_resolve_object(b"LazyClass");
            let caller = interpreter
                .oo_enter_definition(&DefTarget::Class(class), &tape)
                .unwrap();
            let head = word(b"forward");
            let list = word(b"list");
            assert_eq!(
                def_forward(
                    &mut interpreter,
                    &[head.as_ptr(), original.as_ptr(), list.as_ptr()]
                ),
                Code::Ok
            );
            interpreter.oo_leave_definition(caller);
            assert_eq!(UPDATES.with(Cell::get), 1);
            obj::invalidate_string(original.as_ptr());
            UPDATES.with(|count| count.set(0));
            MODE.with(|selected| selected.set(mode));
            let report = info_report(&mut interpreter, b"LazyObject", b"lazyReportName", false);
            assert_eq!(
                UPDATES.with(Cell::get),
                0,
                "report derivation does not ask for String"
            );
            assert!(!obj::has_string_rep(original.as_ptr()));
            let entries = elements(&interpreter, report.as_ptr());
            assert_eq!(elements(&interpreter, entries[0])[1], original.as_ptr());
            assert_eq!(
                unsafe { (*original.as_ptr()).ref_count },
                4,
                "caller, record, table key and returned List member"
            );
            assert_eq!(
                obj::obj_type_ptr(original.as_ptr()),
                &REPORT_NAME as *const obj::TclObjType
            );
            let reached = interpreter.native_object_string_bytes(report.as_ptr());
            assert_eq!(reached.is_ok(), mode == 0);
            assert_eq!(UPDATES.with(Cell::get), 1);
            assert!(obj::has_string_rep(original.as_ptr()));
            if mode == 1 {
                assert!(
                    matches!(interpreter.native_execution_refusal(),Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) if first.reason=="first original call report presentation")
                );
            } else if mode == 2 {
                assert!(matches!(interpreter.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation")))));
            } else {
                assert!(interpreter.native_execution_refusal().is_none());
            }
            assert_eq!(interpreter.get_obj_result(), report.as_ptr());
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}
