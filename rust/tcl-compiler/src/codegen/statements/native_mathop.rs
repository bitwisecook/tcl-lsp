// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native math-operator original stack instruction emission.
use super::{CodegenCtx, NativeEmissionTask as Task, Op, Operand};
use tcl_registry::native_mathop_compilation::{
    NativeMathOperator as M, NativeMathopInstruction, NativeMathopStep as S,
};
fn opcode(operator: M) -> Op {
    match operator {
        M::Invert => Op::BITNOT,
        M::Not => Op::LNOT,
        M::Add => Op::ADD,
        M::Multiply => Op::MULT,
        M::BitAnd => Op::BITAND,
        M::BitOr => Op::BITOR,
        M::BitXor => Op::BITXOR,
        M::Power => Op::EXPON,
        M::LeftShift => Op::LSHIFT,
        M::RightShift => Op::RSHIFT,
        M::Remainder => Op::MOD,
        M::NotEqual => Op::NEQ,
        M::StringNotEqual => Op::STR_NEQ,
        M::In => Op::LIST_IN,
        M::NotIn => Op::LIST_NOT_IN,
        M::Subtract => Op::SUB,
        M::Divide => Op::DIV,
        M::Less => Op::LT,
        M::LessEqual => Op::LE,
        M::Greater => Op::GT,
        M::GreaterEqual => Op::GE,
        M::Equal => Op::EQ,
        M::StringEqual => Op::STR_EQ,
        M::StringLess => Op::STR_LT,
        M::StringLessEqual => Op::STR_LE,
        M::StringGreater => Op::STR_GT,
        M::StringGreaterEqual => Op::STR_GE,
    }
}
impl CodegenCtx<'_> {
    pub(super) fn append_native_mathop_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeMathopInstruction,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<Task>,
    ) -> bool {
        let temporary = std::rc::Rc::new(std::cell::Cell::new(None));
        for step in recipe.steps {
            operations.push(match step {
                S::Word(operand) => {
                    let Some(task) = Self::native_namespace_word_task(&command.words, operand)
                    else {
                        return false;
                    };
                    task
                }
                S::Literal(value) => Task::Literal(value),
                S::Reverse(count) => {
                    let Ok(count) = i32::try_from(count) else {
                        return false;
                    };
                    Task::Operation(Op::REVERSE, vec![Operand::Imm(count)])
                }
                S::Primitive(operator) => {
                    Task::SwitchOperation(opcode(operator), Vec::new(), version)
                }
                S::Negate => Task::SwitchOperation(Op::UMINUS, Vec::new(), version),
                S::DeclareTemporary => Task::DeclareNativeTemporary(std::rc::Rc::clone(&temporary)),
                S::StoreTemporary => Task::NativeTemporaryOperation(
                    Op::STORE_SCALAR1,
                    std::rc::Rc::clone(&temporary),
                    Vec::new(),
                ),
                S::LoadTemporary => Task::NativeTemporaryOperation(
                    Op::LOAD_SCALAR1,
                    std::rc::Rc::clone(&temporary),
                    Vec::new(),
                ),
                S::UnsetTemporary => Task::NativeTemporaryOperation(
                    Op::UNSET_SCALAR,
                    std::rc::Rc::clone(&temporary),
                    vec![Operand::Imm(0)],
                ),
                S::Pop => Task::Operation(Op::POP, Vec::new()),
            });
        }
        true
    }
}
