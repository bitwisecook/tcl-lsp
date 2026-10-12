// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compiled Next operands and actual late OO selection under one body owner.
use super::native_list_conversion::BodyObjectOperation;
use super::*;
use crate::interp::native_arguments::OriginalArguments;

impl Interp {
    fn body_next_concatenate(
        &self,
        operation: &BodyObjectOperation,
        target: obj::Owned,
        source: obj::Owned,
        protocol: NativeStringProtocol,
    ) -> Result<obj::Owned, tcl_cmd_core::CmdError> {
        let inputs = OriginalArguments::enter(self, &[target.as_ptr(), source.as_ptr()])?;
        let reached = crate::list::concatenate_native_lists_using(
            target.as_ptr(),
            source.as_ptr(),
            protocol,
            &|| {
                operation.check(self)?;
                inputs.check_original_for(self, 0, target.as_ptr())?;
                inputs.check_original_for(self, 1, source.as_ptr())
            },
            &|error| operation.refuse(error),
        )
        .map_err(|error| operation.retain(error));
        // Original target/source roles retire in their existing reached order.
        drop(inputs);
        drop(target);
        drop(source);
        if let Err(first) = operation.check(self) {
            drop(reached);
            operation.check(self)?;
            return Err(first);
        }
        reached
    }

    pub(super) fn execute_body_next(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        recipe: &tcl_registry::native_tcloo_compilation::NativeTclOoInstruction,
        literal_slots: &[Option<usize>],
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let tcl_registry::native_tcloo_compilation::NativeTclOoInstruction::Next {
            class,
            words,
            list,
        } = recipe
        else {
            unreachable!("selected Next helper")
        };
        let class = *class;
        let list = list.as_deref();
        let operation =
            BodyObjectOperation::capture(self).map_err(|first| self.report_cmd_error(first))?;
        let mut stack = Vec::<obj::Owned>::new();
        let reached = (|| {
            if let Some(steps) = list {
                for step in steps {
                    operation
                        .check(self)
                        .map_err(|first| self.report_cmd_error(first))?;
                    match *step {
                        NativeArgumentListStep::Word(index) => {
                            stack.push(self.body_helper_operand(
                                artifact,
                                command,
                                words,
                                literal_slots,
                                index,
                                execution,
                            )?);
                            operation
                                .check(self)
                                .map_err(|first| self.report_cmd_error(first))?;
                            if execution.done {
                                return Ok(Code::Ok);
                            }
                        }
                        NativeArgumentListStep::List(count) => {
                            let operands = stack.split_off(stack.len() - count);
                            let members =
                                operands.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                            stack.push(obj::Owned::fresh(crate::list::new_list_obj_native(
                                &members,
                                artifact.stamp.source_protocol,
                            )));
                            drop(operands);
                            operation
                                .check(self)
                                .map_err(|first| self.report_cmd_error(first))?;
                        }
                        NativeArgumentListStep::Concat => {
                            let source = stack.pop().expect("helper List source");
                            let target = stack.pop().expect("helper List target");
                            stack.push(
                                self.body_next_concatenate(
                                    &operation,
                                    target,
                                    source,
                                    artifact.stamp.source_protocol,
                                )
                                .map_err(|first| self.report_cmd_error(first))?,
                            );
                        }
                    }
                }
            } else {
                for index in 0..words.len() {
                    operation
                        .check(self)
                        .map_err(|first| self.report_cmd_error(first))?;
                    stack.push(self.body_helper_operand(
                        artifact,
                        command,
                        words,
                        literal_slots,
                        index,
                        execution,
                    )?);
                    operation
                        .check(self)
                        .map_err(|first| self.report_cmd_error(first))?;
                    if execution.done {
                        return Ok(Code::Ok);
                    }
                }
            }
            // A genuine parent List retains its Native role until the invocation
            // completes. Header/backing/children views add allocation safety only.
            let members = if list.is_some() {
                Some(
                    operation
                        .list(self, stack[0].as_ptr(), artifact.stamp.source_protocol)
                        .map_err(|first| self.report_cmd_error(first))?,
                )
            } else {
                None
            };
            let arguments = members.as_ref().map_or_else(
                || stack.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>(),
                |members| members.members.clone(),
            );
            let inputs = OriginalArguments::enter(self, &arguments)
                .map_err(|first| self.report_cmd_error(first))?;
            if arguments.len() < if class { 2 } else { 1 } {
                return Err(self.report_cmd_error(operation.refuse(unavailable(
                    "native TclOO invocation List has insufficient words",
                ))));
            }
            let current = |actual: &Interp| {
                operation.check(actual)?;
                inputs.check_for(actual)?;
                if let Some(members) = &members {
                    members.check(actual, &operation)?;
                }
                Ok(())
            };
            let code = self.execute_native_oo_next(
                &arguments,
                class,
                artifact.stamp.physical,
                &current,
                &|actual, head| operation.bytes_using(actual, head, &|| current(actual)),
            );
            current(self).map_err(|first| self.report_cmd_error(first))?;
            Ok(code)
        })();
        // Reached operands retire even on refusal. The original operation/frame
        // remains entered through all cleanup; released headers need not survive.
        drop(stack);
        operation
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        reached
    }
}

#[cfg(test)]
#[path = "native_oo_next/original_stages_tests.rs"]
mod original_stages_tests;
