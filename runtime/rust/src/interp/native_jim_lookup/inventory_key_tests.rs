// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const ORIGINAL: &str = include_str!(
    "../../../tests/data/native_jim_command_inventory/original-key-objects-20261015.tsv"
);

fn references(original: *mut TclObj) -> i64 {
    // SAFETY: each observer owns an actual native reference to this header.
    unsafe { (*original).ref_count as i64 }
}

fn primary(original: *mut TclObj) -> Vec<u8> {
    let descriptor = obj::obj_type_ptr(original);
    if descriptor.is_null() {
        b"none".to_vec()
    } else {
        // SAFETY: every actual live type descriptor retains its static C name.
        unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
            .to_bytes()
            .to_vec()
    }
}

#[test]
fn original_jim_command_inventory_keeps_key_identity_lazy_headers_and_native_references() {
    assert_eq!(ORIGINAL.lines().count(), 24);
    for row in ORIGINAL.lines() {
        let fields = row.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 14);
        let kind = fields[1];
        let all = fields[2] == "1";
        let mode = fields[3].parse::<usize>().unwrap();
        let mut interp = Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interp.eval_str(b"proc 123 {} {}; proc 456 {} {}; alias 789 123"),
            Code::Ok
        );
        let name = if kind == "aliases" { b"789" } else { b"123" };
        let candidates = interp
            .namespaces()
            .original_jim_command_inventory_keys(
                tcl_runtime_api::NativeJimCommandInventoryKind::Commands,
                &|_, _| true,
            )
            .unwrap();
        let key = candidates.iter().find(|key| key.report == name).unwrap();
        let original = Owned::retain(key.original().unwrap());
        assert_eq!(
            crate::typed_value::native_wide_int(
                original.as_ptr(),
                interp.native_invocation_dialect()
            ),
            Ok(if kind == "aliases" { 789 } else { 123 })
        );
        obj::invalidate_string(original.as_ptr());
        assert_eq!(
            u8::from(obj::has_string_rep(original.as_ptr())).to_string(),
            fields[5],
            "{row}"
        );
        assert_eq!(
            references(original.as_ptr()).to_string(),
            fields[7],
            "{row}"
        );
        assert_eq!(primary(original.as_ptr()), fields[6].as_bytes(), "{row}");
        let mut words = vec![
            Owned::fresh(obj::new_string_bytes(b"info")),
            Owned::fresh(obj::new_string_bytes(b"-nons")),
            Owned::fresh(obj::new_string_bytes(kind.as_bytes())),
        ];
        if all {
            words.push(Owned::fresh(obj::new_string_bytes(b"-all")));
        }
        if mode != 0 {
            let pattern = match mode {
                1 => name.as_slice(),
                2 => b"456",
                _ if kind == "aliases" => b"7*",
                _ => b"1*",
            };
            words.push(Owned::fresh(obj::new_string_bytes(pattern)));
        }
        let argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
        let code = interp.dispatch(&argv);
        assert_eq!(
            code.as_int().to_string(),
            fields[4],
            "{row}: {:?}",
            interp.native_execution_refusal()
        );
        assert_eq!(
            u8::from(obj::has_string_rep(original.as_ptr())).to_string(),
            fields[8],
            "{row}"
        );
        assert_eq!(
            references(original.as_ptr()).to_string(),
            fields[10],
            "{row}"
        );
        assert_eq!(primary(original.as_ptr()), fields[9].as_bytes(), "{row}");
        let backing = crate::list::native_list_backing(interp.result_obj()).unwrap();
        let found = backing
            .elements()
            .unwrap()
            .iter()
            .filter(|&&member| member == original.as_ptr())
            .count();
        assert_eq!(found.to_string(), fields[11], "{row}");
        // The original SDK's complete registered roster is a separate premise.
        // These cells compare the selected key's actual ownership and getters.
        drop(backing);
        interp.set_result_bytes(b"");
        assert_eq!(
            references(original.as_ptr()).to_string(),
            fields[13],
            "{row}"
        );
        assert!(interp.native_execution_refusal().is_none(), "{row}");
    }
}

