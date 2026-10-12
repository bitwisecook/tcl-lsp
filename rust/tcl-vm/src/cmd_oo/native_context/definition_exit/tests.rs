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

//! Genuine definition producers and callback cleanup; software roles only.
use super::*;
use crate::command::{Command, NativeCommand, ok};
use crate::interp::InterpId;
use std::cell::Cell;
use tcl_runtime_api::Code;

fn provided() -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.1"))
}
fn class(vm: &mut Vm, name: &str) -> OoId {
    let completion =
        vm.invoke_command("oo::class", &[Value::string("create"), Value::string(name)]);
    assert_eq!(completion.code, Code::Ok);
    crate::cmd_oo::resolve_object_value(vm, &Value::string(name))
        .unwrap()
        .unwrap()
}
fn child(vm: &mut Vm) -> InterpId {
    let name = vm.create_child(Some("definition_exit_child".into()), false);
    vm.child_id(&name).unwrap()
}
fn switch(vm: &mut Vm, target: InterpId) {
    crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(vm, target);
}
fn first(vm: &Vm) -> tcl_runtime_api::NativeExecutionError {
    vm.execution_refusal
        .clone()
        .expect("full original definition first cause")
}
fn enter_class(vm: &mut Vm, target: OoId) -> OriginalDefinitionExit {
    enter(
        vm,
        true,
        target,
        vec![
            Value::string("oo::define"),
            Value::string("DefinitionExitC"),
        ],
    )
    .unwrap()
}

struct DefinitionObserver {
    mode: u8,
    child: InterpId,
    target: OoId,
    calls: Rc<Cell<usize>>,
}
impl NativeCommand for DefinitionObserver {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        assert!(
            super::super::definition_index(vm).is_some()
                || super::super::current_method(vm).is_some()
        );
        match self.mode {
            0 => ok(Value::string("DEFINITION_BODY")),
            1 => vm.refuse_host_command("FIRST_DEFINITION_BODY".into()),
            2 => {
                switch(vm, self.child);
                let namespace = vm.definition_namespace_token("definition_child_activation");
                vm.push_ns_eval_token_frame(namespace, Vec::new());
                // Explicit private child input; it supplies no original-definition purpose.
                vm.oo.definition_usage = DefinitionUsage::new(
                    Some(vec![Value::string("CHILD_USAGE")]),
                    vm.native_oo_variable_activation(),
                );
                ok(Value::string("DEFINITION_BODY"))
            }
            3 => {
                let before = Rc::downgrade(&vm.oo.definition_usage.birth);
                let nested = crate::cmd_oo::run_define(
                    vm,
                    &[
                        Value::string("DefinitionExitC"),
                        Value::string("variable"),
                        Value::string("NESTED_SLOT"),
                    ],
                    true,
                );
                assert_eq!(nested.code, Code::Ok);
                assert!(Weak::ptr_eq(
                    &before,
                    &Rc::downgrade(&vm.oo.definition_usage.birth)
                ));
                assert_eq!(crate::cmd_oo::active_target(vm).unwrap().1, self.target);
                ok(Value::string("DEFINITION_BODY"))
            }
            _ => unreachable!(),
        }
    }
}
fn observer(vm: &mut Vm, mode: u8, child: InterpId, target: OoId) -> Rc<Cell<usize>> {
    let calls = Rc::new(Cell::new(0));
    vm.register_command(
        "definition_exit_observer",
        Command::Native(Rc::new(DefinitionObserver {
            mode,
            child,
            target,
            calls: Rc::clone(&calls),
        })),
    );
    calls
}
fn restored(
    vm: &mut Vm,
    original: InterpId,
    child: InterpId,
    child_left: bool,
    expected_objects: usize,
) {
    assert!(vm.st_of(original).unwrap().oo.def_stack.is_empty());
    assert!(
        vm.st_of(original)
            .unwrap()
            .oo
            .definition_usage
            .words()
            .is_none()
    );
    assert_eq!(
        vm.st_of(original).unwrap().oo.objects.len(),
        expected_objects
    );
    if child_left {
        assert_eq!(vm.cur_interp(), child);
        assert_eq!(vm.current_level(), 1);
        assert_eq!(
            vm.oo.definition_usage.words().unwrap()[0]
                .resident_string_bytes()
                .unwrap()
                .as_ref(),
            b"CHILD_USAGE"
        );
        let selected = Rc::downgrade(&vm.retained_frame_identity(false));
        switch(vm, original);
        assert_eq!(vm.current_level(), 0);
        switch(vm, child);
        assert!(Weak::ptr_eq(
            &selected,
            &Rc::downgrade(&vm.retained_frame_identity(false))
        ));
        vm.pop_call_frame();
        vm.pop_ns();
        switch(vm, original);
    } else {
        assert_eq!(vm.cur_interp(), original);
        assert_eq!(vm.current_level(), 0);
    }
}

