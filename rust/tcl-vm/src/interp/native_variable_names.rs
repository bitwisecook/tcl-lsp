// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original C variable-name lookups and authentic compiled-local owners.

#[path = "native_variable_names/native_array.rs"]
mod native_array;
#[path = "native_variable_names/native_exists.rs"]
mod native_exists;
#[path = "native_variable_names/native_tcloo.rs"]
mod native_tcloo;
use super::{HashSet, ResolvedVar, Value, VarBinding, VarTableOwner, Vm};
use crate::command::completion_from_cmd_error;
use crate::value::NativeObjectLifetimeLease;
use std::rc::Rc;
use tcl_runtime_api::Completion;
use tcl_syntax::native_variable_name::{
    NativeParsedVariableElement, NativeParsedVariableName, NativeVariableNameLookupPurpose,
};

mod links;
mod owner_installation;
mod value_access;

type CachedOriginalVariableParts = Option<(NativeObjectLifetimeLease, Rc<[u8]>)>;

type PreparedOriginalVariable = (
    crate::value::NativeObjectLifetimeLease,
    Rc<[u8]>,
    Option<Rc<[u8]>>,
    ResolvedVar,
);

enum OriginalVariablePreparationError {
    Lookup(Completion<Value>),
    Confined(Completion<Value>),
}

impl From<Completion<Value>> for OriginalVariablePreparationError {
    fn from(error: Completion<Value>) -> Self {
        Self::Lookup(error)
    }
}

impl OriginalVariablePreparationError {
    fn into_completion(self) -> Completion<Value> {
        match self {
            Self::Lookup(error) | Self::Confined(error) => error,
        }
    }
}

#[derive(Clone, Copy)]
struct OriginalVariableLookup {
    protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
    namespace: Option<tcl_core_types::NsId>,
    purpose: NativeVariableNameLookupPurpose,
}

