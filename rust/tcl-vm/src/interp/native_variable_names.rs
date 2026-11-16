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
    NativeLocalVariableName, NativeLocalVariableOwner, NativeParsedVariableElement,
    NativeParsedVariableName, NativeVariableNameLookupPurpose,
};
use tcl_syntax::value::ValueError;

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
        tcl_registry::InvocationDialect::for_version(version).native_variable_name_protocol()
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

    /// Tcl 8.4's `Tcl_SetObjErrorCode` uses a bare `CString` with `GLOBAL_ONLY`,
    /// without an object-name getter or `TCL_LEAVE_ERR_MSG`.
    pub(super) fn store_original_c84_global_error_code(
        &mut self,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        if !self.uses_c84_global_error_info()
            || self
                .native_c_variable_name_protocol()
                .is_none_or(|protocol| protocol.version() != tcl_dialect::TclVersion::V8_4)
        {
            return Err(self.refuse_host_command(
                "actual C8.4 CString global error-code setter unavailable".into(),
            ));
        }
        let original_names =
            super::native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
                self,
                Some(tcl_core_types::ROOT_NS),
            )
            .map_err(|error| completion_from_cmd_error(self, error))?;
        let reached = (|| {
            original_names
                .current_for_value(self, &value)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            let name = tcl_registry::special_vars::NativeBootstrapVariable::ErrorCode
                .name()
                .as_bytes();
            let binding = self
                .namespace_var_binding_from_token_bytes(tcl_core_types::ROOT_NS, name)
                .ok_or_else(|| {
                    self.refuse_host_command("actual C8.4 global error-code binding".into())
                })?;
            let resolved = self
                .resolve_binding_var(binding, None, &mut HashSet::new())
                .ok_or_else(|| {
                    self.refuse_host_command("actual C8.4 global error-code receiver".into())
                })?;
            let purpose = NativeVariableNameLookupPurpose::QuietWrite;
            let captured = self.capture_selected_update_with_errors(
                name,
                None,
                &resolved,
                purpose.leaves_error_message(),
            )?;
            self.with_variable_operation(&captured.cell, |vm| {
                vm.store_captured_update_with_original_and_errors(
                    name,
                    None,
                    &captured,
                    value,
                    None,
                    purpose.leaves_error_message(),
                )
                .map(|_| ())
            })
        })();
        original_names
            .finish(self)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        reached
    }

    fn publish_original_local_name_table(
        original: &super::native_command_names::OriginalLiteralOperation<'_>,
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
            .map_err(|_| super::err(""))
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
        let original = super::native_command_names::OriginalLiteralOperation::capture(self);
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
        check().map_err(|_| super::err(""))?;
        if before.is_none() {
            let names = crate::literal_pool::create_local_names_using(
                &world,
                &body.asm.lvt.native_slot_names(),
                tcl_syntax::native_string::NativeStringProtocol::C(protocol.version()),
                Some(&callbacks),
            )
            .map_err(|_| super::err(""))?;
            check().map_err(|_| super::err(""))?;
            let retired = body.native_local_names.borrow_mut().replace(names);
            drop(retired);
            original.check().map_err(|_| super::err(""))?;
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
            .map_err(|_| super::err(""))?;
        drop(retired);
        original.check().map_err(|_| super::err(""))?;
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
                super::err("")
            });
        }
        Self::publish_original_local_name_table(&original, level, names)?;

        Ok(())
    }

    fn native_original_local_binding(&self, original: &Value) -> Option<VarBinding> {
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
            tcl_registry::InvocationDialect,
        ),
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
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
            tcl_registry::InvocationDialect,
        ),
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
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

    fn install_selected_original_variable_cache(
        &mut self,
        original: &Value,
        binding: &VarBinding,
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
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
        dialect: tcl_registry::InvocationDialect,
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

    pub(super) fn link_original_c_namespace_variable(
        &mut self,
        original: &Value,
        namespace: tcl_core_types::NsId,
        local: &VarBinding,
        declare: bool,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            super::native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
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
                .bind_variable_link(local, super::VariableLinkTarget::Cell(target))
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
        cause: super::UpvarLinkError,
    ) -> Completion<Value> {
        if self.refused_completion().is_some() {
            return failure;
        }
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C alias diagnostic");
        let code = if matches!(
            cause,
            super::UpvarLinkError::TargetNamespace | super::UpvarLinkError::LocalNamespace
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

    fn bind_original_c_alias_local(
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
    ) -> Result<(), (Completion<Value>, Option<super::UpvarLinkError>)> {
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
                return Err(failure(super::UpvarLinkError::Inverted));
            }
            if protocol.alias_local_is_element(&bytes) {
                return Err(failure(super::UpvarLinkError::LocalElement));
            }
            let binding = if namespace {
                self.namespace_var_binding_from_token_bytes(self.current_ns_id(), input.selected())
                    .ok_or_else(|| failure(super::UpvarLinkError::LocalNamespace))?
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
                        .ok_or_else(|| failure(super::UpvarLinkError::LocalNamespace))?
                        .retain_original_native_key(&binding.name, key);
                }
            }
            self.bind_variable_link(&binding, super::VariableLinkTarget::Cell(target))
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
    fn settle_original_c_link_target(
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
                    .is_some_and(|cell| matches!(cell.state(), super::Local::Scalar(_)))
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

    pub(super) fn link_original_c_compiled_upvar(
        &mut self,
        original: &Value,
        target_level: usize,
        local: &VarBinding,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            super::native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
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
                self.bind_variable_link(local, super::VariableLinkTarget::Cell(target))
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

    pub(super) fn read_original_c_variable(
        &mut self,
        original: &Value,
    ) -> Result<Value, Completion<Value>> {
        let (_root, bytes, element, resolved) =
            self.prepare_native_original_variable(original, NativeVariableNameLookupPurpose::Read)?;
        self.read_selected_variable_result_bytes_with_original(
            &bytes,
            element.as_deref(),
            resolved,
            Some(original),
        )
    }
    /// Native object access separates the fresh qualified lookup object from
    /// the retained clientData object used for `PtrGetVar` diagnostics and traces.
    pub(crate) fn read_original_c_property_variable(
        &mut self,
        lookup: &Value,
        reported: &Value,
    ) -> Result<Value, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let (_root, _bytes, element, resolved) = self
                .prepare_native_original_variable(lookup, NativeVariableNameLookupPurpose::Link)?;
            let bytes = original_names
                .read(self, reported)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            self.read_selected_variable_result_bytes_with_original(
                &bytes,
                element.as_deref(),
                resolved,
                Some(reported),
            )
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    /// Native `PropertySetter` uses the selected cell without publishing its value
    /// as the interpreter result; the enclosing configure operation resets it.
    pub(crate) fn store_original_c_property_variable(
        &mut self,
        lookup: &Value,
        reported: &Value,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let (_root, _bytes, element, resolved) = self
                .prepare_native_original_variable(lookup, NativeVariableNameLookupPurpose::Write)?;
            let bytes = original_names
                .read(self, reported)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let captured = self.capture_selected_update_with_errors(
                &bytes,
                element.as_deref(),
                &resolved,
                true,
            )?;
            self.with_variable_operation(&captured.cell, |vm| {
                vm.store_captured_update_with_original_and_errors(
                    &bytes,
                    element.as_deref(),
                    &captured,
                    value,
                    Some(reported),
                    true,
                )
                .map(|_| ())
            })
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    pub(crate) fn store_original_regex_variable(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let Some(protocol) = self.native_c_variable_name_protocol() else {
                return self.store_original_named_variable(original, value);
            };
            let purpose = protocol.regex_write_purpose();
            match self.store_original_c_variable_for(original, value, purpose) {
                Ok(value) => Ok(value),
                Err(failure)
                    if !purpose.leaves_error_message() && self.refused_completion().is_none() =>
                {
                    let bytes = original_names
                        .read(self, original)
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                    let current = self
                        .with_native_interp_result(|value| {
                            value.native_lifetime_lease().into_value()
                        })
                        .map_err(|error| {
                            crate::command::completion_from_cmd_error(self, error.into())
                        })?;
                    let mut message = original_names
                        .read(self, &current)
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
                        .to_vec();
                    message.extend_from_slice(b"couldn't set variable \"");
                    message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
                    message.push(b'"');
                    let mut failure = failure;
                    failure.result = Value::from_string_bytes(message);
                    if protocol.diagnostic_string_protocol().is_some() {
                        let recipe = self
                            .actual_native_invocation_dialect()
                            .native_string_materialization(None)
                            .ok_or_else(|| {
                                self.refuse_host_command(
                                    "actual variable-error String producer".into(),
                                )
                            })?;
                        failure
                            .result
                            .retain_native_string_representation(recipe)
                            .map_err(|error| {
                                crate::command::completion_from_cmd_error(self, error.into())
                            })?;
                    }
                    Err(failure)
                }
                Err(failure) => Err(failure),
            }
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    pub(super) fn store_original_c_variable(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        self.store_original_c_variable_for(original, value, NativeVariableNameLookupPurpose::Write)
    }

    /// The array worker supplies two actual operands, independently of combined
    /// name parsing. The existing lookup and captured write own every effect.
    pub(crate) fn store_original_c_array_member(
        &mut self,
        original: &Value,
        key: &Value,
        value: &Value,
    ) -> Result<Value, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            use super::native_operation_currency::NativeOperationScope;
            use crate::command::completion_from_cmd_error;
            let scope = NativeOperationScope::enter(self)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            let root_lifetime = original.native_lifetime_lease();
            let key_lifetime = key.native_lifetime_lease();
            let result = (|| {
                if self.native_c_variable_name_protocol().is_none() {
                    return Err(completion_from_cmd_error(
                        self,
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original C array variable name owner",
                        )
                        .into(),
                    ));
                }
                original
                    .check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
                key.check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
                value
                    .check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
                let reached = original_names.read(self, key);
                scope
                    .check(self)
                    .map_err(|error| completion_from_cmd_error(self, error))?;
                let key_bytes = reached.map_err(|error| completion_from_cmd_error(self, error))?;
                let reached = self.prepare_native_original_variable_parts(
                    original,
                    NativeVariableNameLookupPurpose::Write,
                    None,
                    Some(&key_bytes),
                );
                scope
                    .check(self)
                    .map_err(|error| completion_from_cmd_error(self, error))?;
                let (_root, bytes, element, resolved) = reached?;
                let element_birth = resolved.elem.is_some() && resolved.id.is_none();
                let captured =
                    self.capture_selected_update(&bytes, element.as_deref(), &resolved)?;
                if element_birth
                    && self
                        .native_c_variable_name_protocol()
                        .is_some_and(tcl_syntax::native_variable_name::NativeVariableNameProtocol::element_table_retains_original)
                    && let (Some(array), Some(member)) =
                        (captured.cell.array, captured.cell.elem.as_ref())
                    && !self
                        .var_arena
                        .retain_original_array_key(array, member, key.clone())
                {
                    return Err(completion_from_cmd_error(
                        self,
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original array member key publication",
                        )
                        .into(),
                    ));
                }
                scope
                    .check(self)
                    .map_err(|error| completion_from_cmd_error(self, error))?;
                value
                    .check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
                self.with_variable_operation(&captured.cell, |vm| {
                    vm.store_captured_update_with_original(
                        &bytes,
                        element.as_deref(),
                        &captured,
                        value.native_lifetime_lease().into_value(),
                        Some(original),
                    )
                })
            })();
            drop(key_lifetime);
            drop(root_lifetime);
            scope
                .check(self)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            result
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    fn store_original_c_variable_for(
        &mut self,
        original: &Value,
        value: Value,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Value, Completion<Value>> {
        self.store_original_c_variable_in(original, value, purpose, None)
    }

    /// C8.4 `ObjSetVar2` keeps its original header and `GLOBAL_ONLY`/quiet purpose.
    pub(super) fn store_original_c84_global_error_object(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        if !self.uses_c84_global_error_info()
            || self
                .native_c_variable_name_protocol()
                .is_none_or(|protocol| protocol.version() != tcl_dialect::TclVersion::V8_4)
        {
            return Err(self.refuse_host_command("actual C8.4 original error-name setter".into()));
        }
        self.store_original_c_variable_in(
            original,
            value,
            NativeVariableNameLookupPurpose::QuietWrite,
            Some(tcl_core_types::ROOT_NS),
        )
    }

    /// Prepare the held object name before selecting the actual append cell.
    pub(super) fn append_original_c84_global_error_object(
        &mut self,
        original: &Value,
        source: Value,
    ) -> Result<Value, Completion<Value>> {
        if !self.uses_c84_global_error_info()
            || self
                .native_c_variable_name_protocol()
                .is_none_or(|protocol| protocol.version() != tcl_dialect::TclVersion::V8_4)
        {
            return Err(self.refuse_host_command("actual C8.4 original error-name append".into()));
        }
        let (_root, bytes, element, resolved) = self.prepare_native_original_variable_in(
            original,
            NativeVariableNameLookupPurpose::QuietWrite,
            Some(tcl_core_types::ROOT_NS),
        )?;
        let reached = self.append_original_c84_selected_variable(
            original,
            &bytes,
            element.as_deref(),
            &source,
            resolved,
        );
        drop(source);
        reached
    }

    fn store_original_c_variable_in(
        &mut self,
        original: &Value,
        value: Value,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<tcl_core_types::NsId>,
    ) -> Result<Value, Completion<Value>> {
        let (_root, bytes, element, resolved) =
            self.prepare_native_original_variable_in(original, purpose, namespace)?;
        let element_birth = resolved.elem.is_some() && resolved.id.is_none();
        let captured = self.capture_selected_update_with_errors(
            &bytes,
            element.as_deref(),
            &resolved,
            purpose.leaves_error_message(),
        )?;
        if element_birth
            && let (Some(array), Some(key)) = (captured.cell.array, captured.cell.elem.as_ref())
            && self
                .native_c_variable_name_protocol()
                .is_some_and(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5)
        {
            let key_object = Self::original_c_parsed_element_key(original);
            if let Some(key_object) = key_object {
                self.var_arena
                    .retain_original_array_key(array, key, key_object);
            }
        }
        self.with_variable_operation(&captured.cell, |vm| {
            vm.store_captured_update_with_original_and_errors(
                &bytes,
                element.as_deref(),
                &captured,
                value,
                Some(original),
                purpose.leaves_error_message(),
            )
        })
    }
}

#[cfg(test)]
mod tests;
