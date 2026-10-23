// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected C declaration-map references and old-visible retirement.
use super::*;
use crate::interp::native_arguments::OriginalArguments;
use tcl_cmd_core::{native_oo_variable_mapping::NativeOoVariableMappingOps, CmdError};
use tcl_syntax::{
    naming::{NamePolicyAuthority, NamePolicyProtocol, NativeNameProtocol},
    value::ValueError,
};

type Slot = (OoId, bool, bool);
pub(super) fn selected(policy: NamePolicyProtocol) -> bool {
    policy.authority() == NamePolicyAuthority::Native
        && matches!(policy.recipe(), NativeNameProtocol::C(version) if version >= tcl_dialect::TclVersion::V8_6)
}
fn slot(oo: &OoState, key: Slot) -> Option<&Vec<DeclaredVariable>> {
    let (target, class, private) = key;
    if class {
        oo.classes.get(&target).map(|entry| {
            if private {
                &entry.private_variables
            } else {
                &entry.variables
            }
        })
    } else {
        oo.objects.get(&target).map(|entry| {
            if private {
                &entry.private_variables
            } else {
                &entry.variables
            }
        })
    }
}
fn slot_mut(oo: &mut OoState, key: Slot) -> Option<&mut Vec<DeclaredVariable>> {
    let (target, class, private) = key;
    if class {
        oo.classes.get_mut(&target).map(|entry| {
            if private {
                &mut entry.private_variables
            } else {
                &mut entry.variables
            }
        })
    } else {
        oo.objects.get_mut(&target).map(|entry| {
            if private {
                &mut entry.private_variables
            } else {
                &mut entry.variables
            }
        })
    }
}
fn unavailable() -> CmdError {
    ValueError::CommandProtocolUnavailable("original OO declaration mapping").into()
}
pub(super) fn observation(original: &DeclaredVariable) -> DeclaredVariable {
    DeclaredVariable {
        original: original.original.observation(),
        name: original.name.clone(),
        full_name: original
            .full_name
            .as_ref()
            .map(|value| DeclaredVariableFullName {
                original: value.original.observation(),
                name: value.name.clone(),
            }),
    }
}
struct Retirement {
    token: Rc<()>,
    old: Vec<DeclaredVariable>,
}
pub(super) struct Entry {
    policy: NamePolicyProtocol,
    creation: u64,
    level: usize,
    frame: std::rc::Weak<()>,
}
impl Entry {
    pub(super) fn capture(
        interpreter: &Interp,
        target: &DefTarget,
        policy: NamePolicyProtocol,
        originals: &OriginalArguments,
        owner: &native_context::OriginalOoTarget,
    ) -> Result<Self, CmdError> {
        originals.check_for(interpreter)?;
        owner.check(interpreter)?;
        let object = match target {
            DefTarget::Class(id) | DefTarget::Object(id) => *id,
        };
        let creation = interpreter
            .oo
            .borrow()
            .objects
            .get(&object)
            .ok_or_else(unavailable)?
            .creation_id;
        let frames = interpreter.frames.borrow();
        let level = frames.current_level();
        let frame = frames
            .original_activation_owner_at(level)
            .ok_or_else(unavailable)?;
        Ok(Self {
            policy,
            creation,
            level,
            frame,
        })
    }
    pub(super) fn prepare(
        &self,
        interpreter: &mut Interp,
        target: &DefTarget,
        values: &[*mut TclObj],
        originals: &OriginalArguments,
        owner: &native_context::OriginalOoTarget,
    ) -> Result<Vec<DeclaredVariable>, CmdError> {
        let (object, class) = match target {
            DefTarget::Class(id) => (*id, true),
            DefTarget::Object(id) => (*id, false),
        };
        let worker = Mapping {
            interpreter,
            originals,
            target: owner,
            key: (object, class, false),
            entry: self,
        };
        worker.checked()?;
        let mut descriptors = Vec::new();
        for &value in values {
            let name = worker.read(value)?;
            obj::check_native_liveness(value)
                .map_err(|error| originals.retain_refusal(error.into()))?;
            descriptors.push(DeclaredVariable {
                original: DeclaredVariableHeader::Observed(obj::NativeObjectLifetime::retain(
                    value,
                )),
                name,
                full_name: None,
            });
        }
        worker.checked()?;
        Ok(descriptors)
    }
}
struct Mapping<'a> {
    interpreter: &'a mut Interp,
    originals: &'a OriginalArguments,
    target: &'a native_context::OriginalOoTarget,
    key: Slot,
    entry: &'a Entry,
}
impl Mapping<'_> {
    fn checked(&self) -> Result<(), CmdError> {
        let reached = (|| {
            self.originals.check_for(self.interpreter)?;
            self.target.check(self.interpreter)?;
            let same_frame = {
                let frames = self.interpreter.frames.borrow();
                frames.current_level() == self.entry.level
                    && frames
                        .original_activation_owner_at(self.entry.level)
                        .is_some_and(|current| {
                            self.entry.frame.strong_count() != 0
                                && self.entry.frame.ptr_eq(&current)
                        })
            };
            if self.interpreter.name_policy_protocol() != Some(self.entry.policy) || !same_frame {
                return Err(unavailable());
            }
            Ok(())
        })();
        reached.map_err(|error| self.originals.retain_refusal(error))
    }
    fn read(&self, original: *mut TclObj) -> Result<Vec<u8>, CmdError> {
        self.checked()?;
        let reached = self.interpreter.native_object_string_bytes_using(
            original,
            &|| self.checked(),
            &|error| self.originals.retain_refusal(error.into()),
        );
        if let Err(error) = &reached {
            self.originals.retain_refusal(error.clone());
        }
        self.checked()?;
        Ok(reached?.to_vec())
    }
    fn matches(oo: &OoState, key: Slot, token: &Rc<()>) -> bool {
        oo.variable_mapping_retirements
            .get(&key)
            .and_then(std::rc::Weak::upgrade)
            .is_some_and(|current| Rc::ptr_eq(&current, token))
    }
    fn clear_matching(&self, retired: &Retirement) {
        // Matching cleanup observes only the actual physical map. It grants no
        // live target, frame or replacement authority after refusal.
        let visible = {
            let mut oo = self.interpreter.oo.borrow_mut();
            if Self::matches(&oo, self.key, &retired.token) {
                oo.variable_mapping_retirements.remove(&self.key);
                slot_mut(&mut oo, self.key).map(std::mem::take)
            } else {
                None
            }
        };
        drop(visible);
    }
    fn drain(&self, retired: &mut Retirement) -> Result<(), CmdError> {
        let mut first = None;
        for entry in std::mem::take(&mut retired.old) {
            let DeclaredVariable {
                original,
                name,
                full_name,
            } = entry;
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
        first.map_or(Ok(()), Err)
    }
    fn empty_current(&mut self, original: &Retirement) -> Result<(), CmdError> {
        loop {
            self.checked()?;
            let (matches, empty) = {
                let oo = self.interpreter.oo.borrow();
                (
                    Self::matches(&oo, self.key, &original.token),
                    slot(&oo, self.key).is_some_and(Vec::is_empty),
                )
            };
            if matches {
                self.clear_matching(original);
                return self.checked();
            }
            if empty {
                return Ok(());
            }
            // Legal same-target recursive publication is displaced at the same
            // old-visible stage before outer adoption; no roster epoch rejects it.
            let mut nested = self.capture_old_mapping()?;
            let reached = self.drain(&mut nested);
            let same = Self::matches(&self.interpreter.oo.borrow(), self.key, &nested.token);
            self.clear_matching(&nested);
            self.checked()?;
            reached?;
            if same {
                return Ok(());
            }
        }
    }
}
impl NativeOoVariableMappingOps for Mapping<'_> {
    type Member = DeclaredVariable;
    type Retirement = Retirement;
    fn check_mapping(&mut self) -> Result<(), CmdError> {
        self.checked()
    }
    fn retain_mapping_failure(&mut self, error: &CmdError) {
        self.originals.retain_refusal(error.clone());
    }
    fn acquire_member(
        &mut self,
        original: &DeclaredVariable,
    ) -> Result<DeclaredVariable, CmdError> {
        self.checked()?;
        let value = original.original.as_ptr();
        obj::check_native_liveness(value)
            .map_err(|error| self.originals.retain_refusal(error.into()))?;
        Ok(DeclaredVariable {
            original: DeclaredVariableHeader::Stored(obj::Owned::retain(value)),
            name: original.name.clone(),
            full_name: None,
        })
    }
    fn capture_old_mapping(&mut self) -> Result<Retirement, CmdError> {
        self.checked()?;
        let token = Rc::new(());
        let old = {
            let mut oo = self.interpreter.oo.borrow_mut();
            let target = slot_mut(&mut oo, self.key).ok_or_else(unavailable)?;
            let visible = target.iter().map(observation).collect();
            let old = std::mem::replace(target, visible);
            oo.variable_mapping_retirements
                .insert(self.key, Rc::downgrade(&token));
            old
        };
        Ok(Retirement { token, old })
    }
    fn retire_old_mapping(&mut self, retired: &mut Retirement) -> Result<(), CmdError> {
        self.drain(retired)
    }
    fn begin_new_mapping(&mut self, retired: &mut Retirement) -> Result<(), CmdError> {
        self.empty_current(retired)
    }
    fn member_key(&mut self, original: &DeclaredVariable) -> Result<Vec<u8>, CmdError> {
        self.read(original.original.as_ptr())
    }
    fn prepare_unique_member(
        &mut self,
        mut original: DeclaredVariable,
        key: &[u8],
    ) -> Result<DeclaredVariable, CmdError> {
        let reached = (|| {
            self.checked()?;
            original.name = key.to_vec();
            if self.key.2 {
                let bytes = self.read(original.original.as_ptr())?;
                let mut full = format!("{} : ", self.entry.creation).into_bytes();
                full.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
                original.full_name = Some(DeclaredVariableFullName {
                    original: DeclaredVariableHeader::Stored(obj::Owned::fresh(
                        obj::new_string_bytes(&full),
                    )),
                    name: full,
                });
            }
            self.checked()
        })();
        if let Err(error) = &reached {
            self.retain_mapping_failure(error);
        }
        reached?;
        Ok(original)
    }
    fn publish_mapping(
        &mut self,
        members: Vec<DeclaredVariable>,
        retired: &mut Retirement,
    ) -> Result<(), CmdError> {
        let reached = (|| {
            self.empty_current(retired)?;
            self.checked()?;
            Ok(())
        })();
        if let Err(error) = &reached {
            self.retain_mapping_failure(error);
        }
        reached?;
        let published = {
            let mut oo = self.interpreter.oo.borrow_mut();
            if let Some(target) = slot_mut(&mut oo, self.key) {
                debug_assert!(target.is_empty());
                *target = members;
                return Ok(());
            }
            unavailable()
        };
        self.retain_mapping_failure(&published);
        Err(published)
    }
    fn finish_mapping_retirement(&mut self, retired: Option<Retirement>) -> Result<(), CmdError> {
        if let Some(mut retired) = retired {
            let reached = self.drain(&mut retired);
            if let Err(error) = &reached {
                self.retain_mapping_failure(error);
            }
            self.clear_matching(&retired);
            // Original currency is checked after mandatory matching state cleanup.
            self.originals.check_for(self.interpreter)?;
            reached?;
        }
        self.checked()
    }
}
pub(super) fn install(
    interpreter: &mut Interp,
    target: &DefTarget,
    private: bool,
    entry: &Entry,
    values: &[DeclaredVariable],
    originals: &OriginalArguments,
    owner: &native_context::OriginalOoTarget,
) -> Result<(), CmdError> {
    originals.check_for(interpreter)?;
    owner.check(interpreter)?;
    if !selected(entry.policy)
        || (private
            && !matches!(entry.policy.recipe(), NativeNameProtocol::C(version) if version >= tcl_dialect::TclVersion::V9_0))
    {
        return Err(originals.retain_refusal(unavailable()));
    }
    let (object, class) = match target {
        DefTarget::Class(id) => (*id, true),
        DefTarget::Object(id) => (*id, false),
    };
    let mut worker = Mapping {
        interpreter,
        originals,
        target: owner,
        key: (object, class, private),
        entry,
    };
    let reached = tcl_cmd_core::native_oo_variable_mapping::install_original_variable_mapping(
        &mut worker,
        values,
    );
    if let Err(error) = &reached {
        worker.retain_mapping_failure(error);
    }
    worker.checked()?;
    reached
}

#[cfg(test)]
#[path = "variable_mapping/tests.rs"]
mod tests;
