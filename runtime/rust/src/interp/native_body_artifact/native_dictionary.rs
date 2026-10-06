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
    ) -> Result<DictionaryLookupOperation, ValueError> {
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
        let mut values = Vec::with_capacity(dictionary.operands.len());
        for operand in &dictionary.operands {
            values.push(self.body_namespace_operand(artifact, command, operand, execution)?);
            if execution.done {
                return Ok(Code::Ok);
            }
        }
        let pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
        let receiver = pointers[0];
        let count = usize::try_from(dictionary.recipe.key_count).expect("native key count");
        let keys = &pointers[1..=count];
        let result = match dictionary.recipe.kind {
            NativeDictionaryLookupKind::Get => tcl_cmd_core::dict::get(self, &receiver, keys),
            NativeDictionaryLookupKind::Exists => tcl_cmd_core::dict::exists(self, &receiver, keys),
            NativeDictionaryLookupKind::GetDefault => {
                tcl_cmd_core::dict::getdef(self, &receiver, keys, &pointers[count + 1])
            }
        };
        match result {
            Ok(value) => {
                self.set_result(value);
                Ok(Code::Ok)
            }
            Err(error) => Ok(self.report_cmd_error(error)),
        }
    }
}
