// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original compiled private names, ordered operands and late ordinary lookup.

use super::*;
use tcl_registry::native_compilation::NativeNamedInvocationProtocol;
use tcl_registry::native_instruction_plan::{
    NativeNamedInvocationInstruction, NativeNamedInvocationWord,
};

pub(super) struct NamedOperation {
    recipe: NativeNamedInvocationInstruction,
    head: usize,
    operands: Vec<NamespaceOperand>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    fn named_command_literal(
        &mut self,
        bytes: &[u8],
        prerequisite: Option<
            &tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
        >,
    ) -> Result<usize, ValueError> {
        match prerequisite {
            Some(prerequisite) => self.retained_selected_command_literal(bytes, prerequisite),
            None => self.selected_command_literal(bytes),
        }
    }

    pub(super) fn named_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        recipe: NativeNamedInvocationInstruction,
        prerequisite: Option<
            &tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
        >,
        depth: u32,
    ) -> Result<NamedOperation, ValueError> {
        let direct = recipe.protocol == NativeNamedInvocationProtocol::Direct;
        let head = if direct {
            Some(self.named_command_literal(&recipe.name, prerequisite)?)
        } else {
            None
        };
        let mut prepared_words = HashMap::new();
        let mut operands = Vec::with_capacity(recipe.words.len());
        for word in &recipe.words {
            operands.push(match word {
                NativeNamedInvocationWord::Original(tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(0)) => {
                    prepared_words.insert(0, self.namespace_word(captured, 0, false, depth)?);
                    NamespaceOperand::Original(0)
                }
                NativeNamedInvocationWord::Original(operand) => self.namespace_operand(captured, operand, &mut prepared_words, depth)?,
                NativeNamedInvocationWord::Replacement(bytes) => NamespaceOperand::Literal(self.literals.intern_bytes(bytes)),
            });
        }
        let head = match head {
            Some(head) => head,
            None => self.named_command_literal(&recipe.name, prerequisite)?,
        };
        Ok(NamedOperation {
            recipe,
            head,
            operands,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_named(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        named: &NamedOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let rewrite = named.recipe.protocol == NativeNamedInvocationProtocol::EnsembleRewrite;
        let direct_head = (!rewrite).then(|| {
            obj::Owned::retain(
                artifact
                    .literals
                    .original(named.head)
                    .expect("compiled private name"),
            )
        });
        let mut values = Vec::with_capacity(named.operands.len());
        let mut originals = Vec::with_capacity(named.operands.len());
        for (operand, expanded) in named.operands.iter().zip(&named.recipe.expanded) {
            let value = self.body_namespace_operand(artifact, command, operand, execution)?;
            if execution.done {
                return Ok(Code::Ok);
            }
            if *expanded {
                let members = crate::list::list_elements_native_checked(
                    value.as_ptr(),
                    artifact.stamp.source_protocol,
                )
                .map_err(|error| self.report_cmd_error(error.into()))?;
                originals.extend(std::iter::repeat_n(None, members.len()));
                values.extend(members.into_iter().map(obj::Owned::retain));
            } else {
                originals.push(match operand {
                    NamespaceOperand::Original(index) => Some(&command.words[*index]),
                    NamespaceOperand::Literal(_) => None,
                });
                values.push(value);
            }
        }
        let head = direct_head.unwrap_or_else(|| {
            obj::Owned::retain(
                artifact
                    .literals
                    .original(named.head)
                    .expect("compiled private name"),
            )
        });
        let arguments = if rewrite {
            values
                .get(named.recipe.arguments_from + 1..)
                .ok_or_else(|| {
                    self.report_cmd_error(unavailable("native named rewrite operand layout").into())
                })?
        } else {
            &values
        };
        let mut argv = Vec::with_capacity(arguments.len() + 1);
        argv.push(head.as_ptr());
        argv.extend(arguments.iter().map(obj::Owned::as_ptr));
        let root = rewrite
            && self.begin_ensemble_rewrite(values.to_vec(), named.recipe.arguments_from + 1, 1);
        let file = self
            .cmd_frames
            .borrow()
            .last()
            .and_then(|frame| frame.file.clone());
        let mut added = 0;
        if file.is_some() {
            for (word, value) in originals.iter().zip(&values) {
                if let Some(word) = word.filter(|word| word.literal.is_some()) {
                    let line = self
                        .cmd_frames
                        .borrow()
                        .last()
                        .map_or(0, |frame| frame.line_base)
                        + line_of(
                            artifact.image.bytes(),
                            word.original.span().start() as usize,
                        );
                    self.arg_locs
                        .borrow_mut()
                        .push((value.as_ptr(), file.clone(), line));
                    added += 1;
                }
            }
        }
        let code = if rewrite {
            self.dispatch_invoke(&argv)
        } else {
            self.dispatch(&argv)
        };
        if root {
            self.clear_ensemble_rewrite();
        }
        if added != 0 {
            let mut locations = self.arg_locs.borrow_mut();
            let length = locations.len() - added;
            locations.truncate(length);
        }
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_named_invocations_retain_original_headers_and_late_lookup() {
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = super::super::tests::interpreter(profile);
            assert_eq!(
                interp.eval_str(b"proc p {} {set before 1; info locals}"),
                Code::Ok
            );
            assert_eq!(interp.eval_str(b"p"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"before", "{profile}");
            let procedure = interp.proc_def(b"p").unwrap();
            let artifact =
                cache(procedure.body.as_ptr()).expect("authentic named instruction artifact");
            let named = artifact
                .scripts
                .values()
                .flat_map(|script| &script.commands)
                .find_map(|command| match &command.operation {
                    Operation::NamedInvocation(named) => Some(named),
                    _ => None,
                })
                .unwrap();
            let header = artifact.literals.original(named.head).unwrap();
            assert_eq!(
                crate::dict::native_object_bytes(header, artifact.stamp.source_protocol).unwrap(),
                named.recipe.name
            );
            assert_eq!(interp.eval_str(b"p"), Code::Ok);
            assert_eq!(artifact.literals.original(named.head), Some(header));
            assert_eq!(interp.eval_str(b"proc q {} {array unset a [proc ::tcl::array::unset args {return REPLACED}; set pat *]}"), Code::Ok);
            assert_eq!(interp.eval_str(b"q"), Code::Ok);
            assert_eq!(
                interp.result_bytes(),
                b"REPLACED",
                "{profile}: late private lookup after original argv"
            );
        }
    }
}
