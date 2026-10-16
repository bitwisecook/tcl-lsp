// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original consumer context, independent of selected command and table roles.

use super::{
    NamespaceExecutionContext, Vm,
    native_name_queries::{QueryNamespace, SelectedNamespaceRetirement},
    native_name_world::NativeNameWorld,
    native_operation_currency::{NativeOperationCurrency, NativeOperationScope},
};
use crate::{
    Value, error::TclHostFailure, frame::ActivationIdentity, value::NativeObjectLifetimeLease,
};
use std::cell::RefCell;
use std::rc::Rc;
use tcl_cmd_core::CmdError;
use tcl_core_types::NsId;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

/// Keep the entered currency and the existing caller context through cleanup.
/// A Jim header lease protects allocation only; it issues no table identity.
pub(crate) struct OriginalNameConsumerScope {
    state: OriginalNameConsumerState,
    operation: NativeOperationScope,
}

pub(super) struct OriginalNameConsumerState {
    jim_namespace: Option<NativeObjectLifetimeLease>,
    frame: Option<(Rc<ActivationIdentity>, usize)>,
    namespace: NamespaceExecutionContext,
    namespace_owner: RefCell<Option<QueryNamespace>>,
    selected_retirement: RefCell<Option<SelectedNamespaceRetirement>>,
    selected_namespace: Option<QueryNamespace>,
    world: Rc<RefCell<NativeNameWorld>>,
}

/// Saved context is descriptive until its same original currency is resumed.
pub(crate) struct ParkedOriginalNameConsumerScope {
    state: OriginalNameConsumerState,
    currency: NativeOperationCurrency,
}

impl OriginalNameConsumerState {
    pub(super) fn capture(
        vm: &mut Vm,
        selected: Option<NsId>,
        retain_jim_namespace: bool,
    ) -> Result<Self, CmdError> {
        let frame = vm
            .frames
            .len()
            .checked_sub(1)
            .map(|level| (vm.retained_frame_identity(false), level));
        let namespace = vm.current_namespace_execution_context();
        if let NamespaceExecutionContext::OriginalJim(context) = &namespace {
            context.check_owner(vm)?;
        }
        let namespace_owner = namespace
            .resolved_token()
            .map(|id| QueryNamespace::capture(vm, id))
            .transpose()?;
        let selected_namespace = selected
            .map(|id| QueryNamespace::capture(vm, id))
            .transpose()?;
        let world = Rc::clone(&vm.name_world);
        let jim_namespace = if retain_jim_namespace
            && vm.uses_native_jim_lookup()
            && frame.is_some()
        {
            Some(vm.with_jim_current_namespace(|original| Ok(original.native_lifetime_lease()))?)
        } else {
            None
        };
        Ok(Self {
            jim_namespace,
            frame,
            namespace,
            namespace_owner: RefCell::new(namespace_owner),
            selected_retirement: RefCell::new(None),
            selected_namespace,
            world,
        })
    }
    pub(super) fn retire_header_lease(&mut self) {
        drop(self.jim_namespace.take());
    }
    pub(super) fn check_using(
        &self,
        vm: &mut Vm,
        require_absent_frame: bool,
        require: impl Fn(&mut Vm, bool, &'static str) -> Result<(), CmdError>,
    ) -> Result<(), CmdError> {
        let world_current = Rc::ptr_eq(&self.world, &vm.name_world)
            && self.world.borrow().is_live()
            && self.namespace_owner.borrow().as_ref().is_none_or(|owner| {
                owner.current(vm)
                    || self
                        .selected_retirement
                        .borrow()
                        .as_ref()
                        .is_some_and(|receipt| receipt.current(vm, owner))
            })
            && self
                .selected_namespace
                .as_ref()
                .is_none_or(|selected| selected.current(vm));
        require(
            vm,
            world_current,
            "original Name consumer namespace and world",
        )?;
        let frame_current = self.frame.as_ref().map_or_else(
            || !require_absent_frame || vm.frames.is_empty(),
            |(identity, level)| {
                vm.retained_frame_level(identity) == Some(*level)
                    && vm.frames.len().checked_sub(1) == Some(*level)
            },
        );
        require(vm, frame_current, "original Name consumer frame activation")?;
        if let NamespaceExecutionContext::OriginalJim(context) = &self.namespace {
            context.check_owner(vm)?;
        }
        let namespace_current = vm
            .current_namespace_execution_context()
            .same_context(&self.namespace)
            && self
                .namespace
                .resolved_token()
                .is_none_or(|id| vm.native_namespace_token_has_owner(id));
        require(
            vm,
            namespace_current,
            "original Name consumer namespace incarnation",
        )?;
        let namespace_owner_missing = self.namespace_owner.borrow().is_none();
        if let Some(namespace) = self.namespace.resolved_token()
            && namespace_owner_missing
        {
            let owner = QueryNamespace::capture(vm, namespace)?;
            *self.namespace_owner.borrow_mut() = Some(owner);
        }
        if let Some(original) = &self.jim_namespace {
            let same = vm.with_jim_current_namespace(|current| {
                Ok(current.is_same_object(original.value()))
            })?;
            require(vm, same, "original Jim consumer namespace header")?;
        }
        Ok(())
    }
}

impl OriginalNameConsumerScope {
    pub(crate) fn enter(vm: &mut Vm) -> Result<Self, CmdError> {
        Self::enter_for_namespace(vm, None)
    }

