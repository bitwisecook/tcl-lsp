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

//! Ordered native compiled-local cells, independent of dynamic name tables.

use super::{
    CapturedVariableUpdate, Local, ResolvedVar, Value, VarBinding, VarId, VarTableOwner, Vm,
};
use tcl_core_types::NameBytes;
use tcl_runtime_api::Completion;

impl Vm {
    /// Initialise or reuse the same anonymous native foreach counter header.
    pub(crate) fn initialise_native_compiled_counter(
        &mut self,
        slot: usize,
        version: tcl_dialect::TclVersion,
    ) -> Result<(), Completion<Value>> {
        let Some(id) = self
            .frames
            .get(self.current_level())
            .and_then(|frame| frame.compiled_locals.get(slot))
            .map(|(_, id)| *id)
        else {
            return Err(
                self.refuse_host_command("native foreach counter cell is unavailable".into())
            );
        };
        let existing = self.var_arena.get(id).and_then(|cell| match cell.state() {
            Local::Scalar(value) => Some(value.native_lifetime_lease()),
            _ => None,
        });
        if let Some(existing) = existing {
            existing
                .value()
                .set_native_loop_counter(-1, version)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))
        } else {
            let value = Value::int(-1);
            value
                .set_native_loop_counter(-1, version)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
            self.bind_compiled_formal_slot(slot, value)
        }
    }
    /// Borrow a compiler-owned temporary without variable lookup or read traces.
    pub(crate) fn native_compiled_temporary_value(
        &mut self,
        slot: usize,
    ) -> Result<crate::value::NativeObjectLifetimeLease, Completion<Value>> {
        let value = self
            .frames
            .get(self.current_level())
            .and_then(|frame| frame.compiled_locals.get(slot))
            .and_then(|(_, id)| self.var_arena.get(*id))
            .and_then(|cell| match cell.state() {
                Local::Scalar(value) => Some(value.native_lifetime_lease()),
                _ => None,
            });
        value.ok_or_else(|| {
            self.refuse_host_command("native temporary scalar is unavailable".into())
        })
    }

    /// Assign an original foreach member through its actual auxiliary slot.
    /// Only C8.4's traced `PtrSetVar` call owns the additional transient reference.
    pub(crate) fn native_compiled_each_assign(
        &mut self,
        slot: usize,
        value: Value,
        storage: tcl_registry::native_each_compilation::NativeCompiledEachStorage,
    ) -> Result<(), Completion<Value>> {
        let (name, resolved) = self.compiled_operand(slot, None)?;
        let captured = self.capture_selected_update(name.as_bytes(), None, &resolved)?;
        let pin = (storage
            == tcl_registry::native_each_compilation::NativeCompiledEachStorage::LocalRefetch
            && self.original_variable_trace_requires_name(&captured.cell, false, "write"))
        .then(|| value.clone());
        let result = self.with_variable_operation(&captured.cell, |vm| {
            vm.store_captured_update(name.as_bytes(), None, &captured, value)
                .map(|_| ())
        });
        drop(pin);
        result
    }
    pub(crate) fn declare_constant_bytes(
        &mut self,
        name: &[u8],
        value: Value,
        create_element: bool,
    ) -> Result<(), Completion<Value>> {
        self.require_constant_protocol()?;
        let mut resolved = self
            .resolve_var_from_bytes(name, self.current_level())
            .ok_or_else(|| {
                self.constant_failure(
                    name,
                    "parent namespace doesn't exist",
                    tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                )
            })?;
        if resolved.elem.is_some() {
            let base = resolved
                .base_id
                .or_else(|| self.bind_new_var(&resolved.binding, Local::Undefined))
                .ok_or_else(|| {
                    self.refuse_host_command(
                        "native constant root allocation is unavailable".into(),
                    )
                })?;
            match self.var_arena.get(base).map(crate::vars::VarCell::state) {
                Some(Local::Undefined) => {
                    self.var_arena
                        .replace_state(base, Local::Array(super::VarTable::new()));
                }
                Some(Local::Array(_)) => {}
                _ => {
                    return Err(self.constant_failure(
                        name,
                        "variable isn't array",
                        tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                    ));
                }
            }
            resolved.base_id = Some(base);
            if resolved.id.is_none() && !create_element {
                return Err(self.constant_failure(
                    name,
                    "no such element in array",
                    tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                ));
            }
        }
        if resolved.id.is_none() {
            let id = self
                .ensure_target_var_at_binding(
                    &resolved.binding,
                    resolved
                        .elem
                        .as_ref()
                        .map(tcl_core_types::NameBytes::as_bytes),
                )
                .ok_or_else(|| {
                    self.refuse_host_command(
                        "native constant receiver lookup is unavailable".into(),
                    )
                })?;
            resolved.id = Some(id);
            resolved.base_id = if resolved.elem.is_some() {
                self.var_arena.element_parent(id).map(|(parent, _)| parent)
            } else {
                Some(id)
            };
        }
        self.declare_constant_resolved(name, &resolved, value)
    }

    pub(crate) fn declare_compiled_constant(
        &mut self,
        slot: usize,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        self.require_constant_protocol()?;
        let (name, resolved) = self.compiled_operand(slot, None)?;
        self.declare_constant_resolved(name.as_bytes(), &resolved, value)
    }

    fn constant_failure(
        &mut self,
        name: &[u8],
        reason: &str,
        site: tcl_syntax::naming::NativeVariableFailureSite,
    ) -> Completion<Value> {
        let Some(policy) = self.name_policy_protocol() else {
            return self.refuse_host_command("native constant name policy is unavailable".into());
        };
        let verb = match tcl_syntax::naming::native_constant_failure_verb(policy.recipe(), site) {
            Ok(verb) => verb,
            Err(error) => {
                return self.refuse_host_command(format!(
                    "native constant diagnostic is unavailable: {error:?}"
                ));
            }
        };
        self.variable_access_error_at(
            verb,
            tcl_syntax::naming::NativeVariableInputForm::Combined(name),
            reason,
            site,
        )
    }

    fn require_constant_protocol(&mut self) -> Result<(), Completion<Value>> {
        if self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .is_some_and(|protocol| {
                protocol
                    .tcl_version()
                    .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
            })
        {
            Ok(())
        } else {
            Err(self
                .refuse_host_command("native constant declaration protocol is unavailable".into()))
        }
    }

    fn declare_constant_resolved(
        &mut self,
        name: &[u8],
        resolved: &super::ResolvedVar,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        let id = resolved.id.ok_or_else(|| {
            self.refuse_host_command("native constant receiver is unavailable".into())
        })?;
        if self.const_vars.contains(&id) {
            return Ok(());
        }
        let reason = if resolved.elem.is_some() || self.var_arena.element_parent(id).is_some() {
            Some("name refers to an element in an array")
        } else {
            match self.var_arena.get(id).map(crate::vars::VarCell::state) {
                Some(Local::Array(_)) => Some("variable is array"),
                Some(Local::Scalar(_)) => Some("variable already exists"),
                Some(Local::Undefined) => None,
                _ => {
                    return Err(self.refuse_host_command("native constant receiver expired".into()));
                }
            }
        };
        if let Some(reason) = reason {
            return Err(self.constant_failure(
                name,
                reason,
                tcl_syntax::naming::NativeVariableFailureSite::ValueWrite,
            ));
        }
        let captured = self.capture_selected_update(name, None, resolved)?;
        self.with_variable_operation(&captured.cell, |vm| {
            vm.store_captured_update(name, None, &captured, value)?;
            vm.const_vars.insert(id);
            Ok(())
        })
    }

    /// Allocate every declared slot, retaining repeated names and undefined cells.
    pub(crate) fn install_compiled_local_slots(
        &mut self,
        names: &[NameBytes],
    ) -> Result<(), Completion<Value>> {
        let policy = self.compiled_variable_protocol().ok_or_else(|| {
            self.refuse_host_command("compiled frame slot policy is unavailable".into())
        })?;
        if !policy.has_indexed_locals() {
            return Ok(());
        }
        if !self
            .frames
            .last()
            .expect("activation")
            .compiled_locals
            .is_empty()
        {
            return Err(self.refuse_host_command("compiled frame slots already installed".into()));
        }
        for name in names {
            let Some(cell) = self.var_arena.alloc(Local::Undefined) else {
                return Err(self.refuse_host_command("compiled variable arena exhausted".into()));
            };
            self.var_arena.bind(cell);
            self.frames
                .last_mut()
                .expect("activation")
                .compiled_locals
                .push((name.clone(), cell));
        }
        Ok(())
    }

    pub(crate) fn install_compiled_local_layout(
        &mut self,
        layout: &tcl_runtime_api::native_compilation::NativeCompiledLocalLayout,
    ) -> Result<(), Completion<Value>> {
        if layout.owner != self.native_interpreter_identity() {
            return Err(
                self.refuse_host_command("compiled layout belongs to another interpreter".into())
            );
        }
        let policy = self.compiled_variable_protocol().ok_or_else(|| {
            self.refuse_host_command("compiled frame slot policy is unavailable".into())
        })?;
        if !policy.has_indexed_locals() {
            return Ok(());
        }
        let names = layout
            .names
            .iter()
            .map(|name| name.clone().unwrap_or_else(|| NameBytes::from(&b""[..])))
            .collect::<Vec<_>>();
        self.install_compiled_local_slots(&names)?;
        self.frames
            .last_mut()
            .expect("activation")
            .compiled_local_layout = Some(layout.clone());
        Ok(())
    }

    pub(crate) fn compiled_local_layout_matches(&self, asm: &tcl_bytecode::FunctionAsm) -> bool {
        match asm.required_compiled_local_layout.as_ref() {
            None => true,
            Some(required) => {
                required.owner == self.native_interpreter_identity()
                    && self
                        .frames
                        .last()
                        .and_then(|frame| frame.compiled_local_layout.as_ref())
                        == Some(required)
            }
        }
    }

    /// Bind the physical argument slot without replacing any same-name slot.
    pub(crate) fn link_compiled_upvar(
        &mut self,
        slot: usize,
        level: usize,
        other: &Value,
    ) -> Result<(), Completion<Value>> {
        let binding = self
            .compiled_local_binding(slot)
            .ok_or_else(|| self.refuse_host_command("compiled upvar local unavailable".into()))?;
        if self.native_c_variable_name_protocol().is_some() {
            return self.link_original_c_compiled_upvar(other, level, &binding);
        }
        let bytes = self
            .native_name_operand_bytes(other)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        self.link_upvar_with_binding_bytes(level, &bytes, binding.name.as_bytes(), Some(&binding))
            .map_err(|error| {
                crate::command::upvar_link_error_bytes(error, &bytes, binding.name.as_bytes())
            })
    }

    pub(crate) fn link_compiled_namespace_original(
        &mut self,
        slot: usize,
        namespace: tcl_core_types::NsId,
        original: &Value,
        declare: bool,
    ) -> Result<(), Completion<Value>> {
        let binding = self.compiled_local_binding(slot).ok_or_else(|| {
            self.refuse_host_command("compiled namespace local unavailable".into())
        })?;
        if self.native_c_variable_name_protocol().is_some() {
            return self.link_original_c_namespace_variable(original, namespace, &binding, declare);
        }
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let result = if declare {
            self.link_namespace_variable_with_binding_bytes(
                binding.name.as_bytes(),
                &bytes,
                Some(&binding),
            )
        } else {
            self.link_namespace_upvar_with_binding_bytes(
                namespace,
                &bytes,
                binding.name.as_bytes(),
                Some(&binding),
            )
        };
        result.map_err(|error| {
            crate::command::upvar_link_error_bytes(error, &bytes, binding.name.as_bytes())
        })
    }

    pub(crate) fn bind_compiled_formal_slot(
        &mut self,
        slot: usize,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        let Some(cell) = self
            .frames
            .last()
            .and_then(|frame| frame.compiled_locals.get(slot))
            .map(|(_, cell)| *cell)
        else {
            return Err(self.refuse_host_command("compiled formal slot is unavailable".into()));
        };
        self.var_arena.replace_state(cell, Local::Scalar(value));
        Ok(())
    }

    pub(super) fn compiled_local_dynamic_slot(&self, level: usize, name: &[u8]) -> Option<usize> {
        let recipe = self.compiled_variable_protocol()?.recipe();
        self.frames
            .get(level)?
            .compiled_locals
            .iter()
            .enumerate()
            .find(|(index, (stored, _))| {
                let named = self
                    .frames
                    .get(level)
                    .and_then(|frame| frame.compiled_local_layout.as_ref())
                    .is_none_or(|layout| layout.names.get(*index).is_some_and(Option::is_some));
                named && recipe.dynamic_local_names_equal(stored.as_bytes(), name)
            })
            .map(|(index, _)| index)
    }

    pub(super) fn raw_variable_binding(&self, binding: &VarBinding) -> Option<VarId> {
        match binding.owner {
            VarTableOwner::CompiledLocal { level, slot } => self
                .frames
                .get(level)?
                .compiled_locals
                .get(slot)
                .map(|(_, cell)| *cell),
            VarTableOwner::Frame(level) => self
                .compiled_local_dynamic_slot(level, binding.name.as_bytes())
                .and_then(|slot| {
                    self.frames[level]
                        .compiled_locals
                        .get(slot)
                        .map(|(_, cell)| *cell)
                })
                .or_else(|| self.frames.get(level)?.locals.get(&binding.name).copied()),
            _ => self.var_table(binding.owner)?.get(&binding.name).copied(),
        }
    }

    pub(crate) fn compiled_local_binding(&self, slot: usize) -> Option<VarBinding> {
        let level = self.current_level();
        let (name, _) = self.frames.get(level)?.compiled_locals.get(slot)?;
        Some(VarBinding {
            owner: VarTableOwner::CompiledLocal { level, slot },
            name: name.clone(),
        })
    }

    fn resolve_compiled_local(&self, slot: usize, element: Option<&[u8]>) -> Option<ResolvedVar> {
        self.resolve_binding_var(
            self.compiled_local_binding(slot)?,
            element.map(NameBytes::from),
            &mut std::collections::HashSet::new(),
        )
    }
    fn compiled_operand(
        &mut self,
        slot: usize,
        element: Option<&[u8]>,
    ) -> Result<(NameBytes, ResolvedVar), Completion<Value>> {
        let Some(binding) = self.compiled_local_binding(slot) else {
            return Err(self.refuse_host_command("compiled local receiver is unavailable".into()));
        };
        let name = binding.name.clone();
        let Some(resolved) = self.resolve_compiled_local(slot, element) else {
            return Err(
                self.refuse_host_command("compiled local receiver cannot be resolved".into())
            );
        };
        Ok((name, resolved))
    }

    pub(crate) fn read_compiled_variable_result(
        &mut self,
        slot: usize,
        element: Option<&[u8]>,
    ) -> Result<Value, Completion<Value>> {
        let (name, resolved) = self.compiled_operand(slot, element)?;
        self.read_selected_variable_result_bytes(name.as_bytes(), element, resolved)
    }

    pub(crate) fn store_compiled_variable_result(
        &mut self,
        slot: usize,
        element: Option<&[u8]>,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        let (name, resolved) = self.compiled_operand(slot, element)?;
        let captured = self.capture_selected_update(name.as_bytes(), element, &resolved)?;
        self.with_variable_operation(&captured.cell, |vm| {
            vm.store_captured_update(name.as_bytes(), element, &captured, value)
        })
    }

    pub(crate) fn exists_compiled_original_variable(
        &mut self,
        slot: usize,
        element: Option<&Value>,
    ) -> Result<bool, Completion<Value>> {
        let bytes = element
            .map(|element| self.native_name_operand_bytes(element))
            .transpose()
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let (name, mut resolved) = self.compiled_operand(slot, bytes.as_deref())?;
        self.prepare_existing_array_read(&mut resolved)?;
        if self
            .native_c_variable_name_protocol()
            .is_some_and(|protocol| protocol.element_table_retains_original())
        {
            if let (Some(array), Some(index), Some(original)) =
                (resolved.base_id, resolved.elem.as_ref(), element)
            {
                self.var_arena
                    .retain_original_array_key(array, index, original.clone());
            }
        }
        self.exists_selected_original_variable(name.as_bytes(), bytes.as_deref(), resolved, None)
    }

    pub(crate) fn unset_compiled_variable(
        &mut self,
        slot: usize,
        element: Option<&[u8]>,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        let (name, resolved) = self.compiled_operand(slot, element)?;
        let input = element.map_or(
            tcl_syntax::naming::NativeVariableInputForm::Combined(name.as_bytes()),
            |element| tcl_syntax::naming::NativeVariableInputForm::Separate {
                root: name.as_bytes(),
                element: Some(element),
            },
        );
        if resolved.id.is_some_and(|id| self.const_vars.contains(&id)) {
            return if complain {
                Err(self.variable_access_error_at(
                    "unset",
                    input,
                    "variable is a constant",
                    tcl_syntax::naming::NativeVariableFailureSite::ValueUnset,
                ))
            } else {
                Ok(())
            };
        }
        let reason = if resolved.elem.is_some() {
            match resolved
                .base_id
                .and_then(|id| self.var_arena.get(id))
                .map(crate::vars::VarCell::state)
            {
                Some(Local::Array(_)) => "no such element in array",
                Some(Local::Scalar(_)) => "variable isn't array",
                _ => "no such variable",
            }
        } else {
            "no such variable"
        };
        let existed = self.unset_resolved_checked(name.as_bytes(), &resolved)?;
        if let Some(refusal) = self.refused_completion() {
            return Err(refusal);
        }
        if !existed && complain {
            return Err(self.variable_access_error_input("unset", input, reason));
        }
        Ok(())
    }

    pub(crate) fn lappend_compiled_single(
        &mut self,
        slot: usize,
        element: Option<&[u8]>,
        addition: &Value,
    ) -> Result<Value, Completion<Value>> {
        let (name, captured) = self.capture_compiled_update(slot, element)?;
        self.lappend_instruction_single_captured(name.as_bytes(), element, &captured, addition)
    }

    pub(crate) fn ensure_array_compiled(&mut self, slot: usize) -> Result<(), Completion<Value>> {
        let (name, resolved) = self.compiled_operand(slot, None)?;
        self.ensure_array_original_selected_opcode(name.as_bytes(), &resolved)
    }

    pub(crate) fn array_exists_compiled(&mut self, slot: usize) -> Result<bool, Completion<Value>> {
        let (name, resolved) = self.compiled_operand(slot, None)?;
        let cell = self.trace_cell_from_resolved(&resolved);
        let check = |vm: &mut Self| {
            vm.fire_var_traces_from_cell_bytes(name.as_bytes(), "array", None, None, cell.clone())?;
            Ok(resolved
                .id
                .and_then(|id| vm.var_arena.get(id))
                .is_some_and(|cell| matches!(cell.state(), Local::Array(_))))
        };
        match cell.as_ref() {
            Some(cell) => self.with_variable_operation(cell, check),
            None => check(self),
        }
    }

    pub(super) fn capture_compiled_update(
        &mut self,
        slot: usize,
        element: Option<&[u8]>,
    ) -> Result<(NameBytes, CapturedVariableUpdate), Completion<Value>> {
        let (name, resolved) = self.compiled_operand(slot, element)?;
        let captured = self.capture_selected_update(name.as_bytes(), element, &resolved)?;
        Ok((name, captured))
    }
    pub(crate) fn increment_compiled_variable(
        &mut self,
        slot: usize,
        element: Option<&[u8]>,
        amount: &Value,
    ) -> Result<tcl_runtime_api::VariableUpdateResult<Value>, Completion<Value>> {
        let (name, resolved) = self.compiled_operand(slot, element)?;
        self.increment_selected_bytes(name.as_bytes(), element, amount, Some(resolved))
    }

    pub(crate) fn append_compiled_variable(
        &mut self,
        slot: usize,
        element: Option<&[u8]>,
        sources: &[Value],
    ) -> Result<Value, Completion<Value>> {
        let (name, resolved) = self.compiled_operand(slot, element)?;
        self.append_selected_bytes(name.as_bytes(), element, sources, Some(resolved))
    }
}

