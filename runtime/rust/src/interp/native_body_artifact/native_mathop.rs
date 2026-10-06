// SPDX-License-Identifier: AGPL-3.0-or-later
//! Execute the original mathematical compiler stack against actual frame cells.
use super::*;
use tcl_registry::native_mathop_compilation::{
    NativeMathOperator as M, NativeMathopInstruction, NativeMathopStep as S,
};

pub(super) struct MathopOperation {
    steps: Vec<MathopStep>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}
enum MathopStep {
    Word(NamespaceOperand),
    Literal(usize),
    Reverse(usize),
    Primitive(M),
    Negate,
    StoreTemporary(usize),
    LoadTemporary(usize),
    UnsetTemporary(usize),
    Pop,
}
impl Builder<'_> {
    pub(super) fn mathop_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeMathopInstruction,
        depth: u32,
    ) -> Result<MathopOperation, ValueError> {
        let mut prepared_words = HashMap::new();
        let mut steps = Vec::new();
        let mut temporary = None;
        for step in recipe.steps {
            steps.push(match step {
                S::Word(operand) => MathopStep::Word(self.namespace_operand(
                    words,
                    &operand,
                    &mut prepared_words,
                    depth,
                )?),
                S::Literal(value) => MathopStep::Literal(self.literals.intern_bytes(&value)),
                S::Reverse(count) => MathopStep::Reverse(count),
                S::Primitive(operator) => MathopStep::Primitive(operator),
                S::Negate => MathopStep::Negate,
                S::DeclareTemporary => {
                    temporary = Some(self.lvt.intern_anonymous());
                    continue;
                }
                S::StoreTemporary => MathopStep::StoreTemporary(
                    temporary
                        .ok_or_else(|| unavailable("native math operator temporary declaration"))?,
                ),
                S::LoadTemporary => MathopStep::LoadTemporary(
                    temporary
                        .ok_or_else(|| unavailable("native math operator temporary declaration"))?,
                ),
                S::UnsetTemporary => MathopStep::UnsetTemporary(
                    temporary
                        .ok_or_else(|| unavailable("native math operator temporary declaration"))?,
                ),
                S::Pop => MathopStep::Pop,
            });
        }
        Ok(MathopOperation {
            steps,
            prepared_words,
        })
    }
}
struct PrimitiveContext(
    tcl_registry::InvocationDialect,
    std::rc::Rc<dyn tcl_platform::Host>,
);
impl crate::expr::ExprCtx for PrimitiveContext {
    fn numeric_host(&self) -> Option<std::rc::Rc<dyn tcl_platform::Host>> {
        Some(std::rc::Rc::clone(&self.1))
    }
    fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
        self.0
    }
    fn read_var(&mut self, _: &str) -> Result<obj::Owned, crate::expr_error::ExprError> {
        unreachable!("original mathematical operands already evaluated")
    }
    fn eval_command(&mut self, _: &str) -> Result<obj::Owned, crate::expr_error::ExprError> {
        unreachable!("original mathematical operands already evaluated")
    }
    fn call_function(
        &mut self,
        _: &str,
        _: &[obj::Owned],
    ) -> Result<obj::Owned, crate::expr_error::ExprError> {
        unreachable!("original mathematical primitive has no function call")
    }
}
impl Interp {
    fn compiled_mathop_primitive(
        &mut self,
        operator: M,
        mut operands: Vec<obj::Owned>,
        version: tcl_dialect::TclVersion,
    ) -> Result<obj::Owned, Code> {
        if matches!(operator, M::StringEqual | M::StringNotEqual) {
            let matched = tcl_cmd_core::switch::compiled_equal(
                self,
                &operands[0].as_ptr(),
                &operands[1].as_ptr(),
                version,
            )
            .map_err(|error| self.report_cmd_error(error))?;
            drop(operands.pop());
            return self
                .native_compiled_match_result(
                    operands.pop().expect("original equality left operand"),
                    matched == (operator == M::StringEqual),
                    version,
                    tcl_registry::native_string_compilation::NativeStringMatchOperation::Equal,
                )
                .map_err(|error| self.report_cmd_error(error.into()));
        }
        let boolean = matches!(
            operator,
            M::Not
                | M::NotEqual
                | M::In
                | M::NotIn
                | M::Less
                | M::LessEqual
                | M::Greater
                | M::GreaterEqual
                | M::Equal
                | M::StringLess
                | M::StringLessEqual
                | M::StringGreater
                | M::StringGreaterEqual
        );
        let result = crate::expr::eval_mathop(
            operator.spelling(),
            operands,
            &mut PrimitiveContext(self.native_invocation_dialect(), self.host()),
        )
        .map_err(|error| match error {
            tcl_cmd_core::mathop::MathopError::Op(error) => self.report_expr_error(error),
            tcl_cmd_core::mathop::MathopError::WrongArgs(_) => {
                self.report_cmd_error(unavailable("native math primitive operand geometry").into())
            }
        })?;
        if boolean {
            let truth = crate::expr::to_bool_in(result.as_ptr(), self.native_invocation_dialect())
                .map_err(|error| self.report_expr_error(error))?;
            return self
                .native_execution_boolean_constant(truth)
                .map(obj::Owned::retain)
                .map_err(|error| self.report_cmd_error(error.into()));
        }
        Ok(result)
    }
    fn mathop_stack_primitive(
        &mut self,
        operator: M,
        version: tcl_dialect::TclVersion,
        stack: &mut Vec<obj::Owned>,
    ) -> Result<(), Code> {
        let count = if matches!(operator, M::Invert | M::Not) {
            1
        } else {
            2
        };
        let start = stack.len().checked_sub(count).ok_or_else(|| {
            self.report_cmd_error(unavailable("native math operator stack geometry").into())
        })?;
        let operands = stack.split_off(start);
        stack.push(self.compiled_mathop_primitive(operator, operands, version)?);
        Ok(())
    }
    pub(super) fn execute_body_mathop(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operation: &MathopOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let mut stack = Vec::<obj::Owned>::new();
        let failure = || unavailable("native math operator stack geometry");
        for step in &operation.steps {
            match step {
                MathopStep::Word(operand) => {
                    let value =
                        self.body_namespace_operand(artifact, command, operand, execution)?;
                    if execution.done {
                        return Ok(Code::Ok);
                    }
                    stack.push(value);
                }
                MathopStep::Literal(index) => stack.push(self.body_namespace_operand(
                    artifact,
                    command,
                    &NamespaceOperand::Literal(*index),
                    execution,
                )?),
                MathopStep::Reverse(count) => {
                    let start = stack
                        .len()
                        .checked_sub(*count)
                        .ok_or_else(|| self.report_cmd_error(failure().into()))?;
                    stack[start..].reverse();
                }
                MathopStep::Primitive(operator) => {
                    self.mathop_stack_primitive(*operator, artifact.stamp.physical, &mut stack)?
                }
                MathopStep::Negate => {
                    let value = stack
                        .pop()
                        .ok_or_else(|| self.report_cmd_error(failure().into()))?;
                    stack.push(self.compiled_mathop_primitive(
                        M::Subtract,
                        vec![value],
                        artifact.stamp.physical,
                    )?);
                }
                MathopStep::StoreTemporary(slot) => {
                    let value = stack
                        .last()
                        .ok_or_else(|| self.report_cmd_error(failure().into()))?;
                    let stored = self
                        .frames
                        .borrow_mut()
                        .store_native_compiled_temporary(*slot, value.as_ptr());
                    stored.map_err(|_| self.report_cmd_error(failure().into()))?;
                }
                MathopStep::LoadTemporary(slot) => {
                    let value = self
                        .frames
                        .borrow()
                        .native_compiled_temporary(*slot)
                        .ok()
                        .flatten();
                    let value = value.ok_or_else(|| self.report_cmd_error(failure().into()))?;
                    stack.push(obj::Owned::retain(value));
                }
                MathopStep::UnsetTemporary(slot) => {
                    let unset = self
                        .frames
                        .borrow_mut()
                        .unset_native_compiled_temporary(*slot);
                    unset.map_err(|_| self.report_cmd_error(failure().into()))?;
                }
                MathopStep::Pop => {
                    stack
                        .pop()
                        .ok_or_else(|| self.report_cmd_error(failure().into()))?;
                }
            }
        }
        if stack.len() != 1 {
            return Err(self.report_cmd_error(failure().into()));
        }
        self.set_result(
            stack
                .pop()
                .expect("validated original math operator result")
                .as_ptr(),
        );
        Ok(Code::Ok)
    }
}
