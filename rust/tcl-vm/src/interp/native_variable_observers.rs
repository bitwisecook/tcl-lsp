//! Direct native callbacks share the stable-cell variable trace coordinator.

use super::{Completion, GuardDomain, Rc, Value, VarTrace, Vm};
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceOperation, NativeVariableTraceToken,
};

impl Vm {
    /// Register a direct native C callback on the actual cell selected by the original name.
    pub fn add_native_variable_observer(
        &mut self,
        original: &Value,
        operations: &[NativeVariableTraceOperation],
        observer: Rc<dyn NativeVariableObserver<Vm, Error = tcl_cmd_core::CmdError>>,
    ) -> Result<NativeVariableTraceToken, Completion<Value>> {
        if self
            .actual_native_invocation_dialect()
            .native_variable_trace_protocol()
            .is_none()
        {
            return Err(crate::command::completion_from_cmd_error(
                self,
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native C variable observer",
                )
                .into(),
            ));
        }
        let name = self.native_name_operand_bytes(original).map_err(|error| {
            crate::command::completion_from_cmd_error(
                self,
                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error).into(),
            )
        })?;
        self.ensure_trace_variable_bytes(&name)?;
        let key = self
            .registered_trace_cell_bytes(&name)
            .and_then(|cell| cell.id)
            .ok_or_else(|| {
                self.refuse_host_command("native observer cell is unavailable".into())
            })?;
        self.next_var_trace_id += 1;
        let id = self.next_var_trace_id;
        let token = NativeVariableTraceToken::new();
        if !self.variable_observers.script_traces.contains_key(&key) {
            self.var_arena.pin(key);
        }
        self.variable_observers
            .script_traces
            .entry(key)
            .or_default()
            .push(VarTrace {
                id,
                ops: operations.iter().map(|op| op.as_str().to_owned()).collect(),
                command: None,
                native: Some((token.clone(), observer)),
                old_style: false,
            });
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
        Ok(token)
    }
    /// Remove this actual live native row; no variable-name lookup is replayed.
    pub fn remove_native_variable_observer(&mut self, token: &NativeVariableTraceToken) -> bool {
        let found = self
            .variable_observers
            .script_traces
            .iter()
            .find_map(|(key, list)| {
                list.iter()
                    .position(|t| {
                        t.native
                            .as_ref()
                            .is_some_and(|(live, _)| live.same_registration(token))
                    })
                    .map(|index| (*key, index))
            });
        let Some((key, index)) = found else {
            return false;
        };
        let list = self
            .variable_observers
            .script_traces
            .get_mut(&key)
            .expect("live native row");
        list.remove(index);
        if list.is_empty() {
            self.variable_observers.script_traces.remove(&key);
            self.var_arena.unpin(key);
            if let Some((parent, element)) = self
                .var_arena
                .element_parent(key)
                .map(|(parent, key)| (parent, key.to_owned()))
            {
                self.discard_undefined_array_shell(parent, &element, key);
            }
        }
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::DefaultHost;
    use std::cell::RefCell;
    struct Record {
        events: Rc<RefCell<Vec<(u8, NativeVariableTraceOperation)>>>,
        label: u8,
        nested: bool,
    }
    impl NativeVariableObserver<Vm> for Record {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            vm: &mut Vm,
            access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push((self.label, access.operation));
            assert_eq!(access.name1, b"x");
            if self.nested && access.operation == NativeVariableTraceOperation::Write {
                vm.set_var_bytes(b"x", Value::int(9))
                    .expect("same-cell nested write");
            }
            Ok(())
        }
    }
    #[test]
    fn native_rows_share_cell_order_reentrancy_and_retirement_without_guest_exposure() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = Vm::with_native_core(
                Box::new(Vec::<u8>::new()),
                Rc::new(DefaultHost::new()),
                tcl_registry::model::resolve_environment(profile).unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let events = Rc::new(RefCell::new(Vec::new()));
            let name = Value::new_native_string_bytes(b"x".as_slice());
            let a = vm
                .add_native_variable_observer(
                    &name,
                    &[
                        NativeVariableTraceOperation::Write,
                        NativeVariableTraceOperation::Unset,
                    ],
                    Rc::new(Record {
                        events: events.clone(),
                        label: 1,
                        nested: false,
                    }),
                )
                .unwrap();
            let b = vm
                .add_native_variable_observer(
                    &name,
                    &[NativeVariableTraceOperation::Write],
                    Rc::new(Record {
                        events: events.clone(),
                        label: 2,
                        nested: true,
                    }),
                )
                .unwrap();
            assert!(vm.var_trace_info_bytes(b"x").is_empty());
            vm.set_var_bytes(b"x", Value::int(1)).unwrap();
            assert_eq!(
                *events.borrow(),
                [
                    (2, NativeVariableTraceOperation::Write),
                    (1, NativeVariableTraceOperation::Write)
                ],
                "{profile}"
            );
            assert!(vm.remove_native_variable_observer(&b));
            assert!(!vm.remove_native_variable_observer(&b));
            vm.unset_var_bytes(b"x");
            assert_eq!(
                events.borrow().last(),
                Some(&(1, NativeVariableTraceOperation::Unset))
            );
            assert!(!vm.remove_native_variable_observer(&a));
        }
    }
}
