// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim table keys retain publication spelling separately from storage addresses.

use super::native_numeric_ops::OriginalNumericOps;
use super::native_operation_currency::NativeOperationScope;
use super::native_original_lookup::OriginalLookup;
use super::{Command, CommandSlot, NativeCommandLookupUnavailable, NsId, ROOT_NS, Vm};
use crate::Value;
use tcl_cmd_core::CmdError;
use tcl_syntax::naming::{NativeJimCommandTableKey, NativeNameContext, NativeNamePurpose};
use tcl_syntax::value::ValueError;

/// The actual Jim qualification temporary and its independent table geometry.
pub(crate) struct OriginalJimCommandKey {
    pub(super) original: Value,
    pub(super) table: NativeJimCommandTableKey,
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
