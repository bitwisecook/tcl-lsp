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

//! Child interpreters and hidden commands: `invokehidden` identity checks and profile refresh of hidden and imported procedures.

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

#[test]
fn only_a_changed_root_profile_reestablishes_standard_channel_defaults() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let v90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let v86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    vm.set_dialect_profile(v90);

    assert_eq!(
        eval_value(
            &mut vm,
            "fconfigure stdout -encoding ascii -translation crlf -profile replace; \
                 list [fconfigure stdout -encoding] [fconfigure stdout -translation] \
                      [fconfigure stdout -profile]",
        ),
        "ascii crlf replace"
    );

    vm.set_dialect_profile(v90);
    assert_eq!(
        eval_value(
            &mut vm,
            "list [fconfigure stdout -encoding] [fconfigure stdout -translation] \
                      [fconfigure stdout -profile]",
        ),
        "ascii crlf replace",
        "re-pinning the root to the same profile reset live channel state"
    );

    let child = vm.create_child(Some("child".to_owned()), false);
    assert!(vm.set_child_dialect_profile(&child, v86));
    assert_eq!(
        eval_value(
            &mut vm,
            "list [fconfigure stdout -encoding] [fconfigure stdout -translation] \
                      [fconfigure stdout -profile]",
        ),
        "ascii crlf replace",
        "pinning a child profile reset the VM-wide standard channel"
    );

    vm.set_dialect_profile(v86);
    assert_eq!(
        eval_value(&mut vm, "fconfigure stdout -translation"),
        "lf",
        "changing the root profile did not establish release defaults"
    );
}

fn prepare_stale_hidden_proc(vm: &mut Vm) {
    select_release(vm, "tcl8.5");
    assert_eq!(eval_value(vm, "proc p {} { return hidden }"), "");
    vm.hide_command("p", "held").unwrap();
    assert_eq!(eval_value(vm, "proc p {} { return replacement }"), "");
    select_release(vm, "tcl8.6");
}

fn prepare_trace_deleted_hidden_proc(vm: &mut Vm) {
    select_release(vm, "tcl8.5");
    assert_eq!(eval_value(vm, "proc p {} { return hidden }"), "");
    assert_eq!(
        eval_value(
            vm,
            "proc replace args { rename p {}; interp expose {} held p; rename p {}; proc p {} { return replacement } }"
        ),
        ""
    );
    assert_eq!(eval_value(vm, "trace add execution p enter replace"), "");
    vm.hide_command("p", "held").unwrap();
    assert_eq!(eval_value(vm, "proc p {} { return initial }"), "");
    select_release(vm, "tcl8.6");
}

#[test]
fn refreshed_hidden_proc_stays_hidden_and_preserves_visible_replacement() {
    // Children replay their parent's constructor engine, which the
    // fixtures leave selected.
    let mut vm = release_vm(tcl_dialect::TclVersion::V8_6);
    prepare_stale_hidden_proc(&mut vm);
    let hidden = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
    assert_eq!(hidden.code, Code::Ok);
    assert_eq!(hidden.result.to_str().as_ref(), "hidden");
    assert_eq!(eval_value(&mut vm, "p"), "replacement");
    assert!(matches!(
        vm.hidden.commands.get("held"),
        Some(Command::Proc(proc))
            if proc.body_src.native_bytecode_cache().is_some_and(|cache| cache.unit.profile_generation == vm.profile_generation)
    ));

    let child_name = vm.create_child(Some("child".to_owned()), false);
    let child = vm.child_id(&child_name).unwrap();
    vm.in_interp(child, prepare_stale_hidden_proc);

    let named = vm.invoke_hidden_in_child("child", "held", &[]).unwrap();
    assert_eq!(named.code, Code::Ok);
    assert_eq!(named.result.to_str().as_ref(), "hidden");
    assert_eq!(
        vm.in_interp(child, |child| eval_value(child, "p")),
        "replacement"
    );

    vm.in_interp(child, |child| select_release(child, "tcl8.5"));
    let by_id = vm.invoke_hidden_by_id(child, "held", &[]).unwrap();
    assert_eq!(by_id.code, Code::Ok);
    assert_eq!(by_id.result.to_str().as_ref(), "hidden");
    assert_eq!(
        vm.in_interp(child, |child| eval_value(child, "p")),
        "replacement"
    );
    let state = vm.st_of(child).unwrap();
    assert!(matches!(
        state.hidden.commands.get("held"),
        Some(Command::Proc(proc))
            if proc.body_src.native_bytecode_cache().is_some_and(|cache| cache.unit.profile_generation == state.profile_generation)
    ));
}

