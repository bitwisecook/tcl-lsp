// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original property context and derivation observations, without lookup grants.
use super::super::original_name_operands::OriginalOoNameScope;
use super::*;
use crate::interp::{
    native_numeric_ops::OriginalNumericOps, native_operation_currency::NativeOperationScope,
};
use crate::value::NativeObjectLifetimeLease;
use std::rc::Weak;
use tcl_cmd_core::CmdError;
use tcl_syntax::native_string::NativeStringProtocol;

pub(super) fn retain_error(vm: &mut Vm, error: &CmdError) {
    if let Some(cause) = error.native_execution_refusal() {
        vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause.clone()));
    }
}
pub(super) struct OriginalPropertyOperation {
    pub(super) operation: NativeOperationScope,
    names: OriginalOoNameScope,
    pub(super) strings: NativeStringProtocol,
}
impl OriginalPropertyOperation {
    pub(super) fn enter(
        vm: &mut Vm,
        target: OoId,
        class: bool,
        strings: NativeStringProtocol,
    ) -> Result<Self, CmdError> {
        let operation = NativeOperationScope::enter(vm)?;
        let names = OriginalOoNameScope::enter(vm, Some((class, target)))?;
        let entered = Self {
            operation,
            names,
            strings,
        };
        entered.check(vm)?;
        Ok(entered)
    }
    pub(super) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.operation.check(vm)?;
        self.names.check(vm)
    }
    pub(super) fn require(
        &self,
        vm: &mut Vm,
        current: bool,
        purpose: &'static str,
    ) -> Result<(), CmdError> {
        self.names.require_current(vm, current, purpose)
    }
    pub(super) fn retained_target(
        &self,
    ) -> Option<super::super::original_name_operands::OriginalOoTarget> {
        self.names.retained_target()
    }
    pub(super) fn select_declaring(&mut self, vm: &mut Vm) -> Result<(), CmdError> {
        self.names.select_declaring_provider(vm)
    }
    pub(super) fn string(&self, vm: &mut Vm, original: &Value) -> Result<Rc<[u8]>, CmdError> {
        self.string_checked(vm, original, |_| Ok(()))
    }
    pub(super) fn string_checked(
        &self,
        vm: &mut Vm,
        original: &Value,
        check: impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<Rc<[u8]>, CmdError> {
        self.check(vm)?;
        check(vm)?;
        let reached = OriginalNumericOps::in_original_scope(vm, &self.operation)?
            .string(original, self.strings);
        if let Err(error) = &reached {
            retain_error(vm, error);
        }
        self.check(vm)?;
        check(vm)?;
        reached
    }
    pub(super) fn list(
        &self,
        vm: &mut Vm,
        original: &Value,
    ) -> Result<crate::NativeListItems, CmdError> {
        self.list_checked(vm, original, |_| Ok(()))
    }
    pub(super) fn list_checked(
        &self,
        vm: &mut Vm,
        original: &Value,
        check: impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<crate::NativeListItems, CmdError> {
        self.check(vm)?;
        check(vm)?;
        let reached = {
            let ops = OriginalNumericOps::in_original_scope(vm, &self.operation)?;
            ops.list_with_original_check(original, self.strings, || {
                ops.with_vm_for_value(original, |vm| {
                    self.check(vm)?;
                    check(vm)
                })
            })
        };
        if let Err(error) = &reached {
            retain_error(vm, error);
        }
        self.check(vm)?;
        check(vm)?;
        reached
    }
    pub(super) fn copy_list(&self, vm: &mut Vm, original: &Value) -> Result<Value, CmdError> {
        self.check(vm)?;
        let reached = {
            let context = std::cell::RefCell::new(&mut *vm);
            original.native_list_copy_checked(
                self.strings,
                &mut || self.check(&mut context.borrow_mut()),
                &mut |actual| self.string(&mut context.borrow_mut(), actual),
            )
        };
        if let Err(error) = &reached {
            retain_error(vm, error);
        }
        self.check(vm)?;
        reached
    }
    pub(super) fn dictionary(
        &self,
        vm: &mut Vm,
        pairs: Vec<(Value, Value)>,
    ) -> Result<Value, CmdError> {
        self.check(vm)?;
        let reached = Value::native_dictionary_constructor_checked(
            pairs,
            None,
            self.strings,
            vm,
            &mut |vm, actual| self.string(vm, actual),
            &mut |vm| self.check(vm),
        );
        if let Err(error) = &reached {
            retain_error(vm, error);
        }
        self.check(vm)?;
        reached
    }
    pub(super) fn prepare_dictionary(
        &self,
        vm: &mut Vm,
        original: &Value,
    ) -> Result<crate::value::PreparedNativeDictionary, CmdError> {
        self.check(vm)?;
        let reached = {
            let context = std::cell::RefCell::new(&mut *vm);
            original.prepare_native_dictionary_after_conversion_checked(
                self.strings,
                &mut || self.check(&mut context.borrow_mut()),
                &mut |actual| self.string(&mut context.borrow_mut(), actual),
            )
        };
        if let Err(error) = &reached {
            retain_error(vm, error);
        }
        self.check(vm)?;
        reached
    }
    pub(super) fn put_dictionary(
        &self,
        vm: &mut Vm,
        dictionary: &mut crate::value::PreparedNativeDictionary,
        key: Value,
        value: Value,
    ) -> Result<(), CmdError> {
        self.check(vm)?;
        let root = dictionary.original().native_lifetime_lease();
        let reached = {
            let context = std::cell::RefCell::new(&mut *vm);
            dictionary.set_member_checked(
                key,
                value,
                &mut || {
                    self.check(&mut context.borrow_mut())?;
                    root.value().check_native_header().map_err(Into::into)
                },
                &mut |actual| self.string(&mut context.borrow_mut(), actual),
            )
        };
        if let Err(error) = &reached {
            retain_error(vm, error);
        }
        drop(root);
        self.check(vm)?;
        reached
    }
    pub(super) fn complete_result<T>(
        self,
        vm: &mut Vm,
        reached: Result<T, Completion<Value>>,
    ) -> Result<T, Completion<Value>> {
        let cleanup = self.operation.clone();
        match self.finish(vm) {
            Ok(()) => reached,
            Err(error) => {
                retain_error(vm, &error);
                drop(reached);
                let error = cleanup.check(vm).err().unwrap_or(error);
                Err(crate::command::completion_from_cmd_error(vm, error))
            }
        }
    }
    pub(super) fn complete_invocation(
        self,
        vm: &mut Vm,
        reached: Completion<Value>,
    ) -> Completion<Value> {
        let cleanup = self.operation.clone();
        match self.finish(vm) {
            Ok(()) => reached,
            Err(error) => {
                retain_error(vm, &error);
                drop(reached);
                let error = cleanup.check(vm).err().unwrap_or(error);
                crate::command::completion_from_cmd_error(vm, error)
            }
        }
    }
    pub(super) fn complete<T>(
        self,
        vm: &mut Vm,
        reached: Result<T, CmdError>,
    ) -> Result<T, CmdError> {
        if let Err(error) = &reached {
            retain_error(vm, error);
        }
        let cleanup = self.operation.clone();
        match self.finish(vm) {
            Ok(()) => reached,
            Err(error) => {
                retain_error(vm, &error);
                drop(reached);
                cleanup.check(vm)?;
                Err(error)
            }
        }
    }
    pub(super) fn finish(self, vm: &mut Vm) -> Result<(), CmdError> {
        let before = self.check(vm);
        let context = self.names.finish(vm);
        self.operation.check(vm)?;
        context?;
        before
    }
}

/// Pure original roster and graph observations apply only while deriving a report.
/// Materialized configure snapshots and tables retain their own header roles.
struct ClassObservation {
    id: OoId,
    birth: Weak<()>,
    supers: Vec<OoId>,
    mixins: Vec<OoId>,
}
pub(super) struct PropertyDerivation {
    target: OoId,
    class: bool,
    writable: bool,
    declared: bool,
    epoch: u64,
    method_epochs: (u64, u64),
    roster: Vec<(Vec<u8>, usize)>,
    providers: Vec<ClassObservation>,
    object_class: Option<(OoId, Vec<OoId>)>,
}
pub(super) fn report_members(
    vm: &Vm,
    target: OoId,
    class: bool,
    writable: bool,
    declared: bool,
) -> Result<Vec<(Vec<u8>, NativeObjectLifetimeLease)>, CmdError> {
    let mut members = BTreeMap::new();
    if declared {
        if let Some(originals) = vm.oo.property_members.get(&(target, class, writable)) {
            for (name, original) in originals {
                members.insert(vm.oo.method_name_bytes(name).to_vec(), original);
            }
        }
    } else {
        let mut seen = BTreeSet::new();
        if class {
            vm.oo
                .collect_native_class_properties(target, writable, &mut seen, &mut members)?;
        } else {
            let object =
                vm.oo
                    .objects
                    .get(&target)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "native property object",
                    ))?;
            vm.oo.collect_native_declared_properties(
                target,
                false,
                writable,
                slot_ref_obj(object, writable),
                &mut members,
            )?;
            for mixin in &object.mixins {
                vm.oo
                    .collect_native_class_properties(*mixin, writable, &mut seen, &mut members)?;
            }
            vm.oo.collect_native_class_properties(
                object.class,
                writable,
                &mut seen,
                &mut members,
            )?;
        }
    }
    Ok(members
        .into_iter()
        .map(|(name, original)| (name, original.native_lifetime_lease()))
        .collect())
}
impl PropertyDerivation {
    pub(super) fn capture(
        vm: &Vm,
        target: OoId,
        class: bool,
        writable: bool,
        declared: bool,
    ) -> Result<Self, CmdError> {
        let roster = report_members(vm, target, class, writable, declared)?
            .into_iter()
            .map(|(name, original)| (name, original.value().native_object_identity()))
            .collect();
        // All actual class facets are pure observations; no Method/Value role is added.
        let providers = vm
            .oo
            .classes
            .iter()
            .map(|(id, facet)| ClassObservation {
                id: *id,
                birth: Rc::downgrade(&facet.birth),
                supers: facet.supers.clone(),
                mixins: facet.mixins.clone(),
            })
            .collect();
        let object_class = (!class)
            .then(|| {
                vm.oo
                    .objects
                    .get(&target)
                    .map(|o| (o.class, o.mixins.clone()))
            })
            .flatten();
        Ok(Self {
            target,
            class,
            writable,
            declared,
            epoch: vm.oo.property_foundation_epoch,
            method_epochs: vm.oo.native_methods.observation_epochs(target),
            roster,
            providers,
            object_class,
        })
    }
    pub(super) fn check(
        &self,
        vm: &mut Vm,
        operation: &OriginalPropertyOperation,
    ) -> Result<(), CmdError> {
        operation.check(vm)?;
        let providers_current = self.providers.len() == vm.oo.classes.len()
            && self.providers.iter().all(|old| {
                vm.oo.classes.get(&old.id).is_some_and(|current| {
                    old.birth
                        .upgrade()
                        .is_some_and(|birth| Rc::ptr_eq(&birth, &current.birth))
                        && old.supers == current.supers
                        && old.mixins == current.mixins
                })
            });
        let roster = report_members(vm, self.target, self.class, self.writable, self.declared)?
            .into_iter()
            .map(|(name, original)| (name, original.value().native_object_identity()))
            .collect::<Vec<_>>();
        let object_current = self.object_class.as_ref().is_none_or(|old| {
            vm.oo
                .objects
                .get(&self.target)
                .is_some_and(|o| old == &(o.class, o.mixins.clone()))
        });
        operation.require(
            vm,
            self.epoch == vm.oo.property_foundation_epoch
                && self.method_epochs == vm.oo.native_methods.observation_epochs(self.target)
                && providers_current
                && object_current
                && self.roster == roster,
            "original property report derivation",
        )
    }
}

