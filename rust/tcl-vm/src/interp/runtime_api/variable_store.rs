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

//! `VarStore` over the call-frame stack, the native array element read behind it, and the array-search backend.

use super::super::Vm;
use crate::value::Value;
use crate::vars::{VarState as Local, VarTable};
use tcl_registry::native_dictionary::NativeDictionaryDialect;
use tcl_runtime_api::{
    ArrayElementRead, ArrayInvalidation, ArrayReadFailure, ArrayReadMiss, ArrayTarget, FrameId,
    VarStore, VarUnsetError,
};

/// The Family-B variable store over the call-frame stack. `FrameId` is the
/// absolute frame level (`GLOBAL_FRAME` = 0); access resolves from that frame,
/// following links. A `FrameId` naming the *current* frame delegates to the full
/// by-name accessors (namespace fallback + traces); any other frame uses the
/// frame-addressed storage helpers (no current-eval-context fallback/traces).
impl VarStore for Vm {
    type Value = Value;

    fn variable_diagnostic_at(
        &self,
        operation: tcl_syntax::naming::NativeVariableDiagnosticOperation,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
    ) -> Result<tcl_syntax::naming::NativeVariableDiagnosticProjection, tcl_syntax::value::ValueError>
    {
        let policy = self
            .execution_name_policy()
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable diagnostic",
            ))?;
        tcl_syntax::naming::report_native_variable_diagnostic_at(
            policy.recipe(),
            operation,
            reason,
            site,
            input,
        )
        .map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("variable diagnostic input")
        })
    }

    fn get_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<Option<Value>, tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return self.observed_variable_get(name, frame.0);
        }
        if self.name_policy_protocol().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable name",
            ));
        }
        Ok(self.get_var_from_bytes(frame.0, name))
    }

    fn set_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
        value: Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return self.observed_variable_set(name, frame.0, value);
        }
        if self.name_policy_protocol().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable name",
            ));
        }
        if frame.0 == self.current_level() {
            let _ = self.set_var_bytes(name, value);
        } else {
            self.write_scalar_bytes_from(frame.0, name, value);
        }
        if self.refused_completion().is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable store execution",
            ));
        }
        Ok(())
    }

    fn unset_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return self.observed_variable_unset(name, frame.0);
        }
        if self.name_policy_protocol().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable name",
            ));
        }
        let removed = if frame.0 == self.current_level() {
            self.unset_var_bytes(name)
        } else {
            self.resolve_var_from_bytes(name, frame.0)
                .is_some_and(|resolved| self.unbind_storage_resolved(&resolved))
        };
        if self.refused_completion().is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable removal execution",
            ));
        }
        Ok(removed)
    }

    fn exists_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return self.observed_variable_exists(name, frame.0);
        }
        if self.name_policy_protocol().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable name",
            ));
        }
        Ok(self.exists_var_bytes_from(name, frame.0))
    }

    fn get_elem_bytes(
        &self,
        frame: FrameId,
        root: &[u8],
        element: &[u8],
    ) -> Result<Option<Value>, tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        if self.name_policy_protocol().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable name",
            ));
        }
        Ok(self.get_array_elem_bytes_from(frame.0, root, element))
    }

    fn set_elem_bytes(
        &mut self,
        frame: FrameId,
        root: &[u8],
        element: &[u8],
        value: Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        if self.name_policy_protocol().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable name",
            ));
        }
        let _ = self.set_array_elem_bytes_from(
            frame.0,
            root,
            element,
            value,
            frame.0 == self.current_level(),
        );
        if self.refused_completion().is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable element store execution",
            ));
        }
        Ok(())
    }

    fn array_target_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<ArrayTarget, tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return self.observed_array_target(name, frame.0);
        }
        if self.name_policy_protocol().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable name",
            ));
        }
        Ok(self
            .resolve_var_from_bytes(name, frame.0)
            .and_then(|resolved| resolved.id)
            .map_or_else(
                || ArrayTarget::named_bytes(frame, name),
                |id| ArrayTarget::cell_bytes(frame, name, id),
            ))
    }

    fn array_default_state_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<tcl_runtime_api::ArrayDefaultState<Value>, tcl_syntax::value::ValueError> {
        use tcl_runtime_api::ArrayDefaultState as State;

        if self.observed_names.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }

        self.actual_native_invocation_dialect()
            .native_array_default_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "array default storage",
            ))?;
        Ok(
            match target.cell_id().and_then(|id| self.var_arena.get(id)) {
                None => State::Undefined,
                Some(cell) => match cell.state() {
                    Local::Undefined => State::Undefined,
                    Local::Array(_) => State::Array {
                        default: self
                            .var_arena
                            .array_default(target.cell_id().expect("selected array"))
                            .cloned(),
                    },
                    _ => State::NonArray,
                },
            },
        )
    }

    fn unset_array_default_at(
        &mut self,
        target: &ArrayTarget,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        self.actual_native_invocation_dialect()
            .native_array_default_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "array default storage",
            ))?;
        if let Some(id) = target.cell_id() {
            self.var_arena.set_array_default(id, None);
        }
        Ok(())
    }

    fn set_array_default_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
        value: Value,
    ) -> Result<Result<(), tcl_runtime_api::ArrayDefaultSetFailure>, tcl_syntax::value::ValueError>
    {
        use tcl_syntax::naming::{
            NativeVariableDiagnosticOperation as Operation,
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
            NativeVariableInputForm as Input, report_native_variable_diagnostic_at,
        };

        use tcl_runtime_api::ArrayDefaultSetFailure as Failure;

        if self.observed_names.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }

        self.actual_native_invocation_dialect()
            .native_array_default_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "array default storage",
            ))?;
        let policy = self.name_policy_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("array default name policy"),
        )?;
        let failure = |reason| {
            report_native_variable_diagnostic_at(
                policy.recipe(),
                Operation::Write,
                reason,
                Site::NameLookup,
                Input::Combined(name),
            )
            .map(Failure::Lookup)
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "array default lookup diagnostic",
                )
            })
        };
        let Some(resolved) = self.resolve_var_from_bytes(name, frame.0) else {
            return Ok(Err(failure(Reason::MissingParentNamespace)?));
        };
        if resolved.elem.is_some()
            && resolved
                .base_id
                .and_then(|id| self.var_arena.get(id))
                .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_)))
        {
            return Ok(Err(failure(Reason::NotArray)?));
        }
        let id = self
            .ensure_target_var_at_binding(&resolved.binding, resolved.elem.as_deref())
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "array default receiver allocation",
            ))?;
        if resolved.elem.is_some() || self.var_arena.element_parent(id).is_some() {
            return Ok(Err(Failure::Element));
        }
        match self.var_arena.get(id).map(crate::vars::VarCell::state) {
            Some(Local::Undefined) => {
                self.var_arena
                    .replace_state(id, Local::Array(VarTable::new()));
            }
            Some(Local::Array(_)) => {}
            _ => return Ok(Err(Failure::Scalar)),
        }
        if !self.var_arena.set_array_default(id, Some(value)) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "array default receiver expired",
            ));
        }
        Ok(Ok(()))
    }

    fn variable_container_model(&self) -> tcl_dialect::VariableContainerModel {
        self.native_invocation_dialect()
            .variable_container_model
            .unwrap_or_default()
    }

    fn array_keys_checked_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<String>>, tcl_syntax::value::ValueError> {
        self.array_key_bytes_checked_at(target)?
            .map(|keys| {
                keys.into_iter()
                    .map(|key| {
                        tcl_syntax::raw_string::RawString::from_bytes(key)
                            .unicode()
                            .map(|key| key.to_string())
                            .map_err(Into::into)
                    })
                    .collect()
            })
            .transpose()
    }

    fn array_hash_statistics_at(
        &self,
        target: &ArrayTarget,
        unguarded_empty_average: bool,
    ) -> Result<Option<String>, tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() || self.dictionary_variable_containers() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native array hash table",
            ));
        }
        let id = target.cell_id().or_else(|| {
            self.resolve_var_from_bytes(target.name_bytes(), target.frame().0)
                .filter(|resolved| resolved.elem.is_none())
                .and_then(|resolved| resolved.id)
        });
        let Some(Local::Array(elements)) = id
            .and_then(|id| self.var_arena.get(id))
            .map(super::super::super::vars::VarCell::state)
        else {
            return Ok(None);
        };
        elements
            .hash_statistics(unguarded_empty_average)
            .map(Some)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native array hash recipe",
            ))
    }

    fn array_key_bytes_checked_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return self.observed_array_keys(target);
        }
        if self.dictionary_variable_containers() {
            let value = target.cell_id().map_or_else(
                || self.get_var_from_bytes(target.frame().0, target.name_bytes()),
                |id| self.read_resolved_cell(id),
            );
            return value
                .map(|value| {
                    self.native_variable_dict_pairs(&value).map(|pairs| {
                        pairs
                            .into_iter()
                            .map(|(key, _)| key.string_bytes().to_vec())
                            .collect()
                    })
                })
                .transpose();
        }
        Ok(target.cell_id().map_or_else(
            || self.array_key_bytes_from(target.frame().0, target.name_bytes()),
            |id| self.array_key_bytes_at_id(id),
        ))
    }

    fn array_read_elem_bytes_at(
        &mut self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<ArrayElementRead<Value>, tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        if self.dictionary_variable_containers() {
            let value = target.cell_id().map_or_else(
                || self.get_var_from_bytes(target.frame().0, target.name_bytes()),
                |id| self.read_resolved_cell(id),
            );
            let Some(value) = value else {
                return Ok(ArrayElementRead::ArrayInvalidated(ArrayInvalidation::Unset));
            };
            let pairs = self.native_variable_dict_pairs(&value)?;
            return Ok(pairs
                .into_iter()
                .find(|(name, _)| name.string_bytes().as_ref() == key)
                .map_or_else(
                    || ArrayElementRead::Missing(ArrayReadMiss::missing()),
                    |(_, value)| ArrayElementRead::Value(value),
                ));
        }
        let read = self.array_read_elem_bytes_native_at(target, key);
        if self.refused_completion().is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "array element read execution",
            ));
        }
        Ok(read)
    }

    fn get(&self, frame: FrameId, name: &str) -> Option<Value> {
        if frame.0 == self.current_level() {
            self.get_var(name)
        } else {
            self.get_var_from(frame.0, name)
        }
    }

    fn set(&mut self, frame: FrameId, name: &str, value: Value) {
        if frame.0 == self.current_level() {
            let _ = self.set_var(name, value);
        } else {
            self.write_scalar_from(frame.0, name, value);
        }
    }

    fn unset(&mut self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.current_level() {
            self.unset_var(name)
        } else {
            self.unset_from(frame.0, name)
        }
    }

    fn unset_command_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<Result<bool, VarUnsetError>, tcl_syntax::value::ValueError> {
        if self.name_policy_protocol().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable removal",
            ));
        }
        if self
            .resolve_var_from_bytes(name, frame.0)
            .and_then(|resolved| resolved.id)
            .is_some_and(|id| self.const_vars.contains(&id))
        {
            return Ok(Err(VarUnsetError::IsConstant));
        }
        let removed = if frame.0 == self.current_level() {
            self.unset_var_bytes(name)
        } else {
            self.unset_from_bytes(frame.0, name)
        };
        if self.refused_completion().is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable removal trace",
            ));
        }
        Ok(Ok(removed))
    }

    fn unset_command(&mut self, frame: FrameId, name: &str) -> Result<bool, VarUnsetError> {
        if self.is_constant_from(frame.0, name) {
            return Err(VarUnsetError::IsConstant);
        }
        Ok(self.unset(frame, name))
    }

    fn unset_confined(&self, frame: FrameId, name: &str) -> bool {
        self.store_confined(name, frame.0)
    }

    fn unset_confined_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        self.store_confined_bytes(name, frame.0)
    }

    fn exists(&self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.current_level() {
            // The *complete* existence check: a scalar, an array, or an array
            // element (`exists_var`) — not the scalar-only `var_exists`, which
            // would miss arrays like `::env` (a `VarStore` contract bug surfaced
            // by routing `info exists` through this method).
            self.exists_var(name)
        } else {
            self.exists_from(frame.0, name)
        }
    }

    // Element access carries `(base, key)` through to the array table. Rebuilding
    // `base(key)` and feeding it through the scalar-name parser loses bases that
    // themselves contain `(` (`z(b` + `a` was reparsed as array `z`, key `b(a`).
    // The active frame keeps the inherent accessors' namespace fallback and
    // traces; a non-active frame resolves the base from that frame directly.

    fn get_elem(&self, frame: FrameId, name: &str, key: &str) -> Option<Value> {
        if frame.0 == self.current_level() {
            self.get_array_elem(name, key)
        } else {
            self.get_array_elem_from(frame.0, name, key)
        }
    }

    fn set_elem(&mut self, frame: FrameId, name: &str, key: &str, value: Value) {
        if frame.0 == self.current_level() {
            let _ = self.set_array_elem(name, key, value);
            return;
        }
        let _ = self.write_array_raw_from(frame.0, name, key, value);
    }

    fn unset_elem(&mut self, frame: FrameId, name: &str, key: &str) -> bool {
        if frame.0 == self.current_level() {
            return self.array_unset_elem(name, key);
        }
        self.unset_array_elem_from(frame.0, name, key)
    }

    fn exists_elem(&self, frame: FrameId, name: &str, key: &str) -> bool {
        self.get_elem(frame, name, key).is_some()
    }

    fn array_keys(&self, frame: FrameId, name: &str) -> Option<Vec<String>> {
        self.array_keys_from(frame.0, name)
    }

    fn array_target(&self, frame: FrameId, name: &str) -> ArrayTarget {
        self.resolve_var_from(name, frame.0)
            .and_then(|resolved| resolved.id)
            .map_or_else(
                || ArrayTarget::named(frame, name),
                |id| ArrayTarget::cell(frame, name, id),
            )
    }

    fn array_keys_at(&self, target: &ArrayTarget) -> Option<Vec<String>> {
        target.cell_id().map_or_else(
            || self.array_keys_from(target.frame().0, target.name()),
            |id| self.array_keys_at_id(id),
        )
    }

    fn array_search_key_bytes_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        Ok(target
            .cell_id()
            .and_then(|id| self.array_search_key_bytes_at_id(id)))
    }

    fn array_elem_exists_bytes_at(
        &self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        if self.observed_names.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        Ok(target
            .cell_id()
            .is_some_and(|id| self.read_resolved_elem(id, key).is_some()))
    }

    fn array_search_keys_at(&self, target: &ArrayTarget) -> Option<Vec<String>> {
        target.cell_id().map_or_else(
            || self.array_keys_from(target.frame().0, target.name()),
            |id| self.array_search_keys_at_id(id),
        )
    }

    fn array_elem_exists_at(&self, target: &ArrayTarget, key: &str) -> bool {
        target.cell_id().map_or_else(
            || {
                self.get_array_elem_from(target.frame().0, target.name(), key)
                    .is_some()
            },
            |id| self.array_elem_exists_at_id(id, key),
        )
    }

    fn array_revision_at(&self, target: &ArrayTarget) -> Option<u64> {
        target
            .cell_id()
            .and_then(|id| self.var_arena.array_revision(id))
    }

    fn unset_elem_bytes_at(
        &mut self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        use tcl_runtime_api::variable_destruction::NativeArrayUnsetMemberLookup;
        if self.observed_names.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        if self.dictionary_variable_containers() {
            let id = target.cell_id().or_else(|| {
                self.resolve_var_parts_from_bytes(target.name_bytes(), None, target.frame().0)
                    .and_then(|resolved| resolved.id)
            });
            return id.map_or(Ok(false), |id| {
                self.remove_dictionary_element_bytes(id, key)
            });
        }
        let id = match self
            .actual_native_invocation_dialect()
            .array_unset_member_lookup()
        {
            Some(NativeArrayUnsetMemberLookup::OriginalName) => self
                .resolve_var_parts_from_bytes(target.name_bytes(), None, target.frame().0)
                .and_then(|resolved| resolved.id),
            Some(NativeArrayUnsetMemberLookup::SelectedArray) => target.cell_id(),
            None => {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "array member unset lookup",
                ));
            }
        };
        let Some(id) = id else {
            return Ok(false);
        };
        let removed = self.array_unset_elem_reporting_bytes_at(id, target.name_bytes(), key, None);
        if self.refused_completion().is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "array element removal execution",
            ));
        }
        Ok(removed)
    }

    fn unset_elem_at(&mut self, target: &ArrayTarget, key: &str) -> bool {
        if let Some(id) = target.cell_id() {
            self.array_unset_elem_reporting_at(id, target.name(), key, None)
        } else {
            self.unset_array_elem_from(target.frame().0, target.name(), key)
        }
    }

    fn array_read_elem_at(&mut self, target: &ArrayTarget, key: &str) -> ArrayElementRead<Value> {
        self.array_read_elem_bytes_native_at(target, key.as_bytes())
    }
}

