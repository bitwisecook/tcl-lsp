// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original compiled dictionary scope slots, expansion and protected writeback.

use super::native_list_conversion::BodyObjectOperation;
use super::*;
use crate::value_ops::RuntimeAppendValue;
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
    ) -> Result<Operation, tcl_cmd_core::CmdError> {
        let prepared = self.prepare_control_steps(captured, &selected.preparations, depth)?;
        let recipe = match selected.outcome {
            NativeControlOutcome::Inline(recipe) => recipe,
            NativeControlOutcome::Generic => return Ok(Operation::Invoke),
            NativeControlOutcome::Rejected(failure) => {
                return Err((self.reject_native_compilation(failure)).into());
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
    fn dictionary_store_temporary(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        slot: usize,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let stored = self.store_native_compiled_temporary_checked(slot, value);
            stored.map_err(|_| {
                self.report_cmd_error(unavailable("native dictionary anonymous cell").into())
            })
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn dictionary_temporary(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        slot: usize,
    ) -> Result<obj::Owned, Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let value = self
                .frames
                .borrow()
                .native_compiled_temporary(slot)
                .ok()
                .flatten();
            value.map(obj::Owned::retain).ok_or_else(|| {
                self.report_cmd_error(unavailable("native dictionary temporary value").into())
            })
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn compiled_dictionary_read(
        &mut self,
        original: &Rc<BodyObjectOperation>,
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
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
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
                        original
                            .bytes(self, value.as_ptr())
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
                    .map_err(|first| self.refuse_native_execution(first))?
                    .is_guest_error()
            } else {
                false
            };
            if self.host_refusal_pending() {
                return Err(Code::Error);
            }
            let value = if failed {
                None
            } else {
                // Classify only the read reached after a successful observer.
                // Guest-failed observers retain the existing missing path.
                match captured.receiver.read_initial() {
                    Err(crate::frame::VarError::NameProtocolUnavailable) => {
                        return Err(self.report_cmd_error(original.refuse(unavailable(
                            "original compiled Dictionary scope scalar role",
                        ))));
                    }
                    selected_read => selected_read.ok().flatten(),
                }
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
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn publish_compiled_dictionary(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        target: &EvaluatedTarget,
        captured: &super::super::native_variable_names::OriginalCVariableCapture,
        value: RuntimeAppendValue,
    ) -> Result<(), Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            captured
                .receiver
                .store(value.as_ptr())
                .map_err(|error| crate::builtins::var_error(self, &captured.root, error))?;
            // Capture the original frame cause before the working value's
            // mandatory release can reach another callback.
            let published = original
                .check(self)
                .map_err(|error| self.report_cmd_error(error));
            drop(value);
            original
                .check(self)
                .map_err(|error| self.report_cmd_error(error))?;
            published?;
            captured
                .receiver
                .check_current()
                .map_err(|error| crate::builtins::var_error(self, &captured.root, error))?;
            if self.original_variable_trace_requires_name(
                &captured.home,
                captured.element.as_deref(),
                b"write",
            ) {
                let spelling = target
                    .original_name
                    .as_ref()
                    .map(|value| {
                        original
                            .bytes(self, value.as_ptr())
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
                if self
                    .fire_var_trace_resolved(&captured.home, &access, b"write")
                    .map_err(|first| self.refuse_native_execution(first))?
                    .is_guest_error()
                {
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
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn publish_compiled_dictionary_result(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        result: obj::Owned,
    ) -> Result<Code, Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        self.set_result(result.as_ptr());
        let published = original
            .check(self)
            .map_err(|error| self.report_cmd_error(error));
        drop(result);
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        published.map(|()| Code::Ok)
    }

    fn compiled_dictionary_update_start(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        dictionary: &obj::Owned,
        keys: &obj::Owned,
        names: &[Vec<u8>],
        slots: &[usize],
        protocol: NativeStringProtocol,
    ) -> Result<(), Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let key_view = original
                .list(self, keys.as_ptr(), protocol)
                .map_err(|error| self.report_cmd_error(error))?;
            let keys = key_view.members.clone();
            if keys.len() != slots.len() {
                return Err(self.report_cmd_error(
                    unavailable("native dictionary update auxiliary cardinality").into(),
                ));
            }
            for ((key, name), slot) in keys.into_iter().zip(names).zip(slots) {
                key_view
                    .check(self, original)
                    .map_err(|error| self.report_cmd_error(error))?;
                let pairs = original
                    .pairs(self, dictionary.as_ptr(), protocol)
                    .map_err(|error| self.report_cmd_error(error))?;
                let bytes = original
                    .bytes(self, key)
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                key_view
                    .check(self, original)
                    .map_err(|error| self.report_cmd_error(error))?;
                let member = pairs
                    .lookup(self, original, &bytes)
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
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn compiled_dictionary_reflect(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        mut leaf: crate::dict::PreparedNativeDictionary,
        keys: &obj::Owned,
        names: Option<&[Vec<u8>]>,
        slots: &[usize],
        protocol: NativeStringProtocol,
    ) -> Result<RuntimeAppendValue, tcl_cmd_core::CmdError> {
        original.check(self)?;
        let reached = (|| {
            let key_view = original.list(self, keys.as_ptr(), protocol)?;
            for (index, key) in key_view.members.iter().copied().enumerate() {
                key_view.check(self, original)?;
                let value = if let Some(names) = names {
                    self.compiled_dictionary_read(
                        original,
                        &local_target(&names[index]),
                        Some(slots[index]),
                        false,
                    )
                } else {
                    let name = obj::Owned::retain(key);
                    let root = Vec::new();
                    let target = EvaluatedTarget {
                        root,
                        element: None,
                        original_name: Some(name),
                        original_index: None,
                        combined: true,
                    };
                    self.compiled_dictionary_read(original, &target, None, false)
                };
                let value = value.map_err(|_| {
                    original.check(self).err().unwrap_or_else(|| {
                        original.refuse(unavailable("native dictionary scope target read"))
                    })
                })?;
                key_view.check(self, original)?;
                if let Some((captured, value)) = value {
                    let check = || {
                        original.check(self)?;
                        key_view.check(self, original)?;
                        if captured.receiver.check_current().is_err()
                            || captured.receiver.read_initial().ok().flatten() != Some(value)
                        {
                            return Err(original
                                .refuse(unavailable("original reflected Dictionary cell value")));
                        }
                        Ok(())
                    };
                    check()?;
                    let value = if leaf.original() == value {
                        obj::Owned::fresh(obj::duplicate_with_stage_check(
                            value,
                            &mut || check(),
                            &mut |error| {
                                if let Some(first) = error.native_execution_refusal() {
                                    self.clone().refuse_native_execution(first.clone());
                                }
                            },
                        )?)
                    } else {
                        obj::Owned::retain(value)
                    };
                    leaf.set_member_using(
                        key,
                        value.as_ptr(),
                        &check,
                        &|error| original.refuse(error),
                        &|key, current| original.bytes_using(self, key, current),
                    )?;
                    drop(value);
                    original.check(self)?;
                } else {
                    leaf.remove_member_using(
                        key,
                        &|| original.check(self),
                        &|error| original.refuse(error),
                        &|key, current| original.bytes_using(self, key, current),
                    )?;
                }
            }
            let value = leaf.into_value();
            Ok(RuntimeAppendValue::retain(value.as_ptr()))
        })();
        original.check(self)?;
        reached
    }

    fn compiled_dictionary_writeback(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        target: &EvaluatedTarget,
        operation: &DictionaryScopeOperation,
        keys: &obj::Owned,
        path: &obj::Owned,
        protocol: NativeStringProtocol,
    ) -> Result<(), Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let with = matches!(operation.recipe.kind, Kind::With { .. });
            let Some((captured, current)) =
                self.compiled_dictionary_read(original, target, operation.root_slot, with)?
            else {
                return Ok(());
            };
            let captured = Rc::new(captured);
            let actual = self.clone();
            let cell = captured.clone();
            let entered = original.clone();
            let original_header = Rc::new(
                crate::interp::native_arguments::OriginalArguments::enter(self, &[current])
                    .map_err(|error| self.report_cmd_error(error))?,
            );
            let objects = original
                .objects_using(
                    self,
                    Rc::new(move || {
                        entered.check(&actual)?;
                        original_header.check_original_for(&actual, 0, current)?;
                        if cell.receiver.check_current().is_err()
                            || cell.receiver.read_initial().ok().flatten() != Some(current)
                        {
                            return Err(entered
                                .refuse(unavailable("original compiled Dictionary cell value")));
                        }
                        Ok(())
                    }),
                )
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
            let reached = (|| {
                let value = match &operation.recipe.kind {
                    Kind::Update { .. } => Some(
                        self.compiled_dictionary_reflect(
                            original,
                            root,
                            keys,
                            Some(&operation.target_names),
                            &operation.targets,
                            protocol,
                        )
                        .map_err(|error| self.report_cmd_error(error))?,
                    ),
                    Kind::With { .. } => {
                        let path_view = original
                            .list(self, path.as_ptr(), protocol)
                            .map_err(|error| self.report_cmd_error(error))?;
                        let path = path_view
                            .members
                            .iter()
                            .copied()
                            .map(RuntimeAppendValue::borrowed)
                            .collect::<Vec<_>>();
                        tcl_cmd_core::native_dictionary::transform_existing_prepared_path(
                            &objects,
                            root,
                            &path,
                            |leaf| {
                                path_view.check(self, original)?;
                                self.compiled_dictionary_reflect(
                                    original,
                                    leaf,
                                    keys,
                                    None,
                                    &[],
                                    protocol,
                                )
                            },
                        )
                        .map_err(|error| self.report_cmd_error(error))?
                    }
                };
                if let Some(value) = value {
                    self.publish_compiled_dictionary(original, target, &captured, value)?;
                }
                Ok(())
            })();
            let restored = self
                .restore_dictionary_scope_error(saved_error)
                .map_err(|first| self.refuse_native_execution(first));
            original
                .check(self)
                .map_err(|error| self.report_cmd_error(error))?;
            restored?;
            reached?;
            Ok(())
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn prepare_dictionary_scope_operands(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operation: &DictionaryScopeOperation,
        execution: &mut BodyExecution,
    ) -> Result<Option<DictionaryScopeOperands>, Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
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
                    let name = self.body_control_operand(
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
                        self.dictionary_store_temporary(original, *slot, name.as_ptr())?;
                        name_slot = Some(*slot);
                    }
                    let root = Vec::new();
                    EvaluatedTarget {
                        root,
                        element: None,
                        original_name: Some(name),
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
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                values.push(self.body_control_operand(
                    artifact,
                    command,
                    &operation.prepared,
                    operand,
                    execution,
                )?);
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                if execution.done {
                    return Ok(None);
                }
            }
            let pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
            let path = if pointers.is_empty() && matches!(operation.recipe.kind, Kind::With { .. })
            {
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
                    self.dictionary_store_temporary(original, *slot, path.as_ptr())?;
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
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn expand_compiled_dictionary_scope(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        artifact: &NativeBodyArtifact,
        operation: &DictionaryScopeOperation,
        operands: DictionaryScopeOperands,
    ) -> Result<DictionaryScopeStack, Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
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
            original
                .check(self)
                .map_err(|error| self.report_cmd_error(error))?;
            let dictionary = self.body_read_original_parts(&target, operation.root_slot)?;
            original
                .check(self)
                .map_err(|error| self.report_cmd_error(error))?;
            if name_slot.is_some() {
                target.original_name = None;
            }
            original
                .check(self)
                .map_err(|error| self.report_cmd_error(error))?;
            let path = match path {
                Some(path) => path,
                None => {
                    self.dictionary_temporary(original, path_slot.expect("stored native path"))?
                }
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
                        original,
                        &dictionary,
                        &path,
                        &operation.target_names,
                        &operation.targets,
                        protocol,
                    )?;
                    obj::Owned::retain(path.as_ptr())
                }
                Kind::With { .. } => {
                    let path_view = original
                        .list(self, path.as_ptr(), protocol)
                        .map_err(|error| self.report_cmd_error(error))?;
                    let path_keys = &path_view.members;
                    let leaf = if path_keys.is_empty() {
                        dictionary.as_ptr()
                    } else {
                        original
                            .get(self, dictionary.as_ptr(), path_keys, protocol)
                            .map_err(|error| self.report_cmd_error(error))?
                    };
                    let pairs = original
                        .pairs(self, leaf, protocol)
                        .map_err(|error| self.report_cmd_error(error))?;
                    let mut keys = obj::Owned::fresh(obj::new_obj());
                    for (key, value) in pairs.pairs.iter().copied() {
                        path_view
                            .check(self, original)
                            .map_err(|error| self.report_cmd_error(error))?;
                        pairs
                            .check(self, original)
                            .map_err(|error| self.report_cmd_error(error))?;
                        keys = crate::list::append_native_elements_original_using(
                            Some(keys.as_ptr()),
                            &[key],
                            protocol,
                            &|| {
                                original.check(self)?;
                                path_view.check(self, original)?;
                                pairs.check(self, original)
                            },
                            &|error| original.refuse(error),
                        )
                        .map_err(|error| self.report_cmd_error(error.into()))?;
                        self.assign_original_named_variable(key, value)?;
                    }
                    if let Some(slot) = keys_slot {
                        self.dictionary_store_temporary(original, slot, keys.as_ptr())?;
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
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    pub(super) fn execute_body_dictionary_scope(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operation: &DictionaryScopeOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let original =
            &BodyObjectOperation::capture(self).map_err(|error| self.report_cmd_error(error))?;
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let Some(operands) = self.prepare_dictionary_scope_operands(
                original, artifact, command, operation, execution,
            )?
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
            } = self.expand_compiled_dictionary_scope(original, artifact, operation, operands)?;
            let protocol = artifact.stamp.source_protocol;
            if matches!(
                operation.recipe.kind,
                Kind::With {
                    empty_body: true,
                    ..
                }
            ) {
                self.compiled_dictionary_writeback(
                    original, &target, operation, &keys, &path, protocol,
                )?;
                self.set_result(
                    artifact
                        .literals
                        .original(operation.empty.expect("dictionary empty body result"))
                        .expect("dictionary empty result PUSH"),
                );
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error(error))?;
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
            original
                .check(self)
                .map_err(|error| self.report_cmd_error(error))?;
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
            original
                .check(self)
                .map_err(|error| self.report_cmd_error(error))?;
            if self.host_refusal_pending() || self.exit_pending() {
                return Ok(code);
            }
            let result = (code == Code::Ok).then(|| obj::Owned::retain(self.result_obj()));
            let completion = if code == Code::Ok {
                None
            } else {
                Some(self.capture_body_completion(original, code)?)
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
                        original_name: Some(self.dictionary_temporary(
                            original,
                            name_slot.expect("native name cell"),
                        )?),
                        original_index: None,
                        combined: true,
                    },
                };
                let path = if let Some(slot) = path_slot {
                    self.dictionary_temporary(original, slot)?
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
                    self.dictionary_temporary(original, keys_slot.expect("native key state cell"))?,
                    path,
                )
            };
            self.compiled_dictionary_writeback(
                original, &target, operation, &keys, &path, protocol,
            )?;
            if let Some(completion) = completion {
                self.reapply_body_completion(original, &completion)
            } else {
                self.publish_compiled_dictionary_result(
                    original,
                    result.expect("normal body result"),
                )
            }
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }
}

#[cfg(test)]
mod original_boundary_tests {
    use super::*;
    use crate::cmd_control::original_list_conversion_tests as fixture;
    use std::cell::Cell;
    #[test]
    fn original_compiled_dictionary_writeback_restores_saved_fields_before_frame_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual compiled recipe/locals and software callback transport only.
        for mode in [0, 1, 3] {
            let mut interp = fixture::native("tcl9.0");
            assert_eq!(
                interp.eval_str(b"proc p {} {set d {A B}; dict update d A x {set x C}; return $d}"),
                Code::Ok
            );
            assert_eq!(interp.eval_str(b"p"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"A C");
            let procedure = interp.proc_def(b"p").unwrap();
            let artifact =
                cache(procedure.body.as_ptr()).expect("real original Dictionary scope artifact");
            let operation = artifact
                .scripts
                .values()
                .flat_map(|script| &script.commands)
                .find_map(|command| match &command.operation {
                    Operation::DictionaryScope(operation) => Some(operation),
                    _ => None,
                })
                .expect("actual compiled dictionary update");
            assert!(matches!(operation.recipe.kind, Kind::Update { .. }));
            {
                let mut frames = interp.frames.borrow_mut();
                frames.push(crate::namespace::GLOBAL);
                frames
                    .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
                    .unwrap();
            }
            let dictionary = fixture::string(b"A B");
            let value = fixture::string(b"C");
            let target = local_target(b"d");
            interp
                .body_store_value(&target, operation.root_slot, &dictionary)
                .unwrap();
            interp
                .body_store_value(
                    &local_target(&operation.target_names[0]),
                    Some(operation.targets[0]),
                    &value,
                )
                .unwrap();
            interp.set_c_error_code(b"ORIGINAL SAVED");
            let saved = {
                let exc = interp.exc.borrow();
                (exc.code.clone(), exc.native.code.as_ref().unwrap().as_ptr())
            };
            let _current = fixture::install(&mut interp, mode);
            let keys = fixture::original(b"A");
            let path = fixture::string(b"");
            let original = BodyObjectOperation::capture(&interp).unwrap();
            let reached = interp.compiled_dictionary_writeback(
                &original,
                &target,
                operation,
                &keys,
                &path,
                artifact.stamp.source_protocol,
            );
            if mode == 0 {
                reached.unwrap();
                original.check(&interp).unwrap();
            } else {
                assert_eq!(reached.unwrap_err(), Code::Error);
                assert!(original.check(&interp).is_err());
                if mode == 1 {
                    fixture::host(&interp.native_execution_refusal().unwrap());
                }
            }
            let exc = interp.exc.borrow();
            assert_eq!(exc.code, saved.0);
            assert_eq!(exc.native.code.as_ref().unwrap().as_ptr(), saved.1);
            assert_eq!(fixture::FREES.with(Cell::get), 1);
            assert_eq!(fixture::REFS.with(Cell::get), 1);
            assert_eq!(unsafe { (*keys.as_ptr()).ref_count }, 1);
        }
    }
}

#[cfg(test)]
#[path = "native_dictionary_scope/original_read_boundary_tests.rs"]
mod original_read_boundary_tests;

#[cfg(test)]
#[path = "native_dictionary_scope/original_publication_boundary_tests.rs"]
mod original_publication_boundary_tests;