#[test]
fn completed_inventory_receipts_keep_original_list_correspondence_without_native_roles() {
    let mut interpreter = Interp::with_native_core(
        super::super::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(b"proc snapshot_member {} {}"),
        Code::Ok
    );
    let operation =
        super::super::native_operation_currency::NativeOperationScope::enter(&interpreter).unwrap();
    let keys = interpreter
        .namespaces()
        .original_jim_command_inventory_keys(
            tcl_runtime_api::NativeJimCommandInventoryKind::Commands,
            &|_, _| true,
        )
        .unwrap();
    let key = keys
        .into_iter()
        .find(|key| key.report == b"snapshot_member")
        .unwrap();
    let original = key.original().unwrap();
    let result = Owned::fresh(interpreter.new_list_object(&[original]));
    let count = references(original);
    let receipt =
        OriginalJimCommandInventoryReturn::capture(&interpreter, result.as_ptr(), vec![key])
            .unwrap();
    assert_eq!(references(original), count);
    receipt.check(&interpreter, result.as_ptr()).unwrap();
    let copy = Owned::fresh(obj::duplicate(result.as_ptr()));
    assert!(receipt.check(&interpreter, copy.as_ptr()).is_err());
    assert_eq!(references(original), count + 1);
    drop(copy);
    assert_eq!(references(original), count);
    crate::list::append_prepared_native_elements(
        result.as_ptr(),
        &[original],
        tcl_syntax::native_string::NativeStringProtocol::Jim084,
    )
    .unwrap();
    assert!(receipt.check(&interpreter, result.as_ptr()).is_err());
    drop(receipt);
    assert_eq!(references(original), count + 1);
    operation.currency().ensure_current_or_refuse().unwrap();
    assert!(interpreter.native_execution_refusal().is_none());
}

