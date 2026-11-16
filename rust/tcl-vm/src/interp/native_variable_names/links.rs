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

//! Linking through original C names: namespace variables, `upvar` aliases and compiled upvar links.

use super::super::{ResolvedVar, Value, VarBinding, VarTableOwner, Vm};
use crate::command::completion_from_cmd_error;
use tcl_runtime_api::Completion;
use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose;

impl Vm {
    pub(in super::super) fn link_original_c_namespace_variable(
        &mut self,
        original: &Value,
        namespace: tcl_core_types::NsId,
        local: &VarBinding,
        declare: bool,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
                self,
                Some(namespace),
            )
            .map_err(|error| completion_from_cmd_error(self, error))?;
        let local_cell = original_names
            .capture_binding(self, local)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let (_root, bytes, _element, resolved) = self.prepare_native_original_variable_in(
                original,
                NativeVariableNameLookupPurpose::Link,
                Some(namespace),
            )?;
            let target = self.settle_original_c_link_target(original, &resolved)?;
            if self
                .native_c_variable_name_protocol()
                .is_some_and(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5)
                && let Some((array, key)) = self
                    .var_arena
                    .element_parent(target)
                    .map(|(array, key)| (array, key.clone()))
                && let Some(original_key) = Self::original_c_parsed_element_key(original)
            {
                self.var_arena
                    .retain_original_array_key(array, &key, original_key);
            }
            if declare {
                self.var_arena.mark_namespace_declared(target);
            }
            local_cell
                .check(self, &original_names)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            let result = self
                .bind_variable_link(local, super::super::VariableLinkTarget::Cell(target))
                .map_err(|error| {
                    let protocol = self
                        .native_c_variable_name_protocol()
                        .expect("actual C alias diagnostic");
                    let failure = crate::command::upvar_link_error_bytes_in(
                        error,
                        &bytes,
                        local.name.as_bytes(),
                        Some(protocol),
                    );
                    self.present_original_c_alias_error(failure, error)
                });
            self.retire_released_native_alias_entries();
            result
        })();
        original_names
            .finish(self)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        reached
    }

    pub(crate) fn define_original_c_namespace_variable(
        &mut self,
        original: &Value,
        value: Option<&Value>,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            // The command obtains its spelling before namespace-only cache lookup.
            let spelling = original_names
                .read(self, original)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let namespace = self.current_ns_id();
            let (_root, bytes, element, resolved) = self.prepare_native_original_variable_in(
                original,
                NativeVariableNameLookupPurpose::Define,
                Some(namespace),
            )?;
            if element.is_some() {
                return Err(self.variable_access_error_input(
                    "define",
                    tcl_syntax::naming::NativeVariableInputForm::Combined(&spelling),
                    "name refers to an element in an array",
                ));
            }
            let target = resolved.id.ok_or_else(|| {
                self.refuse_host_command("namespace declaration cell unavailable".into())
            })?;
            self.var_arena.mark_namespace_declared(target);
            if let Some(value) = value {
                let captured = self.capture_selected_update(&bytes, None, &resolved)?;
                value
                    .check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
                self.with_variable_operation(&captured.cell, |vm| {
                    vm.store_captured_update_with_original(
                        &bytes,
                        None,
                        &captured,
                        value.clone(),
                        Some(original),
                    )
                })?;
            }
            if self.frame_owns_local_variables(self.current_level()) {
                let local = tcl_syntax::naming::variable_local_name_bytes(
                    self.name_policy_protocol()
                        .expect("actual C declaration names")
                        .recipe(),
                    &spelling,
                );
                let qualified = self
                    .native_c_variable_name_protocol()
                    .expect("actual C declaration")
                    .alias_local_input(&spelling)
                    .qualification()
                    != tcl_syntax::naming::NativeNameQualification::Unqualified;
                let local = if qualified {
                    Value::new_native_string_bytes(local)
                } else {
                    original.native_lifetime_lease().into_value()
                };
                // ObjMakeUpvar resolves the original target again after setter callbacks.
                let (_root, bytes, _element, other) = self.prepare_native_original_variable_in(
                    original,
                    NativeVariableNameLookupPurpose::Link,
                    Some(namespace),
                )?;
                let target = other.id.ok_or_else(|| {
                    self.refuse_host_command("namespace alias cell unavailable".into())
                })?;
                self.bind_original_c_alias_local(&local, target, false, &bytes)?;
            }
            self.retire_released_native_alias_entries();
            Ok(())
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    /// The local side of `ObjMakeUpvar` is a simple scalar lookup. It retains
    /// an actual hash key at birth, without changing the original name primary.
    fn present_original_c_alias_error(
        &mut self,
        failure: Completion<Value>,
        cause: super::super::UpvarLinkError,
    ) -> Completion<Value> {
        if self.refused_completion().is_some() {
            return failure;
        }
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C alias diagnostic");
        let code = if matches!(
            cause,
            super::super::UpvarLinkError::TargetNamespace
                | super::super::UpvarLinkError::LocalNamespace
        ) || protocol.alias_rejection_sets_error_code()
        {
            let code = match crate::command::opt_get_checked(self, &failure.options, b"-errorcode")
            {
                Ok(Some(code)) => code,
                Ok(None) => {
                    return self.refuse_host_command("alias failure error tuple is absent".into());
                }
                Err(error) => return crate::command::completion_from_tcl_error(self, error),
            };
            let bytes = match tcl_syntax::value::ValueOps::native_string_bytes(self, &code) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(self, error.into()),
            };
            tcl_cmd_core::CmdErrorCodeUpdate::Set(bytes.to_vec())
        } else {
            tcl_cmd_core::CmdErrorCodeUpdate::Default
        };
        let message = match tcl_syntax::value::ValueOps::native_string_bytes(self, &failure.result)
        {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return crate::command::completion_from_cmd_error(self, error.into()),
        };
        crate::command::completion_from_cmd_error(
            self,
            tcl_cmd_core::CmdError::from_byte_details(tcl_cmd_core::CmdErrorDetails {
                message,
                string_result: protocol.diagnostic_string_protocol(),
                error_code: code,
                error_info: None,
                error_line: None,
                primitive_getter: None,
            }),
        )
    }

    pub(super) fn bind_original_c_alias_local(
        &mut self,
        original: &Value,
        target: tcl_core_types::VarId,
        owner_is_proc: bool,
        other: &[u8],
    ) -> Result<(), Completion<Value>> {
        match self.bind_original_c_alias_local_inner(original, target, owner_is_proc, other) {
            Ok(()) => Ok(()),
            Err((failure, Some(cause))) => Err(self.present_original_c_alias_error(failure, cause)),
            Err((failure, None)) => Err(failure),
        }
    }

    fn bind_original_c_alias_local_inner(
        &mut self,
        original: &Value,
        target: tcl_core_types::VarId,
        owner_is_proc: bool,
        other: &[u8],
    ) -> Result<(), (Completion<Value>, Option<super::super::UpvarLinkError>)> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| (crate::command::completion_from_cmd_error(self, error), None))?;
        let target_cell = original_names
            .capture_cell(self, target)
            .map_err(|error| (crate::command::completion_from_cmd_error(self, error), None))?;
        let reached = (|| {
            let protocol = self
                .native_c_variable_name_protocol()
                .expect("actual C alias local");
            let bytes = original_names
                .read(self, original)
                .map_err(|error| (crate::command::completion_from_cmd_error(self, error), None))?;
            original_names
                .check_cell(self, target, &target_cell)
                .map_err(|error| (crate::command::completion_from_cmd_error(self, error), None))?;
            let input = protocol.alias_local_input(&bytes);
            let qualified =
                input.qualification() != tcl_syntax::naming::NativeNameQualification::Unqualified;
            let namespace = qualified || !self.frame_owns_local_variables(self.current_level());
            let failure = |error| {
                (
                    crate::command::upvar_link_error_bytes_in(
                        error,
                        other,
                        tcl_core_types::c_string_extent(&bytes),
                        Some(protocol),
                    ),
                    Some(error),
                )
            };
            if owner_is_proc && namespace {
                return Err(failure(super::super::UpvarLinkError::Inverted));
            }
            if protocol.alias_local_is_element(&bytes) {
                return Err(failure(super::super::UpvarLinkError::LocalElement));
            }
            let binding = if namespace {
                self.namespace_var_binding_from_token_bytes(self.current_ns_id(), input.selected())
                    .ok_or_else(|| failure(super::super::UpvarLinkError::LocalNamespace))?
            } else {
                let level = self.current_level();
                VarBinding {
                    owner: self
                        .compiled_local_dynamic_slot(level, input.selected())
                        .map_or(VarTableOwner::Frame(level), |slot| {
                            VarTableOwner::CompiledLocal { level, slot }
                        }),
                    name: tcl_core_types::NameBytes::from(input.selected()),
                }
            };
            let birth = self.raw_variable_binding(&binding).is_none();
            if birth {
                self.bind_new_var(&binding, crate::vars::VarState::Undefined)
                    .ok_or_else(|| {
                        (
                            self.refuse_host_command(
                                "original alias local cell unavailable".into(),
                            ),
                            None,
                        )
                    })?;
                if protocol.version() >= tcl_dialect::TclVersion::V8_5
                    && !matches!(binding.owner, VarTableOwner::CompiledLocal { .. })
                {
                    let key = if qualified {
                        Value::new_native_string_bytes(binding.name.as_bytes())
                    } else {
                        original.clone()
                    };
                    self.var_table_mut(binding.owner)
                        .ok_or_else(|| failure(super::super::UpvarLinkError::LocalNamespace))?
                        .retain_original_native_key(&binding.name, key);
                }
            }
            self.bind_variable_link(&binding, super::super::VariableLinkTarget::Cell(target))
                .map_err(failure)?;
            self.retire_released_native_alias_entries();
            Ok(())
        })();
        original_names
            .finish(self)
            .map_err(|error| (crate::command::completion_from_cmd_error(self, error), None))?;
        reached
    }

    /// A resolved undefined target is a genuine link cell. Array access to an
    /// actual scalar is instead the native guest lookup failure, not missing
    /// host receiver authority.
    pub(super) fn settle_original_c_link_target(
        &mut self,
        original: &Value,
        resolved: &ResolvedVar,
    ) -> Result<tcl_core_types::VarId, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            if let Some(target) = resolved.id {
                return Ok(target);
            }
            if resolved.elem.is_some()
                && resolved
                    .base_id
                    .and_then(|id| self.var_arena.get(id))
                    .is_some_and(|cell| matches!(cell.state(), super::super::Local::Scalar(_)))
            {
                let bytes = original_names
                    .read(self, original)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                return Err(self.variable_access_error_input(
                    "access",
                    tcl_syntax::naming::NativeVariableInputForm::Combined(&bytes),
                    "variable isn't array",
                ));
            }
            self.ensure_target_var_at_binding(&resolved.binding, resolved.elem.as_deref())
                .ok_or_else(|| {
                    self.refuse_host_command("original alias target cell unavailable".into())
                })
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    fn original_c_upvar_target(
        &mut self,
        original: &Value,
        target_level: usize,
        namespace: Option<tcl_core_types::NsId>,
    ) -> Result<(tcl_core_types::VarId, bool, Vec<u8>), Completion<Value>> {
        let selected = self
            .select_execution_frame(target_level)
            .map_err(|error| self.refuse_host_command(error.clone()))?;
        let prepared = self.prepare_native_original_variable_in(
            original,
            NativeVariableNameLookupPurpose::Link,
            namespace,
        );
        self.restore_execution_frame(selected);
        let (_root, bytes, _element, resolved) = prepared?;
        let target = self.settle_original_c_link_target(original, &resolved)?;
        let root = self
            .var_arena
            .element_parent(target)
            .map_or(target, |(root, _)| root);
        let owner_is_proc = self
            .var_arena
            .get(root)
            .is_some_and(|cell| cell.namespace_owner().is_none())
            && matches!(resolved.binding.owner,
                VarTableOwner::Frame(level) | VarTableOwner::CompiledLocal { level, .. }
                    if self.frame_owns_local_variables(level));
        if self
            .native_c_variable_name_protocol()
            .is_some_and(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5)
            && let Some((array, key)) = self
                .var_arena
                .element_parent(target)
                .map(|(array, key)| (array, key.clone()))
            && let Some(key_object) = Self::original_c_parsed_element_key(original)
        {
            self.var_arena
                .retain_original_array_key(array, &key, key_object);
        }
        Ok((target, owner_is_proc, bytes.to_vec()))
    }

    pub(crate) fn link_original_c_variable_objects(
        &mut self,
        original: &Value,
        target_level: usize,
        namespace: Option<tcl_core_types::NsId>,
        local: &Value,
    ) -> Result<(), Completion<Value>> {
        let (target, owner_is_proc, bytes) =
            self.original_c_upvar_target(original, target_level, namespace)?;
        self.bind_original_c_alias_local(local, target, owner_is_proc, &bytes)
    }

    pub(in super::super) fn link_original_c_compiled_upvar(
        &mut self,
        original: &Value,
        target_level: usize,
        local: &VarBinding,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
                self, None,
            )
            .map_err(|error| completion_from_cmd_error(self, error))?;
        let local_cell = original_names
            .capture_binding(self, local)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let (target, _, bytes) = self.original_c_upvar_target(original, target_level, None)?;
            local_cell
                .check(self, &original_names)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            if let Err(error) =
                self.bind_variable_link(local, super::super::VariableLinkTarget::Cell(target))
            {
                let protocol = self
                    .native_c_variable_name_protocol()
                    .expect("actual C alias diagnostic");
                let failure = crate::command::upvar_link_error_bytes_in(
                    error,
                    &bytes,
                    local.name.as_bytes(),
                    Some(protocol),
                );
                return Err(self.present_original_c_alias_error(failure, error));
            }
            self.retire_released_native_alias_entries();
            Ok(())
        })();
        original_names
            .finish(self)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        reached
    }
}
