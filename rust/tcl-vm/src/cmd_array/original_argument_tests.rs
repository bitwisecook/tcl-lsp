// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original array worker selection and physical VM callback settlement.
use super::*;
use crate::NativeCommand;
use crate::command::Command;
use std::{cell::Cell, rc::Rc};

const ORIGINALS: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!(
            "../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/8.4.20/execute.stdout"
        ),
    ),
    (
        "tcl8.5",
        include_str!(
            "../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/8.5.19/execute.stdout"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/8.6.18/execute.stdout"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/9.0.4/execute.stdout"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/9.1.0/execute.stdout"
        ),
    ),
    (
        "jim",
        include_str!(
            "../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/jim/execute.stdout"
        ),
    ),
];
fn native(version: &str) -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect(version))
}
fn bytes(value: &[u8]) -> Value {
    Value::new_native_string_bytes(value)
}
fn compare_completion(completion: &Completion<Value>, capture: &str, case: usize) {
    let row: Vec<_> = capture
        .lines()
        .find(|row| row.starts_with(&format!("ROW|{case}|worker|")))
        .unwrap()
        .split('|')
        .collect();
    let decoded: Vec<_> = row[4]
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(completion.code.as_int(), row[3].parse::<i64>().unwrap());
    assert_eq!(
        completion.result.resident_string_bytes().unwrap().as_ref(),
        decoded
    );
}

#[test]
fn original_vm_array_set_matches_bounded_successful_public_rows() {
    // Only successful505b4/6 public completion fields; original C extension
    // updater events/reference counts are not VM Value observations.
    for &(version, capture) in ORIGINALS {
        for case in [4, 6] {
            let mut vm = native(version);
            let name = bytes(b"::arr");
            let key = bytes(b"k");
            let second = bytes(if case == 4 { b"VALUE" } else { b"m" });
            let members = if case == 4 {
                vec![key, second]
            } else {
                vec![key, bytes(b"FIRST"), second, bytes(b"SECOND")]
            };
            let list = Value::list(members);
            let completion = array_op(&mut vm, "set", &[name.clone(), list.clone()]);
            compare_completion(&completion, capture, case);
            assert!(vm.execution_refusal.is_none(), "{version}/{case}");
            if version == "jim" {
                let currency = NativeOperationScope::enter(&mut vm).unwrap();
                let stored = vm
                    .original_jim_array_root(&name, currency.currency())
                    .unwrap()
                    .unwrap();
                assert!(stored.value().is_same_object(&list));
                assert!(stored.value().cached_dictionary_representation().is_none());
            }
        }
    }
}

#[test]
fn original_vm_jim_array_set_observes_original_root_before_cow() {
    // Software-only original header/sharing roles, separately selected from
    // the bounded public505b successful rows above.
    for shared in [false, true] {
        let mut vm = native("jim");
        let name = bytes(b"arr");
        let root = Value::dict(vec![(bytes(b"old"), bytes(b"OLD"))]);
        let original = root.native_lifetime_lease();
        let external = shared.then(|| root.clone());
        drop(vm.store_original_named_variable(&name, root).unwrap());
        assert_eq!(
            original.value().native_object_reference_count(),
            1 + usize::from(shared)
        );
        let list = Value::list(vec![bytes(b"new"), bytes(b"NEW")]);
        let completion = array_op(&mut vm, "set", &[name.clone(), list]);
        assert_eq!(completion.code, Code::Ok, "shared={shared}");
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let stored = vm
            .original_jim_array_root(&name, scope.currency())
            .unwrap()
            .unwrap();
        assert_eq!(stored.value().is_same_object(original.value()), !shared);
        let pairs = stored.value().cached_dictionary_representation().unwrap();
        assert_eq!(pairs.len(), 2);
        if shared {
            assert_eq!(
                external
                    .as_ref()
                    .unwrap()
                    .cached_dictionary_representation()
                    .unwrap()
                    .len(),
                1
            );
        }
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_vm_array_for_keeps_the_actual_table_key_header() {
    // C9 successful505b7 code/bytes; exact key identity/counting is a separate
    // software storage assertion, not a public native pointer observation.
    for &(version, capture) in &ORIGINALS[3..5] {
        let mut vm = native(version);
        let key = bytes(b"k");
        let original_key = key.native_lifetime_lease();
        let name = bytes(b"::arr");
        assert_eq!(
            array_op(
                &mut vm,
                "set",
                &[name.clone(), Value::list(vec![key, bytes(b"VALUE")])]
            )
            .code,
            Code::Ok
        );
        let variables = Value::list(vec![bytes(b"::k"), bytes(b"::v")]);
        let completion = array_op(&mut vm, "for", &[variables, name, bytes(b"")]);
        compare_completion(&completion, capture, 7);
        let stored = vm.get_var("k").unwrap();
        assert!(stored.is_same_object(original_key.value()));
        assert_eq!(
            vm.get_var("v")
                .unwrap()
                .resident_string_bytes()
                .unwrap()
                .as_ref(),
            b"VALUE"
        );
        assert!(vm.execution_refusal.is_none());
    }
    let mut vm = native("tcl9.1");
    vm.set_array_elem_bytes(b"arr", b"byte-only", bytes(b"VALUE"))
        .unwrap();
    let completion = array_op(
        &mut vm,
        "for",
        &[
            Value::list(vec![bytes(b"k"), bytes(b"v")]),
            bytes(b"arr"),
            bytes(b""),
        ],
    );
    assert_eq!(completion.code, Code::Error);
    assert!(matches!(
        vm.execution_refusal,
        Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "actual original array member key"
            )
        ))
    ));
    assert!(vm.get_var("k").is_none(), "no synthesized key is assigned");
}

