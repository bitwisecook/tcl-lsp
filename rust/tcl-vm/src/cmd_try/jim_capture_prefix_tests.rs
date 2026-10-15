// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::interp::native_name_consumers::OriginalNameConsumerScope;
use std::cell::Cell;
use tcl_syntax::native_string::NativeStringProtocol;

thread_local! {
    static BODY_CODE: Cell<i32> = const { Cell::new(0) };
    static BODIES: Cell<usize> = const { Cell::new(0) };
    static HANDLERS: Cell<usize> = const { Cell::new(0) };
}
fn word(bytes: &[u8]) -> Value {
    Value::from_native_string_bytes(bytes)
}
fn body(_: &mut Vm, _: &[Value]) -> Completion<Value> {
    BODIES.with(|count| count.set(count.get() + 1));
    Completion::new(
        Code::from_int(BODY_CODE.with(Cell::get)),
        word(b"BODY"),
        Value::empty(),
    )
}
fn handler(_: &mut Vm, _: &[Value]) -> Completion<Value> {
    HANDLERS.with(|count| count.set(count.get() + 1));
    ok(word(b"HANDLED"))
}
fn jim() -> Vm {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    vm.register("body_probe", body);
    vm.register("handler_probe", handler);
    vm
}
fn unhex(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn original_jim_literal_try_prefix_cells_preserve_bounded_execution_and_decline_unbound_shifts() {
    let cases = include_str!(
        "../../../../runtime/rust/tests/data/native_jim_try_leading_switches/cases-20261015.tsv"
    );
    let observations = include_str!(
        "../../../../runtime/rust/tests/data/native_jim_try_leading_switches/observations-20261015.tsv"
    );
    for row in observations.lines().filter(|row| row.starts_with("OBS|")) {
        let fields = row.split('|').collect::<Vec<_>>();
        let case: usize = fields[1].parse().unwrap();
        // Actual custom Jim ObjType updaters have no VM producer here.
        if case >= 20 {
            continue;
        }
        let input = cases
            .lines()
            .nth(case)
            .unwrap()
            .split('|')
            .collect::<Vec<_>>();
        BODY_CODE.with(|code| code.set(fields[2].parse().unwrap()));
        BODIES.with(|count| count.set(0));
        HANDLERS.with(|count| count.set(0));
        let mut vm = jim();
        let args = [
            word(&unhex(input[1])),
            word(b"body_probe"),
            word(b"on"),
            word(fields[2].as_bytes()),
            word(b"message options"),
            word(b"handler_probe"),
        ];
        let reached = vm.invoke_host_original_object_vector(&word(b"try"), &args);
        let reached = vm.finish_host_execution(reached);
        if matches!(case, 2 | 3 | 4 | 5 | 15) {
            // These original-build positives remain unavailable Native cases.
            assert_eq!(reached.unwrap_err(), mask_refusal(), "{row}");
            assert_eq!(BODIES.with(Cell::get), 0);
            assert_eq!(HANDLERS.with(Cell::get), 0);
            continue;
        }
        let reached = reached.unwrap();
        assert_eq!(reached.code.as_int().to_string(), fields[3], "{row}");
        assert_eq!(BODIES.with(Cell::get).to_string(), fields[4], "{row}");
        assert_eq!(HANDLERS.with(Cell::get).to_string(), fields[5], "{row}");
        if fields[4] == "1" {
            assert_eq!(
                vm.native_name_operand_bytes(&reached.result)
                    .unwrap()
                    .as_ref(),
                unhex(fields[7]),
                "{row}"
            );
        }
        assert_eq!(
            vm.get_var_bytes(b"message").is_some(),
            fields[8] == "1",
            "{row}"
        );
    }
}

#[test]
fn actual_jim_prefix_scan_leaves_post_dash_body_headers_unread() {
    for selector in [b"-no0".as_slice(), b"--"] {
        let mut vm = jim();
        let selector =
            Value::native_list_constructor(vec![word(selector)], NativeStringProtocol::Jim084);
        let body =
            Value::native_list_constructor(vec![word(b"body_probe")], NativeStringProtocol::Jim084);
        assert!(selector.resident_string_bytes().is_none());
        assert!(body.resident_string_bytes().is_none());
        let original = OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let args = [
            &selector,
            &body,
            &word(b"on"),
            &word(b"0"),
            &word(b"message options"),
            &word(b"handler_probe"),
        ];
        let dialect = vm.actual_native_invocation_dialect();
        let mut inspected = Vec::new();
        let prefix = tcl_registry::catch_invocation::select_native_jim_capture_prefix(
            args.len(),
            dialect,
            |index| {
                inspected.push(index);
                original
                    .read(&mut vm, args[index])
                    .map(|bytes| bytes.to_vec())
            },
        )
        .unwrap();
        let tcl_registry::catch_invocation::JimCapturePrefixSelection::Valid(prefix) = prefix
        else {
            panic!("actual literal prefix")
        };
        original.check(&mut vm).unwrap();
        assert_eq!(prefix.script_at, 1);
        let stops = selector.resident_string_bytes().unwrap().as_ref() == b"--";
        assert_eq!(body.resident_string_bytes().is_some(), !stops);
        assert_eq!(inspected, if stops { vec![0] } else { vec![0, 1] });
        original.finish(&mut vm).unwrap();
    }
}

#[test]
fn actual_jim_catch_prefix_cannot_execute_a_source_observed_wrapping_mask() {
    let mut vm = jim();
    BODIES.with(|count| count.set(0));
    let reached = vm.invoke_host_original_object_vector(
        &word(b"catch"),
        &[word(b"-no64"), word(b"body_probe")],
    );
    assert_eq!(reached.code, Code::Error);
    assert_eq!(
        vm.finish_host_execution(reached).unwrap_err(),
        mask_refusal()
    );
    assert_eq!(BODIES.with(Cell::get), 0);
}

fn mask_refusal() -> tcl_runtime_api::NativeExecutionError {
    tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "original Jim capture mask shift outside supported Rust64 recipe",
        ),
    )
}
