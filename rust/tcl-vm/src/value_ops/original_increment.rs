// SPDX-License-Identifier: AGPL-3.0-or-later
//! Increment adapters retain the original numeric owner through existing COW and stores.

use super::{Value, Vm, VmIncrementObjects};
use crate::interp::native_numeric_ops::OriginalNumericOps;
use tcl_cmd_core::{CmdError, native_increment::NativeIncrementObjects};
use tcl_syntax::{
    number::Number,
    scalar_getter::{
        NativeLegacyIncrementRecipe, NativeNumberGetterKind, NativeScalarCache,
        NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterValue,
    },
    value::ValueError,
};

pub(crate) struct VmOriginalIncrementObjects<'a> {
    selected: VmIncrementObjects,
    numeric: OriginalNumericOps<'a>,
}
impl<'a> VmOriginalIncrementObjects<'a> {
    pub(crate) fn entered(vm: &'a mut Vm) -> Result<Self, CmdError> {
        let dialect = vm.native_invocation_dialect();
        let selected = VmIncrementObjects::selected(dialect)?;
        let numeric = OriginalNumericOps::entered(vm)?;
        if dialect.native_scalar_getter_protocol()
            != numeric.dialect().native_scalar_getter_protocol()
            || dialect.native_string_protocol() != numeric.dialect().native_string_protocol()
        {
            return Err(ValueError::ScalarNumericInputUnavailable.into());
        }
        Ok(Self { selected, numeric })
    }
    pub(crate) fn complete<T>(&self, reached: Result<T, CmdError>) -> Result<T, CmdError> {
        if let Err(first) = self.numeric.current() {
            drop(reached);
            let _ = self.numeric.current();
            return Err(first);
        }
        reached
    }
}
impl NativeIncrementObjects for VmOriginalIncrementObjects<'_> {
    type Value = Value;
    type Prepared = Value;
    fn prepare(&self, original: Option<&Value>) -> Result<Value, CmdError> {
        self.numeric.current()?;
        if let Some(original) = original {
            self.numeric.current_for_value(original)?;
        }
        let reached = self.selected.prepare(original);
        self.numeric.current()?;
        let prepared = reached?;
        self.numeric.current_for_value(&prepared)?;
        Ok(prepared)
    }
    fn current<'a>(&self, prepared: &'a Value) -> &'a Value {
        prepared
    }
    fn probe(
        &self,
        value: &Value,
        kind: NativeNumberGetterKind,
    ) -> Result<Result<Number, NativeScalarGetterFailure>, CmdError> {
        self.numeric.number(value, kind)
    }
    fn integer_failure(
        &self,
        value: &Value,
        kind: NativeNumberGetterKind,
    ) -> Result<CmdError, CmdError> {
        let getter = if kind == NativeNumberGetterKind::Bignum {
            NativeScalarGetterKind::Wide
        } else {
            NativeScalarGetterKind::Int
        };
        match self.numeric.getter(value, getter) {
            Err(error) if error.native_execution_refusal().is_some() => Err(error),
            Err(error) => Ok(error),
            Ok(_) => Err(ValueError::CommandProtocolUnavailable(
                "native increment integer failure stage",
            )
            .into()),
        }
    }
    fn add(&self, current: Number, amount: Number) -> Result<Number, CmdError> {
        self.numeric.current()?;
        let reached = self.selected.add(current, amount);
        self.numeric.current()?;
        reached
    }
    fn store(&self, prepared: &mut Value, sum: Number) -> Result<(), CmdError> {
        prepared.store_native_increment_number_checked(sum, self.numeric.dialect(), &mut || {
            self.numeric.current_for_value(prepared)
        })
    }
    fn finish(&self, prepared: Value) -> Value {
        prepared
    }
}