struct ContextTrace {
    mode: u8,
    calls: Rc<Cell<usize>>,
}
impl NativeCommand for ContextTrace {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        vm.set_var_bytes(b"reached", bytes(b"REACHED")).unwrap();
        if self.mode == 1 {
            let _ = vm.refuse_host_command("first actual array observer".into());
        } else if self.mode == 2 {
            let original = vm.runtime_context().clone();
            let mut changed = original.clone();
            changed.packages = vec![("actual-array-context".into(), "1".into())];
            vm.pin_context(&changed).unwrap();
            vm.pin_context(&original).unwrap();
        }
        ok(bytes(b"LATER_GUEST"))
    }
}
#[test]
fn original_vm_array_observer_refusal_stops_list_conversion_and_releases_receiver() {
    // Genuine VM NativeCommand variable observer; no custom C updater or
    // native reference-count equivalence is asserted.
    for mode in [1, 2] {
        let mut vm = native("tcl9.1");
        assert_eq!(
            array_op(
                &mut vm,
                "set",
                &[
                    bytes(b"arr"),
                    Value::list(vec![bytes(b"old"), bytes(b"OLD")])
                ]
            )
            .code,
            Code::Ok
        );
        let target =
            VarStore::array_target_bytes(&vm, tcl_runtime_api::Frames::current(&vm), b"arr")
                .unwrap();
        let id = target.cell_id().unwrap();
        let calls = Rc::new(Cell::new(0));
        vm.register_command(
            "context_trace",
            Command::Native(Rc::new(ContextTrace {
                mode,
                calls: calls.clone(),
            })),
        );
        assert_eq!(
            vm.eval_source("trace add variable arr array context_trace")
                .unwrap()
                .code,
            Code::Ok
        );
        let malformed = bytes(b"{");
        let before = malformed.native_object_snapshot();
        let completion = array_op(&mut vm, "set", &[bytes(b"arr"), malformed.clone()]);
        assert_eq!(completion.code, Code::Error);
        assert_eq!(calls.get(), 1);
        assert_eq!(malformed.native_object_snapshot(), before);
        assert_eq!(
            vm.get_var("reached")
                .unwrap()
                .resident_string_bytes()
                .unwrap()
                .as_ref(),
            b"REACHED"
        );
        assert!(!vm.has_variable_operation_refs_for_test(id));
        let first = vm.execution_refusal.clone().unwrap();
        let again = array_op(
            &mut vm,
            "set",
            &[
                bytes(b"arr"),
                Value::list(vec![bytes(b"later"), bytes(b"LATER")]),
            ],
        );
        assert_eq!(again.code, Code::Error);
        assert_eq!(calls.get(), 1);
        assert_eq!(vm.execution_refusal, Some(first));
    }
}
