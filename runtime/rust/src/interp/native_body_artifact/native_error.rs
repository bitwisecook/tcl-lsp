// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual original Error stack objects and immediate completion.

use super::*;
use tcl_registry::native_error_compilation::{NativeErrorInstruction, NativeErrorStep};

pub(super) struct ErrorOperation {
    steps: Vec<ErrorStep>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

enum ErrorStep {
    Operand(NamespaceOperand),
    List(usize),
    DictionaryPut,
    ReturnError,
}

impl Builder<'_> {
    pub(super) fn error_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeErrorInstruction,
        depth: u32,
    ) -> Result<ErrorOperation, ValueError> {
        let mut prepared_words = HashMap::new();
        let mut steps = Vec::with_capacity(recipe.steps.len());
        for step in recipe.steps {
            steps.push(match step {
                NativeErrorStep::Word(operand) => ErrorStep::Operand(self.namespace_operand(
                    words,
                    &operand,
                    &mut prepared_words,
                    depth,
                )?),
                NativeErrorStep::Literal(value) => ErrorStep::Operand(NamespaceOperand::Literal(
                    self.literals.intern_bytes(&value),
                )),
                NativeErrorStep::List(count) => {
                    ErrorStep::List(usize::try_from(count).expect("bounded native Error pairs"))
                }
                NativeErrorStep::DictionaryPut => ErrorStep::DictionaryPut,
                NativeErrorStep::ReturnError => ErrorStep::ReturnError,
            });
        }
        Ok(ErrorOperation {
            steps,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_error(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        error: &ErrorOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let mut stack = Vec::<obj::Owned>::new();
        for step in &error.steps {
            match step {
                ErrorStep::Operand(operand) => {
                    stack.push(self.body_namespace_operand(artifact, command, operand, execution)?);
                    if execution.done {
                        return Ok(Code::Ok);
                    }
                }
                ErrorStep::List(count) => {
                    let values = stack
                        .split_off(stack.len().checked_sub(*count).expect("native Error stack"));
                    let pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                    stack.push(obj::Owned::fresh(crate::list::new_list_obj_native(
                        &pointers,
                        artifact.stamp.source_protocol,
                    )));
                }
                ErrorStep::DictionaryPut => {
                    let value = stack.pop().expect("native Error value");
                    let key = stack.pop().expect("native Error key");
                    let original = stack.pop().expect("native Error options");
                    let mut dictionary = crate::dict::PreparedNativeDictionary::prepare(
                        Some(original.as_ptr()),
                        artifact.stamp.source_protocol,
                    )
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                    dictionary
                        .set_member(key.as_ptr(), value.as_ptr())
                        .map_err(|error| self.report_cmd_error(error.into()))?;
                    stack.push(dictionary.into_value());
                }
                ErrorStep::ReturnError => {
                    let options = stack.pop().expect("native Error options");
                    let value = stack.pop().expect("native Error message");
                    let code = self.process_original_c_return_options(tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate, 1, 0, options.as_ptr()).map_err(|error| self.report_cmd_error(error.into()))?;
                    self.set_result(value.as_ptr());
                    if code == Code::Error {
                        self.capture_original_return_instruction_context(
                            tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                            value.as_ptr(),
                            options.as_ptr(),
                        );
                    }
                    return Ok(code);
                }
            }
        }
        unreachable!("native Error instruction has an immediate completion")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_error_artifacts_match_native_guest_result_and_local_order() {
        let sources = [
            b"error BODY".as_slice(),
            b"error [set message BODY] [set info STACK]",
            b"error [set message BODY] [set info STACK] [set code {FOO BAR}]",
            b"error {*}{BODY STACK {FOO BAR}}",
        ];
        for (profile, observations) in [
            (
                "tcl8.6",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_error_compilation/8.6.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_error_compilation/9.0.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_error_compilation/9.1.tsv"
                ),
            ),
        ] {
            for (index, source) in sources.iter().enumerate() {
                let mut interp = super::super::tests::interpreter(profile);
                let mut declaration = b"proc p {} {".to_vec();
                declaration.extend_from_slice(source);
                declaration.push(b'}');
                assert_eq!(interp.eval_str(&declaration), Code::Ok, "{profile}/{index}");
                assert_eq!(interp.eval_str(b"p"), Code::Error, "{profile}/{index}");
                let row = observations
                    .lines()
                    .nth(index + 1)
                    .unwrap()
                    .split('\t')
                    .collect::<Vec<_>>();
                assert_eq!(row[1], "1");
                assert_eq!(interp.result_bytes(), b"BODY", "{profile}/{index}");
                assert_eq!(row[2], "424f4459");
                let procedure = interp.proc_def(b"p").unwrap();
                let artifact = cache(procedure.body.as_ptr())
                    .unwrap_or_else(|| panic!("{profile}/{index}: actual Error artifact"));
                assert!(artifact
                    .scripts
                    .values()
                    .flat_map(|script| &script.commands)
                    .any(|command| matches!(command.operation, Operation::Error(_))));
                let names = artifact
                    .compiled_local_layout()
                    .unwrap()
                    .names
                    .iter()
                    .flatten()
                    .map(|name| name.as_bytes().to_vec())
                    .collect::<Vec<_>>();
                assert_eq!(
                    names,
                    row[3]
                        .split_whitespace()
                        .map(|name| name.as_bytes().to_vec())
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}