pub(super) fn compare(
    vm: &mut Vm,
    operation: &OriginalPropertyOperation,
    a: &Value,
    b: &Value,
    check: impl Fn(&mut Vm) -> Result<(), CmdError>,
) -> Result<std::cmp::Ordering, CmdError> {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    operation.check(vm)?;
    check(vm)?;
    a.check_native_header()?;
    b.check_native_header()?;
    if a.native_object_identity() == b.native_object_identity() {
        return Ok(std::cmp::Ordering::Equal);
    }
    let sa = a.native_object_snapshot();
    let sb = b.native_object_snapshot();
    if matches!(sa.cache, Cache::String { .. }) && matches!(sb.cache, Cache::String { .. }) {
        let representation = tcl_registry::native_string_length::NativeStringLengthRepresentation::PureProperByteArray;
        let ca = a.native_character_count_with_protocol(operation.strings, representation)?;
        let cb = b.native_character_count_with_protocol(operation.strings, representation)?;
        let reached = if sa.resident.as_ref().is_some_and(|bytes| bytes.len() == ca)
            && sb.resident.as_ref().is_some_and(|bytes| bytes.len() == cb)
        {
            sa.resident.cmp(&sb.resident)
        } else {
            a.native_unicode_units(operation.strings)?
                .cmp(&b.native_unicode_units(operation.strings)?)
        };
        operation.check(vm)?;
        check(vm)?;
        return Ok(reached);
    }
    if let (Cache::ByteArray { bytes: a, .. }, Cache::ByteArray { bytes: b, .. }) =
        (&sa.cache, &sb.cache)
        && sa.resident.is_none()
        && sb.resident.is_none()
    {
        return Ok(a.cmp(b));
    }
    let a = operation.string_checked(vm, a, &check)?;
    let b = operation.string_checked(vm, b, &check)?;
    let version = operation
        .strings
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "property member string comparison",
        ))?;
    let utf = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version);
    Ok(utf.decode_units(&a).cmp(&utf.decode_units(&b)))
}