#[test]
fn original_definition_exit_restores_real_row_usage_and_frame_after_public_callback() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in 0..4 {
        let mut vm = provided();
        let original = vm.cur_interp();
        let child = child(&mut vm);
        let target = class(&mut vm, "DefinitionExitC");
        let objects = vm.oo.objects.len();
        let root_usage = Rc::downgrade(&vm.oo.definition_usage.birth);
        let calls = observer(&mut vm, mode, child, target);
        let completion = crate::cmd_oo::run_define(
            &mut vm,
            &[
                Value::string("DefinitionExitC"),
                Value::string("::definition_exit_observer"),
                Value::string("ARG"),
            ],
            true,
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            completion.code,
            if mode == 1 || mode == 2 {
                Code::Error
            } else {
                Code::Ok
            }
        );
        if mode == 1 || mode == 2 {
            assert!(vm.execution_refusal.is_some());
        } else {
            assert!(vm.execution_refusal.is_none());
            assert_eq!(
                completion.result.resident_string_bytes().unwrap().as_ref(),
                b"DEFINITION_BODY"
            );
        }
        assert!(Weak::ptr_eq(
            &root_usage,
            &Rc::downgrade(&vm.st_of(original).unwrap().oo.definition_usage.birth)
        ));
        restored(&mut vm, original, child, mode == 2, objects);
    }
}

#[test]
fn original_class_definition_failure_tears_down_only_its_allocated_original_record() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in 0..3 {
        let mut vm = provided();
        let original = vm.cur_interp();
        let child = child(&mut vm);
        let target = class(&mut vm, "DefinitionExitC");
        let objects = vm.oo.objects.len();
        let child_objects = vm.st_of(child).unwrap().oo.objects.len();
        let calls = observer(&mut vm, mode, child, target);
        let completion = vm.invoke_command(
            "oo::class",
            &[
                Value::string("create"),
                Value::string("DefinitionExitNewC"),
                Value::string("::definition_exit_observer"),
            ],
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            completion.code,
            if mode == 0 { Code::Ok } else { Code::Error }
        );
        assert_eq!(vm.st_of(child).unwrap().oo.objects.len(), child_objects);
        restored(
            &mut vm,
            original,
            child,
            mode == 2,
            objects + usize::from(mode == 0),
        );
    }
}

#[test]
fn original_definition_worker_and_self_rewrite_restore_the_same_selected_row() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in 0..3 {
        let mut vm = provided();
        let original = vm.cur_interp();
        let child = child(&mut vm);
        let target = class(&mut vm, "DefinitionExitC");
        let objects = vm.oo.objects.len();
        let calls = observer(&mut vm, mode, child, target);
        let mut exit = enter_class(&mut vm, target);
        exit.replace_usage(&mut vm, Some(vec![Value::string("OUTER_USAGE")]))
            .unwrap();
        let row = vm.oo.def_stack[0];
        let usage = Rc::downgrade(&vm.oo.definition_usage.birth);
        let worker = super::super::definition_worker(
            &mut vm,
            &[Value::string("OBJECT_SLOT")],
            false,
            "variable",
        );
        assert_eq!(worker.code, Code::Ok);
        assert!(same_row(&row, &vm.oo.def_stack[0]));
        let reached = super::super::definition_self_at(
            &mut vm,
            &[Value::string("::definition_exit_observer")],
            true,
        );
        assert_eq!(calls.get(), 1);
        assert!(same_row(&row, &vm.st_of(original).unwrap().oo.def_stack[0]));
        assert!(Weak::ptr_eq(
            &usage,
            &Rc::downgrade(&vm.st_of(original).unwrap().oo.definition_usage.birth)
        ));
        let finished = exit.finish(&mut vm, reached);
        assert_eq!(
            finished.code,
            if mode == 0 { Code::Ok } else { Code::Error }
        );
        restored(&mut vm, original, child, mode == 2, objects);
    }
}

#[test]
fn original_definition_rewrite_keeps_full_first_cause_through_physical_restoration() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = provided();
    let target = class(&mut vm, "DefinitionExitC");
    let exit = enter_class(&mut vm, target);
    let row = vm.oo.def_stack[0];
    let reached = with_target(&mut vm, 0, DefTarget::Object(target), |vm| {
        assert!(!crate::cmd_oo::active_target(vm).unwrap().0);
        vm.refuse_host_command("FIRST_TEMPORARY_DEFINITION".into())
    });
    let retained = first(&vm);
    assert!(same_row(&row, &vm.oo.def_stack[0]));
    let finished = exit.finish(&mut vm, reached);
    assert_eq!(finished.code, Code::Error);
    assert_eq!(vm.execution_refusal, Some(retained));
    assert!(vm.oo.def_stack.is_empty());
    assert_eq!(vm.current_level(), 0);
}