impl Vm {
    /// Delegate the captured actual array target to its original-key owner.
    /// This creates neither a key header nor a native reference.
    pub(crate) fn original_array_key_at(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
        key: &[u8],
    ) -> Result<NativeObjectLifetimeLease, tcl_syntax::value::ValueError> {
        let array =
            target
                .cell_id()
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "array for actual receiver",
                ))?;
        self.var_arena
            .original_array_key(array, &tcl_core_types::NameBytes::from(key))
    }

    pub(crate) fn native_c_variable_name_protocol(
        &self,
    ) -> Option<tcl_syntax::native_variable_name::NativeVariableNameProtocol> {
        let selected = self.execution_name_policy()?.native_recipe()?;
        let protocol = self
            .actual_native_invocation_dialect()
            .native_variable_name_protocol()?;
        (selected.authority() == tcl_syntax::naming::NamePolicyAuthority::Native
            && selected.recipe() == tcl_syntax::naming::NativeNameProtocol::C(protocol.version()))
        .then_some(protocol)
    }

    /// The C variable-name recipe of the authored simulation's own policy.
    pub(crate) fn authored_c_variable_name_protocol(
        &self,
    ) -> Option<tcl_syntax::native_variable_name::NativeVariableNameProtocol> {
        let selected = self.execution_name_policy()?.native_recipe()?;
        if selected.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation {
            return None;
        }
        let tcl_syntax::naming::NativeNameProtocol::C(version) = selected.recipe() else {
            return None;
        };
        tcl_native_model::InvocationDialect::for_version(version).native_variable_name_protocol()
    }

    /// The authored simulation's C variable names: an authored recipe with no
    /// native authority, kept by the byte-name owners rather than the
    /// physical engine's object-name getter.
    pub(crate) fn authored_c_variable_names(&self) -> bool {
        self.native_c_variable_name_protocol().is_none()
            && self
                .execution_name_policy()
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                .is_some_and(|selected| {
                    selected.authority()
                        == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                        && matches!(
                            selected.recipe(),
                            tcl_syntax::naming::NativeNameProtocol::C(_)
                        )
                })
    }

    /// Prepare the original name before any selected receiver observers run.
    fn prepare_native_original_variable(
        &mut self,
        original: &Value,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<PreparedOriginalVariable, Completion<Value>> {
        self.prepare_native_original_variable_in(original, purpose, None)
    }

    fn prepare_native_original_variable_in(
        &mut self,
        original: &Value,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<tcl_core_types::NsId>,
    ) -> Result<PreparedOriginalVariable, Completion<Value>> {
        self.prepare_native_original_variable_parts(original, purpose, namespace, None)
    }

    /// Explicit part2 keeps the original part1 scalar name, as requested by
    /// the native array opcode's `TCL_PART1_NOT_PARSED` lookup flag.
    fn cache_native_original_array_name(
        &mut self,
        original: &Value,
        protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
        dialect: tcl_native_model::InvocationDialect,
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<bool, Completion<Value>> {
        let original_bytes = original_names
            .read(self, original)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        let Some((root, element)) = protocol.parsed_array_parts(&original_bytes) else {
            return Ok(false);
        };
        let reached: Result<(), tcl_cmd_core::CmdError> = if let Some(bytes) =
            protocol.new_element_bytes(element)
        {
            original.install_native_parsed_variable_checked(
                dialect,
                || {
                    Ok(NativeParsedVariableName {
                        protocol,
                        array: Some((
                            Value::new_native_string_bytes(root),
                            NativeParsedVariableElement::Bytes(bytes),
                        )),
                    })
                },
                &mut || original_names.current_for_value(self, original),
            )
        } else {
            // The C9 ParsedSet macro prepares both actual incoming member roles
            // before the old primary's free callback, then installs that tuple.
            let cache = NativeParsedVariableName {
                protocol,
                array: Some((
                    Value::new_native_string_bytes(root),
                    NativeParsedVariableElement::Object(Value::new_native_string_bytes(element)),
                )),
            };
            original.install_native_parsed_variable_checked(dialect, || Ok(cache), &mut || {
                original_names.current_for_value(self, original)
            })
        };
        original_names
            .check(self)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        reached.map_err(|error| completion_from_cmd_error(self, error))?;
        Ok(true)
    }

    fn select_native_original_root_binding(
        &mut self,
        root: &Value,
        reported: (&[u8], Option<&[u8]>),
        cached: Option<VarBinding>,
        selection: OriginalVariableLookup,
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<VarBinding, Completion<Value>> {
        let OriginalVariableLookup {
            protocol,
            namespace,
            purpose,
        } = selection;
        let (root_bytes, element) = reported;
        if cached.is_none()
            && protocol.retires_before_simple_lookup(root.native_primary_has_free_hook())
        {
            root.retire_native_variable_primary_checked(&mut || {
                original_names.current_for_value(self, root)
            })
            .map_err(|error| completion_from_cmd_error(self, error))?;
        }
        let binding = cached.or_else(|| {
            let selected = tcl_syntax::naming::NativeNameProtocol::C(protocol.version())
                .variable_root_input(root_bytes);
            if let Some(namespace) = namespace {
                self.namespace_var_binding_from_token_bytes(namespace, selected.selected())
            } else {
                self.var_binding_from_bytes(selected.selected(), self.current_level())
            }
        });
        let Some(mut binding) = binding else {
            if !purpose.leaves_error_message() {
                return Err(super::err(""));
            }

            // A lookup that would not create the variable reports it missing.
            return Err(self.variable_access_error_input(
                purpose.diagnostic_verb(),
                tcl_syntax::naming::NativeVariableInputForm::Separate {
                    root: root_bytes,
                    element,
                },
                if purpose.creates_entries() {
                    "parent namespace doesn't exist"
                } else {
                    "no such variable"
                },
            ));
        };
        if let VarTableOwner::Frame(level) = binding.owner
            && let Some(slot) = self.compiled_local_dynamic_slot(level, binding.name.as_bytes())
        {
            binding.owner = VarTableOwner::CompiledLocal { level, slot };
        }
        Ok(binding)
    }

    fn ensure_native_original_root_entry(
        &mut self,
        root: &Value,
        reported: (&[u8], Option<&[u8]>),
        binding: &VarBinding,
        protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
        purpose: NativeVariableNameLookupPurpose,
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<ResolvedVar, Completion<Value>> {
        let (root_bytes, element) = reported;
        let create = purpose.creates_entries();
        let hash_birth = self.raw_variable_binding(binding).is_none()
            && !matches!(binding.owner, VarTableOwner::CompiledLocal { .. });
        let mut resolved = self
            .resolve_binding_var(binding.clone(), None, &mut HashSet::new())
            .ok_or_else(|| {
                self.refuse_host_command("actual original variable receiver unavailable".into())
            })?;
        if create && resolved.id.is_none() {
            resolved.id = self.ensure_target_var_at_binding(&resolved.binding, None);
            resolved.base_id = resolved.id;
        }
        if create
            && hash_birth
            && resolved.id.is_some()
            && protocol.version() >= tcl_dialect::TclVersion::V8_5
        {
            let key = if binding.name.as_bytes() == root_bytes {
                root.clone()
            } else {
                Value::new_native_string_bytes(binding.name.as_bytes())
            };
            if let Some(table) = self.var_table_mut(binding.owner) {
                table.retain_original_jim_key(&binding.name, key);
            }
        }
        if create
            && hash_birth
            && let Some(id) = resolved.id
        {
            self.var_arena.record_native_entry(id, binding.clone());
        }
        // Only a successful simple-variable lookup converts the original root.
        if resolved.id.is_some() {
            self.install_selected_original_variable_cache(root, binding, original_names)?;
        }
        if create
            && !purpose.creates_element_entries()
            && element.is_some()
            && let Some(base) = resolved.id
            && matches!(
                self.var_arena.get(base).map(crate::vars::VarCell::state),
                Some(super::Local::Undefined)
            )
        {
            self.var_arena
                .replace_state(base, super::Local::Array(super::VarTable::new()));
        }
        Ok(resolved)
    }

    fn cached_original_variable_parts(
        &mut self,
        original: &Value,
        protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
        explicit_element: Option<&[u8]>,
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<CachedOriginalVariableParts, Completion<Value>> {
        if explicit_element.is_some() {
            return Ok(None);
        }
        // Only allocation leases and Rust bytes leave the short primary borrow.
        let parts = original
            .with_native_parsed_variable(|cache| {
                if cache.protocol != protocol {
                    return None;
                }
                cache.array.as_ref().map(|(root, element)| {
                    let element = match element {
                        NativeParsedVariableElement::Bytes(bytes) => Ok(bytes.clone()),
                        NativeParsedVariableElement::Object(original) => {
                            Err(original.native_lifetime_lease())
                        }
                    };
                    (root.native_lifetime_lease(), element)
                })
            })
            .flatten();
        let Some((root, element)) = parts else {
            return Ok(None);
        };
        let bytes = match &element {
            Ok(bytes) => Rc::<[u8]>::from(tcl_core_types::c_string_extent(bytes)),
            Err(element) => original_names
                .read(self, element.value())
                .map_err(|error| completion_from_cmd_error(self, error))?,
        };
        original_names
            .current_for_value(self, original)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        original_names
            .current_for_value(self, root.value())
            .map_err(|error| completion_from_cmd_error(self, error))?;
        let current = original
            .with_native_parsed_variable(|cache| {
                cache.protocol == protocol
                    && cache
                        .array
                        .as_ref()
                        .is_some_and(|(actual_root, actual_element)| {
                            actual_root.is_same_object(root.value())
                                && match (actual_element, &element) {
                                    (NativeParsedVariableElement::Bytes(actual), Ok(saved)) => {
                                        actual.as_ref() == saved.as_ref()
                                    }
                                    (NativeParsedVariableElement::Object(actual), Err(saved)) => {
                                        actual.is_same_object(saved.value())
                                    }
                                    _ => false,
                                }
                        })
            })
            .unwrap_or(false);
        original_names
            .require(self, current, "original parsed variable member owner")
            .map_err(|error| completion_from_cmd_error(self, error))?;
        drop(element);
        original_names
            .check(self)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        Ok(Some((root, bytes)))
    }

    fn confine_original_variable_binding(
        &self,
        root_bytes: &[u8],
        binding: &VarBinding,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<(), OriginalVariablePreparationError> {
        if matches!(
            purpose,
            NativeVariableNameLookupPurpose::Write
                | NativeVariableNameLookupPurpose::QuietWrite
                | NativeVariableNameLookupPurpose::IncrementRead
                | NativeVariableNameLookupPurpose::IncrementExisting
                | NativeVariableNameLookupPurpose::ArrayMake
                | NativeVariableNameLookupPurpose::Unset
                | NativeVariableNameLookupPurpose::QuietUnset
        ) {
            self.confine_store_binding_bytes(
                root_bytes,
                binding,
                self.current_level(),
                matches!(
                    purpose,
                    NativeVariableNameLookupPurpose::Unset
                        | NativeVariableNameLookupPurpose::QuietUnset
                ),
            )
            .map_err(OriginalVariablePreparationError::Confined)?;
        }
        Ok(())
    }

    fn prepare_native_original_variable_parts_entered(
        &mut self,
        original: &Value,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<tcl_core_types::NsId>,
        explicit_element: Option<&[u8]>,
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<PreparedOriginalVariable, OriginalVariablePreparationError> {
        self.retire_released_native_alias_entries();
        let dialect = self.actual_native_invocation_dialect();
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C variable issuer");
        if !protocol.cache_precedes_string_getter() {
            original_names
                .read(self, original)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        }
        let mut parts = self.cached_original_variable_parts(
            original,
            protocol,
            explicit_element,
            original_names,
        )?;
        let parsed_scalar = original
            .with_native_parsed_variable(|cache| {
                cache.protocol == protocol && cache.array.is_none()
            })
            .unwrap_or(false);
        if parts.is_none()
            && explicit_element.is_none()
            && !parsed_scalar
            && original.with_native_local_variable(|_| ()).is_none()
            && self.cache_native_original_array_name(original, protocol, dialect, original_names)?
        {
            return self.prepare_native_original_variable_parts_with_disposition(
                original, purpose, namespace, None,
            );
        }
        let (root, element) = parts.take().map_or_else(
            || {
                (
                    original.native_lifetime_lease(),
                    explicit_element.map(Rc::<[u8]>::from),
                )
            },
            |(root, element)| (root, Some(element)),
        );
        let cached = namespace
            .is_none()
            .then(|| self.native_original_local_binding(root.value()))
            .flatten();
        let root_bytes: Rc<[u8]> = match &cached {
            Some(binding) if protocol.cache_precedes_string_getter() => {
                Rc::from(binding.name.as_bytes())
            }
            _ => original_names
                .read(self, root.value())
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?,
        };
        let binding = self.select_native_original_root_binding(
            root.value(),
            (&root_bytes, element.as_deref()),
            cached,
            OriginalVariableLookup {
                protocol,
                namespace,
                purpose,
            },
            original_names,
        )?;
        self.confine_original_variable_binding(&root_bytes, &binding, purpose)?;
        let resolved = self.ensure_native_original_root_entry(
            root.value(),
            (&root_bytes, element.as_deref()),
            &binding,
            protocol,
            purpose,
            original_names,
        )?;
        let resolved = if let Some(element) = &element {
            let selected = self
                .name_policy_protocol()
                .expect("selected native names")
                .recipe()
                .separate_variable_input(&root_bytes, Some(element));
            self.resolve_binding_var(
                resolved.binding,
                selected
                    .element()
                    .map(|part| tcl_core_types::NameBytes::from(part.selected())),
                &mut HashSet::new(),
            )
            .ok_or_else(|| {
                self.refuse_host_command("actual original array receiver unavailable".into())
            })?
        } else {
            resolved
        };
        Ok((root, root_bytes, element, resolved))
    }

    fn prepare_native_original_variable_parts(
        &mut self,
        original: &Value,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<tcl_core_types::NsId>,
        explicit_element: Option<&[u8]>,
    ) -> Result<PreparedOriginalVariable, Completion<Value>> {
        self.prepare_native_original_variable_parts_with_disposition(
            original,
            purpose,
            namespace,
            explicit_element,
        )
        .map_err(OriginalVariablePreparationError::into_completion)
    }

    fn prepare_native_original_variable_parts_with_disposition(
        &mut self,
        original: &Value,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<tcl_core_types::NsId>,
        explicit_element: Option<&[u8]>,
    ) -> Result<PreparedOriginalVariable, OriginalVariablePreparationError> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
                self, namespace,
            )
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = self.prepare_native_original_variable_parts_entered(
            original,
            purpose,
            namespace,
            explicit_element,
            &original_names,
        );
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    fn original_c_parsed_element_key(original: &Value) -> Option<Value> {
        original
            .with_native_parsed_variable(|cache| {
                let (_, element) = cache.array.as_ref()?;
                match cache.protocol.parsed_element_table_key(element)? {
                    tcl_syntax::native_variable_name::NativeElementTableKey::FreshString(bytes) => {
                        Some(Value::new_native_string_bytes(bytes.to_vec()))
                    }
                    tcl_syntax::native_variable_name::NativeElementTableKey::Original(original) => {
                        Some(original.clone())
                    }
                }
            })
            .flatten()
    }

    pub(super) fn unset_original_c_variable(
        &mut self,
        original: &Value,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| match self.prepare_native_original_variable_parts_with_disposition(
            original,
            Self::original_unset_purpose(complain),
            None,
            None,
        ) {
            Ok((_root, root_bytes, element, resolved)) => {
                let observes = self
                    .trace_cell_from_resolved(&resolved)
                    .is_some_and(|cell| {
                        self.original_variable_trace_requires_name(
                            &cell,
                            element.is_some(),
                            "unset",
                        )
                    });
                let bytes = if element.is_some() || observes {
                    original_names
                        .read(self, original)
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
                } else {
                    root_bytes
                };
                let result =
                    self.unset_selected_original_variable(&bytes, None, Some(&resolved), complain);
                if result.is_err() {
                    original_names
                        .read(self, original)
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                }
                result
            }
            Err(OriginalVariablePreparationError::Confined(error)) => Err(error),
            Err(OriginalVariablePreparationError::Lookup(error))
                if self.refused_completion().is_some() || complain =>
            {
                Err(error)
            }
            Err(_) => Ok(()),
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    #[cfg(test)]
    pub(crate) fn unset_original_c_variable_parts(
        &mut self,
        original: &Value,
        element: &Value,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        self.unset_original_c_variable_parts_reporting(original, element, complain, true)
    }

    /// Unset `original(element)` held as two parts. A compiled command
    /// reports `original` to unset traces as name1, as C's `unsetArrayStk`
    /// does; a directly evaluated one (`reports_parts` false) reports the
    /// joined spelling its generic `unset` hands the lookup.
    pub(crate) fn unset_original_c_variable_parts_reporting(
        &mut self,
        original: &Value,
        element: &Value,
        complain: bool,
        reports_parts: bool,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let element_bytes = original_names
                .read(self, element)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let selected = self.prepare_native_original_variable_parts_with_disposition(
                original,
                Self::original_unset_purpose(complain),
                None,
                Some(&element_bytes),
            );
            match selected {
                Ok((_root, root_bytes, _element, resolved)) => {
                    // These bytes present the diagnostic/trace spelling only. The
                    // receiver remains the original part1 cache and selected cell.
                    let mut spelling = root_bytes.to_vec();
                    spelling.push(b'(');
                    spelling.extend_from_slice(&element_bytes);
                    spelling.push(b')');
                    self.unset_selected_original_variable(
                        &spelling,
                        reports_parts.then_some(&root_bytes[..]),
                        Some(&resolved),
                        complain,
                    )
                }
                Err(OriginalVariablePreparationError::Confined(error)) => Err(error),
                Err(OriginalVariablePreparationError::Lookup(error))
                    if self.refused_completion().is_some() || complain =>
                {
                    Err(error)
                }
                Err(_) => Ok(()),
            }
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    fn original_unset_purpose(complain: bool) -> NativeVariableNameLookupPurpose {
        if complain {
            NativeVariableNameLookupPurpose::Unset
        } else {
            NativeVariableNameLookupPurpose::QuietUnset
        }
    }

    pub(super) fn retire_released_native_alias_entries(&mut self) {
        for (binding, id) in self.var_arena.take_released_native_alias_entries() {
            let discard = self.var_arena.get(id).is_some_and(|cell| {
                matches!(cell.state(), crate::vars::VarState::Undefined)
                    && !cell.namespace_declared()
            }) && !self.var_arena.has_link_refs(id)
                && !self.var_arena.has_operation_refs(id)
                && !self.variable_observers.script_traces.contains_key(&id);
            if !discard {
                continue;
            }
            let removed = self.var_table_mut(binding.owner).and_then(|table| {
                (table.get(&binding.name) == Some(&id))
                    .then(|| table.remove(&binding.name))
                    .flatten()
            });
            if let Some(raw) = removed {
                self.var_arena.unbind(raw);
            }
        }
    }

    pub(super) fn original_variable_trace_requires_name(
        &self,
        cell: &super::VarTraceCell,
        element_access: bool,
        operation: &str,
    ) -> bool {
        let destroyed = tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::from_name(operation)
            .is_some_and(tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::uses_destroyed_cell_callbacks);
        if !destroyed && cell.id.is_some_and(|id| self.active_traces.contains(&id)) {
            return false;
        }
        let array_active = cell
            .array
            .is_some_and(|id| self.active_traces.contains(&id));
        self.variable_trace_groups(element_access, cell, array_active, None, operation)
            .into_iter()
            .any(|group| {
                let super::TraceGroup::Live(id) = group else {
                    return false;
                };
                self.variable_observers
                    .script_traces
                    .get(&id)
                    .is_some_and(|traces| {
                        traces
                            .iter()
                            .any(|trace| trace.ops.iter().any(|op| op == operation))
                    })
                    || (matches!(operation, "read" | "unset")
                        && self.variable_observers.native_error_cells.contains_key(&id))
                    || (matches!(operation, "read" | "write")
                        && self.variable_observers.precision_cell == Some(id))
            })
    }
}

#[cfg(test)]
mod tests;
