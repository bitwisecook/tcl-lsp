// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original dictionary mutation over the genuine retained indexed receiver.

use super::*;

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
    ) -> Result<DictionaryMutationOperation, ValueError> {
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
        use crate::value_ops::{
            RuntimeAppendObjects, RuntimeAppendValue, RuntimeDictionaryObjects,
        };
        use tcl_registry::native_dictionary_compilation::NativeDictionaryMutationKind as Kind;
        let mut values = Vec::with_capacity(dictionary.operands.len());
        for operand in &dictionary.operands {
            values.push(self.body_namespace_operand(artifact, command, operand, execution)?);
            if execution.done {
                return Ok(Code::Ok);
            }
        }
        let dialect = self.native_invocation_dialect();
        let objects = RuntimeDictionaryObjects::selected(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let append_objects = RuntimeAppendObjects {
            dialect,
            binary_recipe: dialect.byte_array_string_recipe(None),
        };
        let mut pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
        let concatenated = if dictionary.recipe.kind == Kind::Append && pointers.len() > 2 {
            let originals = pointers[1..]
                .iter()
                .copied()
                .map(RuntimeAppendValue::borrowed)
                .collect::<Vec<_>>();
            let value = tcl_cmd_core::native_cat::concatenate_compiled(
                &append_objects,
                objects.protocol,
                &originals,
            )
            .map_err(|error| self.report_cmd_error(error.into()))?;
            pointers.truncate(1);
            pointers.push(value.as_ptr());
            Some(value)
        } else {
            None
        };
        let code =
            self.body_dictionary_mutation_update(dictionary, &pointers, &objects, &append_objects)?;
        drop(concatenated);
        Ok(code)
    }

    fn body_dictionary_mutation_update(
        &mut self,
        dictionary: &DictionaryMutationOperation,
        pointers: &[*mut TclObj],
        objects: &crate::value_ops::RuntimeDictionaryObjects,
        append_objects: &crate::value_ops::RuntimeAppendObjects,
    ) -> Result<Code, Code> {
        use crate::value_ops::RuntimeAppendValue;
        use tcl_cmd_core::native_dictionary::NativeDictionaryObjects;
        let evaluated = EvaluatedTarget {
            root: dictionary.receiver.clone(),
            element: None,
            original_name: None,
            original_index: None,
            combined: false,
        };
        let capture = self.body_capture_target(&evaluated, Some(dictionary.slot), true)?;
        let root = capture.root.as_slice();
        let access = self.trace_access(root, root, None, &capture.home, false);
        let failed = self.fire_var_trace_resolved(&capture.home, &access, b"read");
        if self.host_refusal_pending() {
            return Ok(Code::Error);
        }
        let original = if failed {
            None
        } else {
            capture.receiver.read_initial().ok().flatten()
        };
        if original.is_none() {
            self.traces.borrow_mut().pending_err.take();
            let mut exception = self.exc.borrow_mut();
            exception.code = b"TCL READ VARNAME".to_vec();
            exception.code_explicit = false;
        }
        let original = original.map(RuntimeAppendValue::borrowed);
        let prepared = objects
            .prepare(original.as_ref())
            .map_err(|error| self.report_cmd_error(error))?;
        let result = self
            .body_dictionary_mutation_value(objects, append_objects, dictionary, prepared, pointers)
            .map_err(|error| self.report_cmd_error(error))?;
        if let Err(error) = capture.receiver.store(result.as_ptr()) {
            return Ok(crate::builtins::var_error(self, root, error));
        }
        if self.fire_var_trace_resolved(&capture.home, &access, b"write") {
            return Ok(crate::builtins::var_error(
                self,
                root,
                crate::frame::VarError::TraceError,
            ));
        }
        if self.host_refusal_pending() {
            return Ok(Code::Error);
        }
        match capture.receiver.read() {
            Ok(Some(value)) => self.set_result(value),
            Ok(None) | Err(_) => self.set_result_bytes(b""),
        }
        Ok(Code::Ok)
    }

    fn body_dictionary_mutation_value(
        &self,
        objects: &crate::value_ops::RuntimeDictionaryObjects,
        append_objects: &crate::value_ops::RuntimeAppendObjects,
        dictionary: &DictionaryMutationOperation,
        prepared: crate::dict::PreparedNativeDictionary,
        pointers: &[*mut TclObj],
    ) -> Result<crate::value_ops::RuntimeAppendValue, tcl_cmd_core::CmdError> {
        use crate::value_ops::{RuntimeAppendValue, RuntimeIncrementObjects};
        use tcl_registry::native_dictionary_compilation::NativeDictionaryMutationKind as Kind;
        let dialect = self.native_invocation_dialect();
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
                let issued = dialect.native_object_append_protocol(None).ok_or(
                    ValueError::CommandProtocolUnavailable("native compiled dictionary append"),
                )?;
                tcl_cmd_core::native_dictionary::update_prepared_member(
                    objects,
                    prepared,
                    &inputs[0],
                    |original| match original {
                        None => Ok(inputs[1].clone()),
                        Some(_) => tcl_cmd_core::native_append::append_dictionary_operands(
                            append_objects,
                            issued.recipe(),
                            original,
                            &inputs[1..],
                        )
                        .map_err(Into::into),
                    },
                )
            }
            Kind::Lappend => tcl_cmd_core::native_dictionary::update_prepared_member(
                objects,
                prepared,
                &inputs[0],
                |original| {
                    let result = crate::list::append_native_elements(
                        original.map(RuntimeAppendValue::as_ptr),
                        &pointers[1..],
                        objects.protocol,
                    )?;
                    Ok(RuntimeAppendValue::retain(result.as_ptr()))
                },
            ),
            Kind::Incr(amount) => {
                let increment = RuntimeIncrementObjects::selected(dialect)?;
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
