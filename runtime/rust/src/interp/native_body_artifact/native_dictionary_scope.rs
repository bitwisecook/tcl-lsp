// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original compiled dictionary scope slots, expansion and protected writeback.

use super::*;
use crate::value_ops::{RuntimeAppendValue, RuntimeDictionaryObjects};
use tcl_cmd_core::native_dictionary::NativeDictionaryObjects;
use tcl_registry::native_control_compilation::{NativeControlCompilation, NativeControlOutcome};
use tcl_registry::native_control_instructions::NativeControlBody;
use tcl_registry::native_dictionary_scope_compilation::{
    NativeDictionaryScopeInstruction, NativeDictionaryScopeKind as Kind,
    NativeDictionaryScopeReceiver as Receiver,
};

pub(super) struct DictionaryScopeOperation {
    recipe: NativeDictionaryScopeInstruction,
    pub(super) prepared: PreparedControlOperands,
    root_slot: Option<usize>,
    root_name: Option<Vec<u8>>,
    targets: Vec<usize>,
    target_names: Vec<Vec<u8>>,
    empty: Option<usize>,
    path_empty: Option<usize>,
}
impl Builder<'_> {
    pub(super) fn dictionary_scope_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        selected: NativeControlCompilation<NativeDictionaryScopeInstruction>,
        depth: u32,
    ) -> Result<Operation, ValueError> {
        let prepared = self.prepare_control_steps(captured, &selected.preparations, depth)?;
        let recipe = match selected.outcome {
            NativeControlOutcome::Inline(recipe) => recipe,
            NativeControlOutcome::Generic => return Ok(Operation::Invoke),
            NativeControlOutcome::Rejected(failure) => {
                return Err(self.reject_native_compilation(failure));
            }
        };
        let protocol = self
            .interp
            .native_invocation_dialect()
            .native_compiled_variable_protocol()
            .ok_or_else(|| unavailable("native dictionary scope local issuer"))?;
        let slot = |name: &[u8]| {
            self.lvt
                .find_native(protocol, name)
                .ok_or_else(|| unavailable("native dictionary scope prepared local"))
        };
        let root_slot = match &recipe.receiver {
            Receiver::Local(name) => Some(slot(name)?),
            Receiver::Stack(_) => None,
        };
        let targets = match &recipe.kind {
            Kind::Update { targets, .. } => targets
                .iter()
                .map(|name| slot(name))
                .collect::<Result<Vec<_>, _>>()?,
            Kind::With { .. } => Vec::new(),
        };
        let slot_names = self.lvt.native_slot_names();
        let native_name = |slot: usize| {
            slot_names
                .get(slot)
                .and_then(Option::as_ref)
                .map(|name| name.as_bytes().to_vec())
                .ok_or_else(|| unavailable("native dictionary scope local primary"))
        };
        let root_name = root_slot.map(native_name).transpose()?;
        let target_names = targets
            .iter()
            .copied()
            .map(native_name)
            .collect::<Result<Vec<_>, _>>()?;
        let empty = matches!(
            recipe.kind,
            Kind::With {
                empty_body: true,
                ..
            }
        )
        .then(|| self.literals.intern_bytes(b""));
        let path_empty = matches!(&recipe.kind, Kind::With { path, .. } if path.is_empty())
            .then(|| self.literals.intern_bytes(b""));
        Ok(Operation::DictionaryScope(DictionaryScopeOperation {
            recipe,
            prepared,
            root_slot,
            root_name,
            targets,
            target_names,
            empty,
            path_empty,
        }))
    }
}

struct DictionaryScopeOperands {
    target: EvaluatedTarget,
    path: obj::Owned,
    name_slot: Option<usize>,
    path_slot: Option<usize>,
}
struct DictionaryScopeStack {
    target: EvaluatedTarget,
    path: obj::Owned,
    keys: obj::Owned,
    name_slot: Option<usize>,
    path_slot: Option<usize>,
    keys_slot: Option<usize>,
}

fn local_target(name: &[u8]) -> EvaluatedTarget {
    EvaluatedTarget {
        root: name.to_vec(),
        element: None,
        original_name: None,
        original_index: None,
        combined: false,
    }
}

