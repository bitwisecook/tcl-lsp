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

//! Child interpreters and hidden commands: `invokehidden` identity checks.

use super::*;

/// A child is another interpreter of the same build, so it states the
/// world its parent is pinned to, the overlay included.
#[test]
fn a_child_states_the_context_its_parent_is_pinned_to() {
    const OVERLAY: u64 = 0x0C0_1706;
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
    let mut context = tcl_registry::model::runtime_context_for_profile(profile);
    context.overlay_generation = OVERLAY;
    context.packages = vec![("vendor".to_owned(), "2.1".to_owned())];
    let mut vm = Vm::new();
    vm.pin_context(&context).expect("installed, so it pins");

    let name = vm.create_child(Some("child".to_owned()), false);
    let id = vm.child_id(&name).expect("the child is in the arena");
    let (stated, held) = vm.in_interp(id, |child| {
        (
            child.runtime_context().clone(),
            child.held_identity().clone(),
        )
    });
    assert_eq!(stated, context);
    assert_eq!(&held, vm.held_identity());
}

#[test]
fn invokehidden_rechecks_a_hidden_builtin_against_the_child_profile() {
    let mut vm = Vm::new();
    let child_name = vm.create_child(Some("child".to_string()), false);
    let child = vm.child_id(&child_name).unwrap();
    vm.in_interp(child, |child_vm| {
        child_vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
        );
        child_vm.hide_command("lassign", "held").unwrap();
        child_vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        );
    });
    let named = vm.invoke_hidden_in_child("child", "held", &[]).unwrap();
    assert_eq!(named.code, Code::Error);
    let by_id = vm.invoke_hidden_by_id(child, "held", &[]).unwrap();
    assert_eq!(by_id.code, Code::Error);
    assert!(
        vm.st_of(child)
            .unwrap()
            .hidden
            .commands
            .contains_key("held")
    );
    assert!(!vm.st_of(child).unwrap().commands.contains_key("held"));
}

#[test]
fn root_invokehidden_rechecks_identity_and_restores_hidden_state_after_error() {
    let mut vm = Vm::new();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
    );
    vm.hide_command("lassign", "held").unwrap();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );

    let result = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
    assert_eq!(result.code, Code::Error);
    assert!(vm.hidden.commands.contains_key("held"));
    assert!(!vm.commands.contains_key("held"));
    assert!(!vm.builtin_identities.contains_key("held"));
    assert!(!vm.imported_commands.contains_key("held"));
}

struct OriginNative(Rc<RefCell<Option<String>>>);

impl NativeCommand for OriginNative {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        *self.0.borrow_mut() = Some(vm.command_origin_key("held"));
        ok(Value::empty())
    }
}

#[test]
fn invokehidden_rechecks_a_hidden_native_identity_for_both_child_paths() {
    let mut vm = Vm::new();
    let child_name = vm.create_child(Some("child".to_string()), false);
    let child = vm.child_id(&child_name).unwrap();
    vm.in_interp(child, |child_vm| {
        child_vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        child_vm.register_native_command("isfinite", Rc::new(TestNative));
        child_vm.restore_builtin_identity("isfinite", "tcl::mathfunc::isfinite".to_string());
        child_vm.hide_command("isfinite", "held").unwrap();
        child_vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
    });

    assert_eq!(
        vm.invoke_hidden_in_child("child", "held", &[])
            .unwrap()
            .code,
        Code::Error
    );
    assert_eq!(
        vm.invoke_hidden_by_id(child, "held", &[]).unwrap().code,
        Code::Error
    );
    let state = vm.st_of(child).unwrap();
    assert!(state.hidden.commands.contains_key("held"));
    assert!(!state.commands.contains_key("held"));
    assert!(!state.builtin_identities.contains_key("held"));
}

#[test]
fn invokehidden_temporarily_restores_imported_command_provenance() {
    let observed = Rc::new(RefCell::new(None));
    let mut vm = Vm::new();
    vm.register_native_command("src::probe", Rc::new(OriginNative(Rc::clone(&observed))));
    vm.declare_namespace_exports("src", &["*"]);
    assert_eq!(
        vm.import_commands("::src::*", false).unwrap(),
        vec!["probe"]
    );
    vm.hide_command("probe", "held").unwrap();

    assert!(
        vm.invoke_hidden_in_child("", "held", &[])
            .unwrap()
            .code
            .is_ok()
    );
    // Direct hidden dispatch must not publish hidden provenance under the
    // token: normal command lookup from the body sees only visible state.
    assert_eq!(observed.borrow().as_deref(), Some("held"));
    assert_eq!(
        vm.hidden
            .imports
            .get("held")
            .map(|binding| binding.origin.key.name()),
        Some("src::probe")
    );
    assert!(!vm.imported_commands.contains_key("held"));
    assert!(!vm.commands.contains_key("held"));
}
