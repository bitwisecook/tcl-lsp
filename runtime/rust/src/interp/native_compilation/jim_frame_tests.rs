// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn observe(interpreter: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // SAFETY: the actual command argv retains this live variadic List.
    let count = unsafe { (*argv[1]).ref_count };
    let frames = interpreter.frames.borrow();
    assert!(frames.current_c_procedure().is_none());
    let owner = frames.current_original_jim_procedure().unwrap().unwrap();
    assert_eq!(
        frames.current_original_jim_variadic().unwrap(),
        Some(argv[1])
    );
    assert!(owner
        .original_jim_import_origin(interpreter)
        .unwrap()
        .is_none());
    // SAFETY: the same original argv is still entered.
    assert_eq!(unsafe { (*argv[1]).ref_count }, count);
    drop(frames);
    interpreter.set_result(argv[1]);
    Code::Ok
}

#[test]
fn entered_original_jim_frame_observes_its_actual_declaration_and_variadic_list() {
    let mut interpreter = Interp::with_native_core(
        default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    interpreter.register_builtin(b"observe_original_frame", observe);
    assert_eq!(
        interpreter.eval_str(b"proc original_frame args {observe_original_frame $args}"),
        Code::Ok
    );
    let command = obj::Owned::fresh(obj::new_string_bytes(b"original_frame"));
    let first = obj::Owned::fresh(obj::new_wide_int_obj(12));
    let second = obj::Owned::fresh(obj::new_wide_int_obj(34));
    assert_eq!(
        interpreter.dispatch(&[command.as_ptr(), first.as_ptr(), second.as_ptr()]),
        Code::Ok
    );
    let result = crate::list::native_list_backing(interpreter.result_obj()).unwrap();
    assert_eq!(
        &*result.elements().unwrap(),
        &[first.as_ptr(), second.as_ptr()]
    );
    let frames = interpreter.frames.borrow();
    assert!(frames.current_c_procedure().is_none());
    assert!(frames.current_original_jim_procedure().unwrap().is_none());
    assert!(frames.current_original_jim_variadic().unwrap().is_none());
    assert!(interpreter.native_execution_refusal().is_none());
}

#[test]
fn original_list_backing_receipts_reject_equal_copies_mutation_and_shimmer() {
    let interpreter = Interp::with_native_core(
        default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let member = obj::Owned::fresh(obj::new_wide_int_obj(42));
    let original = obj::Owned::fresh(interpreter.new_list_object(&[member.as_ptr()]));
    interpreter
        .associate_native_jim_arguments(&[original.as_ptr()])
        .unwrap();
    let backing = crate::list::native_list_backing(original.as_ptr()).unwrap();
    backing.check_original_header(original.as_ptr()).unwrap();
    let copy = obj::Owned::fresh(obj::duplicate(original.as_ptr()));
    assert!(backing.check_original_header(copy.as_ptr()).is_err());
    assert_eq!(
        crate::typed_value::native_wide_int(
            original.as_ptr(),
            interpreter.native_invocation_dialect()
        ),
        Ok(42)
    );
    assert!(backing.check_original_header(original.as_ptr()).is_err());
    crate::list::list_elements_native_checked(
        original.as_ptr(),
        tcl_syntax::native_string::NativeStringProtocol::Jim084,
    )
    .unwrap();
    assert!(backing.check_original_header(original.as_ptr()).is_err());
    let current = crate::list::native_list_backing(original.as_ptr()).unwrap();
    current.check_original_header(original.as_ptr()).unwrap();
    crate::list::append_prepared_native_elements(
        original.as_ptr(),
        &[member.as_ptr()],
        tcl_syntax::native_string::NativeStringProtocol::Jim084,
    )
    .unwrap();
    assert!(current.check_original_header(original.as_ptr()).is_err());
    assert!(interpreter.native_execution_refusal().is_none());
}