    pub(crate) fn enter_for_namespace(
        vm: &mut Vm,
        selected: Option<NsId>,
    ) -> Result<Self, CmdError> {
        let operation = NativeOperationScope::enter(vm)?;
        Self::capture_with_operation(vm, operation, selected)
    }

    /// Observe the same caller state under an already entered original operation.
    /// This door neither creates an operation nor promotes descriptive currency.
    pub(crate) fn in_original_scope(
        vm: &mut Vm,
        operation: &NativeOperationScope,
    ) -> Result<Self, CmdError> {
        operation.check(vm)?;
        Self::capture_with_operation(vm, operation.clone(), None)
    }

    fn capture_with_operation(
        vm: &mut Vm,
        operation: NativeOperationScope,
        selected: Option<NsId>,
    ) -> Result<Self, CmdError> {
        let scope = Self {
            state: OriginalNameConsumerState::capture(vm, selected, true)?,
            operation,
        };
        scope.check(vm)?;
        Ok(scope)
    }

    /// Retire exactly the namespace selected by the C deletion worker. Only
    /// its live entered birth may become retained under the selected subtree;
    /// unrelated callbacks and every frame/world/header check remain strict.
    pub(crate) fn retire_selected_namespace(
        &self,
        vm: &mut Vm,
        selected: NsId,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check(vm).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "selected namespace retirement original scope",
            )
        })?;
        let pending = self
            .state
            .namespace_owner
            .borrow()
            .as_ref()
            .and_then(|owner| owner.prepare_selected_retirement(vm, selected));
        tcl_cmd_core::namespace::NamespaceDeleteBackend::delete_selected_namespace(vm, selected)?;
        if let Some(pending) = pending {
            let receipt = self
                .state
                .namespace_owner
                .borrow()
                .as_ref()
                .and_then(|owner| pending.complete(vm, owner, selected));
            if let Some(receipt) = receipt {
                *self.state.selected_retirement.borrow_mut() = Some(receipt);
            }
        }
        self.check(vm).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "selected namespace retirement original scope",
            )
        })
    }

    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.operation.check(vm)?;
        self.state.check_using(vm, false, |vm, current, purpose| {
            self.require(vm, current, purpose)
        })
    }

    /// Project the actual entered current namespace, including its captured
    /// world and incarnation, for a no-argument namespace operation.
    pub(crate) fn current_namespace_token(&self, vm: &mut Vm) -> Result<NsId, CmdError> {
        self.check(vm)?;
        let token = self.state.namespace.resolved_token();
        self.require(
            vm,
            token.is_some_and(|id| vm.native_namespace_token_has_owner(id)),
            "selected original current namespace holder",
        )?;
        Ok(token.expect("checked original current namespace holder"))
    }

    pub(crate) fn require(
        &self,
        vm: &mut Vm,
        current: bool,
        purpose: &'static str,
    ) -> Result<(), CmdError> {
        self.operation.check(vm)?;
        if current {
            return Ok(());
        }
        let cause = NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable(purpose),
        );
        vm.retain_tcl_host_failure(TclHostFailure::Execution(cause));
        self.operation.check(vm)
    }

    pub(crate) fn capture_binding(
        &self,
        vm: &mut Vm,
        binding: &super::VarBinding,
    ) -> Result<OriginalNameBinding, CmdError> {
        self.check(vm)?;
        let raw = vm.raw_variable_binding(binding);
        let cell = raw.and_then(|id| vm.var_arena.weak_jim_cell(id));
        self.require(vm, cell.is_some(), "original Name selected binding cell")?;
        Ok(OriginalNameBinding {
            binding: binding.clone(),
            raw: raw.expect("checked original binding"),
            cell: cell.expect("checked original cell"),
        })
    }

    pub(crate) fn capture_cell(
        &self,
        vm: &mut Vm,
        id: super::VarId,
    ) -> Result<crate::vars::WeakJimVariableCell, CmdError> {
        self.check(vm)?;
        let cell = vm.var_arena.weak_jim_cell(id);
        self.require(vm, cell.is_some(), "original Name selected cell birth")?;
        Ok(cell.expect("checked original cell"))
    }

    pub(crate) fn check_cell(
        &self,
        vm: &mut Vm,
        id: super::VarId,
        cell: &crate::vars::WeakJimVariableCell,
    ) -> Result<(), CmdError> {
        self.check(vm)?;
        let current = vm.var_arena.validate_jim_cell(cell) == Some(id);
        self.require(vm, current, "original Name selected cell birth")
    }

    pub(crate) fn capture_frame(
        &self,
        vm: &mut Vm,
        level: usize,
    ) -> Result<OriginalNameFrame, CmdError> {
        self.check(vm)?;
        Ok(OriginalNameFrame {
            level,
            identity: vm
                .frames
                .get(level)
                .map(|frame| Rc::clone(&frame.activation)),
        })
    }

    pub(crate) fn operation_scope(&self) -> &NativeOperationScope {
        &self.operation
    }

    pub(super) fn currency(&self) -> &super::native_operation_currency::NativeOperationCurrency {
        self.operation.currency()
    }

    pub(crate) fn park(self) -> ParkedOriginalNameConsumerScope {
        let Self { state, operation } = self;
        let currency = operation.currency().clone();
        drop(operation);
        ParkedOriginalNameConsumerScope { state, currency }
    }

    pub(crate) fn current_for_value(&self, vm: &mut Vm, original: &Value) -> Result<(), CmdError> {
        self.check(vm)?;
        super::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &self.operation)?
            .current_for_value(original)
    }

    pub(crate) fn list(
        &self,
        vm: &mut Vm,
        original: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<crate::NativeListItems, CmdError> {
        self.check(vm)?;
        super::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &self.operation)?
            .list_with_original_caller_check(original, protocol, |vm| self.check(vm))
    }

    pub(crate) fn dictionary_search(
        &self,
        vm: &mut Vm,
        original: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<crate::value::NativeDictionarySearch, CmdError> {
        self.check(vm)?;
        let selected = vm
            .actual_native_invocation_dialect()
            .native_string_protocol();
        self.require(
            vm,
            selected == Some(protocol),
            "original Name Dictionary protocol",
        )?;
        let vm = RefCell::new(vm);
        let reached = original
            .native_lifetime_lease()
            .into_value()
            .into_native_dictionary_search_with_strings(
                protocol,
                &mut || self.current_for_value(&mut vm.borrow_mut(), original),
                &mut |value| self.read(&mut vm.borrow_mut(), value),
            );
        if let Err(error) = &reached
            && let Some(cause) = error.native_execution_refusal()
        {
            vm.borrow_mut()
                .retain_tcl_host_failure(TclHostFailure::Execution(cause.clone()));
        }
        self.check(&mut vm.borrow_mut())?;
        reached
    }

    pub(crate) fn read(&self, vm: &mut Vm, original: &Value) -> Result<Rc<[u8]>, CmdError> {
        self.check(vm)?;
        let lifetime = original.native_lifetime_lease();
        let reached: Result<Rc<[u8]>, CmdError> = (|| {
            let ops = super::native_numeric_ops::OriginalNumericOps::in_original_scope(
                vm,
                &self.operation,
            )?;
            ops.current_for_value(lifetime.value())?;
            let reached = if let Some(bytes) = lifetime.value().resident_string_bytes() {
                Ok(bytes)
            } else {
                let protocol = ops.dialect().native_string_protocol().ok_or(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(
                        tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
                    ),
                )?;
                ops.string_with_original_check(lifetime.value(), protocol, |vm| self.check(vm))
            };
            if let Err(error) = &reached {
                ops.retain_failure(error);
            }
            ops.current_for_value(lifetime.value())?;
            reached
        })();
        if let Err(error) = &reached {
            if let Some(cause) = error.native_execution_refusal() {
                vm.retain_tcl_host_failure(TclHostFailure::Execution(cause.clone()));
            }
        }
        drop(lifetime);
        self.check(vm)?;
        reached
    }

    /// Retire the allocation lease while its original operation is entered.
    pub(crate) fn finish(mut self, vm: &mut Vm) -> Result<(), CmdError> {
        let before = self.check(vm);
        self.state.retire_header_lease();
        self.check(vm)?;
        before
    }
}

