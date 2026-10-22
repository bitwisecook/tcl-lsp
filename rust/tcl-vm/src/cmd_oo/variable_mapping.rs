// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual selected declaration-map references, old visibility and sequential keys.
use super::original_name_operands::OriginalOoNameScope;
use super::{DeclaredVariable, DeclaredVariableFullName, NameBytes, OoId, OoState, Value, Vm};
use crate::interp::{
    native_numeric_ops::OriginalNumericOps, native_operation_currency::NativeOperationScope,
    oo_cleanup::OriginalOoCleanup,
};
use std::rc::Rc;
use tcl_cmd_core::{
    CmdError,
    native_oo_variable_mapping::{NativeOoVariableMappingOps, install_original_variable_mapping},
};
use tcl_syntax::{
    naming::{NamePolicyAuthority, NamePolicyProtocol, NativeNameProtocol},
    value::ValueError,
};

type Slot = (OoId, bool, bool);

/// This worker selects only a genuine audited C `TclOO` naming provider.
pub(super) fn selected(policy: NamePolicyProtocol) -> bool {
    policy.authority() == NamePolicyAuthority::Native
        && matches!(policy.recipe(), NativeNameProtocol::C(version) if version >= tcl_dialect::TclVersion::V8_6)
}
fn slot(oo: &OoState, key: Slot) -> Option<&Vec<DeclaredVariable>> {
    let (target, class, private) = key;
    if class {
        oo.classes.get(&target).map(|owner| {
            if private {
                &owner.private_variables
            } else {
                &owner.variables
            }
        })
    } else {
        oo.objects.get(&target).map(|owner| {
            if private {
                &owner.private_variables
            } else {
                &owner.variables
            }
        })
    }
}
fn slot_mut(oo: &mut OoState, key: Slot) -> Option<&mut Vec<DeclaredVariable>> {
    let (target, class, private) = key;
    if class {
        oo.classes.get_mut(&target).map(|owner| {
            if private {
                &mut owner.private_variables
            } else {
                &mut owner.variables
            }
        })
    } else {
        oo.objects.get_mut(&target).map(|owner| {
            if private {
                &mut owner.private_variables
            } else {
                &mut owner.variables
            }
        })
    }
}
pub(super) fn observation(original: &DeclaredVariable) -> DeclaredVariable {
    DeclaredVariable {
        original: original.original.native_lifetime_lease().into_value(),
        name: original.name.clone(),
        full_name: original
            .full_name
            .as_ref()
            .map(|value| DeclaredVariableFullName {
                original: value.original.native_lifetime_lease().into_value(),
                name: value.name.clone(),
            }),
    }
}
/// A slot working view adds no incoming reference or private full-name role.
pub(super) fn snapshot(
    vm: &Vm,
    class: bool,
    target: OoId,
    private: bool,
) -> Option<Vec<DeclaredVariable>> {
    slot(&vm.oo, (target, class, private)).map(|entries| entries.iter().map(observation).collect())
}