impl Interp {
    fn dictionary_store_temporary(&mut self, slot: usize, value: *mut TclObj) -> Result<(), Code> {
        let stored = self
            .frames
            .borrow_mut()
            .store_native_compiled_temporary(slot, value);
        stored.map_err(|_| {
            self.report_cmd_error(unavailable("native dictionary anonymous cell").into())
        })
    }

    fn dictionary_temporary(&mut self, slot: usize) -> Result<obj::Owned, Code> {
        let value = self
            .frames
            .borrow()
            .native_compiled_temporary(slot)
            .ok()
            .flatten();
        value.map(obj::Owned::retain).ok_or_else(|| {
            self.report_cmd_error(unavailable("native dictionary temporary value").into())
        })
    }

    fn compiled_dictionary_read(
        &mut self,
        target: &EvaluatedTarget,
        slot: Option<usize>,
        with: bool,
    ) -> Result<
        Option<(
            super::super::native_variable_names::OriginalCVariableCapture,
            *mut TclObj,
        )>,
        Code,
    > {
        use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose as Purpose;
        let captured = if slot.is_some() {
            match self.body_capture_target(target, slot, with) {
                Ok(captured) => Some(captured),
                Err(code) if self.host_refusal_pending() => return Err(code),
                Err(_) => None,
            }
        } else {
            match self.capture_original_c_variable_report(
                target
                    .original_name
                    .as_ref()
                    .expect("original dictionary receiver")
                    .as_ptr(),
                if with {
                    Purpose::Write
                } else {
                    Purpose::Exists
                },
            ) {
                Ok(captured) => captured,
                Err(code) if with || self.host_refusal_pending() => return Err(code),
                Err(_) => None,
            }
        };
        let Some(captured) = captured else {
            return Ok(None);
        };
        let traced = self.original_variable_trace_requires_name(
            &captured.home,
            captured.element.as_deref(),
            b"read",
        );
        let failed = if traced {
            let spelling = target
                .original_name
                .as_ref()
                .map(|value| {
                    self.native_string_bytes(&value.as_ptr())
                        .map(|bytes| bytes.to_vec())
                })
                .transpose()
                .map_err(|error| self.report_cmd_error(error.into()))?
                .unwrap_or_else(|| target.root.clone());
            let access = self.trace_access(
                &spelling,
                &captured.root,
                captured.element.as_deref(),
                &captured.home,
                false,
            );
            self.fire_var_trace_resolved(&captured.home, &access, b"read")
        } else {
            false
        };
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        let value = if failed {
            None
        } else {
            captured.receiver.read_initial().ok().flatten()
        };
        if let Some(value) = value {
            Ok(Some((captured, value)))
        } else {
            self.traces.borrow_mut().pending_err.take();
            {
                let mut exception = self.exc.borrow_mut();
                exception.code = b"TCL READ VARNAME".to_vec();
                exception.code_explicit = false;
            }
            self.replace_native_error_code(b"TCL READ VARNAME");
            Ok(None)
        }
    }

