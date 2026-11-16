// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
#[cfg(debug_assertions)]
use crate::interp::native_operation_currency::NativeOperationScope;
use tcl_runtime_api::{AliasPrefixLookup, Aliases, NsId, ROOT_NS};
use tcl_syntax::value::ValueOps;

fn native() -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jim"))
}
fn prefix(vm: &mut Vm, name: &Value, context: NsId) -> Value {
    let AliasPrefixLookup::Prefix(original) =
        Aliases::alias_prefix_original_value_at(vm, context, name).unwrap()
    else {
        panic!("actual stored original alias List");
    };
    original
}
fn define(vm: &mut Vm, name: &str, members: &[Value]) -> Value {
    let name = Value::string(name);
    let mut argv = vec![name.clone()];
    argv.extend_from_slice(members);
    assert_eq!(vm.invoke_command("alias", &argv).code, Code::Ok);
    prefix(vm, &name, ROOT_NS)
}
fn destination_frame(vm: &mut Vm) -> NsId {
    let token = vm.activate_namespace_operand(b"dest").unwrap();
    vm.push_ns_eval_token_frame(
        token,
        vec![
            Value::string("namespace"),
            Value::string("eval"),
            Value::string("dest"),
            Value::string(""),
        ],
    );
    token
}
fn slot(vm: &Vm) -> tcl_core_types::CommandSlot<tcl_core_types::NameBytes> {
    let selected = vm
        .name_policy_protocol()
        .unwrap()
        .recipe()
        .alias_publication_slot(
            tcl_syntax::naming::NativeNameContext::root(),
            b"dest::branch",
        )
        .unwrap();
    assert!(selected.namespace.is_root());
    tcl_core_types::CommandSlot {
        namespace: ROOT_NS,
        simple: selected.simple,
    }
}

#[test]
fn original_vm_jim_alias_query_keeps_explicit_root_context_after_destination_cache() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    // Authentic root/frame owners, stored List identity and cache context;
    // no original Enum header or C query/refcount equivalence is inferred.
    let mut vm = native();
    let root = define(&mut vm, "src::branch", &[Value::string("list")]);
    let collision = define(
        &mut vm,
        "dest::src::branch",
        &[Value::string("dest::branch")],
    );
    let destination = destination_frame(&mut vm);
    let original = Value::string("src::branch");
    let current = prefix(&mut vm, &original, destination);
    assert_eq!(
        current.native_object_identity(),
        collision.native_object_identity()
    );
    let queried = prefix(&mut vm, &original, ROOT_NS);
    assert_eq!(
        queried.native_object_identity(),
        root.native_object_identity()
    );
    assert_ne!(
        queried.native_object_identity(),
        collision.native_object_identity()
    );
    assert_eq!(tcl_runtime_api::Namespaces::current(&vm), destination);
    let imported = ns_import(&mut vm, &[Value::string("::src::branch")]);
    assert_eq!(imported.code, Code::Ok);
    assert!(
        vm.command_at_exact_slot_checked(&slot(&vm), false)
            .unwrap()
            .is_some()
    );
    assert!(vm.execution_refusal.is_none());
    vm.pop_ns();
    vm.pop_call_frame();

    let mut refused_vm = native();
    let refused_destination = destination_frame(&mut refused_vm);
    let original = Value::string("src::branch");
    assert!(
        Aliases::alias_prefix_original_value_at(&mut refused_vm, NsId(u32::MAX), &original)
            .is_err()
    );
    assert_eq!(
        tcl_runtime_api::Namespaces::current(&refused_vm),
        refused_destination
    );
    let first = refused_vm.execution_refusal.clone().unwrap();
    let later = ns_import(&mut refused_vm, &[Value::string("::src::branch")]);
    assert_eq!(later.code, Code::Error);
    assert_eq!(refused_vm.execution_refusal.as_ref(), Some(&first));
}