impl Vm {
    fn array_read_elem_bytes_native_at(
        &mut self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> ArrayElementRead<Value> {
        let (live_was_array, read) =
            self.read_elem_traced_from_bytes(target.name_bytes(), key, target.frame().0);
        let invalidation = target
            .cell_id()
            .map_or_else(
                || self.array_key_bytes_from(target.frame().0, target.name_bytes()),
                |id| self.array_key_bytes_at_id(id),
            )
            .is_none()
            .then(|| {
                match target
                    .cell_id()
                    .and_then(|id| self.var_arena.get(id))
                    .map(crate::vars::VarCell::state)
                {
                    Some(Local::Scalar(_) | Local::Link(_) | Local::NameLink(_)) => {
                        ArrayInvalidation::Retyped
                    }
                    Some(Local::Undefined | Local::Array(_)) | None => ArrayInvalidation::Unset,
                }
            });
        match read {
            Err(failure) if invalidation.is_some() => {
                let info = self
                    .error_info_value()
                    .map(<[u8]>::to_vec)
                    .or_else(|| Some(failure.result.string_bytes().to_vec()));
                ArrayElementRead::TraceError(ArrayReadFailure::new_bytes(
                    failure.result.string_bytes().to_vec(),
                    b"TCL READ VARNAME".to_vec(),
                    info,
                    Some(i64::from(self.error_line())),
                ))
            }
            Err(failure) => {
                let info = self
                    .take_error_info()
                    .unwrap_or_else(|| failure.result.string_bytes().to_vec());
                let line = i64::from(self.error_line());
                self.publish_error_bytes(&info, &Value::string("TCL READ VARNAME"));
                ArrayElementRead::Missing(ArrayReadMiss::trace_error(Some(info), line))
            }
            Ok(_) if let Some(invalidation) = invalidation => {
                ArrayElementRead::ArrayInvalidated(invalidation)
            }
            Ok(Some(value)) => ArrayElementRead::Value(value),
            Ok(None) => {
                let miss = if live_was_array {
                    ArrayReadMiss::missing()
                } else {
                    let code = Value::list(vec![
                        Value::string("TCL"),
                        Value::string("LOOKUP"),
                        Value::string("VARNAME"),
                        Value::from_string_bytes(target.name_bytes()),
                    ]);
                    ArrayReadMiss::lookup(code.string_bytes().to_vec())
                };
                ArrayElementRead::Missing(miss)
            }
        }
    }
}

impl tcl_cmd_core::native_array_search::NativeArraySearchBackend for Vm {
    fn array_search_protocol(
        &self,
    ) -> Option<tcl_syntax::native_array_search::NativeArraySearchProtocol> {
        self.native_invocation_dialect()
            .native_array_search_protocol(
                tcl_runtime_api::native_hash_abi::supported_backend_array_search_abi()?,
            )
    }
    fn array_search_cache(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
    ) -> Result<Option<tcl_core_types::NativeArraySearchCache>, tcl_syntax::value::ValueError> {
        value.native_array_search_cache_in(protocol)
    }
    fn install_array_search_cache(
        &self,
        value: &Value,
        cache: tcl_core_types::NativeArraySearchCache,
        protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        value.install_native_array_search_cache(cache, protocol)
    }
    fn array_search_on_original(
        &mut self,
        target: &ArrayTarget,
        sub: &str,
        name: &[u8],
        operand: Option<&tcl_cmd_core::native_array_search::NativeArraySearchOperand<'_, Value>>,
        protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
    ) -> Result<
        Result<Value, tcl_syntax::native_array_search::NativeArraySearchFailure>,
        tcl_syntax::value::ValueError,
    > {
        let (handle, bytes, cache) = operand.map_or((None, None, None), |operand| {
            (Some(operand.original), Some(operand.bytes), operand.cache)
        });
        let id =
            target
                .cell_id()
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original array search cell",
                ))?;
        let result = self.var_arena.native_array_search(
            id,
            sub,
            name,
            crate::vars::NativeArraySearchHandle {
                original: handle,
                bytes,
                cache,
            },
            protocol,
        )?;
        if sub == "startsearch"
            && protocol.start_handle_has_string_primary()
            && let Ok(value) = &result
        {
            let materialization = self
                .actual_native_invocation_dialect()
                .native_string_materialization(None)
                .filter(|issuer| issuer.protocol().tcl_version() == Some(protocol.version()))
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native array search String producer",
                ))?;
            value.retain_native_string_representation(materialization)?;
        }
        if sub == "anymore" && protocol.version() == tcl_dialect::TclVersion::V8_4 {
            if let Ok(value) = result {
                let integer = value
                    .native_scalar_cache()
                    .and_then(|cache| match cache {
                        tcl_syntax::scalar_getter::NativeScalarCache::Number(
                            tcl_syntax::number::Number::Int(value),
                        ) => Some(value),
                        _ => None,
                    })
                    .expect("anymore numeric constructor");
                return Ok(Ok(Value::from_native_scalar_cache(
                    tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(integer),
                    None,
                    self.native_invocation_dialect(),
                )?));
            }
            return Ok(result);
        }
        Ok(result)
    }
}
