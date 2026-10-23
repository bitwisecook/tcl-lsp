// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::obj::Owned;
thread_local! {
    static MODE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static OBSERVED: std::cell::RefCell<Option<(usize, Vec<u8>, bool)>> = const { std::cell::RefCell::new(None) };
}
fn record(actual: &mut Interp, argv: &[*mut TclObj], value: i64) -> Code {
    OBSERVED.with(|observed| {
        *observed.borrow_mut() = Some((
            argv.len(),
            crate::interp::obj_bytes(argv[0]),
            actual.current_ns() == crate::namespace::GLOBAL,
        ))
    });
    actual.set_result(crate::obj::new_wide_int_obj(value));
    MODE.with(|mode| match mode.get() {
        3 => Code::Error,
        6 => Code::Break,
        _ => Code::Ok,
    })
}
fn observer(actual: &mut Interp, argv: &[*mut TclObj]) -> Code {
    record(actual, argv, 321)
}
fn replacement(actual: &mut Interp, argv: &[*mut TclObj]) -> Code {
    record(actual, argv, 654)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn original_jim_info_literal_scripts_use_actual_mutable_commands_and_caller_namespace() {
    let observations = include_str!(
        "../../tests/data/native_jim_info_literal_script_forwarding/observations-20261015.tsv"
    );
    for (member, (name, helper)) in [
        ("nameofexecutable", "info nameofexecutable"),
        ("hostname", "os.gethostname"),
    ]
    .into_iter()
    .enumerate()
    {
        for mode in 0..9 {
            MODE.with(|selected| selected.set(mode));
            OBSERVED.with(|observed| *observed.borrow_mut() = None);
            let mut actual = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("jimtcl"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            if mode != 0 {
                actual.register_builtin(helper.as_bytes(), observer);
            }
            if mode == 2 {
                actual.register_builtin(format!("N::{helper}").as_bytes(), observer);
            }
            if mode == 4 {
                actual.register_builtin(helper.as_bytes(), replacement);
            }
            if mode == 5 || mode == 7 {
                assert_eq!(
                    actual.eval_str(format!("rename {{{helper}}} held").as_bytes()),
                    Code::Ok
                );
                if mode == 7 {
                    assert_eq!(
                        actual.eval_str(
                            format!("proc {{{helper}}} {{}} {{return SCRIPTED}}").as_bytes()
                        ),
                        Code::Ok
                    );
                }
            }
            if mode == 8 {
                assert_eq!(actual.eval_str(b"rename info inspect"), Code::Ok);
            }
            let code = if mode == 2 {
                actual.eval_str(format!("namespace eval N {{info {name}}}").as_bytes())
            } else {
                let head = Owned::fresh(new_string(if mode == 8 { b"inspect" } else { b"info" }));
                let selector = Owned::fresh(new_string(name.as_bytes()));
                actual.eval_original_object_vector(&[head.as_ptr(), selector.as_ptr()])
            };
            let row = format!(
                "RESULT|{member}|{mode}|{}|{}",
                code.as_int(),
                hex(&actual.result_bytes())
            );
            assert!(
                observations.lines().any(|line| line == row),
                "{row}; {:?}",
                actual.native_execution_refusal()
            );
            let prefix = format!("OBS|{member}|{mode}|");
            let expected = observations.lines().find(|line| line.starts_with(&prefix));
            OBSERVED.with(|observed| {
                let observed = observed.borrow();
                assert_eq!(observed.is_some(), expected.is_some());
                if let Some(expected) = expected {
                    let fields = expected.split('|').collect::<Vec<_>>();
                    let (argc, head, global) = observed.as_ref().unwrap();
                    assert_eq!(argc.to_string(), fields[3]);
                    assert_eq!(hex(head), fields[4]);
                    assert_eq!(*global, fields[5].is_empty());
                }
            });
            assert!(actual.native_execution_refusal().is_none());
        }
    }
}