#[test]
fn trace_deleted_hidden_refresh_never_resurrects_or_overwrites_replacement() {
    let mut vm = release_vm(tcl_dialect::TclVersion::V8_6);
    prepare_trace_deleted_hidden_proc(&mut vm);
    let root = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
    assert_eq!(root.code, Code::Error);
    assert_eq!(
        root.result.to_str().as_ref(),
        "attempt to invoke a deleted command"
    );
    assert_eq!(eval_value(&mut vm, "p"), "replacement");
    assert!(!vm.hidden.commands.contains_key("held"));

    for by_id in [false, true] {
        let child_name = vm.create_child(None, false);
        let child = vm.child_id(&child_name).unwrap();
        vm.in_interp(child, prepare_trace_deleted_hidden_proc);
        let result = if by_id {
            vm.invoke_hidden_by_id(child, "held", &[]).unwrap()
        } else {
            vm.invoke_hidden_in_child(&child_name, "held", &[]).unwrap()
        };
        assert_eq!(result.code, Code::Error);
        assert_eq!(
            result.result.to_str().as_ref(),
            "attempt to invoke a deleted command"
        );
        assert_eq!(
            vm.in_interp(child, |child| eval_value(child, "p")),
            "replacement"
        );
        assert!(
            !vm.st_of(child)
                .unwrap()
                .hidden
                .commands
                .contains_key("held")
        );
    }
}

#[test]
fn relocated_hidden_refresh_persists_at_the_live_visible_key() {
    let mut vm = Vm::new();
    select_release(&mut vm, "tcl8.5");
    assert_eq!(eval_value(&mut vm, "proc p {} { return hidden }"), "");
    assert_eq!(
        eval_value(
            &mut vm,
            "proc expose args { rename p {}; interp expose {} held p; trace remove execution p enter expose }"
        ),
        ""
    );
    assert_eq!(
        eval_value(&mut vm, "trace add execution p enter expose"),
        ""
    );
    vm.hide_command("p", "held").unwrap();
    assert_eq!(eval_value(&mut vm, "proc p {} { return replacement }"), "");
    select_release(&mut vm, "tcl8.6");

    let result = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(result.result.to_str().as_ref(), "hidden");
    assert!(!vm.hidden.commands.contains_key("held"));
    assert_eq!(eval_value(&mut vm, "p"), "hidden");
}

#[test]
fn imported_proc_profile_refresh_updates_the_resolved_import_not_its_source_name() {
    let mut vm = Vm::new();
    select_release(&mut vm, "tcl8.5");
    assert_eq!(
        eval_value(
            &mut vm,
            "namespace eval src {proc p {} {return original}; namespace export p}; \\
                 namespace eval dst {namespace import ::src::p}; \\
                 rename ::src::p ::src::q; proc ::src::p {} {return replacement}",
        ),
        ""
    );

    select_release(&mut vm, "tcl8.6");
    assert_eq!(eval_value(&mut vm, "::dst::p"), "original");
    assert_eq!(eval_value(&mut vm, "::src::p"), "replacement");
    assert_eq!(eval_value(&mut vm, "::src::q"), "original");
    assert!(matches!(
        vm.commands.get("dst::p"),
        Some(Command::Proc(proc))
            if proc.body_src.native_bytecode_cache().is_some_and(|cache| cache.unit.profile_generation == vm.profile_generation)
    ));
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
