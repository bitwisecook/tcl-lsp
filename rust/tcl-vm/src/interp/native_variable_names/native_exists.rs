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

//! Original existence reads retain their exact receiver through quiet observers.

use super::*;

impl Vm {
    pub(crate) fn exists_original_c_parts(
        &mut self,
        original: &Value,
        element: Option<&Value>,
    ) -> Result<bool, Completion<Value>> {
        // TclObjLookupVarEx obtains part2 before inspecting part1's cache.
        let element_bytes = element
            .map(|element| self.native_name_operand_bytes(element))
            .transpose()
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let prepared = self.prepare_native_original_variable_parts(
            original,
            NativeVariableNameLookupPurpose::Exists,
            None,
            element_bytes.as_deref(),
        );
        let (_root, bytes, element_bytes, mut resolved) = match prepared {
            Ok(prepared) => prepared,
            Err(error) if self.refused_completion().is_some() => return Err(error),
            Err(_) => return Ok(false),
        };
        self.prepare_existing_array_read(&mut resolved)?;
        self.retain_original_exists_element_key(original, element, &resolved);
        self.exists_selected_original_variable(
            &bytes,
            element_bytes.as_deref(),
            resolved,
            Some((original, element.is_some())),
        )
    }

    fn retain_original_exists_element_key(
        &mut self,
        original: &Value,
        element: Option<&Value>,
        resolved: &ResolvedVar,
    ) {
        if !self
            .native_c_variable_name_protocol()
            .is_some_and(|protocol| protocol.element_table_retains_original())
        {
            return;
        }
        let Some(array) = resolved.base_id.filter(|_| resolved.elem.is_some()) else {
            return;
        };
        let key = element
            .cloned()
            .or_else(|| Self::original_c_parsed_element_key(original));
        if let (Some(key), Some(index)) = (key, resolved.elem.as_ref()) {
            self.var_arena.retain_original_array_key(array, index, key);
        }
    }

    pub(in crate::interp) fn exists_selected_original_variable(
        &mut self,
        name: &[u8],
        element: Option<&[u8]>,
        resolved: ResolvedVar,
        original: Option<(&Value, bool)>,
    ) -> Result<bool, Completion<Value>> {
        let cell = self.trace_cell_from_resolved(&resolved);
        let check = |vm: &mut Self| {
            let name = if let Some((original, separate)) = original.filter(|_| {
                cell.as_ref().is_some_and(|cell| {
                    vm.original_variable_trace_requires_name(cell, element.is_some(), "read")
                })
            }) {
                let bytes = vm
                    .native_name_operand_bytes(original)
                    .map_err(|error| vm.refuse_host_command(error.to_string()))?;
                let policy = vm
                    .name_policy_protocol()
                    .expect("selected original C name policy");
                let input = if separate {
                    policy.recipe().separate_variable_input(&bytes, element)
                } else {
                    policy.recipe().combined_variable_input(&bytes)
                };
                std::rc::Rc::from(input.root().selected())
            } else {
                std::rc::Rc::from(name)
            };
            vm.fire_original_existence_read(&name, element, cell.clone());
            if let Some(refusal) = vm.refused_completion() {
                return Err(refusal);
            }
            Ok(vm.read_variable_contents(&resolved).is_some()
                || resolved.elem.is_none()
                    && resolved.id.is_some_and(|id| {
                        matches!(
                            vm.var_arena.get(id).map(crate::vars::VarCell::state),
                            Some(super::super::Local::Array(_))
                        )
                    }))
        };
        let found = match cell.as_ref() {
            Some(cell) => self.with_variable_operation(cell, check),
            None => check(self),
        }?;
        if let (Some(parent), Some(index), Some(id)) =
            (resolved.base_id, resolved.elem.as_ref(), resolved.id)
        {
            self.discard_undefined_array_shell(parent, index, id);
        }
        Ok(found)
    }

    fn fire_original_existence_read(
        &mut self,
        name: &[u8],
        element: Option<&[u8]>,
        cell: Option<super::super::VarTraceCell>,
    ) {
        let mut combined = Vec::new();
        let access = if let Some(element) = element {
            combined.extend_from_slice(name);
            combined.push(b'(');
            combined.extend_from_slice(element);
            combined.push(b')');
            combined.as_slice()
        } else {
            name
        };
        let element_access = element.is_some()
            || self.name_policy_protocol().is_some_and(|policy| {
                policy
                    .recipe()
                    .combined_variable_input(name)
                    .element()
                    .is_some()
            });
        let _ = self.fire_var_traces_from_parts(
            super::super::VarTraceInvocation {
                leave_error_message: false,
                name: access,
                op: "read",
                reported: (name, element),
                element_access,
            },
            None,
            cell,
        );
    }

    pub(crate) fn original_existence_result(
        &mut self,
        found: bool,
    ) -> Result<Value, Completion<Value>> {
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("selected original C existence handler");
        if protocol.version() == tcl_dialect::TclVersion::V8_4 {
            return self
                .with_native_interp_result(|result| {
                    result.set_native_unshared_integer(i64::from(found), protocol.version())?;
                    Ok(result.native_lifetime_lease().into_value())
                })
                .and_then(std::convert::identity)
                .map_err(|error| self.refuse_host_command(error.to_string()));
        }
        Ok(Value::int(i64::from(found)))
    }

    pub(crate) fn compiled_existence_result(
        &mut self,
        found: bool,
    ) -> Result<Value, Completion<Value>> {
        let Some(protocol) = self
            .native_c_variable_name_protocol()
            .filter(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5)
        else {
            return Ok(Value::bool(found));
        };
        self.native_c_execution_boolean(found, protocol.version())
            .map_err(|error| self.refuse_host_command(error.to_string()))
    }
}
