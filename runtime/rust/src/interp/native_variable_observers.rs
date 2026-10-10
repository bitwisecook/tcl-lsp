//! Direct native callbacks share the stable-cell variable trace coordinator.

use super::*;
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    NativeVariableTraceToken,
};

pub(super) type Observer = Rc<dyn NativeVariableObserver<Interp, Error = tcl_cmd_core::CmdError>>;
#[derive(Clone)]
pub(super) enum Callback {
    Script(Vec<u8>),
    Native(Observer),
}
impl Callback {
    pub(super) fn from_trace(trace: &crate::cmd_trace::VarTrace) -> Self {
        match &trace.native {
            Some((_, observer)) => Self::Native(observer.clone()),
            None => Self::Script(trace.command.clone()),
        }
    }
}
impl Interp {
    /// Register a direct C callback on the cell selected by the original name object.
    pub fn add_native_variable_observer(
        &mut self,
        original: *mut TclObj,
        operations: &[NativeVariableTraceOperation],
        observer: Rc<dyn NativeVariableObserver<Interp, Error = tcl_cmd_core::CmdError>>,
    ) -> Result<NativeVariableTraceToken, Code> {
        if self
            .native_invocation_dialect()
            .native_variable_trace_protocol()
            .is_none()
        {
            return Err(self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native C variable observer",
                )
                .into(),
            ));
        }
        let name = tcl_syntax::value::ValueOps::native_string_bytes(self, &original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let policy = self.name_policy_protocol().ok_or_else(|| {
            self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native variable observer name",
                )
                .into(),
            )
        })?;
        let projection = policy
            .recipe()
            .trace_registration_input(&name)
            .map_err(|_error| {
                self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native observer name projection",
                    )
                    .into(),
                )
            })?;
        let (base, spelled) = self
            .variable_name_parts(projection.selected())
            .map_err(|error| crate::cmd_trace::trace_var_error(self, &name, error))?;
        let linked = self.trace_identity(&base).link_elem;
        let elem = spelled.or(linked);
        match elem.as_deref() {
            Some(key) => self.ensure_trace_element(&base, key),
            None => self.ensure_trace_variable(&base),
        }
        .map_err(|error| crate::cmd_trace::trace_var_error(self, &name, error))?;
        let home = self.trace_identity(&base);
        let cell = self.variable_trace_scope(&home, elem.as_deref());
        let token = NativeVariableTraceToken::new();
        let mut table = self.traces.borrow_mut();
        let id = table.next_var_trace_id;
        table.next_var_trace_id += 1;
        table.traces.push(crate::cmd_trace::VarTrace {
            id,
            binding_id: home.binding_id,
            element_binding_id: cell.member_identity(),
            name: name.to_vec(),
            base: home.base,
            elem,
            ops: operations
                .iter()
                .map(|op| op.as_str().as_bytes().to_vec())
                .collect(),
            command: Vec::new(),
            native: Some((token.clone(), observer)),
            frame_level: home.level,
            ns: home.ns,
            old_style: false,
        });
        drop(table);
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
        Ok(token)
    }
    /// Remove only this still-live direct registration; a reused spelling grants no authority.
    pub fn remove_native_variable_observer(&mut self, token: &NativeVariableTraceToken) -> bool {
        let removed = {
            let mut traces = self.traces.borrow_mut();
            let Some(index) = traces.traces.iter().position(|t| {
                t.native
                    .as_ref()
                    .is_some_and(|(live, _)| live.same_registration(token))
            }) else {
                return false;
            };
            traces.traces.remove(index)
        };
        let home = crate::vars::TraceHome {
            binding_id: removed.binding_id,
            selected_member: removed
                .elem
                .as_ref()
                .zip(removed.element_binding_id)
                .map(|(element, identity)| (element.clone(), identity)),
            ns: removed.ns,
            level: removed.frame_level,
            base: removed.base,
            link_elem: removed.elem.clone(),
        };
        let cell = self.variable_trace_scope(&home, removed.elem.as_deref());
        if !self
            .traces
            .borrow()
            .traces
            .iter()
            .any(|trace| cell.owns_registration(trace))
        {
            self.cleanup_trace_shell(&home, removed.elem.as_deref());
        }
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
        true
    }
    pub(super) fn call_native_variable_observer(
        &mut self,
        observer: Observer,
        name1: &[u8],
        name2: &[u8],
        op: &str,
    ) -> Code {
        let operation =
            NativeVariableTraceOperation::from_name(op).expect("selected trace operation");
        if self.check_entered_native_operation().is_err() {
            return Code::Error;
        }
        let result = observer.observe(
            self,
            NativeVariableTraceAccess {
                operation,
                name1,
                name2,
                destroyed: operation == NativeVariableTraceOperation::Unset,
            },
        );
        if self.check_entered_native_operation().is_err() {
            return Code::Error;
        }
        match result {
            Ok(()) => Code::Ok,
            Err(error) => self.report_cmd_error(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    struct Record(Rc<RefCell<Vec<NativeVariableTraceOperation>>>);
    impl NativeVariableObserver<Interp> for Record {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            _: &mut Interp,
            access: NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            assert_eq!(access.name1, b"x");
            self.0.borrow_mut().push(access.operation);
            Ok(())
        }
    }
    #[test]
    fn direct_native_registration_is_hidden_and_retires_with_the_actual_cell() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let name = obj::Owned::fresh(new_string(b"x"));
            let events = Rc::new(RefCell::new(Vec::new()));
            let token = interp
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[
                        NativeVariableTraceOperation::Write,
                        NativeVariableTraceOperation::Unset,
                    ],
                    Rc::new(Record(events.clone())),
                )
                .unwrap();
            // Registration owns its counted name independently of the caller object.
            drop(name);
            assert_eq!(interp.eval_str(b"trace info variable x"), Code::Ok);
            assert!(interp.result_bytes().is_empty());
            let value = obj::Owned::fresh(new_string(b"A"));
            interp.var_set(b"x", value.as_ptr()).unwrap();
            assert_eq!(*events.borrow(), [NativeVariableTraceOperation::Write]);
            assert!(interp.var_unset(b"x"));
            assert_eq!(
                *events.borrow(),
                [
                    NativeVariableTraceOperation::Write,
                    NativeVariableTraceOperation::Unset
                ]
            );
            assert!(!interp.remove_native_variable_observer(&token));
            // The variable releases its reference; the caller still owns this value.
            assert_eq!(obj_bytes(value.as_ptr()), b"A");
        }
    }
}
