// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original dictionary operands shared with the registered native compiler.

use super::*;
use tcl_registry::native_dictionary_compilation::{
    NativeDictionaryLookupInstruction, NativeDictionaryLookupKind,
};

pub(super) struct DictionaryLookupOperation {
    recipe: NativeDictionaryLookupInstruction,
    operands: Vec<NamespaceOperand>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn dictionary_lookup_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        recipe: NativeDictionaryLookupInstruction,
        depth: u32,
    ) -> Result<DictionaryLookupOperation, tcl_cmd_core::CmdError> {
        let mut prepared_words = HashMap::new();
        let mut operands = Vec::with_capacity(recipe.operands.len());
        for operand in &recipe.operands {
            operands.push(self.namespace_operand(captured, operand, &mut prepared_words, depth)?);
        }
        Ok(DictionaryLookupOperation {
            recipe,
            operands,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_dictionary_lookup(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        dictionary: &DictionaryLookupOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let original = native_list_conversion::BodyObjectOperation::capture(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let mut values = Vec::with_capacity(dictionary.operands.len());
        let reached = (|| {
            for operand in &dictionary.operands {
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                values.push(self.body_namespace_operand(artifact, command, operand, execution)?);
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                if execution.done {
                    return Ok(Code::Ok);
                }
            }
            let pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
            let receiver = pointers[0];
            let count = usize::try_from(dictionary.recipe.key_count).expect("native key count");
            let keys = &pointers[1..=count];
            let protocol = crate::value_ops::RuntimeDictionaryObjects::selected(self)
                .map_err(|error| self.report_cmd_error(error))?
                .protocol;
            original
                .check(self)
                .map_err(|error| self.report_cmd_error(error))?;
            let result = original.dictionary_lookup(
                self,
                receiver,
                keys,
                protocol,
                dictionary.recipe.kind,
                (dictionary.recipe.kind == NativeDictionaryLookupKind::GetDefault)
                    .then(|| pointers[count + 1]),
            );
            match result {
                Ok(value) => {
                    self.set_result(value);
                    original
                        .check(self)
                        .map_err(|error| self.report_cmd_error(error))?;
                    Ok(Code::Ok)
                }
                Err(error) => Ok(self.report_cmd_error(error)),
            }
        })();
        drop(values);
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }
}

#[cfg(test)]
mod original_lookup_boundary_tests {
    use super::*;
    use crate::cmd_control::original_list_conversion_tests as fixture;
    use std::cell::Cell;

    #[test]
    fn original_compiled_dictionary_lookup_refusal_precedes_missing_and_default_results() {

        // Same portable Get/Exists/GetDefault path, genuine compiled frame and
        // original primary callback. Native public command observations stay separate.
        for kind in [
            NativeDictionaryLookupKind::Get,
            NativeDictionaryLookupKind::Exists,
            NativeDictionaryLookupKind::GetDefault,
        ] {
            for mode in [1, 2, 3] {
                let mut interpreter = fixture::native("tcl9.0");
                let _current = fixture::install(&mut interpreter, mode);
                interpreter
                    .frames
                    .borrow_mut()
                    .push(crate::namespace::GLOBAL);
                let operation =
                    native_list_conversion::BodyObjectOperation::capture(&interpreter).unwrap();
                let dictionary = fixture::original(b"key VALUE");
                let key = fixture::string(b"key");
                let default = fixture::later();
                interpreter.set_result_bytes(b"PRIOR");
                let prior = interpreter.get_obj_result();
                let before = crate::counters::live_objs();
                let reached = operation.dictionary_lookup(
                    &mut interpreter,
                    dictionary.as_ptr(),
                    &[key.as_ptr()],
                    NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
                    kind,
                    Some(default.as_ptr()),
                );
                let first = reached.unwrap_err();
                assert_eq!(
                    first.native_execution_refusal(),
                    interpreter.native_execution_refusal().as_ref()
                );
                assert!(operation.check(&interpreter).is_err());
                assert_eq!(fixture::FREES.with(Cell::get), 1);
                assert_eq!(fixture::REFS.with(Cell::get), 1);
                assert_eq!(fixture::GETTERS.with(Cell::get), 0);
                assert_eq!(unsafe { (*dictionary.as_ptr()).ref_count }, 1);
                assert!(!core::ptr::eq(
                    obj::obj_type_ptr(dictionary.as_ptr()),
                    &crate::dict::TCL_DICT_TYPE
                ));
                assert_eq!(interpreter.get_obj_result(), prior);
                assert_eq!(crate::counters::live_objs(), before);
            }
        }
    }

    #[test]
    fn original_compiled_dictionary_lookup_keeps_current_leaf_and_guest_default_schedule() {

        let mut interpreter = fixture::native("tcl9.0");
        let _current = fixture::install(&mut interpreter, 0);
        interpreter
            .frames
            .borrow_mut()
            .push(crate::namespace::GLOBAL);
        let operation = native_list_conversion::BodyObjectOperation::capture(&interpreter).unwrap();
        let dictionary = fixture::original(b"outer {key VALUE}");
        let outer = fixture::string(b"outer");
        let key = fixture::string(b"key");
        let missing = fixture::string(b"missing");
        let default = fixture::later();
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
        let result = operation
            .dictionary_lookup(
                &mut interpreter,
                dictionary.as_ptr(),
                &[outer.as_ptr(), key.as_ptr()],
                protocol,
                NativeDictionaryLookupKind::Get,
                None,
            )
            .unwrap();
        assert_eq!(obj::bytes_of(result), b"VALUE");
        let absent = obj::Owned::fresh(
            operation
                .dictionary_lookup(
                    &mut interpreter,
                    dictionary.as_ptr(),
                    &[missing.as_ptr()],
                    protocol,
                    NativeDictionaryLookupKind::Exists,
                    None,
                )
                .unwrap(),
        );
        assert_eq!(
            operation
                .bytes(&interpreter, absent.as_ptr())
                .unwrap()
                .as_ref(),
            b"0"
        );
        let selected = operation
            .dictionary_lookup(
                &mut interpreter,
                dictionary.as_ptr(),
                &[missing.as_ptr()],
                protocol,
                NativeDictionaryLookupKind::GetDefault,
                Some(default.as_ptr()),
            )
            .unwrap();
        assert_eq!(selected, default.as_ptr());
        assert_eq!(fixture::GETTERS.with(Cell::get), 0);
        assert_eq!(fixture::FREES.with(Cell::get), 1);
        assert_eq!(fixture::REFS.with(Cell::get), 1);
        assert_eq!(unsafe { (*dictionary.as_ptr()).ref_count }, 1);
        assert!(operation.check(&interpreter).is_ok());
        assert!(interpreter.native_execution_refusal().is_none());
    }
}