pub(crate) struct VmLegacyIncrementObjects<'a> {
    numeric: OriginalNumericOps<'a>,
    recipe: NativeLegacyIncrementRecipe,
}
impl<'a> VmLegacyIncrementObjects<'a> {
    pub(crate) fn entered(vm: &'a mut Vm) -> Result<Self, CmdError> {
        let dialect = vm.native_invocation_dialect();
        let selected = dialect
            .native_legacy_increment_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let numeric = OriginalNumericOps::entered(vm)?;
        if Some(selected) != numeric.dialect().native_legacy_increment_protocol()
            || dialect.native_scalar_getter_protocol()
                != numeric.dialect().native_scalar_getter_protocol()
            || dialect.native_string_protocol() != numeric.dialect().native_string_protocol()
        {
            return Err(ValueError::ScalarNumericInputUnavailable.into());
        }
        Ok(Self {
            numeric,
            recipe: selected.recipe(),
        })
    }
    pub(crate) fn complete<T>(&self, reached: Result<T, CmdError>) -> Result<T, CmdError> {
        if let Err(first) = self.numeric.current() {
            drop(reached);
            let _ = self.numeric.current();
            return Err(first);
        }
        reached
    }
}
impl tcl_cmd_core::native_increment::LegacyIncrementObjects for VmLegacyIncrementObjects<'_> {
    type Value = Value;
    type Prepared = Value;
    fn recipe(&self) -> NativeLegacyIncrementRecipe {
        self.recipe
    }
    fn cache(&self, value: &Value) -> Result<Option<NativeScalarCache>, CmdError> {
        self.numeric.current_for_value(value)?;
        Ok(value.native_scalar_cache())
    }
    fn wide(&self, value: &Value) -> Result<i64, CmdError> {
        match self.numeric.getter(value, NativeScalarGetterKind::Wide)? {
            NativeScalarGetterValue::Wide(value) => Ok(value),
            _ => Err(ValueError::ScalarNumericInputUnavailable.into()),
        }
    }
    fn prepare(&self, original: Option<&Value>) -> Result<Value, CmdError> {
        self.numeric.current()?;
        let string = self
            .numeric
            .dialect()
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if let Some(original) = original {
            self.numeric.current_for_value(original)?;
        }
        let prepared = match original {
            Some(value) if value.native_object_is_shared() => {
                value.duplicate_native_object_in(string)
            }
            Some(value) => value.clone(),
            None => Value::int(0),
        };
        self.numeric.current_for_value(&prepared)?;
        Ok(prepared)
    }
    fn current<'a>(&self, prepared: &'a Value) -> &'a Value {
        prepared
    }
    fn store(&self, prepared: &mut Value, cache: NativeScalarCache) -> Result<(), CmdError> {
        prepared.store_native_legacy_increment_cache_checked(
            cache,
            self.numeric.dialect(),
            &mut || self.numeric.current_for_value(prepared),
        )
    }
    fn finish(&self, prepared: Value) -> Value {
        prepared
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::native_operation_currency::NativeOperationScope;
    use tcl_cmd_core::native_increment::LegacyIncrementObjects;

    fn core(engine: &str) -> Vm {
        crate::native_fixture::core(crate::environment::profile_for_dialect(engine))
    }

    #[test]
    fn original_increment_adapters_keep_selected_cached_receivers_and_amount_owners() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // These are actual software adapters/owners, not external getter/refcount chronology.
        for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = core(engine);
            let current = Value::int(6);
            let amount = Value::int(18);
            let amount_before = amount.native_object_snapshot();
            let current_refs = current.native_object_reference_count();
            let held = current.native_lifetime_lease();
            assert_eq!(current.native_object_reference_count(), current_refs);
            let ops = VmOriginalIncrementObjects::entered(&mut vm).unwrap();
            let reached =
                tcl_cmd_core::native_increment::increment(&ops, Some(held.value()), &amount);
            let result = ops.complete(reached).unwrap();
            assert_eq!(
                result.native_scalar_cache(),
                Some(NativeScalarCache::Number(Number::Int(24)))
            );
            assert_eq!(amount.native_object_snapshot(), amount_before);
            assert!(result.resident_string_bytes().is_none());
            drop(ops);
            assert!(vm.execution_refusal.is_none());
        }
        let mut vm = core("tcl8.4");
        let dialect = vm.actual_native_invocation_dialect();
        let current =
            Value::from_native_scalar_cache(NativeScalarCache::Tcl84Long(6), None, dialect)
                .unwrap();
        let independent =
            Value::from_native_scalar_cache(NativeScalarCache::Tcl84Long(6), None, dialect)
                .unwrap();
        independent
            .store_native_legacy_increment_cache(NativeScalarCache::Tcl84Long(6), dialect)
            .unwrap();
        assert_eq!(
            independent.native_scalar_cache(),
            Some(NativeScalarCache::Tcl84Long(6))
        );
        let ops = VmLegacyIncrementObjects::entered(&mut vm).unwrap();
        let amount = tcl_cmd_core::native_increment::default_legacy_amount(ops.recipe());
        let reached =
            tcl_cmd_core::native_increment::increment_legacy(&ops, Some(&current), amount);
        assert_eq!(
            ops.complete(reached).unwrap().native_scalar_cache(),
            Some(NativeScalarCache::Tcl84Long(7))
        );
    }

    #[test]
    fn original_increment_adapters_refuse_retired_inputs_before_prepare_probe_and_store() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for legacy in [false, true] {
            let mut vm = core(if legacy { "tcl8.4" } else { "tcl8.6" });
            let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
            let original = Value::int(6);
            let held = original.native_lifetime_lease();
            drop(original);
            let error = if legacy {
                let ops = VmLegacyIncrementObjects::entered(&mut vm).unwrap();
                ops.prepare(Some(held.value())).unwrap_err()
            } else {
                let ops = VmOriginalIncrementObjects::entered(&mut vm).unwrap();
                ops.prepare(Some(held.value())).unwrap_err()
            };
            let first = vm.execution_refusal.clone().unwrap();
            assert_eq!(error.native_execution_refusal(), Some(&first));
            assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
            assert!(held.value().resident_string_bytes().is_none());
            assert!(held.value().native_scalar_cache().is_none());
        }
    }

    #[test]
    fn original_increment_completion_keeps_prior_full_cause_and_stops_variable_publication() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut vm = core("tcl8.6");
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let _ = vm.refuse_host_command("original increment owner".into());
        let first = vm.execution_refusal.clone().unwrap();
        let error = VmOriginalIncrementObjects::entered(&mut vm).err().unwrap();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    }

    #[test]
    fn original_increment_store_checks_withdrawal_before_cache_or_string_invalidation() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // A supplied stage checker releases the real header; it is not a C free callback issuer.
        for legacy in [false, true] {
            let mut vm = core(if legacy { "tcl8.4" } else { "tcl8.6" });
            let original = Value::new_native_string_bytes(b"6");
            let held = original.native_lifetime_lease();
            let mut owner = Some(original);
            let dialect = vm.actual_native_invocation_dialect();
            let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
            let numeric = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
            let mut calls = 0;
            let mut check = || {
                calls += 1;
                if calls == 2 {
                    drop(owner.take());
                }
                numeric.current_for_value(held.value())
            };
            let error = if legacy {
                held.value()
                    .store_native_legacy_increment_cache_checked(
                        NativeScalarCache::Tcl84Long(7),
                        dialect,
                        &mut check,
                    )
                    .unwrap_err()
            } else {
                held.value()
                    .store_native_increment_number_checked(Number::Int(7), dialect, &mut check)
                    .unwrap_err()
            };
            assert!(!held.value().native_object_is_live());
            assert!(held.value().native_scalar_cache().is_none());
            drop(numeric);
            let first = vm.execution_refusal.clone().unwrap();
            assert_eq!(error.native_execution_refusal(), Some(&first));
            assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
        }
    }
    #[test]
    fn original_increment_adapter_refuses_actual_scope_aba_after_probe_before_store() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // This software mutable adapter callback changes and restores the actual Host;
        // it does not claim a callback in the closed Number getter itself.
        let mut vm = core("tcl8.6");
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let current = Value::int(6);
        let amount = Value::int(18);
        let ops = VmOriginalIncrementObjects::entered(&mut vm).unwrap();
        let mut prepared = ops.prepare(Some(&current)).unwrap();
        assert_eq!(
            ops.probe(&prepared, NativeNumberGetterKind::IncrementNumber)
                .unwrap()
                .unwrap(),
            Number::Int(6)
        );
        let before = prepared.native_object_snapshot();
        let first = ops
            .numeric
            .with_vm_for_value(&amount, |vm| {
                let original = vm.host_rc();
                vm.set_host(std::rc::Rc::new(crate::host_native::NativeHost::new()));
                vm.set_host(original);
                Ok(())
            })
            .unwrap_err();
        let store = ops.store(&mut prepared, Number::Int(24)).unwrap_err();
        assert_eq!(
            store.native_execution_refusal(),
            first.native_execution_refusal()
        );
        assert_eq!(prepared.native_object_snapshot(), before);
        drop(prepared);
        let reached: Result<(), CmdError> = Err(store);
        assert_eq!(
            ops.complete(reached)
                .unwrap_err()
                .native_execution_refusal(),
            first.native_execution_refusal()
        );
        drop(ops);
        assert_eq!(
            scope.check_native(&mut vm).unwrap_err(),
            first.native_execution_refusal().unwrap().clone()
        );
    }

    #[test]
    fn original_jim_wide_expression_keeps_cached_current_context_and_refuses_foreign_or_retired_owners()
     {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Cached current-context branch and actual foreign context admission only;
        // external Jim expression getter chronology is a separate proof.
        let mut vm = core("jim");
        let original = Value::int(6);
        let before = original.native_object_snapshot();
        let refs = original.native_object_reference_count();
        assert_eq!(vm.native_jim_wide_expression(&original).unwrap(), 6);
        assert_eq!(original.native_object_snapshot(), before);
        assert_eq!(original.native_object_reference_count(), refs);
        let foreign = core("jim");
        let foreign_context = foreign.native_jim_object_context().unwrap();
        let other = Value::int(18);
        other.bind_native_jim_context(&foreign_context).unwrap();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let error = vm.native_jim_wide_expression(&other).unwrap_err();
        assert!(error.native_execution_refusal().is_some());
        assert_eq!(
            other.native_scalar_cache(),
            Some(NativeScalarCache::Number(Number::Int(18)))
        );
        drop(crate::command::completion_from_cmd_error(
            &mut vm,
            error.clone(),
        ));
        assert_eq!(
            scope.check_native(&mut vm).unwrap_err(),
            error.native_execution_refusal().unwrap().clone()
        );
    }

    #[test]
    fn original_jim_wide_expression_refuses_retired_header_before_context_or_guest_work() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut vm = core("jim");
        let original = Value::int(6);
        let held = original.native_lifetime_lease();
        drop(original);
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let error = vm.native_jim_wide_expression(held.value()).unwrap_err();
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert!(!held.value().native_object_is_live());
        assert!(held.value().native_scalar_cache().is_none());
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    }
}