impl ParkedOriginalNameConsumerScope {
    /// Observe saved currency and caller state without entering or resuming work.
    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.currency.check(vm)?;
        self.state.check_using(vm, false, |vm, current, purpose| {
            self.currency.check(vm)?;
            if !current {
                let cause = NativeExecutionError::ValueAccessRefusal(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(purpose),
                );
                vm.retain_tcl_host_failure(TclHostFailure::Execution(cause));
            }
            self.currency.check(vm)
        })
    }

    pub(crate) fn current_for_value(&self, vm: &mut Vm, value: &Value) -> Result<(), CmdError> {
        self.check(vm)?;
        if let Err(refusal) = value.original_native_header_currentness() {
            return Err(CmdError::from_execution_refusal(
                vm.retain_tcl_host_failure(TclHostFailure::ValueAccess(refusal)),
            ));
        }
        self.check(vm)
    }
    pub(crate) fn currency(&self) -> &NativeOperationCurrency {
        &self.currency
    }

    pub(crate) fn resume(self, vm: &mut Vm) -> Result<OriginalNameConsumerScope, CmdError> {
        let Self { state, currency } = self;
        let operation = match NativeOperationScope::resume_saved(vm, &currency) {
            Ok(operation) => operation,
            Err(first) => {
                if let Ok(cleanup) = NativeOperationScope::cleanup_saved(vm, &currency) {
                    drop(state);
                    let _ = cleanup.check(vm);
                } else {
                    drop(state);
                }
                return Err(CmdError::from_execution_refusal(first));
            }
        };
        let original = OriginalNameConsumerScope { state, operation };
        if let Err(first) = original.check(vm) {
            let _ = original.finish(vm);
            return Err(first);
        }
        Ok(original)
    }
}

