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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual software OO cleanup transport; no Native callback/reference claim.
use super::*;

fn native() -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl8.6"))
}

#[test]
fn original_oo_cleanup_selects_physical_namespace_teardown_order() {
    for entered in [false, true] {
        let mut vm = native();
        vm.set_dialect_profile(crate::environment::profile_for_dialect("f5-irules"));
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        assert_eq!(
            vm.name_policy_protocol()
                .unwrap()
                .recipe()
                .namespace_command_teardown(),
            Some(tcl_syntax::native_namespace_name::NativeNamespaceCommandTeardown::RepeatedFirstEntry)
        );
        let original = if entered {
            OriginalOoCleanup::enter(&mut vm)
        } else {
            OriginalOoCleanup::observe(&mut vm)
        };
        assert_eq!(original.operation.is_some(), entered);
        assert_eq!(
            original.namespace_traversal,
            Some(tcl_syntax::native_namespace_name::NativeNamespaceCommandTeardown::SnapshotPass)
        );
        let saved = original.saved();
        assert_eq!(saved.namespace_traversal, original.namespace_traversal);
        drop(original);
        let attached = saved.attach(&mut vm);
        assert_eq!(attached.namespace_traversal, saved.namespace_traversal);
    }
}

#[test]
fn original_oo_cleanup_keeps_reached_metadata_and_default_caller_restoration() {
    for entered in [false, true] {
        let mut vm = native();
        let parent = vm.cur_interp();
        let name = vm.create_child(Some("oo_cleanup_child".into()), false);
        let child = vm.child_id(&name).unwrap();
        let original = if entered {
            OriginalOoCleanup::enter(&mut vm)
        } else {
            OriginalOoCleanup::observe(&mut vm)
        };
        assert_eq!(original.operation.is_some(), entered);
        let metadata = original
            .in_original(&mut vm, |vm| {
                assert!(vm.interp_is_current(parent));
                vm.activation_depth = 37;
                select_cleanup_fixture_interpreter(vm, child);
                vm.activation_depth = 11;
                73u64
            })
            .unwrap();
        assert_eq!(metadata, 73);
        assert!(vm.interp_is_current(parent));
        assert_eq!(vm.activation_depth, 37);
        vm.in_interp(child, |vm| assert_eq!(vm.activation_depth, 11));
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(
            original
                .check(&mut vm)
                .unwrap_err()
                .native_execution_refusal(),
            Some(&first)
        );
        drop(original);
        assert!(!vm.native_operation_context.has_entered_operation());
        assert_eq!(vm.execution_refusal, Some(first));
    }
}

#[test]
fn original_oo_saved_cleanup_transports_same_target_after_scope_expiry_and_first_cause() {
    let mut vm = native();
    let entered = NativeOperationScope::enter(&mut vm).unwrap();
    let observed = OriginalOoCleanup::observe(&mut vm);
    let saved = observed.saved();
    let target = observed.interpreter;
    drop((observed, entered));
    assert!(!vm.native_operation_context.has_entered_operation());
    assert!(saved.target.owns_target(&vm));
    let _ = vm.refuse_host_command("OO_SAVED_CLEANUP_FIRST".into());
    let first = vm.execution_refusal.clone().unwrap();
    let attached = saved.attach(&mut vm);
    assert_eq!(attached.interpreter, target);
    assert!(attached.target.owns_target(&vm));
    assert!(vm.native_operation_context.has_entered_operation());
    let mut retired = 0;
    let metadata = attached
        .in_original(&mut vm, |vm| {
            assert!(vm.interp_is_current(target));
            assert!(NativeOperationScope::enter(vm).is_err());
            retired += 1;
            91u64
        })
        .unwrap();
    assert_eq!(metadata, 91);
    assert_eq!(retired, 1);
    assert_eq!(
        attached
            .check(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
    drop(attached);
    assert!(!vm.native_operation_context.has_entered_operation());
    assert_eq!(vm.execution_refusal, Some(first));
}
