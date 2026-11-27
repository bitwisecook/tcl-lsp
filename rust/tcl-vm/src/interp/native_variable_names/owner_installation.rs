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

//! Installing the native variable-name owners and the procedure, compiled and selected variable caches of an activation.

use super::super::{Value, VarBinding, VarTableOwner, Vm};
use crate::command::completion_from_cmd_error;
use std::rc::Rc;
use tcl_runtime_api::Completion;
use tcl_syntax::native_variable_name::{
    NativeLocalVariableName, NativeLocalVariableOwner, NativeParsedVariableName,
};
use tcl_syntax::value::ValueError;

impl Vm {
    fn publish_original_local_name_table(
        original: &super::super::native_command_names::OriginalLiteralOperation<'_>,
        level: usize,
        names: Option<Rc<crate::literal_pool::NativeLocalNameTable>>,
    ) -> Result<(), Completion<Value>> {
        original
            .with_vm(|vm| {
                if vm.frames[level].native_local_names.is_some() {
                    let error: tcl_cmd_core::CmdError = ValueError::CommandProtocolUnavailable(
                        "replaced original frame local-name slot",
                    )
                    .into();
                    if let Some(first) = error.native_execution_refusal() {
                        vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                            first.clone(),
                        ));
                    }
                    return Err(crate::literal_pool::NativeLiteralUnavailable::unavailable(
                        "replaced original frame local-name slot",
                    ));
                }
                vm.frames[level].native_local_names = names;
                Ok(())
            })
            .map_err(|_| super::super::err(""))
    }

    pub(crate) fn install_native_variable_name_owners(
        &mut self,
        procedure: &Rc<crate::command::ProcDef>,
        body: &crate::compiled::CompiledUnit,
    ) -> Result<(), Completion<Value>> {
        // A frame the authored simulation activates owns its names under that
        // simulation's own C recipe.
        let Some(protocol) = self
            .native_c_variable_name_protocol()
            .or_else(|| self.authored_c_variable_name_protocol())
        else {
            return Ok(());
        };
        procedure.retain_native_compiled_names(body.asm.lvt.native_slot_names());
        if protocol.local_cache_owns_procedure() {
            self.frames
                .last_mut()
                .expect("actual procedure activation")
                .native_c_procedure = Some(Rc::clone(procedure));
            return Ok(());
        }
        let world = Rc::clone(&self.native_literal_world);
        let level = self.frames.len() - 1;
        let original = super::super::native_command_names::OriginalLiteralOperation::capture(self);
        let before = body.native_local_names.borrow().as_ref().map(Rc::downgrade);
        let check = || {
            original.check()?;
            let selected = body.native_local_names.borrow();
            let same = match (&before, selected.as_ref()) {
                (None, None) => true,
                (Some(before), Some(current)) => {
                    std::rc::Weak::ptr_eq(before, &Rc::downgrade(current))
                }
                _ => false,
            };
            if !same {
                return Err(original.refuse(ValueError::CommandProtocolUnavailable(
                    "original compiled local-name publication slot",
                )));
            }
            Ok(())
        };
        let string = |value: &Value,
                      protocol: tcl_syntax::native_string::NativeStringProtocol,
                      registration: &(
                           dyn Fn() -> Result<(), crate::literal_pool::NativeLiteralUnavailable>
                               + '_
                       )| {
            check()?;
            original.string(value, protocol, &|| {
                registration()?;
                check()
            })
        };
        let refuse = |error| original.refuse(error);
        let callbacks = crate::literal_pool::OriginalLiteralCallbacks {
            check: &check,
            string: &string,
            refuse: &refuse,
            objects: None,
        };
        check().map_err(|_| super::super::err(""))?;
        if before.is_none() {
            let names = crate::literal_pool::create_local_names_using(
                &world,
                &body.asm.lvt.native_slot_names(),
                tcl_syntax::native_string::NativeStringProtocol::C(protocol.version()),
                Some(&callbacks),
            )
            .map_err(|_| super::super::err(""))?;
            check().map_err(|_| super::super::err(""))?;
            let retired = body.native_local_names.borrow_mut().replace(names);
            drop(retired);
            original.check().map_err(|_| super::super::err(""))?;
        }
        let names = body.native_local_names.borrow().clone();
        let selected_names = names.as_ref().map(Rc::downgrade);
        let retired = original
            .with_vm(|vm| {
                Ok(vm
                    .frames
                    .get_mut(level)
                    .expect("same original local-name frame")
                    .native_local_names
                    .take())
            })
            .map_err(|_| super::super::err(""))?;
        drop(retired);
        original.check().map_err(|_| super::super::err(""))?;
        let selected = body.native_local_names.borrow();
        let same = match (&selected_names, selected.as_ref()) {
            (Some(before), Some(current)) => std::rc::Weak::ptr_eq(before, &Rc::downgrade(current)),
            (None, None) => true,
            _ => false,
        };
        drop(selected);
        if !same {
            return Err({
                original.refuse(ValueError::CommandProtocolUnavailable(
                    "changed original compiled local-name table",
                ));
                super::super::err("")
            });
        }
        Self::publish_original_local_name_table(&original, level, names)?;

        Ok(())
    }

    pub(super) fn native_original_local_binding(&self, original: &Value) -> Option<VarBinding> {
        let frame = self.frames.last()?;
        original
            .with_native_local_variable(|cache| {
                let current = self.native_c_variable_name_protocol()?;
                if current != cache.protocol || cache.index >= frame.compiled_locals.len() {
                    return None;
                }
                let matches = match &cache.owner {
                    NativeLocalVariableOwner::Procedure(procedure) => frame
                        .native_c_procedure
                        .as_ref()
                        .is_some_and(|current| Rc::ptr_eq(procedure.declaration(), current)),
                    NativeLocalVariableOwner::Name(canonical) => frame
                        .native_local_names
                        .as_ref()?
                        .names
                        .get(cache.index)?
                        .as_ref()
                        .is_some_and(|current| {
                            current.is_same_object(canonical.as_ref().unwrap_or(original))
                        }),
                };
                matches.then(|| VarBinding {
                    owner: VarTableOwner::CompiledLocal {
                        level: self.current_level(),
                        slot: cache.index,
                    },
                    name: frame.compiled_locals[cache.index].0.clone(),
                })
            })
            .flatten()
    }

    fn install_original_procedure_variable_cache(
        &mut self,
        original: &Value,
        coordinates: (usize, usize),
        selected: (
            tcl_syntax::native_variable_name::NativeVariableNameProtocol,
            tcl_native_model::InvocationDialect,
        ),
        original_names: &super::super::native_name_consumers::OriginalNameConsumerScope,
        check_binding: &impl Fn(&mut Vm) -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let (level, slot) = coordinates;
        let (protocol, dialect) = selected;
        let procedure = self
            .frames
            .get(level)
            .and_then(|frame| frame.native_c_procedure.clone())
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual C8.4 local procedure owner unavailable",
            ))?;
        original.install_native_local_variable_checked(
            dialect,
            || {
                Ok(NativeLocalVariableName {
                    protocol,
                    index: slot,
                    owner: NativeLocalVariableOwner::Procedure(
                        crate::command::NativeProcedureReference::acquire(&procedure),
                    ),
                })
            },
            &mut || {
                check_binding(self)?;
                let current = self
                    .frames
                    .get(level)
                    .and_then(|frame| frame.native_c_procedure.as_ref())
                    .is_some_and(|actual| Rc::ptr_eq(actual, &procedure));
                original_names.require(self, current, "original C local procedure cache owner")
            },
        )
    }

    fn install_original_compiled_variable_cache(
        &mut self,
        original: &Value,
        coordinates: (usize, usize),
        selected: (
            tcl_syntax::native_variable_name::NativeVariableNameProtocol,
            tcl_native_model::InvocationDialect,
        ),
        original_names: &super::super::native_name_consumers::OriginalNameConsumerScope,
        check_binding: &impl Fn(&mut Vm) -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let (level, slot) = coordinates;
        let (protocol, dialect) = selected;
        // The procedure owns the names of a frame activated under another
        // realm's recipe; this realm installs no cache in it.
        if !protocol.local_cache_owns_procedure()
            && self
                .frames
                .get(level)
                .is_some_and(|frame| frame.native_c_procedure.is_some())
        {
            return Ok(());
        }
        if protocol.local_cache_owns_procedure() {
            return self.install_original_procedure_variable_cache(
                original,
                (level, slot),
                (protocol, dialect),
                original_names,
                check_binding,
            );
        }
        let canonical = self
            .frames
            .get(level)
            .and_then(|frame| frame.native_local_names.as_ref())
            .and_then(|table| table.names.get(slot))
            .and_then(Option::as_ref)
            .map(Value::native_lifetime_lease)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual canonical local name unavailable",
            ))?;
        let canonical = canonical.value();
        let same = canonical.is_same_object(original);
        let owner = if same {
            NativeLocalVariableOwner::Name(None)
        } else {
            NativeLocalVariableOwner::Name(Some(canonical.clone()))
        };
        // C9's LocalSet prepares this actual incoming name reference before
        // the same StoreInternalRep free/adoption worker. C8 cleared the
        // root before lookup, so this role still follows that retirement.
        original.install_native_local_variable_checked(
            dialect,
            || {
                Ok(NativeLocalVariableName {
                    protocol,
                    index: slot,
                    owner,
                })
            },
            &mut || {
                check_binding(self)?;
                original_names.current_for_value(self, canonical)?;
                let current = self
                    .frames
                    .get(level)
                    .and_then(|frame| frame.native_local_names.as_ref())
                    .and_then(|table| table.names.get(slot))
                    .and_then(Option::as_ref)
                    .is_some_and(|actual| actual.is_same_object(canonical));
                original_names.require(self, current, "original canonical local cache owner")
            },
        )?;
        let check_canonical = |vm: &mut Vm| -> Result<(), tcl_cmd_core::CmdError> {
            check_binding(vm)?;
            original_names.current_for_value(vm, canonical)?;
            let current = vm
                .frames
                .get(level)
                .and_then(|frame| frame.native_local_names.as_ref())
                .and_then(|table| table.names.get(slot))
                .and_then(Option::as_ref)
                .is_some_and(|actual| actual.is_same_object(canonical));
            original_names.require(vm, current, "original canonical local cache owner")
        };
        if !same && protocol.installs_canonical_local_self_cache() {
            canonical.install_native_local_variable_checked(
                dialect,
                || {
                    Ok(NativeLocalVariableName {
                        protocol,
                        index: slot,
                        owner: NativeLocalVariableOwner::Name(None),
                    })
                },
                &mut || check_canonical(self),
            )?;
        } else if protocol.version() == tcl_dialect::TclVersion::V8_6
            && !same
            && !canonical
                .with_native_local_variable(|cache| {
                    matches!(cache.owner, NativeLocalVariableOwner::Name(None))
                })
                .unwrap_or(false)
        {
            canonical.retire_native_variable_primary_checked(&mut || check_canonical(self))?;
        }
        Ok(())
    }

    pub(super) fn install_selected_original_variable_cache(
        &mut self,
        original: &Value,
        binding: &VarBinding,
        original_names: &super::super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<(), Completion<Value>> {
        use tcl_cmd_core::CmdError;
        if self.native_original_local_binding(original).as_ref() == Some(binding) {
            return Ok(());
        }
        let dialect = self.actual_native_invocation_dialect();
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("selected actual C lookup");
        let raw = self.raw_variable_binding(binding).ok_or_else(|| {
            completion_from_cmd_error(
                self,
                tcl_syntax::value::ValueError::CommandProtocolUnavailable("original C cache cell")
                    .into(),
            )
        })?;
        let cell = self.var_arena.weak_jim_cell(raw).ok_or_else(|| {
            completion_from_cmd_error(
                self,
                tcl_syntax::value::ValueError::CommandProtocolUnavailable("original C cache birth")
                    .into(),
            )
        })?;
        let check_binding = |vm: &mut Vm| -> Result<(), CmdError> {
            original_names.current_for_value(vm, original)?;
            let current = vm.raw_variable_binding(binding) == Some(raw)
                && vm.var_arena.validate_jim_cell(&cell) == Some(raw);
            original_names.require(vm, current, "original C variable cache binding")
        };
        let reached: Result<(), CmdError> = (|| {
            if let VarTableOwner::CompiledLocal { level, slot } = binding.owner {
                return self.install_original_compiled_variable_cache(
                    original,
                    (level, slot),
                    (protocol, dialect),
                    original_names,
                    &check_binding,
                );
            }
            original.install_native_parsed_variable_checked(
                dialect,
                || {
                    Ok(NativeParsedVariableName {
                        protocol,
                        array: None,
                    })
                },
                &mut || check_binding(self),
            )
        })();
        original_names
            .check(self)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        reached.map_err(|error| completion_from_cmd_error(self, error))
    }
}
