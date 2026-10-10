// SPDX-License-Identifier: AGPL-3.0-or-later
//! Host command publication through the documented script declaration policy.

use super::{Command, NativeCommandLookupUnavailable, ROOT_NS, Vm};
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{Code, Completion};

struct HostValue(i64);

impl crate::command::NativeCommand for HostValue {
    fn invoke(&self, _vm: &mut Vm, _args: &[crate::Value]) -> Completion<crate::Value> {
        Completion::new(Code::Ok, crate::Value::int(self.0), crate::Value::empty())
    }
}

fn host_result(vm: &mut Vm, key: &str) -> i64 {
    let Command::Native(command) = vm.visible_command_at_key(key).unwrap() else {
        panic!("registered host implementation")
    };
    command.invoke(vm, &[]).result.as_int().unwrap()
}

#[test]
fn embedded_registration_keeps_script_publication_slots_and_counted_name_boundaries() {
    // Host API contract only: these checks do not claim Tcl_CreateObjCommand
    // equivalence or a newly executed native process. Actual stored slots and
    // independently valued implementations are inspected without display reparse.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        let first = vm
            .register_written_command("r2286\0tail", Command::Native(Rc::new(HostValue(1))))
            .unwrap();
        assert_eq!(host_result(&mut vm, &first), 1);
        let first_slot = vm.command_slot(&first).unwrap();
        assert_eq!(first_slot.namespace, ROOT_NS);
        assert_eq!(
            first_slot.simple.as_bytes(),
            if engine == "jim" {
                b"r2286\0tail".as_slice()
            } else {
                b"r2286".as_slice()
            }
        );
        vm.try_register_native_command("\u{e9}", Rc::new(HostValue(3)))
            .unwrap();
        let unicode = vm
            .lookup_command_bytes_checked(ROOT_NS, "\u{e9}".as_bytes())
            .unwrap()
            .unwrap()
            .0;
        assert_eq!(
            vm.command_slot(&unicode).unwrap().simple.as_bytes(),
            "\u{e9}".as_bytes()
        );
        assert_eq!(host_result(&mut vm, &unicode), 3);

        let holder = if engine == "jim" {
            vm.intern_jim_namespace_object(b"a:")
        } else {
            vm.activate_namespace_written("::a:")
        };
        vm.resolution_stacks.ns_id_stack.push(holder);
        let local = vm
            .register_written_command("p", Command::Native(Rc::new(HostValue(4))))
            .unwrap();
        let local_slot = vm.command_slot(&local).unwrap();
        if engine == "jim" {
            assert_eq!(local_slot.namespace, ROOT_NS);
            assert_eq!(local_slot.simple.as_bytes(), b"a:::p");
        } else if matches!(engine, "tcl8.4" | "tcl8.5") {
            assert_ne!(local_slot.namespace, holder);
            assert_eq!(vm.ns_path(local_slot.namespace), ["a"]);
        } else {
            assert_eq!(local_slot.namespace, holder);
            assert_eq!(local_slot.simple.as_bytes(), b"p");
        }
        let rooted = vm
            .register_written_command("::a::p", Command::Native(Rc::new(HostValue(5))))
            .unwrap();
        assert_eq!(local == rooted, matches!(engine, "tcl8.4" | "tcl8.5"));
        assert_eq!(host_result(&mut vm, &rooted), 5);
        if local != rooted {
            assert_eq!(host_result(&mut vm, &local), 4);
        }
        assert_eq!(vm.resolution_stacks.ns_id_stack.pop(), Some(holder));
    }
}

#[test]
fn embedded_registration_refuses_missing_or_foreign_policies_before_publication() {
    // Removing or replacing this test host's selected engine is an explicit
    // unavailable-policy fixture, never evidence of an actual vendor engine.
    for engine in ["tcl", "irules", "no-such-dialect"] {
        let mut vm = Vm::new();
        vm.actual_engine_profile =
            Some(tcl_registry::model::ingress::resolve_environment(engine).unit_profile());
        vm.active_native_profile = None;
        vm.logical_providers.names = None;
        let original = vm.registered_command_entries();
        assert_eq!(
            vm.try_register_native_command("r2286_missing", Rc::new(HostValue(1))),
            Err(NativeCommandLookupUnavailable::ProtocolUnavailable)
        );
        assert_eq!(vm.registered_command_entries(), original);
        assert!(vm.refused_completion().is_none());
        vm.register_native_command("r2286_wrapper", Rc::new(HostValue(2)));
        assert!(vm.refused_completion().is_some());
        assert_eq!(vm.registered_command_entries(), original);
    }
}
