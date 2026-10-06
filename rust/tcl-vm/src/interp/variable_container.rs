// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dictionary-valued roots share ordinary variable cells and value objects.

use super::{Local, ResolvedVar, Value, VarBinding, VarId, Vm, err};
use tcl_dialect::VariableContainerModel;
use tcl_runtime_api::Completion;

impl Vm {
    pub(super) fn select_dictionary_containers(&mut self) {
        if !self.dictionary_variable_containers() {
            return;
        }
        for id in self.var_arena.array_cells() {
            let Some(Local::Array(elements)) =
                self.var_arena.get(id).map(crate::vars::VarCell::state)
            else {
                continue;
            };
            let pairs = elements
                .iter()
                .filter_map(|(key, raw)| {
                    let value = self.read_resolved_cell(self.var_arena.resolve(*raw)?)?;
                    Some((Value::from_string_bytes(key.as_ref()), value))
                })
                .collect();
            let _ = self
                .var_arena
                .replace_state(id, Local::Scalar(Value::dict(pairs)));
        }
    }

    pub(crate) fn dictionary_variable_containers(&self) -> bool {
        self.native_invocation_dialect().variable_container_model
            == Some(VariableContainerModel::DictionaryValue)
    }

    pub(super) fn read_variable_contents(&self, resolved: &ResolvedVar) -> Option<Value> {
        if self.dictionary_variable_containers()
            && let Some(key) = &resolved.elem
        {
            return self.read_resolved_elem(resolved.base_id?, key);
        }
        self.read_resolved_cell(resolved.id?)
    }

    pub(super) fn native_variable_dict_pairs(
        &self,
        value: &Value,
    ) -> Result<Vec<(Value, Value)>, tcl_syntax::value::ValueError> {
        let protocol = self
            .name_policy_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native variable dictionary issuer",
            ))?
            .string_protocol();
        value.native_object_dict_pairs(protocol)
    }

    pub(super) fn dictionary_pairs_at(
        &self,
        id: VarId,
    ) -> Result<Option<Vec<(Value, Value)>>, tcl_syntax::value::ValueError> {
        match self.var_arena.get(id).map(crate::vars::VarCell::state) {
            Some(Local::Scalar(value)) => self.native_variable_dict_pairs(value).map(Some),
            _ => Ok(None),
        }
    }

    pub(super) fn write_dictionary_element(
        &mut self,
        binding: &VarBinding,
        key: impl AsRef<[u8]>,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        let resolved = self
            .resolve_binding_var(binding.clone(), None, &mut std::collections::HashSet::new())
            .ok_or_else(|| err("variable link cycle"))?;
        self.write_dictionary_member(&resolved, key, value)
    }

    pub(super) fn remove_dictionary_element(&mut self, id: VarId, key: impl AsRef<[u8]>) -> bool {
        match self.remove_dictionary_element_bytes(id, key.as_ref()) {
            Ok(removed) => removed,
            Err(error) => {
                if let Some(refusal) = error.native_access_refusal() {
                    let _ = self.refuse_host_command(refusal.to_string());
                }
                false
            }
        }
    }

    pub(super) fn remove_dictionary_element_bytes(
        &mut self,
        id: VarId,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let Some(mut pairs) = self.dictionary_pairs_at(id)? else {
            return Ok(false);
        };
        let before = pairs.len();
        pairs.retain(|(name, _)| name.string_bytes().as_ref() != key);
        if pairs.len() == before {
            return Ok(false);
        }
        let _ = self
            .var_arena
            .replace_state(id, Local::Scalar(Value::dict(pairs)));
        Ok(true)
    }

    pub(super) fn remove_dictionary_member(
        &mut self,
        resolved: &ResolvedVar,
        key: impl AsRef<[u8]>,
    ) -> bool {
        let Some(value) = self.read_variable_contents(resolved) else {
            return false;
        };
        let mut pairs = match self.native_variable_dict_pairs(&value) {
            Ok(pairs) => pairs,
            Err(error) => {
                if let Some(refusal) = error.native_access_refusal() {
                    let _ = self.refuse_host_command(refusal.to_string());
                }
                return false;
            }
        };
        let before = pairs.len();
        pairs.retain(|(name, _)| name.string_bytes().as_ref() != key.as_ref());
        pairs.len() != before
            && self
                .store_dictionary_root(resolved, Value::dict(pairs))
                .is_ok()
    }
    pub(super) fn write_dictionary_member(
        &mut self,
        resolved: &ResolvedVar,
        key: impl AsRef<[u8]>,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        let pairs = self
            .read_variable_contents(resolved)
            .map(|value| self.native_variable_dict_pairs(&value))
            .transpose();
        let mut pairs = pairs
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?
            .unwrap_or_default();
        if let Some((_, old)) = pairs
            .iter_mut()
            .find(|(name, _)| name.string_bytes().as_ref() == key.as_ref())
        {
            *old = value;
        } else {
            pairs.push((Value::from_string_bytes(key.as_ref()), value));
        }
        self.store_dictionary_root(resolved, Value::dict(pairs))
    }

    pub(super) fn store_dictionary_root(
        &mut self,
        resolved: &ResolvedVar,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        if let Some(key) = &resolved.elem {
            return self.write_dictionary_element(&resolved.binding, key, value);
        }
        if let Some(id) = resolved.id {
            let _ = self.var_arena.replace_state(id, Local::Scalar(value));
        } else {
            self.bind_new_var(&resolved.binding, Local::Scalar(value))
                .ok_or_else(|| err("too many variable cells"))?;
        }
        Ok(())
    }
}
