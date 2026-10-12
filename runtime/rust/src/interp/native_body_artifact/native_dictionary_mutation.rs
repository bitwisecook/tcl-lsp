// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original dictionary mutation over the genuine retained indexed receiver.

use super::*;
use std::cell::Cell;

pub(super) struct DictionaryMutationOperation {
    recipe: tcl_registry::native_dictionary_compilation::NativeDictionaryMutationInstruction,
    slot: usize,
    receiver: Vec<u8>,
    operands: Vec<NamespaceOperand>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn dictionary_mutation_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        recipe: tcl_registry::native_dictionary_compilation::NativeDictionaryMutationInstruction,
        depth: u32,
    ) -> Result<DictionaryMutationOperation, tcl_cmd_core::CmdError> {
        let slot = self
            .local(&recipe.receiver, None)
            .ok_or_else(|| unavailable("native dictionary mutation receiver slot"))?;
        let receiver = self
            .lvt
            .native_slot_names()
            .get(slot)
            .and_then(Option::as_ref)
            .map(|name| name.as_bytes().to_vec())
            .ok_or_else(|| unavailable("native dictionary mutation physical local name"))?;
        let mut prepared_words = HashMap::new();
        let mut operands = Vec::with_capacity(recipe.operands.len());
        for operand in &recipe.operands {
            operands.push(self.namespace_operand(captured, operand, &mut prepared_words, depth)?);
        }
        Ok(DictionaryMutationOperation {
            recipe,
            slot,
            receiver,
            operands,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_dictionary_mutation(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        dictionary: &DictionaryMutationOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        use super::native_list_conversion::BodyObjectOperation;
        use crate::value_ops::{RuntimeAppendObjects, RuntimeAppendValue};
        use tcl_registry::native_dictionary_compilation::NativeDictionaryMutationKind as Kind;
        let operation =
            BodyObjectOperation::capture(self).map_err(|first| self.report_cmd_error(first))?;
        let actual = self.clone();
        let variable_stage = || operation.check(&actual);
        let mut values = Vec::with_capacity(dictionary.operands.len());
        let reached = (|| {
            for operand in &dictionary.operands {
                operation
                    .check(self)
                    .map_err(|first| self.report_cmd_error(first))?;
                values.push(self.body_namespace_operand(artifact, command, operand, execution)?);
                operation
                    .check(self)
                    .map_err(|first| self.report_cmd_error(first))?;
                if execution.done {
                    return Ok(Code::Ok);
                }
            }
            let dialect = self.native_invocation_dialect();
            let objects = operation
                .objects(self)
                .map_err(|error| self.report_cmd_error(error))?;
            let append_objects = objects
                .original_append_objects(RuntimeAppendObjects {
                    dialect,
                    binary_recipe: dialect.byte_array_string_recipe(None),
                })
                .map_err(|error| self.report_cmd_error(error))?;
            let mut pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
            let concatenated = if dictionary.recipe.kind == Kind::Append && pointers.len() > 2 {
                let originals = pointers[1..]
                    .iter()
                    .copied()
                    .map(RuntimeAppendValue::borrowed)
                    .collect::<Vec<_>>();
                let reached = tcl_cmd_core::native_cat::concatenate_compiled(
                    &append_objects,
                    objects.protocol,
                    &originals,
                );
                let value = append_objects
                    .finish(reached)
                    .map_err(|error| self.report_cmd_error(error))?;
                pointers.truncate(1);
                pointers.push(value.as_ptr());
                Some(value)
            } else {
                None
            };
            let reached = self.body_dictionary_mutation_update(dictionary, &pointers, &operation);
            drop(concatenated);
            operation
                .check(self)
                .map_err(|first| self.report_cmd_error(first))?;
            reached
        })();
        drop(values);
        operation
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        reached
    }

    fn body_dictionary_mutation_update(
        &mut self,
        dictionary: &DictionaryMutationOperation,
        pointers: &[*mut TclObj],
        operation: &Rc<super::native_list_conversion::BodyObjectOperation>,
    ) -> Result<Code, Code> {
        use crate::value_ops::{RuntimeAppendObjects, RuntimeAppendValue};
        use tcl_cmd_core::native_dictionary::NativeDictionaryObjects;
        operation
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        let evaluated = EvaluatedTarget {
            root: dictionary.receiver.clone(),
            element: None,
            original_name: None,
            original_index: None,
            combined: false,
        };
        let capture = Rc::new(self.body_capture_target_using(
            &evaluated,
            Some(dictionary.slot),
            true,
            Some(operation),
        )?);
        operation
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        let root = capture.root.as_slice();
        let access = self.trace_access(root, root, None, &capture.home, false);
        let reached = (|| {
            let failed = self
                .fire_var_trace_resolved(&capture.home, &access, b"read")
                .map_err(|first| self.refuse_native_execution(first))?
                .is_guest_error();
            operation
                .check(self)
                .map_err(|first| self.report_cmd_error(first))?;
            let selected_read = capture.receiver.read_initial();
            if selected_read == Err(crate::frame::VarError::NameProtocolUnavailable) {
                return Err(self.report_cmd_error(
                    operation.refuse(unavailable("original compiled dictionary scalar role")),
                ));
            }
            // A quiet Guest read outcome is distinct from withdrawal of the
            // actual selected scalar role. Keep that outcome in the observer.
            let original = if failed {
                None
            } else {
                selected_read.ok().flatten()
            };
            if original.is_none() {
                self.traces.borrow_mut().pending_err.take();
                let mut exception = self.exc.borrow_mut();
                exception.code = b"TCL READ VARNAME".to_vec();
                exception.code_explicit = false;
            }
            let original_lifetime = selected_read
                .ok()
                .flatten()
                .map(obj::NativeObjectLifetime::retain);
            let expected = Rc::new(Cell::new(selected_read));
            let selected = capture.clone();
            let selected_operation = operation.clone();
            let selected_expected = expected.clone();
            let selected_interp = self.clone();
            let check: Rc<dyn Fn() -> Result<(), tcl_cmd_core::CmdError>> = Rc::new(move || {
                selected_operation.check(&selected_interp)?;
                selected.receiver.check_current().map_err(|_| {
                    selected_operation.refuse(unavailable("original compiled dictionary receiver"))
                })?;
                let actual = selected.receiver.read_initial();
                if actual == Err(crate::frame::VarError::NameProtocolUnavailable) {
                    return Err(selected_operation
                        .refuse(unavailable("original compiled dictionary scalar role")));
                }
                if actual != selected_expected.get() {
                    return Err(selected_operation
                        .refuse(unavailable("original compiled dictionary receiver header")));
                }
                if let Ok(Some(value)) = actual {
                    obj::check_native_liveness(value)
                        .map_err(|error| selected_operation.refuse(error))?;
                }
                Ok(())
            });
            let objects = operation
                .objects_using(self, check.clone())
                .map_err(|error| self.report_cmd_error(error))?;
            let dialect = self.native_invocation_dialect();
            let append_objects = objects
                .original_append_objects(RuntimeAppendObjects {
                    dialect,
                    binary_recipe: dialect.byte_array_string_recipe(None),
                })
                .map_err(|error| self.report_cmd_error(error))?;
            let original = original.map(RuntimeAppendValue::borrowed);
            let prepared = objects
                .prepare(original.as_ref())
                .map_err(|error| self.report_cmd_error(error))?;
            let result = self
                .body_dictionary_mutation_value(
                    &objects,
                    &append_objects,
                    dictionary,
                    prepared,
                    pointers,
                )
                .map_err(|error| self.report_cmd_error(error))?;
            let reached = (|| {
                check().map_err(|first| self.report_cmd_error(first))?;
                // The actual store publishes its selected result before releasing
                // the displaced value. This observation adds no owning role.
                expected.set(Ok(Some(result.as_ptr())));
                if let Err(error) = capture.receiver.store(result.as_ptr()) {
                    operation
                        .check(self)
                        .map_err(|first| self.report_cmd_error(first))?;
                    return Ok(crate::builtins::var_error(self, root, error));
                }
                check().map_err(|first| self.report_cmd_error(first))?;
                let failed = self
                    .fire_var_trace_resolved(&capture.home, &access, b"write")
                    .map_err(|first| self.refuse_native_execution(first))?
                    .is_guest_error();
                operation
                    .check(self)
                    .map_err(|first| self.report_cmd_error(first))?;
                if failed {
                    return Ok(crate::builtins::var_error(
                        self,
                        root,
                        crate::frame::VarError::TraceError,
                    ));
                }
                // A legal write trace may change the actual receiver. Read the
                // selected live cell after the original operation/frame check.
                capture.receiver.check_current().map_err(|_| {
                    self.report_cmd_error(
                        operation.refuse(unavailable("original compiled dictionary receiver")),
                    )
                })?;
                match capture.receiver.read() {
                    Ok(Some(value)) => self.set_result(value),
                    Err(crate::frame::VarError::NameProtocolUnavailable) => {
                        return Err(self.report_cmd_error(operation.refuse(unavailable(
                            "original compiled dictionary result scalar role",
                        ))));
                    }
                    Ok(None) | Err(_) => self.set_result_bytes(b""),
                }
                operation
                    .check(self)
                    .map_err(|first| self.report_cmd_error(first))?;
                Ok(Code::Ok)
            })();
            drop(result);
            drop(original_lifetime);
            operation
                .check(self)
                .map_err(|first| self.report_cmd_error(first))?;
            reached
        })();
        drop(capture);
        operation
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        reached
    }

    fn body_dictionary_mutation_value(
        &self,
        objects: &crate::value_ops::RuntimeDictionaryObjects,
        append_objects: &crate::value_ops::OriginalRuntimeAppendObjects,
        dictionary: &DictionaryMutationOperation,
        prepared: crate::dict::PreparedNativeDictionary,
        pointers: &[*mut TclObj],
    ) -> Result<crate::value_ops::RuntimeAppendValue, tcl_cmd_core::CmdError> {
        use crate::value_ops::RuntimeAppendValue;
        use tcl_registry::native_dictionary_compilation::NativeDictionaryMutationKind as Kind;
        let keys =
            usize::try_from(dictionary.recipe.key_count).expect("native dictionary key count");
        let inputs = pointers
            .iter()
            .copied()
            .map(RuntimeAppendValue::borrowed)
            .collect::<Vec<_>>();
        match dictionary.recipe.kind {
            Kind::Set => tcl_cmd_core::native_dictionary::set_prepared_path(
                objects,
                prepared,
                &inputs[..keys],
                inputs[keys].clone(),
            ),
            Kind::Unset => tcl_cmd_core::native_dictionary::remove_prepared_path(
                objects,
                prepared,
                &inputs[..keys],
            ),
            Kind::Append => {
                let issued = append_objects
                    .dialect()
                    .native_object_append_protocol(None)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "native compiled dictionary append",
                    ))?;
                tcl_cmd_core::native_dictionary::update_prepared_member(
                    objects,
                    prepared,
                    &inputs[0],
                    |original| match original {
                        None => Ok(inputs[1].clone()),
                        Some(_) => append_objects.finish(
                            tcl_cmd_core::native_append::append_dictionary_operands(
                                append_objects,
                                issued.recipe(),
                                original,
                                &inputs[1..],
                            ),
                        ),
                    },
                )
            }
            Kind::Lappend => tcl_cmd_core::native_dictionary::update_prepared_member(
                objects,
                prepared,
                &inputs[0],
                |original| {
                    let result = crate::list::append_native_elements_original_using(
                        original.map(RuntimeAppendValue::as_ptr),
                        &pointers[1..],
                        objects.protocol,
                        &|| objects.original_check(),
                        &|error| objects.original_refuse(error),
                    )?;
                    Ok(RuntimeAppendValue::retain(result.as_ptr()))
                },
            ),
            Kind::Incr(amount) => {
                let increment = objects.original_increment_objects(self)?;
                let amount = obj::Owned::fresh(obj::new_wide_int_obj(i64::from(amount)));
                let amount = RuntimeAppendValue::borrowed(amount.as_ptr());
                tcl_cmd_core::native_dictionary::update_prepared_member(
                    objects,
                    prepared,
                    &inputs[0],
                    |original| {
                        if original.is_none() {
                            Ok(amount.clone())
                        } else {
                            tcl_cmd_core::native_increment::increment(&increment, original, &amount)
                        }
                    },
                )
            }
        }
    }
}

#[cfg(test)]
#[path = "native_dictionary_mutation/original_boundary_tests.rs"]
mod original_boundary_tests;

#[cfg(test)]
#[path = "native_dictionary_mutation/original_increment_tests.rs"]
mod original_increment_tests;
