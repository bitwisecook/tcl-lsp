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

fn native() -> Vm {
    crate::native_fixture::interpreter(tcl_dialect::DialectProfile::find("tcl9.0").unwrap())
}
fn capture(
    vm: &Vm,
) -> (
    tcl_runtime_api::native_compiler_pass::NativeCompilerPassEnvironment,
    tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
) {
    let entry = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
    (entry.compiler_pass_environment.unwrap(), entry.interpreter)
}
fn remove_trace_while_firing(vm: &mut Vm, _argv: &[Value]) -> Completion<Value> {
    assert_eq!(
        vm.try_eval_source("trace remove execution watched enter callback")
            .unwrap()
            .code,
        tcl_runtime_api::Code::Ok
    );
    assert!(vm.exec_traces.values().all(Vec::is_empty));
    let (environment, id) = capture(vm);
    assert!(!environment.ordinary_entry_observers_absent_for(id));
    ok(Value::string(""))
}

#[test]
fn self_removing_execution_trace_remains_an_observer_until_callback_settles() {
    let mut vm = native();
    vm.register("callback", remove_trace_while_firing);
    assert_eq!(
        vm.try_eval_source(
            "proc watched {} {}; trace add execution watched enter callback; watched"
        )
        .unwrap()
        .code,
        tcl_runtime_api::Code::Ok
    );
    let (environment, id) = capture(&vm);
    assert!(environment.ordinary_entry_observers_absent_for(id));
}

#[test]
fn actual_declaration_initialization_plan_rejects_old_capture_foreign_owner_and_restore() {
    let mut vm = native();
    assert_eq!(
        vm.try_eval_source("proc p {} {set x A; set x B}")
            .unwrap()
            .code,
        tcl_runtime_api::Code::Ok
    );
    let declaration = vm.proc_def("p").unwrap();
    let bytes = vm
        .native_name_operand_bytes_checked(&declaration.body_src)
        .unwrap();
    let source = tcl_runtime_api::SourceImage::native(bytes.as_ref());
    let entry = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
    let namespace = tcl_core_types::ByteNamespacePath::root();
    let inputs = ProcedureCompilationInputs::new(
        ProcedureBodySource {
            image: &source,
            declaration: Some(&declaration),
        },
        &declaration.params,
        &namespace,
        source.len(),
        &vm.native_compiler_pass_owner,
        entry.interpreter,
        tcl_registry::native_procedure::procedure_activation_protocol(
            vm.actual_native_invocation_dialect(),
        ),
    )
    .unwrap();
    let request = inputs.original.unwrap();
    assert!(request.ordinary_frame_initialization_for(&entry, tcl_dialect::TclVersion::V9_0));
    assert!(!request.ordinary_frame_initialization_for(&entry, tcl_dialect::TclVersion::V8_6));
    let mut missing_policy = entry.clone();
    missing_policy.invocation_policy = None;
    assert!(
        !request.ordinary_frame_initialization_for(&missing_policy, tcl_dialect::TclVersion::V9_0)
    );
    let mut logical_policy = entry.clone();
    logical_policy.invocation_policy = Some(
        tcl_dialect::DialectProfile::find("f5-irules")
            .unwrap()
            .cache_key(),
    );
    assert!(
        !request.ordinary_frame_initialization_for(&logical_policy, tcl_dialect::TclVersion::V9_0)
    );
    let mut foreign_engine = entry.clone();
    foreign_engine.execution_point = Some(tcl_dialect::model::DialectPoint::for_tcl_version(
        tcl_dialect::TclVersion::V8_6,
    ));
    assert!(
        !request.ordinary_frame_initialization_for(&foreign_engine, tcl_dialect::TclVersion::V9_0)
    );
    let foreign =
        tcl_runtime_api::native_compiler_pass::NativeCompilerPassOwner::new(entry.interpreter);
    let rejected = request
        .clone()
        .with_ordinary_frame_initialization(&foreign, tcl_dialect::TclVersion::V9_0);
    assert!(!rejected.ordinary_frame_initialization_for(&entry, tcl_dialect::TclVersion::V9_0));
    let mut old_entry = entry.clone();
    old_entry.compiler_pass_environment = Some(
        vm.native_compiler_pass_owner
            .capture(true, false, false, None),
    );
    assert!(!request.ordinary_frame_initialization_for(&old_entry, tcl_dialect::TclVersion::V9_0));
    vm.set_debug_hook(Some(Box::new(|_| crate::debug::DebugAction::Continue)));
    vm.set_debug_hook(None);
    let restored = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
    assert!(
        restored
            .compiler_pass_environment
            .as_ref()
            .unwrap()
            .ordinary_entry_observers_absent_for(entry.interpreter)
    );
    assert!(!request.ordinary_frame_initialization_for(&restored, tcl_dialect::TclVersion::V9_0));
    let fresh = request.with_ordinary_frame_initialization(
        &vm.native_compiler_pass_owner,
        tcl_dialect::TclVersion::V9_0,
    );
    assert!(fresh.ordinary_frame_initialization_for(&restored, tcl_dialect::TclVersion::V9_0));
}