/// Allocation-only original declarations are compared against the same inventory.
pub(super) struct PrivateVariableRoster {
    provider: OoId,
    object: bool,
    originals: Vec<(
        tcl_core_types::NameBytes,
        NativeObjectLifetimeLease,
        Option<NativeObjectLifetimeLease>,
    )>,
}
impl PrivateVariableRoster {
    pub(super) fn capture(vm: &Vm, provider: OoId, object: bool) -> Result<Self, CmdError> {
        let variables = if object {
            vm.oo.objects.get(&provider).map(|o| &o.private_variables)
        } else {
            vm.oo.classes.get(&provider).map(|c| &c.private_variables)
        };
        let variables = variables.ok_or(ValueError::CommandProtocolUnavailable(
            "private property variable provider",
        ))?;
        Ok(Self {
            provider,
            object,
            originals: variables
                .iter()
                .map(|variable| {
                    (
                        variable.name.clone(),
                        variable.original.native_lifetime_lease(),
                        variable
                            .full_name
                            .as_ref()
                            .map(|full_name| full_name.original.native_lifetime_lease()),
                    )
                })
                .collect(),
        })
    }
    pub(super) fn check(
        &self,
        vm: &mut Vm,
        operation: &OriginalPropertyOperation,
    ) -> Result<(), CmdError> {
        operation.check(vm)?;
        let current = if self.object {
            vm.oo
                .objects
                .get(&self.provider)
                .map(|o| &o.private_variables)
        } else {
            vm.oo
                .classes
                .get(&self.provider)
                .map(|c| &c.private_variables)
        };
        let same = current.is_some_and(|current| {
            current.len() == self.originals.len()
                && current.iter().zip(&self.originals).all(
                    |(current, (name, original, full_name))| {
                        &current.name == name
                            && current.original.is_same_object(original.value())
                            && match (&current.full_name, full_name) {
                                (None, None) => true,
                                (Some(current), Some(original)) => {
                                    current.original.is_same_object(original.value())
                                }
                                _ => false,
                            }
                    },
                )
        });
        operation.require(vm, same, "original property private-variable roster")
    }
    pub(super) fn lookup(
        &self,
        vm: &mut Vm,
        operation: &OriginalPropertyOperation,
        original: &Value,
    ) -> Result<Option<Vec<u8>>, CmdError> {
        self.check(vm, operation)?;
        for (name, variable, full_name) in &self.originals {
            if compare(vm, operation, variable.value(), original, |vm| {
                self.check(vm, operation)
            })? == std::cmp::Ordering::Equal
            {
                self.check(vm, operation)?;
                if let Some(full_name) = full_name {
                    let bytes = operation.string(vm, full_name.value());
                    self.check(vm, operation)?;
                    return Ok(Some(bytes?.to_vec()));
                }
                let creation = vm
                    .oo
                    .objects
                    .get(&self.provider)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "private property variable provider",
                    ))?
                    .creation_id;
                return Ok(Some(
                    private_storage_name_bytes(creation, name.as_bytes())
                        .as_bytes()
                        .to_vec(),
                ));
            }
        }
        Ok(None)
    }
}