struct Retirement {
    token: Rc<()>,
    old: Vec<DeclaredVariable>,
}
struct Mapping<'a> {
    vm: &'a mut Vm,
    names: &'a OriginalOoNameScope,
    operation: NativeOperationScope,
    cleanup: OriginalOoCleanup,
    key: Slot,
    policy: NamePolicyProtocol,
    creation: u64,
}
impl Mapping<'_> {
    fn retain(&mut self, error: &CmdError) {
        crate::interp::oo_cleanup::retain_error(self.vm, error);
    }
    fn checked(&mut self) -> Result<(), CmdError> {
        let reached = self.check_mapping();
        if let Err(error) = &reached {
            self.retain(error);
        }
        reached
    }
    fn ticket_matches(oo: &OoState, key: Slot, token: &Rc<()>) -> bool {
        oo.variable_mapping_retirements
            .get(&key)
            .and_then(std::rc::Weak::upgrade)
            .is_some_and(|current| Rc::ptr_eq(&current, token))
    }
    fn clear_matching(&mut self, retired: &Retirement) -> Result<(), CmdError> {
        // Physical mandatory cleanup is distinct from a current target/operation.
        let visible = {
            let oo = self.cleanup.oo_mut(self.vm)?;
            if Self::ticket_matches(oo, self.key, &retired.token) {
                oo.variable_mapping_retirements.remove(&self.key);
                slot_mut(oo, self.key).map(std::mem::take)
            } else {
                None
            }
        };
        drop(visible);
        Ok(())
    }
    fn drain_roles(&mut self, retired: &mut Retirement) -> Result<(), CmdError> {
        let mut first = None;
        for entry in std::mem::take(&mut retired.old) {
            let DeclaredVariable {
                original,
                name,
                full_name,
            } = entry;
            // The inspected private worker releases variableObj before fullNameObj.
            drop(original);
            if let Err(error) = self.checked() {
                if first.is_none() {
                    first = Some(error);
                }
            }
            drop(full_name);
            if let Err(error) = self.checked() {
                if first.is_none() {
                    first = Some(error);
                }
            }
            drop(name);
        }
        match first {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    fn empty_current(&mut self, primary: &Retirement) -> Result<(), CmdError> {
        loop {
            self.checked()?;
            if Self::ticket_matches(&self.vm.oo, self.key, &primary.token) {
                self.clear_matching(primary)?;
                return self.checked();
            }
            if slot(&self.vm.oo, self.key).is_some_and(Vec::is_empty) {
                return Ok(());
            }
            // An ordinary recursive declaration is permitted. Settle its real
            // displaced roles at the same old-visible stage before outer adoption.
            let mut recursive = self.capture_old_mapping()?;
            let reached = self.drain_roles(&mut recursive);
            let still_selected = Self::ticket_matches(&self.vm.oo, self.key, &recursive.token);
            let cleanup = self.clear_matching(&recursive);
            if let Err(error) = &cleanup {
                self.retain(error);
            }
            self.checked()?;
            reached?;
            cleanup?;
            if still_selected {
                return Ok(());
            }
        }
    }
}
impl NativeOoVariableMappingOps for Mapping<'_> {
    type Member = DeclaredVariable;
    type Retirement = Retirement;
    fn check_mapping(&mut self) -> Result<(), CmdError> {
        self.operation.check(self.vm)?;
        self.names.check(self.vm)?;
        let same_provider = self.vm.name_policy_protocol() == Some(self.policy);
        self.names.require_current(
            self.vm,
            same_provider,
            "original OO declaration naming provider",
        )
    }
    fn retain_mapping_failure(&mut self, error: &CmdError) {
        self.retain(error);
    }
    fn acquire_member(
        &mut self,
        original: &DeclaredVariable,
    ) -> Result<DeclaredVariable, CmdError> {
        self.checked()?;
        OriginalNumericOps::in_original_scope(self.vm, &self.operation)?
            .current_for_value(&original.original)?;
        // The source incoming +1 is issued here, after validation and for every
        // supplied entry, including equal keys. No old full-name role is copied.
        Ok(DeclaredVariable {
            original: original.original.clone(),
            name: original.name.clone(),
            full_name: None,
        })
    }
    fn capture_old_mapping(&mut self) -> Result<Retirement, CmdError> {
        self.checked()?;
        let token = Rc::new(());
        let target = slot_mut(&mut self.vm.oo, self.key).ok_or(
            ValueError::CommandProtocolUnavailable("original OO declaration map"),
        )?;
        let visible = target.iter().map(observation).collect();
        let old = std::mem::replace(target, visible);
        self.vm
            .oo
            .variable_mapping_retirements
            .insert(self.key, Rc::downgrade(&token));
        Ok(Retirement { token, old })
    }
    fn retire_old_mapping(&mut self, retired: &mut Retirement) -> Result<(), CmdError> {
        self.drain_roles(retired)
    }
    fn begin_new_mapping(&mut self, retired: &mut Retirement) -> Result<(), CmdError> {
        self.empty_current(retired)
    }
    fn member_key(&mut self, original: &DeclaredVariable) -> Result<Vec<u8>, CmdError> {
        self.checked()?;
        let reached = self.names.read(self.vm, &original.original);
        if let Err(error) = &reached {
            self.retain(error);
        }
        self.checked()?;
        Ok(reached?.to_vec())
    }
    fn prepare_unique_member(
        &mut self,
        mut original: DeclaredVariable,
        key: &[u8],
    ) -> Result<DeclaredVariable, CmdError> {
        let reached = (|| {
            self.checked()?;
            original.name = NameBytes::from(key);
            if self.key.2 {
                // Tcl_ObjPrintf uses the reached original header's CString; the
                // map key itself remains counted. The resulting actual object
                // role belongs to this private record, independently of `name`.
                let bytes = self.names.read(self.vm, &original.original)?;
                self.checked()?;
                let storage = super::private_storage_name_bytes(
                    self.creation,
                    tcl_core_types::c_string_extent(&bytes),
                );
                original.full_name = Some(DeclaredVariableFullName {
                    original: Value::new_native_string_bytes(storage.as_bytes())
                        .into_native_unowned_lifetime()
                        .into_native_reference(),
                    name: storage,
                });
            }
            self.checked()
        })();
        if let Err(error) = &reached {
            self.retain(error);
        }
        reached?;
        Ok(original)
    }
    fn publish_mapping(
        &mut self,
        members: Vec<DeclaredVariable>,
        retired: &mut Retirement,
    ) -> Result<(), CmdError> {
        let reached = self.empty_current(retired);
        if let Err(error) = &reached {
            self.retain(error);
        }
        reached?;
        self.checked()?;
        let Some(target) = slot_mut(&mut self.vm.oo, self.key) else {
            let error = CmdError::from(ValueError::CommandProtocolUnavailable(
                "original OO declaration map",
            ));
            self.retain(&error);
            return Err(error);
        };
        debug_assert!(target.is_empty());
        *target = members;
        Ok(())
    }
    fn finish_mapping_retirement(&mut self, retired: Option<Retirement>) -> Result<(), CmdError> {
        if let Some(mut retired) = retired {
            let reached = self.drain_roles(&mut retired);
            if let Err(error) = &reached {
                self.retain(error);
            }
            let cleanup = self.clear_matching(&retired);
            if let Err(error) = &cleanup {
                self.retain(error);
            }
            self.cleanup.check_cleanup(self.vm)?;
            reached?;
            cleanup?;
        }
        self.checked()
    }
}

pub(super) fn install(
    vm: &mut Vm,
    names: &OriginalOoNameScope,
    class: bool,
    target: OoId,
    private: bool,
    policy: NamePolicyProtocol,
    originals: &[DeclaredVariable],
) -> Result<(), CmdError> {
    names.check(vm)?;
    let available = selected(policy)
        && (!private
            || matches!(policy.recipe(), NativeNameProtocol::C(version) if version >= tcl_dialect::TclVersion::V9_0));
    names.require_current(vm, available, "original C OO declaration mapping worker")?;
    let operation = NativeOperationScope::enter(vm)?;
    let cleanup = OriginalOoCleanup::enter(vm);
    let creation = vm
        .oo
        .objects
        .get(&target)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original OO declaration provider",
        ))?
        .creation_id;
    let mut worker = Mapping {
        vm,
        names,
        operation,
        cleanup,
        key: (target, class, private),
        policy,
        creation,
    };
    let reached = install_original_variable_mapping(&mut worker, originals);
    if let Err(error) = &reached {
        worker.retain(error);
    }
    worker.checked()?;
    reached
}

#[cfg(test)]
mod tests;
