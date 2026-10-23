// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::{
    interp::{Code, Interp},
    obj::{self, Owned, TclObj},
};
use std::cell::{Cell, RefCell};

thread_local! {
    static ACTUAL: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static MODE: Cell<usize> = const { Cell::new(0) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
}

extern "C" fn update(original: *mut TclObj) {
    let mut actual = ACTUAL.with(|slot| slot.borrow().as_ref().unwrap().clone());
    GETTERS.with(|count| count.set(count.get() + 1));
    let mode = MODE.with(Cell::get);
    actual.set_result_bytes(if mode == 1 { b"MSGNAME" } else { b"OPTNAME" });
    if mode == 3 {
        let name = Owned::fresh(obj::new_string_bytes(b"errorCode"));
        let replacement = Owned::fresh(obj::new_string_bytes(b"REPLACED"));
        assert_eq!(
            actual.assign_original_named_variable(name.as_ptr(), replacement.as_ptr()),
            Ok(())
        );
    }
    let name: &[u8] = match mode {
        1 => b"message",
        2 => b"options",
        3 => b"",
        _ => unreachable!(),
    };
    // SAFETY: the actual original getter owns this live extension header.
    unsafe { obj::set_native_updater_string_rep(original, name, false) };
}

static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"outputNameStage".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};

fn body(actual: &mut Interp, _: &[*mut TclObj]) -> Code {
    let name = Owned::fresh(obj::new_string_bytes(b"errorCode"));
    let member = Owned::fresh(obj::new_string_bytes(b"777"));
    let code = Owned::fresh(actual.new_list_object(&[member.as_ptr()]));
    if let Err(code) = actual.assign_original_named_variable(name.as_ptr(), code.as_ptr()) {
        return code;
    }
    actual.set_result_bytes(b"BODY");
    Code::Error
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn jim_output_names_reach_length_before_message_and_options_production() {
    // Actual Runtime extension stages; this does not issue a Jim ObjType ABI claim.
    let observations =
        include_str!("../../../tests/data/native_jim_output_name_stages/observations-20261015.tsv");
    let original_try = include_str!(
        "../../../../../rust/tcl-registry/tests/data/source_contract_vectors/results/jim-try-reached-message-getters/capture07/execution.stdout"
    );
    for is_try in 0..2 {
        for mode in 0..4 {
            let mut actual = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("jimtcl"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            actual.register_builtin(b"body_error", body);
            let message = Owned::fresh(if mode == 1 {
                obj::alloc_typed(&ORIGINAL, 0)
            } else {
                obj::new_string_bytes(b"message")
            });
            let options = Owned::fresh(if mode >= 2 {
                obj::alloc_typed(&ORIGINAL, 0)
            } else {
                obj::new_string_bytes(b"options")
            });
            let head = Owned::fresh(obj::new_string_bytes(if is_try == 0 {
                b"catch"
            } else {
                b"try"
            }));
            let body = Owned::fresh(obj::new_string_bytes(b"body_error"));
            let mut owners = Vec::new();
            let mut argv = vec![head.as_ptr(), body.as_ptr()];
            if is_try == 1 {
                owners.extend([
                    Owned::fresh(obj::new_string_bytes(b"on")),
                    Owned::fresh(obj::new_string_bytes(b"error")),
                    Owned::fresh(actual.new_list_object(&[message.as_ptr(), options.as_ptr()])),
                    Owned::fresh(obj::new_string_bytes(b"set ::handled $::message")),
                ]);
                argv.extend(owners.iter().map(Owned::as_ptr));
            } else {
                argv.extend([message.as_ptr(), options.as_ptr()]);
            }
            MODE.with(|selected| selected.set(mode));
            GETTERS.with(|count| count.set(0));
            ACTUAL.with(|slot| *slot.borrow_mut() = Some(actual.clone()));
            let code = actual.eval_original_object_vector(&argv);
            ACTUAL.with(|slot| *slot.borrow_mut() = None);
            assert!(actual.native_execution_refusal().is_none());
            let result = hex(&actual.result_bytes());
            let message = actual
                .var_get(b"message")
                .map(|value| hex(&obj::bytes_of(value)))
                .unwrap_or_else(|| "-".into());
            let handled = actual
                .var_get(b"handled")
                .map(|value| hex(&obj::bytes_of(value)))
                .unwrap_or_else(|| "-".into());
            let row = format!(
                "OBS|{is_try}|{mode}|{}|{}|{result}|{message}|{handled}|{}",
                code.as_int(),
                GETTERS.with(Cell::get),
                usize::from(actual.var_get(b"options").is_some())
            );
            assert!(
                observations.lines().any(|observed| observed == row),
                "{row}"
            );
            if is_try == 1 && (mode == 1 || mode == 2) {
                let row = format!(
                    "OBS|{}|{}|{}|{result}|{message}|{handled}|-",
                    if mode == 1 { 14 } else { 15 },
                    code.as_int(),
                    GETTERS.with(Cell::get)
                );
                assert!(
                    original_try.lines().any(|observed| observed == row),
                    "{row}"
                );
            }
        }
    }
}
