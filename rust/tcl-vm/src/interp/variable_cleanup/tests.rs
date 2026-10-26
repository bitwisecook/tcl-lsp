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

//! Actual software arena/trace roles; no Native callback chronology grant.
use super::*;
use crate::Value;
use std::cell::Cell;
use tcl_runtime_api::{
    Traces,
    native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    },
};

fn native() -> Vm {
    crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.1"))
}
fn child(vm: &mut Vm) -> InterpId {
    let name = vm.create_child(Some("variable_cleanup_child".into()), false);
    vm.child_id(&name).unwrap()
}
fn first(vm: &Vm) -> NativeExecutionError {
    vm.execution_refusal
        .clone()
        .expect("full original first cause")
}

struct Observer {
    child: InterpId,
    mode: u8,
    calls: Rc<Cell<usize>>,
    drops: Rc<Cell<usize>>,
}
impl NativeVariableObserver<Vm> for Observer {
    type Error = CmdError;
    fn observe(&self, vm: &mut Vm, _: NativeVariableTraceAccess<'_>) -> Result<(), CmdError> {
        self.calls.set(self.calls.get() + 1);
        match self.mode {
            0 => Ok(()),
            1 => {
                vm.switch_to(self.child);
                Ok(())
            }
            2 => {
                vm.push_ns_eval_token_frame(super::super::ROOT_NS, Vec::new());
                Ok(())
            }
            3 => {
                vm.push_ns_eval_token_frame(super::super::ROOT_NS, Vec::new());
                vm.pop_call_frame();
                vm.pop_ns();
                Ok(())
            }
            4 => {
                let _ = vm.refuse_host_command("FIRST_VARIABLE_CLEANUP".into());
                Ok(())
            }
            _ => unreachable!(),
        }
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
fn observer(
    vm: &mut Vm,
    name: &[u8],
    operation: NativeVariableTraceOperation,
    target: InterpId,
    mode: u8,
) -> (Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    vm.add_native_variable_observer(
        &Value::new_native_string_bytes(name),
        &[operation],
        Rc::new(Observer {
            child: target,
            mode,
            calls: Rc::clone(&calls),
            drops: Rc::clone(&drops),
        }),
    )
    .unwrap();
    (calls, drops)
}

#[test]
fn original_trace_epilogue_releases_original_operations_and_keeps_child_storage() {

    let mut vm = native();
    let original_id = vm.cur;
    let target = child(&mut vm);
    vm.set_var_bytes(b"x", Value::int(1)).unwrap();
    let original_cell = vm.trace_cell_bytes(b"x").unwrap().id.unwrap();
    let child_cell = vm.in_interp(target, |vm| {
        vm.set_var_bytes(b"x", Value::int(9)).unwrap();
        vm.trace_cell_bytes(b"x").unwrap().id.unwrap()
    });
    let (calls, _) = observer(&mut vm, b"x", NativeVariableTraceOperation::Read, target, 1);
    let held = OriginalVariableCleanup::enter(&mut vm);
    let error = Traces::fire_bytes(&mut vm, b"x", "read").unwrap_err();
    assert_eq!(calls.get(), 1);
    assert_eq!(error, first(&vm));
    assert_eq!(vm.cur, target);
    let original = vm.st_of(original_id).unwrap();
    assert!(original.active_traces.is_empty());
    assert!(!original.var_arena.has_operation_refs(original_cell));
    assert!(vm.active_traces.is_empty());
    assert!(!vm.var_arena.has_operation_refs(child_cell));
    assert!(
        matches!(vm.var_arena.get(child_cell).unwrap().state(), Local::Scalar(value)
        if matches!(value.native_scalar_cache(), Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(tcl_syntax::number::Number::Int(9)))))
    );
    assert_eq!(held.check_native(&mut vm).unwrap_err(), error);
    vm.switch_to(original_id);
}

#[test]
fn original_owned_variable_teardown_keeps_first_cause_and_releases_its_real_trace_roles() {

    for mode in [0, 1, 4] {
        let mut vm = native();
        let original_id = vm.cur;
        let target = child(&mut vm);
        vm.set_var_bytes(b"x", Value::new_native_string_bytes(b"OWNED".as_slice()))
            .unwrap();
        let raw = vm.trace_cell_bytes(b"x").unwrap().id.unwrap();
        let header = match vm.var_arena.get(raw).unwrap().state() {
            Local::Scalar(value) => value.native_lifetime_lease(),
            _ => panic!("actual scalar"),
        };
        let (calls, drops) = observer(
            &mut vm,
            b"x",
            NativeVariableTraceOperation::Unset,
            target,
            mode,
        );
        let removed = vm.frames[0].locals.take_binding(b"x").unwrap();
        assert_eq!(removed.release_key(), raw);
        let held = OriginalVariableCleanup::enter(&mut vm);
        vm.destroy_owned_variable(b"x", raw, None);
        assert_eq!(calls.get(), 1);
        assert_eq!(drops.get(), 1);
        let original = vm.st_of(original_id).unwrap();
        assert!(original.var_arena.get(raw).is_none());
        assert!(original.active_traces.is_empty());
        assert!(!original.variable_observers.script_traces.contains_key(&raw));
        assert_eq!(header.value().native_object_reference_count(), 0);
        assert!(!header.value().native_object_is_live());
        if mode == 0 {
            held.check(&mut vm).unwrap();
        } else {
            let retained = first(&vm);
            assert_eq!(held.check_native(&mut vm).unwrap_err(), retained);
            assert_eq!(vm.cur, if mode == 1 { target } else { original_id });
        }
        vm.switch_to(original_id);
    }
}

#[test]
fn original_variable_finish_refuses_unbalanced_frame_and_allows_balanced_reentry() {

    for mode in [2, 3] {
        let mut vm = native();
        let target = child(&mut vm);
        vm.set_var_bytes(b"x", Value::int(1)).unwrap();
        let id = vm.trace_cell_bytes(b"x").unwrap().id.unwrap();
        observer(
            &mut vm,
            b"x",
            NativeVariableTraceOperation::Read,
            target,
            mode,
        );
        let held = OriginalVariableCleanup::enter(&mut vm);
        let reached = Traces::fire_bytes(&mut vm, b"x", "read");
        assert!(vm.active_traces.is_empty());
        assert!(!vm.var_arena.has_operation_refs(id));
        if mode == 2 {
            assert_eq!(reached.unwrap_err(), first(&vm));
            assert_eq!(vm.current_level(), 1);
            assert_eq!(held.check_native(&mut vm).unwrap_err(), first(&vm));
            vm.pop_call_frame();
            vm.pop_ns();
        } else {
            let _ = reached.unwrap();
            held.check(&mut vm).unwrap();
            assert_eq!(vm.current_level(), 0);
        }
    }
}

#[test]
fn original_arena_withdrawal_never_releases_same_numbered_replacement_cell() {

    // Explicit real storage allocation fixture; a VarId grants no Native purpose.
    let mut vm = native();
    vm.set_var_bytes(b"x", Value::int(1)).unwrap();
    let id = vm.trace_cell_bytes(b"x").unwrap().id.unwrap();
    vm.var_arena.pin(id);
    let held = OriginalVariableCleanup::enter(&mut vm);
    let original = std::mem::take(&mut vm.var_arena);
    for ordinal in 0..=id.0 {
        assert_eq!(
            vm.var_arena.alloc(Local::Undefined).unwrap(),
            VarId(ordinal)
        );
    }
    vm.var_arena.pin(id);
    let _ = held.retire(&mut vm, VariableRetirement::unpin(id));
    let retained = first(&vm);
    assert_eq!(vm.var_arena.cleanup_pin_count(id), Some(1));
    assert_eq!(original.cleanup_pin_count(id), Some(1));
    let replacement = std::mem::replace(&mut vm.var_arena, original);
    assert_eq!(held.check_native(&mut vm).unwrap_err(), retained);
    vm.var_arena.unpin(id);
    drop(replacement);
}

#[test]
fn original_dying_namespace_table_finishes_owned_rows_after_child_callback() {

    let mut vm = native();
    let original_id = vm.cur;
    let target = child(&mut vm);
    let ns = vm.definition_namespace_token("cleanup_ns");
    vm.set_var_bytes(b"::cleanup_ns::a", Value::int(1)).unwrap();
    vm.set_var_bytes(b"::cleanup_ns::z", Value::int(2)).unwrap();
    let rows: Vec<_> = vm.ns_vars[&ns].values().copied().collect();
    vm.in_interp(target, |vm| {
        vm.set_var_bytes(
            b"child_keep",
            Value::new_native_string_bytes(b"CHILD".as_slice()),
        )
        .unwrap();
    });
    let (calls, drops) = observer(
        &mut vm,
        b"::cleanup_ns::a",
        NativeVariableTraceOperation::Unset,
        target,
        1,
    );
    let held = OriginalVariableCleanup::enter(&mut vm);
    vm.teardown_namespace_variables(b"cleanup_ns", ns);
    assert_eq!(calls.get(), 1);
    assert_eq!(drops.get(), 1);
    assert_eq!(vm.cur, target);
    let original = vm.st_of(original_id).unwrap();
    assert!(!original.ns_vars.contains_key(&ns));
    for id in rows {
        assert!(original.var_arena.get(id).is_none());
    }
    assert!(original.active_traces.is_empty());
    assert_eq!(held.check_native(&mut vm).unwrap_err(), first(&vm));
    vm.switch_to(original_id);
}

#[test]
fn original_c_array_detach_keeps_default_roles_until_actual_old_member_retirement() {

    // Software counts of the actual C91 descriptor, not public Native free callbacks.
    let mut vm = native();
    vm.write_array_raw(
        "a",
        "k",
        Value::new_native_string_bytes(b"MEMBER".as_slice()),
    )
    .unwrap();
    let id = vm.trace_cell_bytes(b"a").unwrap().id.unwrap();
    let default = Value::new_native_string_bytes(b"DEFAULT".as_slice());
    assert!(vm.var_arena.set_array_default(id, Some(default.clone())));
    assert_eq!(default.native_object_reference_count(), 3);
    let held = OriginalVariableCleanup::enter(&mut vm);
    let detached = held.unset_state(&mut vm, id, false).unwrap();
    assert!(matches!(
        vm.var_arena.get(id).unwrap().state(),
        Local::Undefined
    ));
    assert_eq!(default.native_object_reference_count(), 3);
    let (old, roles) = detached.into_parts();
    let _ = held.retire(&mut vm, VariableRetirement::state(old));
    assert_eq!(default.native_object_reference_count(), 3);
    held.in_original(&mut vm, |_| drop(roles)).unwrap();
    assert_eq!(default.native_object_reference_count(), 1);
    held.check(&mut vm).unwrap();
}