#[test]
fn registered_removed_and_firing_execution_observers_never_revive_old_receipt() {
    let mut vm = native();
    let (original, id) = capture(&vm);
    assert!(original.ordinary_entry_observers_absent_for(id));
    assert_eq!(
        vm.try_eval_source(
            "proc watched {} {}; proc callback args {}; trace add execution watched enter callback"
        )
        .unwrap()
        .code,
        tcl_runtime_api::Code::Ok
    );
    assert!(!original.ordinary_entry_observers_absent_for(id));
    assert!(!capture(&vm).0.ordinary_entry_observers_absent_for(id));
    assert_eq!(
        vm.try_eval_source("trace remove execution watched enter callback")
            .unwrap()
            .code,
        tcl_runtime_api::Code::Ok
    );
    assert!(capture(&vm).0.ordinary_entry_observers_absent_for(id));
    assert!(!original.ordinary_entry_observers_absent_for(id));
}
#[test]
fn debug_hooks_limits_and_retirement_withdraw_but_line_watch_does_not() {
    let mut vm = native();
    let (original, id) = capture(&vm);
    vm.set_line_watch(Some(Rc::new(std::cell::Cell::new(0))));
    assert!(original.ordinary_entry_observers_absent_for(id));
    let marker = Rc::clone(&vm.debug_hooks.debug_hook_active);
    let outer = DebugHookActivation::enter(&marker);
    let nested = DebugHookActivation::enter(&marker);
    assert!(!capture(&vm).0.ordinary_entry_observers_absent_for(id));
    drop(nested);
    assert!(marker.get());
    drop(outer);
    assert!(!marker.get());
    let calls = Rc::new(std::cell::Cell::new(0));
    let seen = Rc::clone(&calls);
    vm.set_debug_hook(Some(Box::new(move |_| {
        seen.set(seen.get() + 1);
        crate::debug::DebugAction::Continue
    })));
    assert!(!original.ordinary_entry_observers_absent_for(id));
    assert!(!capture(&vm).0.ordinary_entry_observers_absent_for(id));
    vm.debug_hooks.observer_probe = Some(|vm| {
        assert!(vm.debug_hooks.debug_hook.is_none());
        assert!(vm.debug_hooks.debug_hook_active.get());
        let (environment, id) = capture(vm);
        assert!(!environment.ordinary_entry_observers_absent_for(id));
    });
    assert!(!vm.fire_debug_hook(1, "set x 1"));
    vm.debug_hooks.observer_probe = None;
    assert_eq!(calls.get(), 1);
    assert!(!vm.debug_hooks.debug_hook_active.get());
    vm.set_debug_hook(Some(Box::new(|_| panic!("controlled debug hook unwind"))));
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        vm.fire_debug_hook(2, "set x 2")
    }));
    assert!(unwound.is_err());
    assert!(!marker.get());
    assert!(!original.ordinary_entry_observers_absent_for(id));
    vm.set_debug_hook(None);
    let (restored, _) = capture(&vm);
    assert!(restored.ordinary_entry_observers_absent_for(id));
    vm.set_command_limit_value(Some(10));
    assert!(!restored.ordinary_entry_observers_absent_for(id));
    assert!(!capture(&vm).0.ordinary_entry_observers_absent_for(id));
    vm.set_command_limit_value(None);
    let (restored, _) = capture(&vm);
    vm.set_time_limit_deadline(Some(100));
    assert!(!restored.ordinary_entry_observers_absent_for(id));
    vm.set_time_limit_deadline(None);
    let (restored, _) = capture(&vm);
    assert!(restored.ordinary_entry_observers_absent_for(id));
    drop(vm);
    assert!(!restored.ordinary_entry_observers_absent_for(id));
}
