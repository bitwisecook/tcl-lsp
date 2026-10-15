// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim table keys retain publication spelling separately from storage addresses.

use super::native_numeric_ops::OriginalNumericOps;
use super::native_operation_currency::NativeOperationScope;
use super::native_original_lookup::OriginalLookup;
use super::{Command, CommandSlot, NativeCommandLookupUnavailable, NsId, ROOT_NS, Vm};
use crate::{Value, value::NativeObjectLifetimeLease};
use tcl_cmd_core::CmdError;
use tcl_syntax::naming::{NativeJimCommandTableKey, NativeNameContext, NativeNamePurpose};
use tcl_syntax::value::ValueError;

/// The actual Jim qualification temporary and its independent table geometry.
pub(crate) struct OriginalJimCommandKey {
    pub(super) original: Value,
    pub(super) table: NativeJimCommandTableKey,
}

struct OriginalJimInventoryKey {
    slot: CommandSlot,
    storage: String,
    generation: u64,
    original: NativeObjectLifetimeLease,
    report: Vec<u8>,
}

impl OriginalJimInventoryKey {
    fn check(
        &self,
        vm: &mut Vm,
        scope: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<(), CmdError> {
        scope.current_for_value(vm, self.original.value())?;
        let same_binding = vm.visible_command_generation(&self.storage) == Some(self.generation)
            && vm.command_slot(&self.storage).as_ref() == Some(&self.slot);
        let same_key = vm
            .name_world
            .borrow()
            .jim_command_original_keys
            .get(&self.slot)
            .is_some_and(|key| key.original.is_same_object(self.original.value()));
        scope.require(
            vm,
            same_binding && same_key,
            "original Jim inventory current key and generation",
        )
    }
}

struct UnownedJimKey(Option<Value>);
impl UnownedJimKey {
    fn value(&self) -> &Value {
        self.0.as_ref().expect("unpromoted original Jim key")
    }
    fn promote(mut self) -> Value {
        self.0
            .take()
            .expect("original Jim key")
            .into_native_reference()
    }
}
impl Drop for UnownedJimKey {
    fn drop(&mut self) {
        if let Some(value) = &self.0 {
            value.retire_unowned_native_header();
        }
    }
}

impl Vm {
    pub(super) fn initialize_original_jim_host_keys(
        &mut self,
    ) -> Result<(), NativeCommandLookupUnavailable> {
        if !self.uses_native_jim_lookup() {
            return Ok(());
        }
        let publications: Vec<_> = self
            .name_world
            .borrow()
            .command_api_publications
            .iter()
            .map(|(slot, (token, bytes))| (slot.clone(), *token, bytes.clone()))
            .collect();
        for (slot, token, original) in publications {
            // A C namespace slot is not a Jim flat table publication.
            if slot.namespace != ROOT_NS {
                continue;
            }
            let Some(storage) = self.command_storage_key_at_slot(&slot) else {
                continue;
            };
            if self.visible_command_generation(&storage) != Some(token)
                || self
                    .name_world
                    .borrow()
                    .jim_command_original_keys
                    .contains_key(&slot)
            {
                continue;
            }
            let Some(key) = self.jim_command_table_key_for_original(
                &original,
                NativeNamePurpose::CommandCApiPublication,
            )?
            else {
                return Err(NativeCommandLookupUnavailable::ProtocolUnavailable);
            };
            if key.table.comparison_bytes() != slot.simple.as_bytes()
                || self.visible_command_generation(&storage) != Some(token)
            {
                return Err(NativeCommandLookupUnavailable::ProtocolUnavailable);
            }
            let mut world = self.name_world.borrow_mut();
            world
                .jim_command_table_keys
                .insert(storage, key.table.clone());
            world.jim_command_original_keys.insert(slot, key);
        }
        Ok(())
    }