#[cfg(test)]
#[path = "native_compiled_locals/namespace_upvar_tests.rs"]
mod namespace_upvar_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::NativeCommand;
    use std::rc::Rc;
    use tcl_runtime_api::Code;
    use tcl_syntax::value::ValueOps;

    fn unhex(text: &str) -> Vec<u8> {
        assert!(text.len().is_multiple_of(2));
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    struct OriginalName(Value);
    impl NativeCommand for OriginalName {
        fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
            super::super::ok(self.0.clone())
        }
    }
    struct OriginalScript(Vec<u8>);
    impl NativeCommand for OriginalScript {
        fn invoke(&self, _vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            let mut bytes = b"set {".to_vec();
            bytes.extend_from_slice(&self.0);
            bytes.extend_from_slice(if args.is_empty() {
                b"};set {"
            } else {
                b"} ALTER;set {"
            });
            bytes.extend_from_slice(&self.0);
            bytes.push(b'}');
            super::super::ok(Value::from_native_string_bytes(bytes))
        }
    }

    fn install_original_name_providers(
        vm: &mut Vm,
        first: &[u8],
        second: &[u8],
        duplicate: bool,
    ) -> Vec<Value> {
        vm.register_native_command(
            "originalA",
            Rc::new(OriginalName(Value::from_native_string_bytes(
                first.to_vec(),
            ))),
        );
        vm.register_native_command(
            "originalB",
            Rc::new(OriginalName(Value::from_native_string_bytes(
                second.to_vec(),
            ))),
        );
        vm.register_native_command("originalBScript", Rc::new(OriginalScript(second.to_vec())));
        let mut formals = vec![Value::from_native_string_bytes(first.to_vec())];
        if duplicate {
            formals.push(Value::from_native_string_bytes(second.to_vec()));
        }
        formals
    }

    type BorrowedSourceNames = std::collections::BTreeMap<&'static str, (Vec<u8>, Vec<u8>)>;

    fn borrowed_source_names() -> BorrowedSourceNames {
        include_str!("../../tests/data/native_borrowed_frame_slots/names.tsv")
            .lines()
            .map(|row| {
                let mut fields = row.split('\t');
                (
                    fields.next().unwrap(),
                    (unhex(fields.next().unwrap()), unhex(fields.next().unwrap())),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    }

    fn borrowed_source_vm(
        profile: &'static tcl_dialect::DialectProfile,
        engine: &str,
        case: &str,
        receipt_start: std::time::Instant,
    ) -> Vm {
        let mut vm = Vm::new();
        tcl_test_support::oracle_phase_progress(
            "borrowed-source240",
            engine,
            case,
            "vm-new-complete",
            receipt_start,
        );
        vm.set_dialect_profile(profile);
        tcl_test_support::oracle_phase_progress(
            "borrowed-source240",
            engine,
            case,
            "profile-complete",
            receipt_start,
        );
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm
    }

    const BORROWED_SOURCE_FIXTURES: [(&str, &str); 6] = [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_borrowed_frame_slots/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_borrowed_frame_slots/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_borrowed_frame_slots/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_borrowed_frame_slots/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_borrowed_frame_slots/9.1.0.tsv"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_borrowed_frame_slots/Jim.tsv"),
        ),
    ];

    #[test]
    fn original_eval_and_uplevel_sources_match_240_borrowed_slot_native_references() {
        let names = borrowed_source_names();
        let body =
            unhex(include_str!("../../tests/data/native_borrowed_frame_slots/body.hex").trim());
        let engines = BORROWED_SOURCE_FIXTURES;
        let mut compared = 0;
        for (engine, expected) in engines {
            let rows = expected.lines().collect::<Vec<_>>();
            assert_eq!(rows.len(), 40);
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            for pair in rows.as_chunks::<2>().0 {
                let fields = pair[0].split('\t').collect::<Vec<_>>();
                let (first, second) = &names[fields[0]];
                let duplicate = fields[1] == "two-original-formals";
                let case = format!("{}/{}", fields[0], fields[1]);
                let receipt_start = std::time::Instant::now();
                tcl_test_support::oracle_row_progress("borrowed-source240", engine, &case, None);
                tcl_test_support::oracle_phase_progress(
                    "borrowed-source240",
                    engine,
                    &case,
                    "vm-new-start",
                    receipt_start,
                );
                let mut vm = borrowed_source_vm(profile, engine, &case, receipt_start);
                let formals = install_original_name_providers(&mut vm, first, second, duplicate);
                tcl_test_support::oracle_phase_progress(
                    "borrowed-source240",
                    engine,
                    &case,
                    "definition-start",
                    receipt_start,
                );
                let definition = vm.invoke_command(
                    "proc",
                    &[
                        Value::string("p"),
                        Value::list(formals),
                        Value::from_native_string_bytes(body.clone()),
                    ],
                );
                tcl_test_support::oracle_phase_progress(
                    "borrowed-source240",
                    engine,
                    &case,
                    "definition-complete",
                    receipt_start,
                );
                let mut arguments = vec![Value::string("ONE")];
                if duplicate {
                    arguments.push(Value::string("TWO"));
                }
                tcl_test_support::oracle_phase_progress(
                    "borrowed-source240",
                    engine,
                    &case,
                    "invoke-start",
                    receipt_start,
                );
                let invocation = vm.invoke_command("p", &arguments);
                tcl_test_support::oracle_phase_progress(
                    "borrowed-source240",
                    engine,
                    &case,
                    "invoke-complete",
                    receipt_start,
                );
                for (row, actual) in pair.iter().zip([definition, invocation]) {
                    let fields = row.split('\t').collect::<Vec<_>>();
                    assert_eq!(
                        actual.code.as_int(),
                        fields[3].parse::<i64>().unwrap(),
                        "{engine}/{}/{}/{}",
                        fields[0],
                        fields[1],
                        fields[2]
                    );
                    let bytes = vm.native_string_bytes(&actual.result).unwrap();
                    assert_eq!(
                        bytes.as_ref(),
                        unhex(fields[4]).as_slice(),
                        "{engine}/{}/{}/{}",
                        fields[0],
                        fields[1],
                        fields[2]
                    );
                    compared += 1;
                    tcl_test_support::oracle_row_progress(
                        "borrowed-source240",
                        engine,
                        &case,
                        Some(compared),
                    );
                }
            }
        }
        assert_eq!(compared, 240);
    }

    #[test]
    fn compiled_foreach_matches_twelve_original_native_slot_records() {
        let body = unhex(include_str!("../../tests/data/native_foreach_slots/body.hex").trim());
        let engines = [
            (
                "tcl8.4",
                include_str!("../../tests/data/native_foreach_slots/8.4.20.tsv"),
            ),
            (
                "tcl8.5",
                include_str!("../../tests/data/native_foreach_slots/8.5.19.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../../tests/data/native_foreach_slots/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_foreach_slots/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_foreach_slots/9.1.0.tsv"),
            ),
            (
                "jim",
                include_str!("../../tests/data/native_foreach_slots/Jim.tsv"),
            ),
        ];
        let mut compared = 0;
        for (engine, expected) in engines {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let first = Value::from_native_string_bytes(b"k\0a".to_vec());
            let second = Value::from_native_string_bytes(b"k\0b".to_vec());
            vm.register_native_command("originalA", Rc::new(OriginalName(first.clone())));
            vm.register_native_command("originalB", Rc::new(OriginalName(second.clone())));
            let definition = vm.invoke_command(
                "proc",
                &[
                    Value::string("p"),
                    Value::list(vec![first, second]),
                    Value::from_native_string_bytes(body.clone()),
                ],
            );
            let invocation = vm.invoke_command("p", &[Value::string("ONE"), Value::string("TWO")]);
            let rows = expected.lines().collect::<Vec<_>>();
            assert_eq!(rows.len(), 2);
            for (row, actual) in rows.iter().zip([definition, invocation]) {
                let fields = row.split('\t').collect::<Vec<_>>();
                assert_eq!(
                    actual.code.as_int(),
                    fields[3].parse::<i64>().unwrap(),
                    "{engine}/{}",
                    fields[2]
                );
                assert_eq!(
                    vm.native_string_bytes(&actual.result).unwrap().as_ref(),
                    unhex(fields[4]).as_slice(),
                    "{engine}/{}",
                    fields[2]
                );
                compared += 1;
            }
        }
        assert_eq!(compared, 12);
    }

    fn dictionary_search_window(
        mode: usize,
        stage: &str,
        root: Option<&Value>,
        key: &Value,
        value: &Value,
        selected: bool,
    ) -> String {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot;
        let kind = root.map_or("dropped", |root| {
            match root.native_object_snapshot().cache {
                NativeObjectCacheSnapshot::Dictionary { .. } => "dict",
                NativeObjectCacheSnapshot::List { .. } => "list",
                other => panic!("unexpected search root cache {other:?}"),
            }
        });
        format!(
            "{mode}\t{stage}\t{kind}\t{}\t{}\t{}\t{}\t{}",
            usize::from(root.is_some_and(|root| root.resident_string_bytes().is_some())),
            root.map_or(0, Value::native_object_reference_count),
            key.native_object_reference_count(),
            value.native_object_reference_count(),
            usize::from(selected)
        )
    }

    fn check_unavailable_dictionary_searches() {
        for engine in ["tcl8.4", "jim"] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let protocol = tcl_registry::InvocationDialect::of_profile(profile)
                .native_string_protocol()
                .unwrap();
            assert!(
                Value::from_native_dictionary_cache(Vec::new())
                    .into_native_dictionary_search(protocol)
                    .is_err(),
                "{engine}"
            );
        }
    }

    fn dictionary_epilogue_vm(
        profile: &'static tcl_dialect::DialectProfile,
        engine: &str,
        case: &str,
        receipt_start: std::time::Instant,
    ) -> Vm {
        let mut vm = Vm::new();
        tcl_test_support::oracle_phase_progress(
            "dictionary-epilogues35",
            engine,
            case,
            "vm-new-complete",
            receipt_start,
        );
        vm.set_dialect_profile(profile);
        tcl_test_support::oracle_phase_progress(
            "dictionary-epilogues35",
            engine,
            case,
            "profile-complete",
            receipt_start,
        );
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm
    }

    const DICTIONARY_SEARCH_FIXTURES: [(&str, &str); 4] = [
        (
            "tcl8.5",
            include_str!("../../tests/data/native_dictionary_search/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_dictionary_search/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_dictionary_search/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_dictionary_search/9.1.0.tsv"),
        ),
    ];

    #[test]
    fn original_dictionary_search_matches_100_native_storage_windows() {
        let engines = DICTIONARY_SEARCH_FIXTURES;
        let mut compared = 0;
        for (engine, expected) in engines {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let protocol = tcl_registry::InvocationDialect::of_profile(profile)
                .native_string_protocol()
                .unwrap();
            let expected = expected.lines().collect::<Vec<_>>();
            assert_eq!(expected.len(), 25);
            for mode in 0..5 {
                let key = Value::from_native_string_bytes(b"k\0x".to_vec());
                let value = Value::from_native_string_bytes(b"V\xff".to_vec());
                let root = Value::from_native_dictionary_cache(vec![(key.clone(), value.clone())]);
                let mut actual = vec![dictionary_search_window(
                    mode,
                    "before-search",
                    Some(&root),
                    &key,
                    &value,
                    false,
                )];
                let mut search = root.into_native_dictionary_search(protocol).unwrap();
                let (found_key, found_value) = search.next_pair().unwrap().unwrap();
                assert!(found_key.is_same_object(&key) && found_value.is_same_object(&value));
                drop((found_key, found_value));
                actual.push(dictionary_search_window(
                    mode,
                    "after-search",
                    Some(search.original_root()),
                    &key,
                    &value,
                    true,
                ));
                if mode == 1 || mode == 2 {
                    drop(
                        search
                            .original_root()
                            .native_object_list_elements(protocol)
                            .unwrap(),
                    );
                }
                if mode == 3 {
                    let copy = search.original_root().duplicate_native_object_in(protocol);
                    let changed = copy
                        .native_dictionary_set_member(
                            Value::string("other"),
                            Value::string("OTHER"),
                            protocol,
                        )
                        .unwrap();
                    drop(changed);
                    drop(copy);
                }
                if mode == 2 || mode == 4 {
                    assert!(search.next_pair().unwrap().is_none());
                }
                actual.push(dictionary_search_window(
                    mode,
                    "after-action",
                    Some(search.original_root()),
                    &key,
                    &value,
                    mode != 2 && mode != 4,
                ));
                search.close_search();
                actual.push(dictionary_search_window(
                    mode,
                    "after-done",
                    Some(search.original_root()),
                    &key,
                    &value,
                    false,
                ));
                drop(search);
                actual.push(dictionary_search_window(
                    mode,
                    "after-root-drop",
                    None,
                    &key,
                    &value,
                    false,
                ));
                for (actual, expected) in actual.iter().zip(&expected[mode * 5..mode * 5 + 5]) {
                    assert_eq!(actual, expected, "{engine}/{mode}");
                    compared += 1;
                }
            }
        }
        assert_eq!(compared, 100);
        check_unavailable_dictionary_searches();
    }

    fn dictionary_callback_vm(
        profile: &'static tcl_dialect::DialectProfile,
        engine: &str,
        case: &str,
        add: bool,
        receipt_start: std::time::Instant,
    ) -> Vm {
        let mut vm = Vm::new();
        tcl_test_support::oracle_phase_progress(
            "dictionary-callback14",
            engine,
            case,
            "vm-new-complete",
            receipt_start,
        );
        vm.set_dialect_profile(profile);
        tcl_test_support::oracle_phase_progress(
            "dictionary-callback14",
            engine,
            case,
            "profile-complete",
            receipt_start,
        );
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm.register_native_command(
            "advance",
            crate::native_conformance::dictionary_search_mutation_driver(
                tcl_registry::InvocationDialect::of_profile(profile),
                add,
            )
            .unwrap(),
        );
        vm
    }

    #[test]
    fn mutated_dictionary_callback_search_preserves_fourteen_native_fatal_boundaries() {
        let mut compared = 0;
        for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            for add in [false, true] {
                let scripts: &[&[u8]] = if engine == "tcl8.5" {
                    &[b"catch {advance};set reached yes"]
                } else {
                    &[
                        b"catch {advance};set reached yes",
                        b"try {advance} on error {r o} {set caught yes};set reached yes",
                    ]
                };
                for source in scripts {
                    let case = format!(
                        "{}/{:}",
                        if add { "add" } else { "replace" },
                        if source.starts_with(b"catch") {
                            "catch"
                        } else {
                            "try"
                        }
                    );
                    let receipt_start = std::time::Instant::now();
                    tcl_test_support::oracle_row_progress(
                        "dictionary-callback14",
                        engine,
                        &case,
                        None,
                    );
                    tcl_test_support::oracle_phase_progress(
                        "dictionary-callback14",
                        engine,
                        &case,
                        "vm-new-start",
                        receipt_start,
                    );
                    let mut vm = dictionary_callback_vm(profile, engine, &case, add, receipt_start);
                    tcl_test_support::oracle_phase_progress(
                        "dictionary-callback14",
                        engine,
                        &case,
                        "eval-start",
                        receipt_start,
                    );
                    let error = vm.try_eval_source_bytes(source).unwrap_err();
                    tcl_test_support::oracle_phase_progress(
                        "dictionary-callback14",
                        engine,
                        &case,
                        "eval-complete",
                        receipt_start,
                    );
                    assert!(
                        matches!(
                            error,
                            tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_)
                        ),
                        "{engine}: {error:?}"
                    );
                    assert!(vm.get_var_bytes(b"reached").is_none());
                    assert!(vm.get_var_bytes(b"caught").is_none());
                    drop(vm);
                    tcl_test_support::oracle_phase_progress(
                        "dictionary-callback14",
                        engine,
                        &case,
                        "vm-drop-complete",
                        receipt_start,
                    );
                    compared += 1;
                    tcl_test_support::oracle_row_progress(
                        "dictionary-callback14",
                        engine,
                        &case,
                        Some(compared),
                    );
                }
            }
        }
        assert_eq!(compared, 14);
    }

    type SearchReferenceObservations = Rc<std::cell::RefCell<Vec<(Vec<u8>, usize)>>>;

    struct CaptureSearchReferences(SearchReferenceObservations);
    impl NativeCommand for CaptureSearchReferences {
        fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            assert_eq!(args.len(), 2);
            let label = vm.native_string_bytes(&args[0]).unwrap().to_vec();
            self.0
                .borrow_mut()
                .push((label, args[1].native_object_reference_count()));
            super::super::ok(Value::empty())
        }
    }

    fn check_compiled_dictionary_epilogue(engine: &str, row: &str) {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let fields = row.split('\t').collect::<Vec<_>>();
        let (family, action) = fields[0].split_once('-').unwrap();
        let body = match action {
            "normal" => "set ignored OK",
            "error" => "error BODY",
            "break" => "break",
            "continue" => "continue",
            "return" => "return BODY",
            _ => unreachable!(),
        };
        let source = format!(
            "proc p {{}} {{set d {{k V}}; capture before $d;set c [catch {{dict {family} {{k v}} $d {{capture during $d;{body}}}}} r o];capture after $d;list $c $r [dict get $o -code] [dict get $o -level] $o}};p"
        );
        let receipt_start = std::time::Instant::now();
        tcl_test_support::oracle_row_progress("dictionary-epilogues35", engine, fields[0], None);
        tcl_test_support::oracle_phase_progress(
            "dictionary-epilogues35",
            engine,
            fields[0],
            "vm-new-start",
            receipt_start,
        );
        let mut vm = dictionary_epilogue_vm(profile, engine, fields[0], receipt_start);
        let windows = Rc::new(std::cell::RefCell::new(Vec::new()));
        vm.register_native_command(
            "capture",
            Rc::new(CaptureSearchReferences(Rc::clone(&windows))),
        );
        tcl_test_support::oracle_phase_progress(
            "dictionary-epilogues35",
            engine,
            fields[0],
            "eval-start",
            receipt_start,
        );
        let completion = vm.try_eval_source_bytes(source.as_bytes()).unwrap();
        tcl_test_support::oracle_phase_progress(
            "dictionary-epilogues35",
            engine,
            fields[0],
            "eval-complete",
            receipt_start,
        );
        assert_eq!(
            completion.code.as_int(),
            fields[1].parse::<i64>().unwrap(),
            "{engine}/{}",
            fields[0]
        );
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            unhex(fields[2]).as_slice(),
            "{engine}/{}",
            fields[0]
        );
        let observations = Value::list(
            windows
                .borrow()
                .iter()
                .map(|(label, count)| {
                    Value::list(vec![
                        Value::from_native_string_bytes(label.clone()),
                        Value::int(i64::try_from(*count).unwrap()),
                    ])
                })
                .collect(),
        );
        assert_eq!(
            vm.actual_dictionary_search_entries, 1,
            "{engine}/{} must execute the selected compiled search",
            fields[0]
        );
        assert_eq!(
            vm.native_string_bytes(&observations).unwrap().as_ref(),
            unhex(fields[3]).as_slice(),
            "{engine}/{}",
            fields[0]
        );
        let procedure = vm.proc_def_bytes(b"p").unwrap();
        assert!(
            procedure
                .body
                .as_ref()
                .expect("actual compiled procedure activation")
                .asm
                .instructions
                .iter()
                .any(|instruction| instruction.op == tcl_bytecode::Op::DICT_FIRST),
            "{engine}/{} must exercise a compiled search",
            fields[0]
        );
        drop(vm);
        tcl_test_support::oracle_phase_progress(
            "dictionary-epilogues35",
            engine,
            fields[0],
            "vm-drop-complete",
            receipt_start,
        );
    }

    #[test]
    fn compiled_dictionary_c86_for_error_preserves_original_completion_and_reference_windows() {
        let row = include_str!("../../tests/data/native_dictionary_search/epilogues/8.6.18.tsv")
            .lines()
            .find(|row| row.starts_with("for-error\t"))
            .expect("original native C86 for-error row");
        check_compiled_dictionary_epilogue("tcl8.6", row);
    }

    #[test]
    fn compiled_dictionary_epilogues_match_35_native_completions_and_reference_windows() {
        let engines = [
            (
                "tcl8.5",
                include_str!("../../tests/data/native_dictionary_search/epilogues/8.5.19.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../../tests/data/native_dictionary_search/epilogues/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_dictionary_search/epilogues/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_dictionary_search/epilogues/9.1.0.tsv"),
            ),
        ];
        let mut compared = 0;
        for (engine, expected) in engines {
            for row in expected.lines() {
                let fields = row.split('\t').collect::<Vec<_>>();
                check_compiled_dictionary_epilogue(engine, row);
                compared += 1;
                tcl_test_support::oracle_row_progress(
                    "dictionary-epilogues35",
                    engine,
                    fields[0],
                    Some(compared),
                );
            }
        }
        assert_eq!(compared, 35);
    }

    #[test]
    fn reusable_layout_keeps_activations_separate_and_rejects_replacement() {
        let mut vm = Vm::new();
        let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
        vm.set_dialect_profile(profile);
        let asm = tcl_bytecode::FunctionAsm {
            lvt: tcl_bytecode::LocalVarTable::from_native_names(&[
                NameBytes::from("x"),
                NameBytes::from("x"),
            ]),
            ..tcl_bytecode::FunctionAsm::default()
        };
        let unit = vm.compiled_unit(
            Rc::new(asm.clone()),
            tcl_core_types::ByteNamespacePath::root(),
        );
        vm.push_call_frame(Some("p".into()), Vec::new());
        vm.install_compiled_local_layout(
            unit.compiled_local_layout
                .as_ref()
                .expect("real compiled layout"),
        )
        .unwrap();
        vm.bind_compiled_formal_slot(0, Value::int(1)).unwrap();
        vm.bind_compiled_formal_slot(1, Value::int(2)).unwrap();
        let first = vm
            .raw_variable_binding(&vm.compiled_local_binding(0).unwrap())
            .unwrap();
        let second = vm
            .raw_variable_binding(&vm.compiled_local_binding(1).unwrap())
            .unwrap();
        assert_ne!(first, second);
        let mut borrowed = asm.clone();
        borrowed.required_compiled_local_layout = unit.compiled_local_layout.clone();
        assert!(vm.compiled_local_layout_matches(&borrowed));
        vm.pop_call_frame();
        vm.push_call_frame(Some("p".into()), Vec::new());
        vm.install_compiled_local_layout(
            unit.compiled_local_layout
                .as_ref()
                .expect("real compiled layout"),
        )
        .unwrap();
        assert!(vm.compiled_local_layout_matches(&borrowed));
        assert_ne!(
            first,
            vm.raw_variable_binding(&vm.compiled_local_binding(0).unwrap())
                .unwrap()
        );
        vm.pop_call_frame();
        let replacement = vm.compiled_unit(Rc::new(asm), tcl_core_types::ByteNamespacePath::root());
        vm.push_call_frame(Some("p".into()), Vec::new());
        vm.install_compiled_local_layout(
            replacement
                .compiled_local_layout
                .as_ref()
                .expect("real compiled layout"),
        )
        .unwrap();
        assert!(!vm.compiled_local_layout_matches(&borrowed));
        assert_eq!(
            vm.read_compiled_variable_result(0, None).unwrap_err().code,
            Code::Error
        );
        vm.pop_call_frame();
        vm.set_local_bytes(b"x", Value::int(9));
        assert!(!vm.compiled_local_layout_matches(&borrowed));
    }
}