#[test]
fn original_vm_jim_import_uses_cached_prefix_without_reading_original_members() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    let mut vm = native();
    let target = Value::string("list");
    let member = Value::int(5);
    let stored = define(&mut vm, "src::branch", &[target.clone(), member.clone()]);
    assert_eq!(vm.native_string_bytes(&stored).unwrap().as_ref(), b"list 5");
    member.invalidate_native_string_for_test();
    assert!(member.resident_string_bytes().is_none());
    assert_eq!(stored.native_object_type_name(), "list");
    let refs = member.native_object_reference_count();
    let destination = destination_frame(&mut vm);
    let result = ns_import(&mut vm, &[Value::string("::src::branch")]);
    assert_eq!(result.code, Code::Ok);
    assert!(member.resident_string_bytes().is_none());
    assert_eq!(member.native_object_reference_count(), refs);
    assert_eq!(stored.native_object_type_name(), "list");
    let queried = prefix(&mut vm, &Value::string("src::branch"), ROOT_NS);
    assert_eq!(
        queried.native_object_identity(),
        stored.native_object_identity()
    );
    let members = vm.list_elements(&queried).unwrap();
    assert_eq!(
        members[0].native_object_identity(),
        target.native_object_identity()
    );
    assert_eq!(
        members[1].native_object_identity(),
        member.native_object_identity()
    );
    assert_eq!(tcl_runtime_api::Namespaces::current(&vm), destination);
    assert!(vm.execution_refusal.is_none());
    vm.pop_ns();
    vm.pop_call_frame();
}

#[test]
fn original_vm_jim_import_compares_whole_prefix_before_following_alias_chains() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    for case in 0..3 {
        let mut vm = native();
        let source = match case {
            0 => define(
                &mut vm,
                "src::branch",
                &[Value::string("dest::branch"), Value::string("EXTRA")],
            ),
            1 => define(&mut vm, "src::branch", &[Value::string("dest::branch")]),
            2 => {
                let _middle = define(&mut vm, "chain", &[Value::string("dest::branch")]);
                define(&mut vm, "src::branch", &[Value::string("chain")])
            }
            _ => unreachable!(),
        };
        destination_frame(&mut vm);
        let destination_slot = slot(&vm);
        let result = ns_import(&mut vm, &[Value::string("::src::branch")]);
        assert_eq!(result.code, if case == 0 { Code::Ok } else { Code::Error });
        assert_eq!(
            vm.command_at_exact_slot_checked(&destination_slot, false)
                .unwrap()
                .is_some(),
            case == 0
        );
        if case != 0 {
            assert_eq!(
                vm.native_string_bytes(&result.result).unwrap().as_ref(),
                b"import pattern \"::src::branch\" would create a loop"
            );
        }
        assert!(source.resident_string_bytes().is_some());
        assert!(vm.execution_refusal.is_none());
        vm.pop_ns();
        vm.pop_call_frame();
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_vm_jim_import_keeps_first_host_and_restored_context_terminal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Real entered scope and actual context setters; not injected object
    // getter results or an original Native chronology comparison.
    for restore in [false, true] {
        let mut vm = native();
        let member = Value::int(5);
        let stored = define(
            &mut vm,
            "src::branch",
            &[Value::string("list"), member.clone()],
        );
        destination_frame(&mut vm);
        let destination_slot = slot(&vm);
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let current = vm.runtime_context().clone();
        if restore {
            let mut changed = current.clone();
            changed.packages = vec![("original-vm-import".to_owned(), "1.0".to_owned())];
            vm.pin_context(&changed).unwrap();
            vm.pin_context(&current).unwrap();
        } else {
            let _ = vm.refuse_host_command("original VM import first cause".to_owned());
        }
        let first = scope.check(&mut vm).unwrap_err();
        let retained = vm.execution_refusal.clone().unwrap();
        let refs = member.native_object_reference_count();
        let result = ns_import(&mut vm, &[Value::string("::src::branch")]);
        assert_eq!(result.code, Code::Error);
        assert_eq!(vm.execution_refusal, Some(retained.clone()));
        assert_eq!(first.native_execution_refusal(), Some(&retained));
        assert!(
            vm.command_at_exact_slot_checked(&destination_slot, false)
                .unwrap()
                .is_none()
        );
        assert!(stored.resident_string_bytes().is_none());
        assert!(member.resident_string_bytes().is_none());
        assert_eq!(member.native_object_reference_count(), refs);
        assert_eq!(vm.runtime_context(), &current);
        drop(scope);
        vm.pop_ns();
        vm.pop_call_frame();
    }
}
