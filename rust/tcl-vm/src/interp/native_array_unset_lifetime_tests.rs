// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Software cell-retention controls independent of public native observations.

use std::cell::Cell;
use std::rc::Rc;

use super::{Local, VarId, Vm};
use crate::value::Value;
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
};

#[derive(Clone, Copy)]
enum Mutation {
    None,
    Refill,
    DestroyRoot,
}

struct ObserveMember {
    root: VarId,
    member: VarId,
    mutation: Mutation,
    callbacks: Rc<Cell<usize>>,
}

impl NativeVariableObserver<Vm> for ObserveMember {
    type Error = tcl_cmd_core::CmdError;

    fn observe(
        &self,
        vm: &mut Vm,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        if access.name2.is_empty() {
            return Ok(());
        }
        assert_eq!(access.operation, NativeVariableTraceOperation::Unset);
        assert_eq!(access.name1, b"a");
        assert_eq!(access.name2, b"x");
        assert!(vm.var_arena.has_operation_refs(self.root));
        assert!(vm.var_arena.has_operation_refs(self.member));
        assert!(matches!(
            vm.var_arena
                .get(self.member)
                .map(crate::vars::VarCell::state),
            Some(Local::Undefined)
        ));
        assert_eq!(
            vm.resolve_var_parts_from_bytes(b"a", Some(b"x"), 0)
                .and_then(|selected| selected.id),
            Some(self.member)
        );
        self.callbacks.set(self.callbacks.get() + 1);
        match self.mutation {
            Mutation::None => {}
            Mutation::Refill => vm
                .write_array_raw(
                    "a",
                    "x",
                    Value::new_native_string_bytes(b"REFILL".as_slice()),
                )
                .unwrap(),
            Mutation::DestroyRoot => assert!(vm.unset_var("a")),
        }
        Ok(())
    }
}

#[test]
fn selected_array_member_is_live_through_parent_unset_callback_and_conditional_cleanup() {
    // Implementation binding: naming.variable.original-array-unset-trace-refill
    // docs/design/analysis/name-resolution-proofs/variable-original-array-unset-trace-refill.md
    // These internal software IDs and operation holds are independent of the
    // original public Tcl results; they do not identify private native cells.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mutation in [Mutation::None, Mutation::Refill, Mutation::DestroyRoot] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::core(profile);
            vm.write_array_raw("a", "x", Value::new_native_string_bytes(b"OLD".as_slice()))
                .unwrap();
            let selected = vm
                .resolve_var_parts_from_bytes(b"a", Some(b"x"), 0)
                .unwrap();
            let root = selected.base_id.unwrap();
            let member = selected.id.unwrap();
            let callbacks = Rc::new(Cell::new(0));
            vm.add_native_variable_observer(
                &Value::new_native_string_bytes(b"a".as_slice()),
                &[NativeVariableTraceOperation::Unset],
                Rc::new(ObserveMember {
                    root,
                    member,
                    mutation,
                    callbacks: Rc::clone(&callbacks),
                }),
            )
            .unwrap();
            assert!(!vm.variable_observers.script_traces.contains_key(&member));
            assert!(vm.array_unset_elem_reporting_bytes_at(root, b"a", b"x", None));
            assert_eq!(callbacks.get(), 1, "{engine}");
            assert!(vm.refused_completion().is_none(), "{engine}");
            match mutation {
                Mutation::None => {
                    assert!(vm.var_arena.get(member).is_none());
                    assert!(vm.get_array_elem("a", "x").is_none());
                    assert!(matches!(
                        vm.var_arena.get(root).map(crate::vars::VarCell::state),
                        Some(Local::Array(_))
                    ));
                }
                Mutation::Refill => {
                    assert_eq!(
                        vm.resolve_var_parts_from_bytes(b"a", Some(b"x"), 0)
                            .and_then(|selected| selected.id),
                        Some(member)
                    );
                    assert_eq!(
                        vm.get_array_elem("a", "x").unwrap().string_bytes().as_ref(),
                        b"REFILL"
                    );
                    assert!(!vm.var_arena.has_operation_refs(member));
                }
                Mutation::DestroyRoot => {
                    assert!(vm.var_arena.get(member).is_none());
                    assert!(vm.var_arena.get(root).is_none());
                    assert!(vm.get_array_elem("a", "x").is_none());
                }
            }
        }
    }
}