#[test]
fn completed_inventory_data_crosses_frames_but_keeps_its_actual_interpreter() {
    let mut actual = Interp::with_native_core(
        super::super::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let foreign = Interp::with_native_core(
        super::super::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(actual.eval_str(b"proc original_data {} {}"), Code::Ok);
    let operation =
        super::super::native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
    let key = actual
        .namespaces()
        .original_jim_command_inventory_keys(
            tcl_runtime_api::NativeJimCommandInventoryKind::Commands,
            &|_, _| true,
        )
        .unwrap()
        .into_iter()
        .find(|key| key.report == b"original_data")
        .unwrap();
    let result = Owned::fresh(actual.new_list_object(&[key.original().unwrap()]));
    let receipt =
        OriginalJimCommandInventoryReturn::capture(&actual, result.as_ptr(), vec![key]).unwrap();
    actual.frames.borrow_mut().push(crate::namespace::GLOBAL);
    receipt.check(&actual, result.as_ptr()).unwrap();
    assert_eq!(
        actual.frames.borrow_mut().pop(),
        Some(crate::namespace::GLOBAL)
    );
    let foreign_operation =
        super::super::native_operation_currency::NativeOperationScope::enter(&foreign).unwrap();
    assert!(receipt.check(&foreign, result.as_ptr()).is_err());
    assert!(crate::native_source::bind_context(
        result.as_ptr(),
        &foreign.native_jim_object_context().unwrap()
    )
    .is_err());
    foreign_operation
        .currency()
        .ensure_current_or_refuse()
        .unwrap();
    drop(foreign_operation);
    receipt.check(&actual, result.as_ptr()).unwrap();
    operation.currency().ensure_current_or_refuse().unwrap();
    assert!(actual.native_execution_refusal().is_none());
    assert!(foreign.native_execution_refusal().is_none());
}

mod variable_helper_head_tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    thread_local! {
        static EXPECTED: Cell<[*mut TclObj; 2]> = const { Cell::new([std::ptr::null_mut(); 2]) };
        static CALLS: Cell<usize> = const { Cell::new(0) };
        static MODE: Cell<u8> = const { Cell::new(0) };
        static ACTUAL: RefCell<Option<Interp>> = const { RefCell::new(None) };
        static NAMESPACE_GETTERS: Cell<usize> = const { Cell::new(0) };
        static PATTERN_GETTERS: Cell<usize> = const { Cell::new(0) };
        static FIRST: RefCell<Option<tcl_runtime_api::NativeExecutionError>> = const { RefCell::new(None) };
    }
    fn descriptor(actual: &Interp) -> tcl_registry::native_jim_lookup::NativeJimNamespaceHelper {
        actual
            .native_invocation_dialect()
            .native_jim_namespace_helper(
                tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Info,
            )
            .unwrap()
    }
    fn replacement(actual: &mut Interp, argv: &[*mut TclObj]) -> Code {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        assert_eq!(argv.len(), 3);
        assert_eq!(obj::bytes_of(argv[0]), descriptor(actual).name().as_bytes());
        assert_eq!(&argv[1..], &EXPECTED.with(Cell::get));
        assert!(!obj::has_string_rep(argv[2]));
        assert_eq!(PATTERN_GETTERS.with(Cell::get), 0);
        match MODE.with(Cell::get) {
            0 => {
                actual.set_result(argv[2]);
                Code::Ok
            }
            1 => {
                actual.set_result_bytes(b"REPLACEMENT ABRUPT");
                Code::Other(7)
            }
            2 => {
                actual.set_result_bytes(b"REPLACEMENT FIRST CAUSE");
                actual.refuse_host_command("original namespace-info replacement first cause");
                FIRST.with(|first| *first.borrow_mut() = actual.native_execution_refusal());
                Code::Error
            }
            3 => {
                actual.set_result_bytes(b"REPLACEMENT NAMESPACE EFFECT");
                actual.set_current_ns(crate::namespace::GLOBAL);
                Code::Ok
            }
            _ => unreachable!(),
        }
    }
    extern "C" fn namespace_updater(original: *mut TclObj) {
        NAMESPACE_GETTERS.with(|calls| calls.set(calls.get() + 1));
        let mut actual = ACTUAL.with(|owner| owner.borrow().as_ref().unwrap().clone());
        let helper = descriptor(&actual);
        actual.register_builtin(helper.name().as_bytes(), replacement);
        // SAFETY: this actual extension updater owns its original String slot.
        unsafe { obj::set_native_updater_string_rep(original, b"42", false) };
    }
    extern "C" fn pattern_updater(original: *mut TclObj) {
        PATTERN_GETTERS.with(|calls| calls.set(calls.get() + 1));
        unsafe { obj::set_native_updater_string_rep(original, b"target\0counted", false) };
    }
    static NAMESPACE: obj::TclObjType = obj::TclObjType {
        name: c"selectedNamespaceInfoMutableHelper".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(namespace_updater),
        set_from_any_proc: None,
    };
    static PATTERN: obj::TclObjType = obj::TclObjType {
        name: c"selectedNamespaceInfoOriginalPattern".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(pattern_updater),
        set_from_any_proc: None,
    };
    struct Cleanup;
    impl Drop for Cleanup {
        fn drop(&mut self) {
            ACTUAL.with(|owner| owner.borrow_mut().take());
            EXPECTED.with(|expected| expected.set([std::ptr::null_mut(); 2]));
        }
    }
    fn native() -> (Interp, Cleanup) {
        ACTUAL.with(|owner| owner.borrow_mut().take());
        CALLS.with(|calls| calls.set(0));
        NAMESPACE_GETTERS.with(|calls| calls.set(0));
        PATTERN_GETTERS.with(|calls| calls.set(0));
        MODE.with(|mode| mode.set(0));
        FIRST.with(|first| first.borrow_mut().take());
        let actual = Interp::with_native_core(
            super::super::super::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        ACTUAL.with(|owner| *owner.borrow_mut() = Some(actual.clone()));
        (actual, Cleanup)
    }
    fn invoke(actual: &mut Interp, namespace: &Owned, words: &[*mut TclObj]) -> Code {
        let body = Owned::fresh(actual.new_list_object(words));
        let context = actual.native_jim_object_context().unwrap();
        crate::native_source::bind_context(namespace.as_ptr(), &context).unwrap();
        for &word in words {
            crate::native_source::bind_context(word, &context).unwrap();
        }
        let namespace = actual
            .namespaces_mut()
            .retain_unmaterialized_jim_namespace(Owned::retain(namespace.as_ptr()));
        let code = actual.ns_eval_in_token(
            namespace,
            &[],
            None,
            false,
            Some((
                tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
                body.as_ptr(),
            )),
            None,
        );
        assert_eq!(actual.entered_native_operation_count(), 0);
        code
    }

    #[test]
    fn selected_namespace_info_helper_head_keeps_original_argv_and_mutable_lookup() {
        // Genuine extension controls preserve successful ordinary replacement dispatch.
        for member in [b"vars".as_slice(), b"globals", b"locals"] {
            let (mut actual, _cleanup) = native();
            let helper = descriptor(&actual);
            let installed = actual.resolve_cmd_token(helper.name().as_bytes()).unwrap();
            let namespace = Owned::fresh(obj::alloc_typed(&NAMESPACE, 0));
            let head = Owned::fresh(obj::new_string_bytes(b"info"));
            let selector = Owned::fresh(obj::new_string_bytes(member));
            let pattern = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
            EXPECTED.with(|expected| expected.set([selector.as_ptr(), pattern.as_ptr()]));
            assert_eq!(
                invoke(
                    &mut actual,
                    &namespace,
                    &[head.as_ptr(), selector.as_ptr(), pattern.as_ptr()]
                ),
                Code::Ok
            );
            assert_eq!(actual.result_obj(), pattern.as_ptr());
            assert_eq!(CALLS.with(Cell::get), 1);
            assert_eq!(NAMESPACE_GETTERS.with(Cell::get), 1);
            assert_eq!(PATTERN_GETTERS.with(Cell::get), 0);
            assert!(!obj::has_string_rep(pattern.as_ptr()));
            assert_ne!(
                actual.resolve_cmd_token(helper.name().as_bytes()),
                Some(installed)
            );
            assert!(actual.native_execution_refusal().is_none());
        }
    }

    #[test]
    fn selected_namespace_info_helper_head_keeps_actual_abrupt_first_cause_and_owner_checks() {
        for mode in [1, 2, 3] {
            let (mut actual, _cleanup) = native();
            MODE.with(|selected| selected.set(mode));
            let helper = descriptor(&actual);
            actual.register_builtin(helper.name().as_bytes(), replacement);
            let namespace = Owned::fresh(obj::new_string_bytes(b"42"));
            let head = Owned::fresh(obj::new_string_bytes(b"info"));
            let selector = Owned::fresh(obj::new_string_bytes(b"vars"));
            let pattern = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
            EXPECTED.with(|expected| expected.set([selector.as_ptr(), pattern.as_ptr()]));
            let code = invoke(
                &mut actual,
                &namespace,
                &[head.as_ptr(), selector.as_ptr(), pattern.as_ptr()],
            );
            assert_eq!(CALLS.with(Cell::get), 1);
            assert_eq!(PATTERN_GETTERS.with(Cell::get), 0);
            assert!(!obj::has_string_rep(pattern.as_ptr()));
            if mode == 1 {
                assert_eq!(code, Code::Other(7));
                assert_eq!(actual.result_bytes(), b"REPLACEMENT ABRUPT");
                assert!(actual.native_execution_refusal().is_none());
            } else {
                assert_eq!(code, Code::Error);
                assert!(actual.native_execution_refusal().is_some());
                if mode == 2 {
                    assert_eq!(
                        FIRST.with(|first| first.borrow().clone()),
                        actual.native_execution_refusal()
                    );
                    assert_eq!(actual.result_bytes(), b"REPLACEMENT FIRST CAUSE");
                } else {
                    assert_eq!(actual.result_bytes(), b"REPLACEMENT NAMESPACE EFFECT");
                }
            }
        }
    }

    #[test]
    fn selected_namespace_info_helper_descriptor_does_not_revive_a_removed_binding() {
        let (mut actual, _cleanup) = native();
        let helper = descriptor(&actual);
        actual.rename_command(helper.name().as_bytes(), b"");
        assert!(actual.resolve_cmd_token(helper.name().as_bytes()).is_none());
        let namespace = Owned::fresh(obj::new_string_bytes(b"42"));
        let head = Owned::fresh(obj::new_string_bytes(b"info"));
        let selector = Owned::fresh(obj::new_string_bytes(b"vars"));
        let pattern = Owned::fresh(obj::new_string_bytes(b"*"));
        assert_eq!(
            invoke(
                &mut actual,
                &namespace,
                &[head.as_ptr(), selector.as_ptr(), pattern.as_ptr()]
            ),
            Code::Error
        );
        assert_eq!(CALLS.with(Cell::get), 0);
        assert!(actual.resolve_cmd_token(helper.name().as_bytes()).is_none());
        assert!(actual.native_execution_refusal().is_none());
    }
}