    pub(crate) fn original_jim_publication_key(
        &mut self,
        original: &Value,
        purpose: NativeNamePurpose,
    ) -> Result<Option<OriginalJimCommandKey>, CmdError> {
        if !self.uses_native_jim_lookup() {
            return Ok(None);
        }
        let scope = NativeOperationScope::enter(self)?;
        let entered = OriginalLookup::enter(self, self.current_ns_id(), original)?;
        let namespace = if matches!(
            purpose,
            NativeNamePurpose::CommandPublication
                | NativeNamePurpose::RenameSource
                | NativeNamePurpose::RenameDestination
        ) {
            entered.read_jim_namespace(self)?
        } else {
            std::rc::Rc::<[u8]>::from([])
        };
        let incoming = if namespace.is_empty() {
            std::rc::Rc::<[u8]>::from([])
        } else {
            entered.read(self)?
        };
        let path = self.ns_path(self.current_ns_id());
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim publication key issuer",
            ))?
            .recipe();
        let construction = policy
            .jim_command_key_construction(
                NativeNameContext::with_jim_namespace(&path, &namespace),
                &incoming,
                purpose,
            )
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim publication key purpose")
            })?;
        use tcl_syntax::naming::NativeJimNamespaceConstruction;
        let key = match construction {
            NativeJimNamespaceConstruction::RetainOriginal => original.clone(),
            NativeJimNamespaceConstruction::DuplicateNamespaceAndAppend => {
                let context = self.native_jim_object_context()?;
                let numeric = OriginalNumericOps::in_original_scope(self, &scope)?;
                let namespace = entered.jim_namespace_original().ok_or(
                    ValueError::CommandProtocolUnavailable("original Jim key namespace header"),
                )?;
                let protocol = tcl_syntax::native_string::NativeStringProtocol::Jim084;
                let duplicate = UnownedJimKey(Some(
                    namespace
                        .duplicate_native_object_checked(
                            protocol,
                            &mut || numeric.current_for_value(namespace),
                            &mut |value| numeric.string(value, protocol),
                            &mut |error| numeric.retain_failure(error),
                        )?
                        .into_native_unowned_lifetime(),
                ));
                let append = numeric
                    .dialect()
                    .native_object_append_protocol(None)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original Jim key CString append issuer",
                    ))?;
                let mut check = || {
                    for value in [namespace, original, duplicate.value()] {
                        if let Err(error) = numeric.current_for_value(value) {
                            numeric.retain_failure(&error);
                            return Err(OriginalNumericOps::value_unwind());
                        }
                    }
                    Ok(())
                };
                for part in [b"::".as_slice(), tcl_core_types::c_string_extent(&incoming)] {
                    let reached = tcl_cmd_core::native_append::append_unshared_jim_cstring(
                        &crate::value::VmAppendObjects,
                        append.recipe(),
                        duplicate.value(),
                        part,
                        &mut check,
                    );
                    if let Err(error) = reached {
                        let error = CmdError::from(error);
                        numeric.retain_failure(&error);
                        return Err(error);
                    }
                }
                check()?;
                duplicate.value().bind_native_jim_context(&context)?;
                check()?;
                duplicate.promote()
            }
            NativeJimNamespaceConstruction::FreshString => {
                let incoming = entered.read(self)?;
                let created =
                    Value::from_native_string_bytes(tcl_core_types::c_string_extent(&incoming));
                created.bind_native_jim_context(&self.native_jim_object_context()?)?;
                created
            }
        };
        entered.check(self)?;
        let bytes = self.native_name_operand_bytes_checked(&key)?;
        entered.check(self)?;
        let projected = policy
            .command_publication_input(NativeNameContext::root(), &bytes)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim command hash-key geometry")
            })?;
        let table = NativeJimCommandTableKey::from_projection(&projected).ok_or(
            ValueError::CommandProtocolUnavailable("original Jim command table key"),
        )?;
        Ok(Some(OriginalJimCommandKey {
            original: key,
            table,
        }))
    }
    /// A byte API constructs its actual input object before qualification.
    /// This cannot replace the header supplied by an object-vector caller.
    pub(crate) fn jim_command_table_key_for_original(
        &mut self,
        original: &[u8],
        purpose: NativeNamePurpose,
    ) -> Result<Option<OriginalJimCommandKey>, NativeCommandLookupUnavailable> {
        if !self.uses_native_jim_lookup() {
            return Ok(None);
        }
        let prepared = (|| {
            let scope = NativeOperationScope::enter(self)?;
            if matches!(
                purpose,
                NativeNamePurpose::CommandCApiPublication
                    | NativeNamePurpose::ChildAliasPublication
            ) {
                let policy = self
                    .name_policy_protocol()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original Jim CString registration issuer",
                    ))?
                    .recipe();
                if policy.jim_command_key_construction(NativeNameContext::root(), original, purpose)
                    != Ok(tcl_syntax::naming::NativeJimNamespaceConstruction::FreshString)
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original Jim CString registration purpose",
                    )
                    .into());
                }
                let created =
                    Value::from_native_string_bytes(tcl_core_types::c_string_extent(original));
                created.bind_native_jim_context(&self.native_jim_object_context()?)?;
                scope.check(self)?;
                let bytes = self.native_name_operand_bytes_checked(&created)?;
                scope.check(self)?;
                let projected = policy
                    .command_publication_input(NativeNameContext::root(), &bytes)
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable(
                            "original Jim CString table geometry",
                        )
                    })?;
                let table = NativeJimCommandTableKey::from_projection(&projected).ok_or(
                    ValueError::CommandProtocolUnavailable("original Jim CString table key"),
                )?;
                return Ok(Some(OriginalJimCommandKey {
                    original: created,
                    table,
                }));
            }
            let created = Value::from_native_string_bytes(original);
            self.original_jim_publication_key(&created, purpose)
        })();
        match prepared {
            Ok(key) => Ok(key),
            Err(error) => {
                super::oo_cleanup::retain_error(self, &error);
                Err(NativeCommandLookupUnavailable::ProtocolUnavailable)
            }
        }
    }

    pub(super) fn jim_command_table_key_for_slot(
        &mut self,
        slot: &CommandSlot,
        incoming: Option<OriginalJimCommandKey>,
    ) -> Option<OriginalJimCommandKey> {
        if !self.uses_native_jim_lookup() {
            return None;
        }
        if slot.namespace != ROOT_NS {
            return None;
        }
        let incoming = incoming.or_else(|| {
            self.jim_command_table_key_for_original(
                slot.simple.as_bytes(),
                NativeNamePurpose::CommandCApiPublication,
            )
            .ok()
            .flatten()
        })?;
        assert_eq!(
            incoming.table.comparison_bytes(),
            slot.simple.as_bytes(),
            "actual Jim table comparison slot"
        );
        Some(incoming)
    }

    fn original_jim_inventory_keys(
        &self,
        kind: tcl_runtime_api::NativeJimCommandInventoryKind,
    ) -> Result<Vec<OriginalJimInventoryKey>, CmdError> {
        use tcl_runtime_api::NativeJimCommandInventoryKind as Kind;
        let world = self.name_world.borrow();
        let mut keys = Vec::new();
        for (storage, command) in &self.commands {
            if !self.builtin_command_visible_for_surface(storage, command)
                || !match kind {
                    Kind::Commands => true,
                    Kind::Procs => matches!(command, Command::Proc(_)),
                    Kind::Aliases => matches!(command, Command::CallerAlias(_)),
                }
            {
                continue;
            }
            let slot = self
                .command_slot(storage)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original Jim inventory command slot",
                ))?;
            if slot.namespace != ROOT_NS {
                continue;
            }
            let generation = self.visible_command_generation(storage).ok_or(
                ValueError::CommandProtocolUnavailable("original Jim inventory command generation"),
            )?;
            let original = world.jim_command_original_keys.get(&slot).ok_or(
                ValueError::CommandProtocolUnavailable("original Jim inventory actual table key"),
            )?;
            let table = world.jim_command_table_keys.get(storage).ok_or(
                ValueError::CommandProtocolUnavailable("original Jim inventory table geometry"),
            )?;
            if original.table.comparison_bytes() != slot.simple.as_bytes()
                || table.report_bytes() != original.table.report_bytes()
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim inventory key geometry",
                )
                .into());
            }
            original.original.check_native_header()?;
            keys.push(OriginalJimInventoryKey {
                slot,
                storage: storage.clone(),
                generation,
                original: original.original.native_lifetime_lease(),
                report: table.report_bytes().to_vec(),
            });
        }
        Ok(keys)
    }

    pub(super) fn original_jim_command_inventory_result(
        &mut self,
        root: NsId,
        pattern: Option<&[u8]>,
        include_spaces: bool,
        kind: tcl_runtime_api::NativeJimCommandInventoryKind,
    ) -> Result<Value, CmdError> {
        use super::native_name_consumers::OriginalNameConsumerScope;
        if !self.uses_native_jim_lookup() || root != ROOT_NS {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim command inventory selected table",
            )
            .into());
        }
        let scope = OriginalNameConsumerScope::enter_for_namespace(self, Some(root))?;
        let revision = self.compilation_epochs.cmd_epoch.get();
        let keys = self.original_jim_inventory_keys(kind)?;
        let reports: Vec<_> = keys.iter().map(|key| key.report.clone()).collect();
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim command inventory naming issuer",
            ))?;
        let reached = policy
            .recipe()
            .jim_core_command_reached_ordinals(pattern, &reports)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim inventory table selection")
            })?;
        let keys: Vec<_> = reached.into_iter().map(|index| &keys[index]).collect();
        let mut candidates = Vec::with_capacity(keys.len());
        for key in &keys {
            let current = |vm: &mut Vm| {
                let unchanged = vm.compilation_epochs.cmd_epoch.get() == revision;
                scope.require(vm, unchanged, "original Jim inventory table revision")?;
                key.check(vm, &scope)
            };
            current(self)?;
            let first = if include_spaces {
                key.report.clone()
            } else {
                let bytes =
                    self.native_name_operand_bytes_checked_using(key.original.value(), &current);
                current(self)?;
                bytes?.to_vec()
            };
            let admitted = policy
                .recipe()
                .jim_core_command_candidate_admitted(&first, include_spaces)
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original Jim inventory key admission")
                })?;
            if pattern.is_some() && admitted {
                let held = key.original.value().clone();
                let bytes = self.native_name_operand_bytes_checked_using(&held, &current);
                drop(held);
                current(self)?;
                candidates.push(bytes?.to_vec());
            } else {
                candidates.push(first);
            }
        }
        let selected = policy
            .recipe()
            .jim_core_command_ordinals(pattern, &candidates, include_spaces)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable(
                    "original Jim command inventory native match",
                )
            })?;
        let unchanged = self.compilation_epochs.cmd_epoch.get() == revision;
        scope.require(self, unchanged, "original Jim inventory table revision")?;
        let mut members = Vec::with_capacity(selected.len());
        for &index in &selected {
            let key = &keys[index];
            key.check(self, &scope)?;
            members.push(key.original.value().clone());
        }
        let result = tcl_syntax::value::ValueOps::new_list(self, members);
        result.bind_native_jim_context(&self.native_jim_object_context()?)?;
        scope.current_for_value(self, &result)?;
        let unchanged = self.compilation_epochs.cmd_epoch.get() == revision;
        scope.require(self, unchanged, "original Jim inventory table revision")?;
        for index in selected {
            keys[index].check(self, &scope)?;
        }
        scope.finish(self)?;
        Ok(result)
    }

    pub(super) fn jim_command_table_report_names(
        &self,
        namespace: NsId,
        procs_only: bool,
    ) -> Vec<Vec<u8>> {
        self.jim_command_table_report_names_filtered(namespace, |command| {
            !procs_only || matches!(command, Command::Proc(_))
        })
    }

    pub(super) fn jim_command_table_report_names_filtered(
        &self,
        namespace: NsId,
        selected_kind: impl Fn(&Command) -> bool,
    ) -> Vec<Vec<u8>> {
        if namespace != ROOT_NS {
            return Vec::new();
        }
        let world = self.name_world.borrow();
        self.commands
            .iter()
            .filter(|(key, command)| self.builtin_command_visible_for_surface(key, command))
            .filter(|(_, command)| selected_kind(command))
            .filter_map(|(key, _)| {
                self.command_slot(key)
                    .filter(|slot| slot.namespace == ROOT_NS)?;
                world
                    .jim_command_table_keys
                    .get(key)
                    .map(|key| key.report_bytes().to_vec())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory_fixture() -> Vm {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jim"));
        assert_eq!(
            vm.eval_source("proc inventoryA {} {}; proc inventoryB {} {}")
                .unwrap()
                .code,
            tcl_runtime_api::Code::Ok
        );
        vm
    }

    #[test]
    fn command_inventory_list_owns_the_actual_keys_and_all_without_pattern_is_lazy() {
        use tcl_runtime_api::NativeJimCommandInventoryKind as Kind;
        let mut vm = inventory_fixture();
        let keys = vm.original_jim_inventory_keys(Kind::Procs).unwrap();
        let key = keys.iter().find(|key| key.report == b"inventoryA").unwrap();
        let count = key.original.value().native_object_reference_count();
        key.original.value().invalidate_native_string_for_test();
        assert!(key.original.value().resident_string_bytes().is_none());
        let result = tcl_runtime_api::Namespaces::native_jim_command_inventory_result(
            &mut vm,
            ROOT_NS,
            None,
            true,
            Kind::Procs,
        )
        .unwrap();
        assert_eq!(result.native_object_type_name(), "list");
        let members = result
            .native_object_list_elements(tcl_syntax::native_string::NativeStringProtocol::Jim084)
            .unwrap();
        assert!(
            members
                .iter()
                .any(|member| member.is_same_object(key.original.value()))
        );
        assert!(key.original.value().resident_string_bytes().is_none());
        assert_eq!(
            key.original.value().native_object_reference_count(),
            count + 1
        );
        drop(members);
        drop(result);
        assert_eq!(key.original.value().native_object_reference_count(), count);
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn exact_inventory_pattern_does_not_read_an_unselected_original_key() {
        use tcl_runtime_api::NativeJimCommandInventoryKind as Kind;
        let mut vm = inventory_fixture();
        let keys = vm.original_jim_inventory_keys(Kind::Procs).unwrap();
        let expected = keys.iter().find(|key| key.report == b"inventoryA").unwrap();
        let unread = keys.iter().find(|key| key.report == b"inventoryB").unwrap();
        unread.original.value().invalidate_native_string_for_test();
        let pattern = Value::new_native_string_bytes(b"inventoryA".as_slice());
        let result =
            tcl_cmd_core::info::jim_core_command_list(&mut vm, Some(&pattern), true, Kind::Procs)
                .unwrap();
        let members = result
            .native_object_list_elements(tcl_syntax::native_string::NativeStringProtocol::Jim084)
            .unwrap();
        assert_eq!(members.len(), 1);
        assert!(members[0].is_same_object(expected.original.value()));
        assert!(unread.original.value().resident_string_bytes().is_none());
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn inventory_receipt_rejects_a_replacement_that_keeps_the_same_table_key() {
        use tcl_runtime_api::NativeJimCommandInventoryKind as Kind;
        let mut vm = inventory_fixture();
        let keys = vm.original_jim_inventory_keys(Kind::Procs).unwrap();
        let before = keys.iter().find(|key| key.report == b"inventoryA").unwrap();
        let scope =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
        before.check(&mut vm, &scope).unwrap();
        assert_eq!(
            vm.eval_source("proc inventoryA {} {return REPLACED}")
                .unwrap()
                .code,
            tcl_runtime_api::Code::Ok
        );
        let after = vm.original_jim_inventory_keys(Kind::Procs).unwrap();
        let after = after
            .iter()
            .find(|key| key.report == b"inventoryA")
            .unwrap();
        assert!(
            before
                .original
                .value()
                .is_same_object(after.original.value())
        );
        assert_ne!(before.generation, after.generation);
        assert!(before.check(&mut vm, &scope).is_err());
        assert!(vm.execution_refusal.is_some());
    }

    #[test]
    fn actual_cstring_registration_uses_one_birth_and_retains_rooted_input() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
        let _scope = NativeOperationScope::enter(&mut vm).unwrap();
        for purpose in [
            NativeNamePurpose::CommandCApiPublication,
            NativeNamePurpose::ChildAliasPublication,
        ] {
            let reusable = Value::new_native_string_bytes(b"REUSABLE".as_slice());
            let observed = crate::value::JimCommandNameObservation::capture(&reusable).unwrap();
            drop(reusable);
            assert!(observed.current().is_err());
            let key = vm
                .jim_command_table_key_for_original(b"::host\0ignored", purpose)
                .unwrap()
                .unwrap();
            let reached = observed.current().unwrap();
            assert!(reached.is_same_object(&key.original));
            assert_eq!(
                key.original.resident_string_bytes().unwrap().as_ref(),
                b"::host"
            );
            assert_eq!(key.table.comparison_bytes(), b"host");
            drop(reached);
            assert_eq!(key.original.native_object_reference_count(), 1);
        }
    }
}