#[test]
fn original_definition_exit_preserves_genuine_replacement_row_and_usage_birth() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = provided();
    let target = class(&mut vm, "DefinitionExitC");
    let mut original = enter_class(&mut vm, target);
    original
        .replace_usage(&mut vm, Some(vec![Value::string("OLD_USAGE")]))
        .unwrap();
    let old_frame = Rc::downgrade(&vm.retained_frame_identity(false));
    vm.pop_call_frame();
    vm.pop_ns();
    let mut fresh = enter_class(&mut vm, target);
    fresh
        .replace_usage(&mut vm, Some(vec![Value::string("FRESH_USAGE")]))
        .unwrap();
    let fresh_frame = Rc::downgrade(&vm.retained_frame_identity(false));
    let fresh_usage = Rc::downgrade(&vm.oo.definition_usage.birth);
    let fresh_row = vm.oo.def_stack[1];
    let result = original.finish(&mut vm, ok(Value::empty()));
    let retained = first(&vm);
    assert_eq!(result.code, Code::Error);
    assert!(old_frame.upgrade().is_none());
    assert!(Weak::ptr_eq(
        &fresh_frame,
        &Rc::downgrade(&vm.retained_frame_identity(false))
    ));
    assert_eq!(vm.current_level(), 1);
    assert_eq!(vm.oo.def_stack.len(), 1);
    assert!(same_row(&fresh_row, &vm.oo.def_stack[0]));
    assert!(Weak::ptr_eq(
        &fresh_usage,
        &Rc::downgrade(&vm.oo.definition_usage.birth)
    ));
    assert_eq!(
        vm.oo.definition_usage.words().unwrap()[0]
            .resident_string_bytes()
            .unwrap()
            .as_ref(),
        b"FRESH_USAGE"
    );
    let result = fresh.finish(&mut vm, ok(Value::empty()));
    assert_eq!(result.code, Code::Error);
    assert_eq!(vm.execution_refusal, Some(retained));
    assert!(vm.oo.def_stack.is_empty());
    assert!(vm.oo.definition_usage.words().is_none());
    assert_eq!(vm.current_level(), 0);
}

#[test]
fn original_failed_procedure_constructor_keeps_child_state_and_retires_original_object() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in 0..3 {
        let mut vm = provided();
        let original = vm.cur_interp();
        let child = child(&mut vm);
        let target = class(&mut vm, "DefinitionExitC");
        let setup = crate::cmd_oo::run_define(
            &mut vm,
            &[
                Value::string("DefinitionExitC"),
                Value::string("constructor"),
                Value::empty(),
                Value::string("::definition_exit_observer"),
            ],
            true,
        );
        assert_eq!(setup.code, Code::Ok);
        let calls = observer(&mut vm, mode, child, target);
        let objects = vm.oo.objects.len();
        let child_objects = vm.st_of(child).unwrap().oo.objects.len();
        let completion = vm.invoke_command(
            "DefinitionExitC",
            &[
                Value::string("create"),
                Value::string("definitionExitObject"),
            ],
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            completion.code,
            if mode == 0 { Code::Ok } else { Code::Error }
        );
        assert_eq!(vm.st_of(child).unwrap().oo.objects.len(), child_objects);
        restored(
            &mut vm,
            original,
            child,
            mode == 2,
            objects + usize::from(mode == 0),
        );
    }
}

#[test]
fn original_definition_namespace_lookup_keeps_full_intrinsic_refusal_through_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for withdrawn in [false, true] {
        let mut vm = provided();
        let active = OriginalOoActivation::enter(&mut vm);
        let level = vm.current_level();
        let context = vm.current_ns_id();
        if withdrawn {
            crate::interp::oo_cleanup::withdraw_cleanup_fixture_name_world(&mut vm);
        }
        let reached = Namespaces::find_namespace_bytes_checked(&vm, context, b"::oo::define");
        let intrinsic = reached.as_ref().err().map(|error| {
            CmdError::from(error.clone())
                .native_access_refusal()
                .unwrap()
        });
        assert_eq!(intrinsic.is_some(), withdrawn);
        let joined = checked_definition_namespace(&mut vm, &active, reached);
        if withdrawn {
            let first = first(&vm);
            assert_eq!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(intrinsic.unwrap())
            );
            assert_eq!(joined.unwrap_err().native_execution_refusal(), Some(&first));
            let completion = active.finish(&mut vm, ok(Value::empty()));
            assert_eq!(completion.code, Code::Error);
            assert_eq!(vm.execution_refusal, Some(first));
        } else {
            assert!(joined.unwrap().is_some());
            let completion = active.finish(&mut vm, ok(Value::empty()));
            assert_eq!(completion.code, Code::Ok);
            assert!(vm.execution_refusal.is_none());
        }
        assert_eq!(vm.current_level(), level);
        assert!(vm.oo.def_stack.is_empty());
        assert!(vm.oo.definition_usage.words().is_none());
    }
}
