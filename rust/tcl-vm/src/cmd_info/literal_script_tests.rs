// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
thread_local! {
    static MODE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static OBSERVED: std::cell::RefCell<Option<(usize, Vec<u8>, bool)>> = const { std::cell::RefCell::new(None) };
}
fn record(vm: &mut Vm, args: &[Value], value: i64) -> Completion<Value> {
    let head = vm.invoked_name_value().unwrap();
    let bytes = vm.native_name_operand_bytes(&head).unwrap().to_vec();
    OBSERVED.with(|observed| {
        *observed.borrow_mut() = Some((
            args.len() + 1,
            bytes,
            vm.current_ns_id() == tcl_core_types::ROOT_NS,
        ));
    });
    let code = MODE.with(|mode| match mode.get() {
        3 => crate::Code::Error,
        6 => crate::Code::Break,
        _ => crate::Code::Ok,
    });
    Completion::new(code, Value::int(value), Value::empty())
}
fn observer(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    record(vm, args, 321)
}
fn replacement(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    record(vm, args, 654)
}
fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|&byte| {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            [
                char::from(HEX[usize::from(byte >> 4)]),
                char::from(HEX[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

#[test]
fn original_jim_info_literal_scripts_use_actual_mutable_commands_and_caller_namespace() {
    let observations = include_str!(
        "../../../../runtime/rust/tests/data/native_jim_info_literal_script_forwarding/observations-20261015.tsv"
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
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
            if mode != 0 {
                vm.register(helper, observer);
            }
            if mode == 2 {
                vm.register(&format!("N::{helper}"), observer);
            }
            if mode == 4 {
                vm.register(helper, replacement);
            }
            if mode == 5 || mode == 7 {
                assert_eq!(
                    vm.try_eval_source(&format!("rename {{{helper}}} held"))
                        .unwrap()
                        .code,
                    crate::Code::Ok
                );
                if mode == 7 {
                    assert_eq!(
                        vm.try_eval_source(&format!("proc {{{helper}}} {{}} {{return SCRIPTED}}"))
                            .unwrap()
                            .code,
                        crate::Code::Ok
                    );
                }
            }
            if mode == 8 {
                assert_eq!(
                    vm.try_eval_source("rename info inspect").unwrap().code,
                    crate::Code::Ok
                );
            }
            let result = if mode == 2 {
                vm.try_eval_source(&format!("namespace eval N {{info {name}}}"))
                    .unwrap()
            } else {
                vm.try_invoke_command(
                    if mode == 8 { "inspect" } else { "info" },
                    &[Value::from_native_string_bytes(name.as_bytes())],
                )
                .unwrap()
            };
            let bytes = vm.native_name_operand_bytes(&result.result).unwrap();
            let row = format!(
                "RESULT|{member}|{mode}|{}|{}",
                result.code.as_int(),
                hex(&bytes)
            );
            assert!(observations.lines().any(|line| line == row), "{row}");
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
        }
    }
}