    fn publish_compiled_dictionary(
        &mut self,
        target: &EvaluatedTarget,
        captured: &super::super::native_variable_names::OriginalCVariableCapture,
        value: RuntimeAppendValue,
    ) -> Result<(), Code> {
        captured
            .receiver
            .store(value.as_ptr())
            .map_err(|error| crate::builtins::var_error(self, &captured.root, error))?;
        drop(value);
        if self.original_variable_trace_requires_name(
            &captured.home,
            captured.element.as_deref(),
            b"write",
        ) {
            let spelling = target
                .original_name
                .as_ref()
                .map(|value| {
                    self.native_string_bytes(&value.as_ptr())
                        .map(|bytes| bytes.to_vec())
                })
                .transpose()
                .map_err(|error| self.report_cmd_error(error.into()))?
                .unwrap_or_else(|| target.root.clone());
            let access = self.trace_access(
                &spelling,
                &captured.root,
                captured.element.as_deref(),
                &captured.home,
                false,
            );
            if self.fire_var_trace_resolved(&captured.home, &access, b"write") {
                return Err(crate::builtins::var_error(
                    self,
                    &spelling,
                    crate::frame::VarError::TraceError,
                ));
            }
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        Ok(())
    }

    fn compiled_dictionary_update_start(
        &mut self,
        dictionary: &obj::Owned,
        keys: &obj::Owned,
        names: &[Vec<u8>],
        slots: &[usize],
        protocol: NativeStringProtocol,
    ) -> Result<(), Code> {
        let keys = crate::list::list_elements_native_checked(keys.as_ptr(), protocol)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if keys.len() != slots.len() {
            return Err(self.report_cmd_error(
                unavailable("native dictionary update auxiliary cardinality").into(),
            ));
        }
        for ((key, name), slot) in keys.into_iter().zip(names).zip(slots) {
            let pairs = crate::dict::native_dict_pairs(dictionary.as_ptr(), protocol)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let bytes = self
                .native_string_bytes(&key)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let member = tcl_cmd_core::dict::lookup(self, &pairs, &bytes)
                .map_err(|error| self.report_cmd_error(error))?;
            if let Some(value) = member {
                self.body_store_value(
                    &local_target(name),
                    Some(*slot),
                    &obj::Owned::retain(value),
                )?;
            } else {
                let original = obj::Owned::fresh(obj::new_string_bytes(name));
                let _ = self.unset_original_c_variable(original.as_ptr(), false);
                if self.host_refusal_pending() {
                    return Err(Code::Error);
                }
            }
        }
        Ok(())
    }

    fn compiled_dictionary_reflect(
        &mut self,
        mut leaf: crate::dict::PreparedNativeDictionary,
        keys: &obj::Owned,
        names: Option<&[Vec<u8>]>,
        slots: &[usize],
        protocol: NativeStringProtocol,
    ) -> Result<RuntimeAppendValue, tcl_cmd_core::CmdError> {
        let keys = crate::list::list_elements_native_checked(keys.as_ptr(), protocol)?;
        for (index, key) in keys.into_iter().enumerate() {
            let value = if let Some(names) = names {
                self.compiled_dictionary_read(
                    &local_target(&names[index]),
                    Some(slots[index]),
                    false,
                )
            } else {
                let original = obj::Owned::retain(key);
                let root = Vec::new();
                let target = EvaluatedTarget {
                    root,
                    element: None,
                    original_name: Some(original),
                    original_index: None,
                    combined: true,
                };
                self.compiled_dictionary_read(&target, None, false)
            };
            let value = value.map_err(|_| unavailable("native dictionary scope target read"))?;
            if let Some((_captured, value)) = value {
                let value = if leaf.original() == value {
                    obj::Owned::fresh(obj::duplicate(value))
                } else {
                    obj::Owned::retain(value)
                };
                leaf.set_member(key, value.as_ptr())?;
            } else {
                leaf.remove_member(key)?;
            }
        }
        let value = leaf.into_value();
        Ok(RuntimeAppendValue::retain(value.as_ptr()))
    }

    fn compiled_dictionary_writeback(
        &mut self,
        target: &EvaluatedTarget,
        operation: &DictionaryScopeOperation,
        keys: &obj::Owned,
        path: &obj::Owned,
        protocol: NativeStringProtocol,
    ) -> Result<(), Code> {
        let with = matches!(operation.recipe.kind, Kind::With { .. });
        let Some((captured, current)) =
            self.compiled_dictionary_read(target, operation.root_slot, with)?
        else {
            return Ok(());
        };
        let objects = RuntimeDictionaryObjects::selected(self)
            .map_err(|error| self.report_cmd_error(error))?
            .with_preparation(
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::ConvertBeforeCopy,
            );
        let current = RuntimeAppendValue::borrowed(current);
        let root = objects
            .prepare(Some(&current))
            .map_err(|error| self.report_cmd_error(error))?;
        if matches!(operation.recipe.kind, Kind::Update { .. }) {
            obj::invalidate_string(root.original());
        }
        let saved_error = self.save_dictionary_scope_error();
        let value = match &operation.recipe.kind {
            Kind::Update { .. } => Some(
                self.compiled_dictionary_reflect(
                    root,
                    keys,
                    Some(&operation.target_names),
                    &operation.targets,
                    protocol,
                )
                .map_err(|error| self.report_cmd_error(error))?,
            ),
            Kind::With { .. } => {
                let path = crate::list::list_elements_native_checked(path.as_ptr(), protocol)
                    .map_err(|error| self.report_cmd_error(error.into()))?
                    .into_iter()
                    .map(RuntimeAppendValue::borrowed)
                    .collect::<Vec<_>>();
                tcl_cmd_core::native_dictionary::transform_existing_prepared_path(
                    &objects,
                    root,
                    &path,
                    |leaf| self.compiled_dictionary_reflect(leaf, keys, None, &[], protocol),
                )
                .map_err(|error| self.report_cmd_error(error))?
            }
        };
        if let Some(value) = value {
            self.publish_compiled_dictionary(target, &captured, value)?;
        }
        self.restore_dictionary_scope_error(saved_error);
        Ok(())
    }

    fn prepare_dictionary_scope_operands(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operation: &DictionaryScopeOperation,
        execution: &mut BodyExecution,
    ) -> Result<Option<DictionaryScopeOperands>, Code> {
        let protocol = artifact.stamp.source_protocol;
        let mut temporary = operation.prepared.temporaries.iter();
        let mut name_slot = None;
        let mut path_slot = None;
        let target = match &operation.recipe.receiver {
            Receiver::Local(_) => local_target(
                operation
                    .root_name
                    .as_deref()
                    .expect("native root local primary"),
            ),
            Receiver::Stack(operand) => {
                let original = self.body_control_operand(
                    artifact,
                    command,
                    &operation.prepared,
                    operand,
                    execution,
                )?;
                if execution.done {
                    return Ok(None);
                }
                if let Some(slot) = temporary.next() {
                    self.dictionary_store_temporary(*slot, original.as_ptr())?;
                    name_slot = Some(*slot);
                }
                let root = Vec::new();
                EvaluatedTarget {
                    root,
                    element: None,
                    original_name: Some(original),
                    original_index: None,
                    combined: true,
                }
            }
        };
        let operands = match &operation.recipe.kind {
            Kind::Update { keys, .. } => keys,
            Kind::With { path, .. } => path,
        };
        let mut values = Vec::new();
        for operand in operands {
            values.push(self.body_control_operand(
                artifact,
                command,
                &operation.prepared,
                operand,
                execution,
            )?);
            if execution.done {
                return Ok(None);
            }
        }
        let pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
        let path = if pointers.is_empty() && matches!(operation.recipe.kind, Kind::With { .. }) {
            let index = operation
                .path_empty
                .expect("native dictionary empty path literal");
            obj::Owned::retain(
                artifact
                    .literals
                    .original(index)
                    .expect("dictionary path PUSH"),
            )
        } else {
            obj::Owned::fresh(crate::list::new_list_obj_native(&pointers, protocol))
        };
        if matches!(operation.recipe.kind, Kind::With { .. }) && !pointers.is_empty() {
            if let Some(slot) = temporary.next() {
                self.dictionary_store_temporary(*slot, path.as_ptr())?;
                path_slot = Some(*slot);
            }
        }
        drop(values);
        Ok(Some(DictionaryScopeOperands {
            target,
            path,
            name_slot,
            path_slot,
        }))
    }

    fn expand_compiled_dictionary_scope(
        &mut self,
        artifact: &NativeBodyArtifact,
        operation: &DictionaryScopeOperation,
        operands: DictionaryScopeOperands,
    ) -> Result<DictionaryScopeStack, Code> {
        let protocol = artifact.stamp.source_protocol;
        let DictionaryScopeOperands {
            mut target,
            path,
            name_slot,
            path_slot,
        } = operands;
        let path = if path_slot.is_some() {
            drop(path);
            None
        } else {
            Some(path)
        };
        let dictionary = self.body_read_original_parts(&target, operation.root_slot)?;
        if name_slot.is_some() {
            target.original_name = None;
        }
        let path = match path {
            Some(path) => path,
            None => self.dictionary_temporary(path_slot.expect("stored native path"))?,
        };
        let keys_slot = matches!(
            operation.recipe.kind,
            Kind::With {
                empty_body: false,
                ..
            }
        )
        .then(|| {
            *operation
                .prepared
                .temporaries
                .last()
                .expect("native key temporary")
        });
        let keys = match &operation.recipe.kind {
            Kind::Update { .. } => {
                self.compiled_dictionary_update_start(
                    &dictionary,
                    &path,
                    &operation.target_names,
                    &operation.targets,
                    protocol,
                )?;
                obj::Owned::retain(path.as_ptr())
            }
            Kind::With { .. } => {
                let path_keys = crate::list::list_elements_native_checked(path.as_ptr(), protocol)
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                let leaf = if path_keys.is_empty() {
                    dictionary.as_ptr()
                } else {
                    tcl_cmd_core::dict::get(self, &dictionary.as_ptr(), &path_keys)
                        .map_err(|error| self.report_cmd_error(error))?
                };
                let pairs = crate::dict::native_dict_pairs(leaf, protocol)
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                let mut keys = obj::Owned::fresh(obj::new_obj());
                for (key, value) in pairs {
                    keys =
                        crate::list::append_native_elements(Some(keys.as_ptr()), &[key], protocol)
                            .map_err(|error| self.report_cmd_error(error.into()))?;
                    self.assign_original_named_variable(key, value)?;
                }
                if let Some(slot) = keys_slot {
                    self.dictionary_store_temporary(slot, keys.as_ptr())?;
                }
                keys
            }
        };
        drop(dictionary);
        Ok(DictionaryScopeStack {
            target,
            path,
            keys,
            name_slot,
            path_slot,
            keys_slot,
        })
    }

    pub(super) fn execute_body_dictionary_scope(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operation: &DictionaryScopeOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let Some(operands) =
            self.prepare_dictionary_scope_operands(artifact, command, operation, execution)?
        else {
            return Ok(Code::Ok);
        };
        let DictionaryScopeStack {
            target,
            path,
            keys,
            name_slot,
            path_slot,
            keys_slot,
        } = self.expand_compiled_dictionary_scope(artifact, operation, operands)?;
        let protocol = artifact.stamp.source_protocol;
        if matches!(
            operation.recipe.kind,
            Kind::With {
                empty_body: true,
                ..
            }
        ) {
            self.compiled_dictionary_writeback(&target, operation, &keys, &path, protocol)?;
            self.set_result(
                artifact
                    .literals
                    .original(operation.empty.expect("dictionary empty body result"))
                    .expect("dictionary empty result PUSH"),
            );
            return Ok(Code::Ok);
        }
        let with_temporaries = matches!(operation.recipe.kind, Kind::With { .. });
        let retained = if with_temporaries {
            drop(target);
            drop(path);
            drop(keys);
            None
        } else {
            drop(path);
            Some((target, keys))
        };
        let code = self.execute_control_body(
            artifact,
            command,
            &operation.prepared,
            &NativeControlBody {
                operand: operation.recipe.body.clone(),
                script: Some(operation.recipe.body_span),
            },
            execution,
        );
        if self.host_refusal_pending() || self.exit_pending() {
            return Ok(code);
        }
        let result = (code == Code::Ok).then(|| obj::Owned::retain(self.result_obj()));
        let completion = if code == Code::Ok {
            None
        } else {
            Some(self.capture_body_completion(code)?)
        };
        let (target, keys, path) = if let Some((target, keys)) = retained {
            let path = obj::Owned::retain(keys.as_ptr());
            (target, keys, path)
        } else {
            let target = match &operation.recipe.receiver {
                Receiver::Local(_) => local_target(
                    operation
                        .root_name
                        .as_deref()
                        .expect("native root local primary"),
                ),
                Receiver::Stack(_) => EvaluatedTarget {
                    root: Vec::new(),
                    element: None,
                    original_name: Some(
                        self.dictionary_temporary(name_slot.expect("native name cell"))?,
                    ),
                    original_index: None,
                    combined: true,
                },
            };
            let path = if let Some(slot) = path_slot {
                self.dictionary_temporary(slot)?
            } else {
                obj::Owned::retain(
                    artifact
                        .literals
                        .original(operation.path_empty.expect("native empty path"))
                        .expect("path PUSH"),
                )
            };
            (
                target,
                self.dictionary_temporary(keys_slot.expect("native key state cell"))?,
                path,
            )
        };
        self.compiled_dictionary_writeback(&target, operation, &keys, &path, protocol)?;
        if let Some(completion) = completion {
            self.reapply_body_completion(&completion)
        } else {
            self.set_result(result.expect("normal body result").as_ptr());
            Ok(Code::Ok)
        }
    }
}