/// A weak observation of the actual selected slot, never a variable role grant.
pub(crate) struct OriginalNameBinding {
    binding: super::VarBinding,
    raw: super::VarId,
    cell: crate::vars::WeakJimVariableCell,
}
impl OriginalNameBinding {
    pub(crate) fn check(
        &self,
        vm: &mut Vm,
        scope: &OriginalNameConsumerScope,
    ) -> Result<(), CmdError> {
        scope.check_cell(vm, self.raw, &self.cell)?;
        let current = vm.raw_variable_binding(&self.binding) == Some(self.raw);
        scope.require(vm, current, "original Name selected binding slot")
    }
}

/// The already selected target frame; a missing frame stays missing.
pub(crate) struct OriginalNameFrame {
    level: usize,
    identity: Option<Rc<ActivationIdentity>>,
}
impl OriginalNameFrame {
    pub(crate) fn check(
        &self,
        vm: &mut Vm,
        scope: &OriginalNameConsumerScope,
    ) -> Result<(), CmdError> {
        scope.check(vm)?;
        let current = match (&self.identity, vm.frames.get(self.level)) {
            (Some(saved), Some(actual)) => Rc::ptr_eq(saved, &actual.activation),
            (None, None) => true,
            _ => false,
        };
        scope.require(vm, current, "original Name selected target activation")
    }
}

#[cfg(test)]
mod tests;
