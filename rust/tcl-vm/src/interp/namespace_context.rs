// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original execution namespaces remain distinct from materialized coordinates.

use super::{
    Vm, native_name_world::NativeNameWorld, native_numeric_ops::OriginalNumericOps,
    native_operation_currency::NativeOperationScope,
};
use crate::{Value, error::TclHostFailure, value::NativeObjectLifetimeLease};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use tcl_cmd_core::CmdError;
use tcl_core_types::{NameBytes, NsId};
use tcl_runtime_api::{NativeExecutionError, native_compilation::NativeInterpreterIdentity};
use tcl_syntax::{
    naming::{NamePolicyAuthority, NamePolicyProtocol},
    raw_string::NativeValueAccessRefusal,
};

#[derive(Clone)]
pub(crate) enum NamespaceExecutionContext {
    Resolved(NsId),
    OriginalJim(Rc<OriginalJimNamespaceContext>),
}

impl From<NsId> for NamespaceExecutionContext {
    fn from(token: NsId) -> Self {
        Self::Resolved(token)
    }
}

impl PartialEq for NamespaceExecutionContext {
    fn eq(&self, other: &Self) -> bool {
        self.same_context(other)
    }
}
impl Eq for NamespaceExecutionContext {}

impl std::fmt::Debug for NamespaceExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolved(token) => f.debug_tuple("Resolved").field(token).finish(),
            Self::OriginalJim(_) => f.write_str("OriginalJim"),
        }
    }
}

impl NamespaceExecutionContext {
    pub(crate) fn same_context(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Resolved(left), Self::Resolved(right)) => left == right,
            (Self::OriginalJim(left), Self::OriginalJim(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }

    pub(crate) fn check_owner(&self, vm: &mut Vm) -> Result<(), CmdError> {
        match self {
            Self::Resolved(token) => {
                let scope = NativeOperationScope::enter(vm)?;
                if !vm.native_namespace_token_has_owner(*token) {
                    return Err(refuse(vm, "original namespace token owner"));
                }
                scope.check(vm)
            }
            Self::OriginalJim(original) => original.check_owner(vm),
        }
    }

    /// Observe an existing coordinate without obtaining a namespace getter.
    pub(crate) fn resolved_token(&self) -> Option<NsId> {
        match self {
            Self::Resolved(token) => Some(*token),
            Self::OriginalJim(original) => original.resolved_token(),
        }
    }
}

pub(crate) struct OriginalJimNamespaceContext {
    original: NativeObjectLifetimeLease,
    interpreter: NativeInterpreterIdentity,
    world: Weak<RefCell<NativeNameWorld>>,
    policy: NamePolicyProtocol,
    materialized: RefCell<Option<(NameBytes, NsId)>>,
}

impl OriginalJimNamespaceContext {
    pub(crate) fn for_lambda(vm: &mut Vm, original: &Value) -> Result<Rc<Self>, CmdError> {
        let scope = NativeOperationScope::enter(vm)?;
        let policy = vm.name_policy_protocol().filter(|policy| {
            policy.authority() == NamePolicyAuthority::Native && policy.recipe().is_jim084()
        });
        let Some(policy) = policy else {
            return Err(refuse(vm, "original Jim namespace issuer"));
        };
        OriginalNumericOps::in_original_scope(vm, &scope)?.current_for_value(original)?;
        let context = Rc::new(Self {
            original: original.native_lifetime_lease(),
            interpreter: vm.native_interpreter_identity(),
            world: Rc::downgrade(&vm.name_world),
            policy,
            materialized: RefCell::new(None),
        });
        scope.check(vm)?;
        context.check_owner(vm)?;
        Ok(context)
    }

    pub(crate) fn original(&self) -> &Value {
        self.original.value()
    }

    pub(crate) fn owner_is_current(&self, vm: &Vm) -> bool {
        self.interpreter == vm.native_interpreter_identity()
            && vm.name_policy_protocol() == Some(self.policy)
            && self.original().check_native_header().is_ok()
            && self
                .world
                .upgrade()
                .is_some_and(|world| Rc::ptr_eq(&world, &vm.name_world) && world.borrow().is_live())
    }

    pub(crate) fn check_owner(&self, vm: &mut Vm) -> Result<(), CmdError> {
        let scope = NativeOperationScope::enter(vm)?;
        if !self.owner_is_current(vm) {
            return Err(refuse(vm, "original Jim namespace owner"));
        }
        OriginalNumericOps::in_original_scope(vm, &scope)?.current_for_value(self.original())?;
        scope.check(vm)
    }

    fn resolved_token(&self) -> Option<NsId> {
        let bytes = self.original().resident_string_bytes()?;
        self.materialized
            .borrow()
            .as_ref()
            .filter(|(selected, _)| selected.as_bytes() == bytes.as_ref())
            .map(|(_, token)| *token)
    }

