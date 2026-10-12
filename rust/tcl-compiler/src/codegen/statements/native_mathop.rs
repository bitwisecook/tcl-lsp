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

#[cfg(test)]
mod tests {
    use tcl_runtime_api::CompileService;

    #[test]
    fn original_comparison_preparation_reaches_nested_words_without_an_inline_hook() {
        // naming.mathop-procedure-source-controls
        // docs/design/analysis/name-resolution-proofs/mathop-procedure-source-controls.md
        // Emission keeps the actual compiler recipe; the unchanged native
        // control separately checks public completion and operand effects.
        let operands = "[::tcl::mathop::<] [::tcl::mathop::< [tick]] [::tcl::mathop::eq [tick]]";
        for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
            let (_owner, entry) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            for source in [
                format!("list {operands}"),
                format!("return [list {operands}]"),
            ] {
                let module = service
                    .compile_procedure_with_entry(
                        tcl_runtime_api::ProcedureCompileTarget {
                            source: &source,
                            namespace: "::",
                            parameters: &[],
                        },
                        profile,
                        &entry,
                        tcl_runtime_api::ProcedureDispatch::Optimised,
                    )
                    .unwrap();
                for original in [
                    b"::tcl::mathop::<".as_slice(),
                    b"::tcl::mathop::< [tick]".as_slice(),
                    b"::tcl::mathop::eq [tick]".as_slice(),
                ] {
                    assert!(
                        module.top_level.instructions.iter().any(|instruction| {
                            instruction.native_compiler_selection.is_some()
                                && instruction.source_cmd_text.bytes() == original
                        }),
                        "{engine}/{source}: original compiler selection for {original:?}",
                    );
                }
                assert!(
                    module
                        .top_level
                        .literals
                        .entries()
                        .iter()
                        .all(|literal| literal.bytes() != b"tick"),
                    "{engine}/{source}: the selected comparison recipe has no operand invocation",
                );
            }
        }
    }
}
