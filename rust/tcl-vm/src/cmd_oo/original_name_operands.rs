// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual OO object/facet selection through checked Name and publication stages.
use super::{Completion, OoId, Rc, RefCell, Value, Vm};
use crate::interp::native_name_consumers::OriginalNameConsumerScope;
use std::rc::Weak;
use tcl_cmd_core::CmdError;
use tcl_runtime_api::NsId;

#[derive(Clone)]
struct OriginalOoTarget {
    id: OoId,
    object: Weak<RefCell<Option<Value>>>,
    creation: u64,
    namespace: Option<NsId>,
    destroyed: bool,
    class: Option<Weak<()>>,
}
impl OriginalOoTarget {
    fn capture(vm: &Vm, class: bool, id: OoId) -> Option<Self> {
        let object = vm.oo.objects.get(&id)?;
        let facet = if class {
            Some(Rc::downgrade(&vm.oo.classes.get(&id)?.birth))
        } else {
            None
        };
        Some(Self {
            id,
            object: Rc::downgrade(&object.cached_name),
            creation: object.creation_id,
            namespace: object.namespace_token,
            destroyed: object.destroyed,
            class: facet,
        })
    }
    fn is_current(&self, vm: &Vm) -> bool {
        let Some(original) = self.object.upgrade() else {
            return false;
        };
        let Some(object) = vm.oo.objects.get(&self.id) else {
            return false;
        };
        Rc::ptr_eq(&original, &object.cached_name)
            && object.creation_id == self.creation
            && object.namespace_token == self.namespace
            && object.destroyed == self.destroyed
            && self
                .namespace
                .is_none_or(|namespace| vm.native_namespace_token_has_owner(namespace))
            && self.class.as_ref().is_none_or(|birth| {
                birth.upgrade().is_some_and(|birth| {
                    vm.oo
                        .classes
                        .get(&self.id)
                        .is_some_and(|facet| Rc::ptr_eq(&birth, &facet.birth))
                })
            })
    }
}

pub(super) struct OriginalOoNameScope {
    context: OriginalNameConsumerScope,
    target: Option<OriginalOoTarget>,
    declaring: Option<OriginalOoTarget>,
}
impl OriginalOoNameScope {
    fn enter(vm: &mut Vm, target: Option<(bool, OoId)>) -> Result<Self, CmdError> {
        let context = OriginalNameConsumerScope::enter(vm)?;
        let mut scope = Self {
            context,
            target: None,
            declaring: None,
        };
        if let Some((class, id)) = target {
            scope.select_target(vm, class, id)?;
        }
        Ok(scope)
    }
    pub(super) fn select_target(
        &mut self,
        vm: &mut Vm,
        class: bool,
        id: OoId,
    ) -> Result<(), CmdError> {
        self.context.check(vm)?;
        let selected = OriginalOoTarget::capture(vm, class, id);
        self.context
            .require(vm, selected.is_some(), "original OO Name target/facet")?;
        self.target = selected;
        self.check_target(vm)
    }
    /// The existing entered chain selects its private-variable declaring owner.
    pub(super) fn select_declaring_provider(&mut self, vm: &mut Vm) -> Result<(), CmdError> {
        self.context.check(vm)?;
        let provider = match super::native_context::current_method(vm) {
            None => None,
            Some(frame) => {
                let selected = frame
                    .chain
                    .get(frame.index)
                    .map(|step| (!step.is_object, step.provider));
                self.context
                    .require(vm, selected.is_some(), "original OO Name declaring step")?;
                selected
            }
        };
        if let Some((class, id)) = provider {
            let selected = OriginalOoTarget::capture(vm, class, id);
            self.context
                .require(vm, selected.is_some(), "original OO Name declaring facet")?;
            self.declaring = selected;
        }
        self.check_target(vm)
    }
    /// The target joins the original operation independently of frame transitions.
    pub(super) fn check_target(&self, vm: &mut Vm) -> Result<(), CmdError> {
        let current = self
            .target
            .as_ref()
            .is_none_or(|target| target.is_current(vm));
        self.context
            .require(vm, current, "original OO Name target/facet")?;
        let declaring_current = self
            .declaring
            .as_ref()
            .is_none_or(|owner| owner.is_current(vm));
        self.context
            .require(vm, declaring_current, "original OO Name declaring facet")
    }
    /// A fixed method-table observation joins the existing target and currency.
    pub(super) fn require_method_record(&self, vm: &mut Vm, current: bool) -> Result<(), CmdError> {
        self.check_target(vm)?;
        self.context
            .require(vm, current, "original OO rename source record")
    }
    pub(super) fn read(&self, vm: &mut Vm, original: &Value) -> Result<Rc<[u8]>, CmdError> {
        self.check_target(vm)?;
        let reached = self.context.read(vm, original);
        self.check_target(vm)?;
        reached
    }
    /// Retain the original target while entering the worker's genuine definition frame.
    pub(super) fn in_current_frame(
        &self,
        vm: &mut Vm,
        work: impl FnOnce(&mut Vm, &mut Self) -> Completion<Value>,
    ) -> Completion<Value> {
        if let Err(error) = self.check_target(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let context = match OriginalNameConsumerScope::enter(vm) {
            Ok(context) => context,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let mut entered = Self {
            context,
            target: self.target.clone(),
            declaring: self.declaring.clone(),
        };
        let reached = work(vm, &mut entered);
        match entered.context.finish(vm) {
            Ok(()) => reached,
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        }
    }
}

pub(super) fn run_result<T>(
    vm: &mut Vm,
    target: Option<(bool, OoId)>,
    work: impl FnOnce(&mut Vm, &mut OriginalOoNameScope) -> Result<T, Completion<Value>>,
) -> Result<T, Completion<Value>> {
    let mut scope = OriginalOoNameScope::enter(vm, target)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let reached = work(vm, &mut scope);
    scope
        .context
        .finish(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    reached
}
pub(super) fn run(
    vm: &mut Vm,
    target: Option<(bool, OoId)>,
    work: impl FnOnce(&mut Vm, &mut OriginalOoNameScope) -> Completion<Value>,
) -> Completion<Value> {
    run_result(vm, target, |vm, scope| Ok(work(vm, scope))).unwrap_or_else(|completion| completion)
}

#[cfg(test)]
mod tests;