    /// A reached original namespace operation owns this getter independently of capture.
    pub(crate) fn materialize(&self, vm: &mut Vm) -> Result<NsId, CmdError> {
        self.check_owner(vm)?;
        let bytes = vm.native_name_operand_bytes_checked(self.original())?;
        self.check_owner(vm)?;
        let selected = self
            .policy
            .recipe()
            .jim_lambda_namespace_input(&bytes)
            .map_err(|_| refuse(vm, "original Jim namespace context input"))?;
        let token = vm.intern_jim_namespace_object(selected.selected());
        self.check_owner(vm)?;
        *self.materialized.borrow_mut() = Some((NameBytes::from(bytes.as_ref()), token));
        vm.note_original_jim_namespace_materialized(self, token);
        Ok(token)
    }
}

fn refuse(vm: &mut Vm, purpose: &'static str) -> CmdError {
    let cause = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable(purpose),
    );
    vm.retain_tcl_host_failure(TclHostFailure::Execution(cause.clone()));
    CmdError::from_execution_refusal(cause)
}

#[derive(Clone)]
pub(crate) enum ProcedureNamespace {
    Resolved {
        path: tcl_core_types::ByteNamespacePath,
        token: NsId,
    },
    OriginalJim(Rc<OriginalJimNamespaceContext>),
}

impl ProcedureNamespace {
    pub(crate) fn context(&self) -> NamespaceExecutionContext {
        match self {
            Self::Resolved { token, .. } => NamespaceExecutionContext::Resolved(*token),
            Self::OriginalJim(original) => {
                NamespaceExecutionContext::OriginalJim(Rc::clone(original))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::{ParkedFlow, ok, oo_cleanup::OriginalOoActivation};
    use tcl_core_types::ROOT_NS;
    use tcl_runtime_api::Code;

    fn interpreter() -> Vm {
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jimtcl"))
    }

    fn enter_original(vm: &mut Vm, original: &Value) -> NamespaceExecutionContext {
        let namespace = OriginalJimNamespaceContext::for_lambda(vm, original).unwrap();
        let context = NamespaceExecutionContext::OriginalJim(Rc::clone(&namespace));
        vm.push_call_frame(Some("original_context_control".into()), Vec::new());
        vm.install_jim_procedure_namespace_owner(None, original.clone());
        vm.push_procedure_namespace_context(ProcedureNamespace::OriginalJim(namespace));
        context
    }

    #[test]
    fn original_jim_context_preserves_stringless_namespace_through_selection_and_flow_transport() {
        // Software activation ownership only; this does not claim native Jim coroutine support.
        let mut vm = interpreter();
        let counts = vm.name_world.borrow().ns_deferral.counts.clone();
        let original = Value::int(42);
        let context = enter_original(&mut vm, &original);
        let activation = vm.resolution_stacks.observe_activation().unwrap();
        assert!(context.resolved_token().is_none());
        assert!(vm.current_native_compilation_namespace().is_err());
        assert!(original.resident_string_bytes().is_none());
        let selected = vm.select_execution_frame_checked(0).unwrap();
        assert!(!activation.original_present(&vm.resolution_stacks));
        assert_eq!(
            vm.current_namespace_execution_context().resolved_token(),
            Some(ROOT_NS)
        );
        vm.restore_execution_frame(selected);
        assert!(activation.original_present(&vm.resolution_stacks));
        assert!(
            vm.current_namespace_execution_context()
                .same_context(&context)
        );
        let mut parked = ParkedFlow::default();
        vm.swap_flow(&mut parked);
        assert!(!activation.original_present(&vm.resolution_stacks));
        assert!(
            parked
                .current_namespace_execution_context()
                .same_context(&context)
        );
        assert!(original.resident_string_bytes().is_none());
        vm.swap_flow(&mut parked);
        assert!(activation.original_present(&vm.resolution_stacks));
        assert!(
            vm.current_namespace_execution_context()
                .same_context(&context)
        );
        vm.pop_call_frame();
        assert!(activation.original_present(&vm.resolution_stacks));
        vm.pop_ns();
        assert!(activation.caller_restored(&vm.resolution_stacks));
        assert_eq!(vm.frames.len(), 1);
        assert_eq!(vm.name_world.borrow().ns_deferral.counts, counts);
        assert!(original.resident_string_bytes().is_none());
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn original_jim_context_cleanup_preserves_first_host_and_retires_only_owned_activation() {
        // Actual cleanup and original Value owners; no native callable receipt is inferred.
        let mut vm = interpreter();
        let counts = vm.name_world.borrow().ns_deferral.counts.clone();
        let original = Value::int(42);
        let mut active = OriginalOoActivation::enter(&mut vm);
        let context = enter_original(&mut vm, &original);
        active.capture_call_and_namespace(&mut vm).unwrap();
        let header = Rc::downgrade(&vm.frames.last().unwrap().activation);
        let _ = vm.refuse_host_command("FIRST_ORIGINAL_JIM_CONTEXT_CLEANUP".into());
        let retained = vm.execution_refusal.clone().unwrap();
        active.retire_call(&mut vm, false);
        active.retire_namespace(&mut vm, false);
        assert_eq!(vm.frames.len(), 1);
        assert_eq!(vm.resolution_stacks.ns_id_stack.len(), 1);
        assert!(header.upgrade().is_none());
        assert_eq!(vm.name_world.borrow().ns_deferral.counts, counts);
        assert!(context.resolved_token().is_none());
        assert!(original.resident_string_bytes().is_none());
        let result = active.finish(&mut vm, ok(Value::empty()));
        assert_eq!(result.code, Code::Error);
        assert_eq!(vm.execution_refusal, Some(retained));
    }
}
