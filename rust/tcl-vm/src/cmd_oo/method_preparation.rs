// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original method preparation owners, independent of mutable table membership.
use super::{
    OoId, Step, Vm,
    native_context::OriginalMethodFrame,
    native_method_cache::{MethodOwner, ReachedMethodChain},
    original_name_operands::OriginalOoNameScope,
};
use crate::error::TclError;
use std::rc::{Rc, Weak};
use tcl_cmd_core::CmdError;

/// Weak observations add no Method, payload or native reference role.
pub(super) struct OriginalMethodPreparation {
    names: OriginalOoNameScope,
    steps: Weak<Vec<Step>>,
    owner: Weak<std::cell::RefCell<super::Method>>,
    index: usize,
    provider: OoId,
    is_object: bool,
    caller: Option<OriginalMethodFrame>,
}
impl OriginalMethodPreparation {
    pub(super) fn enter(
        vm: &mut Vm,
        object: OoId,
        chain: &ReachedMethodChain,
        index: usize,
    ) -> Result<Self, CmdError> {
        let mut names = OriginalOoNameScope::enter(vm, Some((false, object)))?;
        let selected = chain.get(index);
        names.require_current(vm, selected.is_some(), "original OO preparation chain step")?;
        let step = selected.expect("checked reached step");
        names.select_entered_provider(vm, !step.is_object, step.provider)?;
        let owner: Option<&MethodOwner> = step.method_owner.as_ref();
        names.require_current(vm, owner.is_some(), "original OO preparation method record")?;
        let result = Self {
            names,
            steps: chain.original_steps(),
            owner: Rc::downgrade(owner.expect("checked selected method record")),
            index,
            provider: step.provider,
            is_object: step.is_object,
            caller: OriginalMethodFrame::capture(vm),
        };
        result.check(vm)?;
        Ok(result)
    }
    pub(super) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.names.check(vm)?;
        let selected = self.steps.upgrade().is_some_and(|steps| {
            steps.get(self.index).is_some_and(|step| {
                step.provider == self.provider
                    && step.is_object == self.is_object
                    && step
                        .method_owner
                        .as_ref()
                        .is_some_and(|owner| Weak::ptr_eq(&self.owner, &Rc::downgrade(owner)))
            })
        });
        self.names
            .require_current(vm, selected, "original OO preparation method record")?;
        let caller = self.caller.as_ref().map_or_else(
            || super::native_context::current_method(vm).is_none(),
            |caller| caller.is_current(vm),
        );
        self.names
            .require_current(vm, caller, "original OO preparation caller frame")
    }
    /// Preserve a reached Host cause before any refused activation is retired.
    pub(super) fn settle<T>(
        &self,
        vm: &mut Vm,
        reached: Result<T, TclError>,
    ) -> Result<T, TclError> {
        if let Err(TclError::Host(first)) = &reached {
            vm.retain_tcl_host_failure(first.clone());
        }
        if let Err(first) = self.check(vm) {
            drop(reached);
            let first = self.check(vm).err().unwrap_or(first);
            return Err(crate::command::completion_option_failure(vm, first));
        }
        reached
    }
    /// Finish before the intentional new method frame changes caller context.
    pub(super) fn finish(self, vm: &mut Vm) -> Result<(), CmdError> {
        let reached = self.check(vm);
        self.names.finish(vm)?;
        reached
    }
}

#[cfg(test)]
mod tests;
