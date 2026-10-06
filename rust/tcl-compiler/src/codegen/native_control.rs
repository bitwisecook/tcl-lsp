// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original structured-control emission and checked native expression tasks.
use super::{CodegenCtx, NativeEmissionTask, NativeNamespaceRollback, Op, Operand};
use tcl_registry::native_compilation::NativeCompiledBodyContext;
use tcl_registry::native_control_compilation::{NativeControlCompilation, NativeControlOutcome};
use tcl_registry::native_control_instructions::{
    NativeControlBody, NativeControlInstruction, NativeControlTest,
};
use tcl_registry::native_expression_program::{NativeExpressionProgram, NativeExpressionTree};
use tcl_syntax::expr::{BinOp, ExprNode};

struct NativeExpressionCall {
    function: Vec<u8>,
    args: Vec<tcl_syntax::expr::NativeExprNode>,
    start: u32,
    end: u32,
}

struct NativeControlTaskSource {
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
}
impl NativeControlTaskSource {
    fn script(
        &self,
        body: NativeControlBody,
        context: NativeCompiledBodyContext,
    ) -> NativeEmissionTask {
        NativeEmissionTask::ControlScript(self.image.clone(), body, context, self.config)
    }
}
struct NativeCatchTaskContext {
    protocol: tcl_registry::native_control_instructions::NativeCatchProtocol,
    result: Option<Vec<u8>>,
    options: Option<Vec<u8>>,
    handler: String,
    merge: String,
}

impl CodegenCtx<'_> {
    pub(super) fn native_expression_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_expression_program::NativeExpressionInstruction,
    ) -> Option<Vec<NativeEmissionTask>> {
        use NativeEmissionTask as Task;
        if let Some(program) = recipe.program.as_ref()
            && matches!(program.tree, NativeExpressionTree::Rejected { .. })
        {
            if self.native_entry.is_some_and(|entry| {
                entry.execution_point.is_some_and(|point| {
                    program.evaluation_policy_matches(
                        entry.expression_policy.as_ref(),
                        tcl_registry::InvocationDialect::of_point(point),
                    )
                })
            }) {
                return Some(vec![Self::native_expression_task(program)]);
            }
            self.refuse_native_dependency();
            return None;
        }
        if !self.native_entry.is_some_and(
            tcl_registry::native_expression_program::compilation_expression_source_evaluation_supported,
        ) {
            self.refuse_native_dependency();
            return None;
        }
        if let Some(program) = recipe.program {
            use tcl_registry::native_expression_program::{
                ExpressionProgramEmission, expression_program_emission,
            };
            match self
                .native_entry
                .map(|entry| expression_program_emission(&program, entry))
            {
                Some(ExpressionProgramEmission::Native) => {}
                Some(ExpressionProgramEmission::AuthoredSource) => {
                    return Some(vec![
                        Task::Literal(program.source),
                        Task::Operation(Op::EXPR_STK, vec![]),
                    ]);
                }
                Some(ExpressionProgramEmission::Unavailable) | None => {
                    self.refuse_native_dependency();
                    return None;
                }
            }
            let converts = matches!(
                &program.tree,
                NativeExpressionTree::Parsed(
                    ExprNode::Literal { .. }
                        | ExprNode::String { .. }
                        | ExprNode::Var { .. }
                        | ExprNode::Command { .. }
                        | ExprNode::Call { .. }
                )
            );
            let mut tasks = vec![Self::native_expression_task(&program)];
            if converts {
                tasks.push(Task::Operation(Op::TRY_CVT_TO_NUMERIC, vec![]));
            }
            return Some(tasks);
        }
        let mut tasks = Vec::new();
        let count = recipe.operands.len();
        for (index, operand) in recipe.operands.into_iter().enumerate() {
            tasks.push(Self::native_namespace_word_task(&command.words, operand)?);
            if index + 1 < count {
                tasks.push(Task::Literal(b" ".to_vec()));
            }
        }
        let mut parts = 2 * count - 1;
        while parts > 255 {
            tasks.push(Task::Operation(Op::STR_CONCAT1, vec![Operand::Imm(255)]));
            parts -= 254;
        }
        if parts > 1 {
            tasks.push(Task::Operation(
                Op::STR_CONCAT1,
                vec![Operand::Imm(super::super::bytecode_imm(parts))],
            ));
        }
        tasks.push(Task::Operation(Op::EXPR_STK, vec![]));
        Some(tasks)
    }
    fn native_expression_task(program: &NativeExpressionProgram) -> NativeEmissionTask {
        match &program.tree {
            NativeExpressionTree::Parsed(tree) => {
                NativeEmissionTask::ExpressionNode(program.clone(), tree.clone())
            }
            NativeExpressionTree::Rejected {
                message,
                error_code,
            } => NativeEmissionTask::ExpressionSyntax(message.clone(), error_code.clone()),
        }
    }
    pub(super) fn native_control_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeControlCompilation<NativeControlInstruction>,
    ) -> Option<Vec<NativeEmissionTask>> {
        let source = NativeControlTaskSource {
            image: command.words.first()?.image().clone(),
            config: command.words[0].config(),
        };
        if !matches!(recipe.outcome, NativeControlOutcome::Rejected(_))
            && !self.native_boolean_probes_admitted(command, &recipe.preparations)
        {
            self.refuse_native_dependency();
            return None;
        }
        let instruction = match recipe.outcome {
            NativeControlOutcome::Rejected(failure) => {
                return Self::native_control_rejected_tasks(&failure, &recipe.preparations);
            }
            NativeControlOutcome::Generic => {
                return self.native_control_declined_tasks(command, recipe.preparations);
            }
            NativeControlOutcome::Inline(instruction) => instruction,
        };
        match instruction {
            NativeControlInstruction::Conditional(clauses) => {
                Some(self.native_conditional_tasks(&source, clauses))
            }
            NativeControlInstruction::While { test, body } => {
                Some(self.native_while_tasks(&source, test, body))
            }
            NativeControlInstruction::For {
                start,
                test,
                next,
                body,
            } => self.native_for_tasks(command, &source, start, &test, next, body),
            NativeControlInstruction::Catch {
                protocol,
                body,
                result,
                options,
            } => self.native_catch_tasks(command, &source, protocol, body, result, options),
        }
    }

    fn native_boolean_probes_admitted(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
        preparations: &[tcl_registry::native_control_compilation::NativeControlPreparationStep],
    ) -> bool {
        use tcl_registry::native_control_compilation::NativeControlPreparationStep;
        if !preparations
            .iter()
            .any(|step| matches!(step, NativeControlPreparationStep::BooleanProbe(_)))
        {
            return true;
        }
        self.native_entry.is_some_and(|entry| {
            let Some(point) = entry.execution_point else {
                return false;
            };
            let Some(protocol) = entry.source_string_protocol else {
                return false;
            };
            let Ok(captured) = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                &command.words,
                protocol,
            ) else {
                return false;
            };
            let policy = entry
                .expression_policy
                .as_ref()
                .filter(|policy| entry.invocation_policy == Some(policy.profile));
            tcl_registry::native_expression_program::control_boolean_probes_match(
                preparations,
                &captured,
                policy,
                tcl_registry::InvocationDialect::of_point(point),
            )
        })
    }

    fn native_control_rejected_tasks(
        failure: &tcl_registry::native_compilation::NativeCompilationFailure,
        preparations: &[tcl_registry::native_control_compilation::NativeControlPreparationStep],
    ) -> Option<Vec<NativeEmissionTask>> {
        use NativeEmissionTask as Task;
        let mut tasks = Vec::new();
        for visit in preparations {
            if let tcl_registry::native_control_compilation::NativeControlPreparationStep::DeclareLocal(name) = visit {
                tasks.push(Task::DeclareNamespaceLocal(name.clone()));
            }
        }
        tasks.push(Task::ExpressionSyntax(
            failure.message.as_ref()?.as_bytes().to_vec(),
            failure
                .error_code
                .as_ref()
                .map(|code| code.as_bytes().to_vec()),
        ));
        Some(tasks)
    }

    fn native_control_declined_tasks(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
        preparations: Vec<tcl_registry::native_control_compilation::NativeControlPreparationStep>,
    ) -> Option<Vec<NativeEmissionTask>> {
        use NativeEmissionTask as Task;
        let mut tasks = Vec::new();
        // Generic prefix visits are compiled speculatively and then rolled
        // back through the same local/literal-retaining owner as namespaces.
        let saved = NativeNamespaceRollback {
            instructions: self.instructions.len(),
            labels: self.label_positions.clone(),
            loop_regions: self.inline_loop_regions.len(),
            command_index: self.cmd_index,
        };
        for visit in preparations {
            use tcl_registry::native_control_compilation::NativeControlPreparationStep as Visit;
            match visit {
                Visit::BooleanProbe(_) => {}
                Visit::DeclareLocal(name) => tasks.push(Task::DeclareNamespaceLocal(name)),
                Visit::Literal(bytes) => tasks.push(Task::Literal(bytes)),
                Visit::Word(operand) => {
                    tasks.push(Self::native_namespace_word_task(&command.words, operand)?);
                }
                _ => return None,
            }
        }
        tasks.push(Task::NamespaceGenericRollback(saved, command.clone()));
        Some(tasks)
    }

    fn native_conditional_tasks(
        &mut self,
        source: &NativeControlTaskSource,
        clauses: Vec<tcl_registry::native_control_instructions::NativeConditionalClause>,
    ) -> Vec<NativeEmissionTask> {
        use NativeEmissionTask as Task;
        let mut tasks = Vec::new();
        let end = self.fresh_label("native_if_end");
        let mut terminal = false;
        for clause in clauses {
            let next = self.fresh_label("native_if_next");
            if let Some(NativeControlTest::Expression(program)) = clause.test {
                tasks.push(Self::native_expression_task(&program));
                tasks.push(Task::Operation(
                    Op::JUMP_FALSE4,
                    vec![Operand::Label(next.clone())],
                ));
            } else {
                terminal = true;
            }
            tasks.push(source.script(clause.body, NativeCompiledBodyContext::Inherit));
            tasks.push(Task::Operation(
                Op::JUMP4,
                vec![Operand::Label(end.clone())],
            ));
            tasks.push(Task::Label(next));
        }
        if !terminal {
            tasks.push(Task::Literal(Vec::new()));
        }
        tasks.push(Task::Label(end));
        tasks
    }

    fn native_while_tasks(
        &mut self,
        source: &NativeControlTaskSource,
        test: NativeControlTest,
        body: NativeControlBody,
    ) -> Vec<NativeEmissionTask> {
        use NativeEmissionTask as Task;
        let mut tasks = Vec::new();
        if test == NativeControlTest::Constant(false) {
            return vec![Task::Literal(Vec::new())];
        }
        let start = self.fresh_label("native_while_body");
        let body_end = self.fresh_label("native_while_body_end");
        let condition = self.fresh_label("native_while_test");
        let done = self.fresh_label("native_while_done");
        if matches!(test, NativeControlTest::Expression(_)) {
            tasks.push(Task::Operation(
                Op::JUMP4,
                vec![Operand::Label(condition.clone())],
            ));
        }
        tasks.push(Task::Label(start.clone()));
        tasks.push(source.script(body, NativeCompiledBodyContext::Loop));
        tasks.push(Task::Operation(Op::POP, vec![]));
        tasks.push(Task::Label(body_end.clone()));
        tasks.push(Task::Label(condition.clone()));
        let infinite = test == NativeControlTest::Constant(true);
        match test {
            NativeControlTest::Expression(program) => {
                tasks.push(Self::native_expression_task(&program));
                tasks.push(Task::Operation(
                    Op::JUMP_TRUE4,
                    vec![Operand::Label(start.clone())],
                ));
            }
            NativeControlTest::Constant(_) => tasks.push(Task::Operation(
                Op::JUMP4,
                vec![Operand::Label(start.clone())],
            )),
        }
        tasks.push(Task::Label(done.clone()));
        tasks.push(Task::Literal(Vec::new()));
        self.inline_loop_regions
            .push(super::super::InlineLoopRegion {
                start: start.clone(),
                end: body_end,
                continue_target: Some(if infinite { start } else { condition }),
                break_target: done,
            });
        tasks
    }

    fn native_for_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        source: &NativeControlTaskSource,
        start: NativeControlBody,
        test: &NativeExpressionProgram,
        next: NativeControlBody,
        body: NativeControlBody,
    ) -> Option<Vec<NativeEmissionTask>> {
        use NativeEmissionTask as Task;
        let mut tasks = Vec::new();
        // Dynamic start is evaluated exactly once before the first test.
        if start.script.is_some() {
            tasks.push(source.script(start, NativeCompiledBodyContext::Inherit));
        } else {
            tasks.push(Self::native_namespace_word_task(
                &command.words,
                start.operand,
            )?);
            tasks.push(Task::Operation(Op::EVAL_STK, vec![]));
        }
        tasks.push(Task::Operation(Op::POP, vec![]));
        let condition = self.fresh_label("native_for_test");
        let body_start = self.fresh_label("native_for_body");
        let body_end = self.fresh_label("native_for_body_end");
        let next_start = self.fresh_label("native_for_next");
        let next_end = self.fresh_label("native_for_next_end");
        let done = self.fresh_label("native_for_done");
        tasks.push(Task::Operation(
            Op::JUMP4,
            vec![Operand::Label(condition.clone())],
        ));
        tasks.push(Task::Label(body_start.clone()));
        tasks.push(source.script(body, NativeCompiledBodyContext::Loop));
        tasks.push(Task::Operation(Op::POP, vec![]));
        tasks.push(Task::Label(body_end.clone()));
        tasks.push(Task::Label(next_start.clone()));
        tasks.push(source.script(next, NativeCompiledBodyContext::Loop));
        tasks.push(Task::Operation(Op::POP, vec![]));
        tasks.push(Task::Label(next_end.clone()));
        tasks.push(Task::Label(condition));
        tasks.push(Self::native_expression_task(test));
        tasks.push(Task::Operation(
            Op::JUMP_TRUE4,
            vec![Operand::Label(body_start.clone())],
        ));
        tasks.push(Task::Label(done.clone()));
        tasks.push(Task::Literal(Vec::new()));
        self.inline_loop_regions
            .push(super::super::InlineLoopRegion {
                start: body_start,
                end: body_end,
                continue_target: Some(next_start.clone()),
                break_target: done.clone(),
            });
        self.inline_loop_regions
            .push(super::super::InlineLoopRegion {
                start: next_start,
                end: next_end,
                continue_target: None,
                break_target: done,
            });
        Some(tasks)
    }

    fn native_catch_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        source: &NativeControlTaskSource,
        protocol: tcl_registry::native_control_instructions::NativeCatchProtocol,
        body: NativeControlBody,
        result: Option<Vec<u8>>,
        options: Option<Vec<u8>>,
    ) -> Option<Vec<NativeEmissionTask>> {
        use NativeEmissionTask as Task;
        use tcl_registry::native_control_instructions::NativeCatchProtocol as Catch;
        let mut tasks = Vec::new();
        for name in result.iter().chain(options.iter()) {
            tasks.push(Task::DeclareNamespaceLocal(name.clone()));
        }
        let context = NativeCatchTaskContext {
            protocol,
            result,
            options,
            handler: self.fresh_label("native_catch_exception"),
            merge: self.fresh_label("native_catch_merge"),
        };
        if protocol == Catch::Tcl84 {
            self.native_catch84_tasks(command, source, body, context, &mut tasks)?;
        } else {
            Self::native_catch_modern_tasks(command, source, body, context, &mut tasks)?;
        }
        Some(tasks)
    }

    fn native_catch84_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        source: &NativeControlTaskSource,
        body: NativeControlBody,
        context: NativeCatchTaskContext,
        tasks: &mut Vec<NativeEmissionTask>,
    ) -> Option<()> {
        use NativeEmissionTask as Task;
        let NativeCatchTaskContext {
            result,
            handler,
            merge,
            ..
        } = context;
        let dynamic = body.script.is_none();
        let store = |name| Task::NamespaceLocalOperation(Op::STORE_SCALAR1, name);
        let saved = NativeNamespaceRollback {
            instructions: self.instructions.len(),
            labels: self.label_positions.clone(),
            loop_regions: self.inline_loop_regions.len(),
            command_index: self.cmd_index,
        };
        let failed = std::rc::Rc::new(std::cell::Cell::new(false));
        tasks.push(Task::BeginSpeculativeNativeCompilation(std::rc::Rc::clone(
            &failed,
        )));
        let protected = self.fresh_label("native_catch_protected");
        let protected_end = self.fresh_label("native_catch_protected_end");
        tasks.push(Task::BeginNativeCatchAt(
            handler.clone(),
            protected.clone(),
            protected_end.clone(),
        ));
        if dynamic {
            tasks.push(Self::native_namespace_word_task(
                &command.words,
                body.operand.clone(),
            )?);
        }
        tasks.push(Task::Label(protected));
        if dynamic {
            tasks.push(Task::Operation(Op::EVAL_STK, vec![]));
        } else {
            tasks.push(source.script(body, NativeCompiledBodyContext::ExceptionRange));
        }
        tasks.push(Task::Label(protected_end));
        if let Some(name) = &result {
            tasks.push(store(name.clone()));
        }
        tasks.push(Task::Operation(Op::POP, vec![]));
        tasks.push(Task::Literal(b"0".to_vec()));
        tasks.push(Task::Operation(
            Op::JUMP4,
            vec![Operand::Label(merge.clone())],
        ));
        tasks.push(Task::Label(handler));
        if let Some(name) = result {
            tasks.push(Task::Operation(Op::PUSH_RESULT, vec![]));
            tasks.push(store(name));
            tasks.push(Task::Operation(Op::POP, vec![]));
        }
        tasks.push(Task::Operation(Op::PUSH_RETURN_CODE, vec![]));
        tasks.push(Task::Label(merge));
        tasks.push(Task::EndNativeCatch);
        tasks.push(Task::FinishSpeculativeNativeCatch(
            saved,
            command.clone(),
            failed,
            self.native_compilation,
            self.catch_depth,
        ));
        Some(())
    }

    fn native_catch_modern_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        source: &NativeControlTaskSource,
        body: NativeControlBody,
        context: NativeCatchTaskContext,
        tasks: &mut Vec<NativeEmissionTask>,
    ) -> Option<()> {
        use NativeEmissionTask as Task;
        use tcl_registry::native_control_instructions::NativeCatchProtocol as Catch;
        let NativeCatchTaskContext {
            protocol,
            result,
            options,
            handler,
            merge,
        } = context;
        let dynamic = body.script.is_none();
        let reverse = |count| Task::Operation(Op::REVERSE, vec![Operand::Imm(count)]);
        if dynamic {
            tasks.push(Self::native_namespace_word_task(
                &command.words,
                body.operand.clone(),
            )?);
        }
        tasks.push(Task::BeginNativeCatch(handler.clone()));
        if dynamic {
            tasks.push(Task::Operation(Op::DUP, vec![]));
            tasks.push(Task::Operation(Op::EVAL_STK, vec![]));
            if protocol != Catch::Tcl85 {
                tasks.push(if protocol == Catch::Tcl91 {
                    Task::Operation(Op::SWAP, vec![])
                } else {
                    reverse(2)
                });
                tasks.push(Task::Operation(Op::POP, vec![]));
            }
        } else {
            tasks.push(source.script(body, NativeCompiledBodyContext::ExceptionRange));
        }
        tasks.push(Task::Literal(b"0".to_vec()));
        if protocol == Catch::Tcl91 {
            tasks.push(Task::Operation(Op::SWAP, vec![]));
        }
        tasks.push(Task::Operation(
            Op::JUMP4,
            vec![Operand::Label(merge.clone())],
        ));
        tasks.push(Task::Label(handler));
        if dynamic && protocol != Catch::Tcl85 {
            tasks.push(Task::Operation(Op::POP, vec![]));
        }
        if protocol == Catch::Tcl91 {
            tasks.push(Task::Operation(Op::PUSH_RETURN_CODE, vec![]));
            tasks.push(Task::Operation(Op::PUSH_RESULT, vec![]));
        } else {
            tasks.push(Task::Operation(Op::PUSH_RESULT, vec![]));
            tasks.push(Task::Operation(Op::PUSH_RETURN_CODE, vec![]));
        }
        tasks.push(Task::Label(merge));
        if options.is_some() {
            tasks.push(Task::Operation(Op::PUSH_RETURN_OPTS, vec![]));
        }
        tasks.push(Task::EndNativeCatch);
        Self::native_catch_store_tasks(protocol, result, options, dynamic, tasks);
        Some(())
    }

    fn native_catch_store_tasks(
        protocol: tcl_registry::native_control_instructions::NativeCatchProtocol,
        result: Option<Vec<u8>>,
        options: Option<Vec<u8>>,
        dynamic: bool,
        tasks: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        use tcl_registry::native_control_instructions::NativeCatchProtocol as Catch;
        let reverse = |count| Task::Operation(Op::REVERSE, vec![Operand::Imm(count)]);
        let store = |name| Task::NamespaceLocalOperation(Op::STORE_SCALAR1, name);
        if protocol == Catch::Tcl85 {
            tasks.push(reverse(if options.is_some() { 3 } else { 2 }));
            if let Some(name) = result {
                tasks.push(store(name));
            }
            tasks.push(Task::Operation(Op::POP, vec![]));
            if let Some(name) = options {
                tasks.push(reverse(2));
                tasks.push(store(name));
                tasks.push(Task::Operation(Op::POP, vec![]));
            }
            if dynamic {
                tasks.push(reverse(2));
                tasks.push(Task::Operation(Op::POP, vec![]));
            }
        } else {
            if let Some(name) = options {
                tasks.push(store(name));
                tasks.push(Task::Operation(Op::POP, vec![]));
            }
            if protocol == Catch::Tcl86 {
                tasks.push(reverse(2));
            }
            if let Some(name) = result {
                tasks.push(store(name));
            }
            tasks.push(Task::Operation(Op::POP, vec![]));
        }
    }

    fn native_expression_number(
        value: crate::tcl_expr_eval::TclValue,
    ) -> tcl_bytecode::NativeExpressionNumberLiteral {
        use crate::tcl_expr_eval::TclValue;
        match value {
            TclValue::Int(value) => tcl_bytecode::NativeExpressionNumberLiteral::Integer(value),
            TclValue::Float(value) => {
                tcl_bytecode::NativeExpressionNumberLiteral::Double(value.to_bits())
            }
            TclValue::Big(value) => {
                let text = value.to_str_radix(10);
                tcl_bytecode::NativeExpressionNumberLiteral::BigInteger {
                    negative: text.starts_with('-'),
                    digits: text.trim_start_matches('-').to_owned(),
                }
            }
        }
    }

    fn schedule_native_pooled_expression(
        &mut self,
        program: &NativeExpressionProgram,
        node: &tcl_syntax::expr::NativeExprNode,
        config: tcl_lexer::LexerConfig,
        pending: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        if tcl_registry::runtime_expr_validation::native_expression_pooled_subtrees(
            node,
            program.context.native_syntax,
        )
        .is_some_and(|trees| trees.first().is_some_and(|tree| *tree == node))
        {
            let mut valid = true;
            let constant = (*node).clone().map_text(|bytes| {
                String::from_utf8(bytes).unwrap_or_else(|_| {
                    valid = false;
                    String::new()
                })
            });
            let policy = crate::tcl_expr_eval::FoldPolicy::for_retained_entry(
                self.registry,
                self.invocation_dialect,
                &config,
            );
            if valid {
                use crate::tcl_expr_eval::NativeConstantResult;
                let version = self
                    .invocation_dialect
                    .and_then(|dialect| dialect.tcl_version)
                    .expect("selected native expression compiler");

                match crate::tcl_expr_eval::eval_native_constant(&constant, policy) {
                    NativeConstantResult::Number(value) => {
                        if tcl_registry::native_expression_program::native_expression_boolean_operator(node) {
                            if tcl_registry::native_expression_program::native_expression_private_logical_boolean85(node, version) {
                                pending.push(Task::PrivateLogicalBoolean85(value.is_truthy()));
                            } else {
                                pending.push(Task::Literal(if value.is_truthy() { b"1" } else { b"0" }.to_vec()));
                            }
                        } else {
                            pending.push(Task::PrivateExpressionNumber(version, Self::native_expression_number(value)));
                        }
                        return true;
                    }
                    NativeConstantResult::Resident {
                        bytes,
                        number: Some(value),
                    } => {
                        pending.push(Task::RegisteredExpressionNumber(
                            version,
                            bytes,
                            Self::native_expression_number(value),
                        ));
                        return true;
                    }
                    NativeConstantResult::Resident {
                        bytes,
                        number: None,
                    } => {
                        if version == tcl_dialect::TclVersion::V8_4
                            && tcl_registry::native_expression_program::native_expression_boolean_word84(&bytes)
                        {
                            pending.push(Task::RegisteredExpressionBoolean84(bytes));
                        } else {
                            pending.push(Task::Literal(bytes));
                        }
                        return true;
                    }
                    NativeConstantResult::Failure(failure) => {
                        if let Some(message) = failure.message {
                            pending.push(Task::ExpressionSyntax(
                                message.into_bytes(),
                                failure.error_code.map(String::into_bytes),
                            ));
                        } else {
                            self.refuse_native_dependency();
                        }
                        return true;
                    }
                    NativeConstantResult::Unavailable => {}
                }
            }
        }
        false
    }

    fn schedule_native_expression_literal(
        &mut self,
        text: Vec<u8>,
        start: u32,
        end: u32,
        config: tcl_lexer::LexerConfig,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        let policy = crate::tcl_expr_eval::FoldPolicy::for_retained_entry(
            self.registry,
            self.invocation_dialect,
            &config,
        );
        let value = core::str::from_utf8(&text).ok().map(|text| {
            crate::tcl_expr_eval::eval_native_constant(
                &ExprNode::Literal {
                    text: text.to_owned(),
                    start,
                    end,
                },
                policy,
            )
        });
        if let Some(crate::tcl_expr_eval::NativeConstantResult::Resident {
            number: Some(value),
            ..
        }) = value
        {
            let value = Self::native_expression_number(value);
            pending.push(Task::RegisteredExpressionNumber(
                self.invocation_dialect
                    .and_then(|dialect| dialect.tcl_version)
                    .expect("native literal cache issuer"),
                text,
                value,
            ));
        } else if self
            .invocation_dialect
            .and_then(|dialect| dialect.tcl_version)
            == Some(tcl_dialect::TclVersion::V8_4)
            && tcl_registry::native_expression_program::native_expression_boolean_word84(&text)
        {
            pending.push(Task::RegisteredExpressionBoolean84(text));
        } else {
            pending.push(Task::Literal(text));
        }
    }

    fn schedule_native_logical_expression(
        &mut self,
        program: NativeExpressionProgram,
        op: BinOp,
        left: tcl_syntax::expr::NativeExprNode,
        right: tcl_syntax::expr::NativeExprNode,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        let value = op == BinOp::Or;
        if tcl_registry::native_expression_program::native_logical_expression_compilation(
            program.context.native_syntax,
        ) == Some(tcl_registry::native_expression_program::NativeLogicalExpressionCompilation::NormalizedLeft84) {
            let true_label = self.fresh_label("native_expr_left_true");
            let normalized = self.fresh_label("native_expr_left_normalized");
            let end = self.fresh_label("native_expr_end");
            let tasks = vec![
                Task::ExpressionNode(program.clone(), left),
                Task::Operation(Op::JUMP_TRUE4, vec![Operand::Label(true_label.clone())]),
                Task::Literal(b"0".to_vec()),
                Task::Operation(Op::JUMP4, vec![Operand::Label(normalized.clone())]),
                Task::Label(true_label),
                Task::Literal(b"1".to_vec()),
                Task::Label(normalized),
                Task::Operation(Op::DUP, vec![]),
                Task::Operation(if value { Op::JUMP_TRUE4 } else { Op::JUMP_FALSE4 }, vec![Operand::Label(end.clone())]),
                Task::ExpressionNode(program, right),
                Task::Operation(if value { Op::LOR } else { Op::LAND }, vec![]),
                Task::Label(end),
            ];
            pending.extend(tasks.into_iter().rev());
            return;
        }
        let selected = self.fresh_label("native_expr_boolean");
        let end = self.fresh_label("native_expr_end");
        let jump = if value {
            Op::JUMP_TRUE4
        } else {
            Op::JUMP_FALSE4
        };
        let tasks = vec![
            Task::ExpressionNode(program.clone(), left),
            Task::Operation(jump, vec![Operand::Label(selected.clone())]),
            Task::ExpressionNode(program, right),
            Task::Operation(jump, vec![Operand::Label(selected.clone())]),
            Task::Literal(if value { b"0" } else { b"1" }.to_vec()),
            Task::Operation(Op::JUMP4, vec![Operand::Label(end.clone())]),
            Task::Label(selected),
            Task::Literal(if value { b"1" } else { b"0" }.to_vec()),
            Task::Label(end),
        ];
        pending.extend(tasks.into_iter().rev());
    }

    fn schedule_native_ternary_expression(
        &mut self,
        program: NativeExpressionProgram,
        condition: tcl_syntax::expr::NativeExprNode,
        true_branch: tcl_syntax::expr::NativeExprNode,
        false_branch: tcl_syntax::expr::NativeExprNode,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        let false_label = self.fresh_label("native_expr_false");
        let end = self.fresh_label("native_expr_end");
        let tasks = vec![
            Task::ExpressionNode(program.clone(), condition),
            Task::Operation(Op::JUMP_FALSE4, vec![Operand::Label(false_label.clone())]),
            Task::ExpressionNode(program.clone(), true_branch),
            Task::Operation(Op::JUMP4, vec![Operand::Label(end.clone())]),
            Task::Label(false_label),
            Task::ExpressionNode(program, false_branch),
            Task::Label(end),
        ];
        pending.extend(tasks.into_iter().rev());
    }

    fn schedule_native_math_call(
        &mut self,
        program: &NativeExpressionProgram,
        call: NativeExpressionCall,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use tcl_registry::mathfunc::NativeMathFunctionDispatch;
        match self.native_entry.and_then(
            tcl_registry::native_expression_program::compilation_expression_function_dispatch,
        ) {
            Some(NativeMathFunctionDispatch::FixedTable) => {
                self.schedule_native_fixed_math_call84(program, call, pending);
            }
            Some(NativeMathFunctionDispatch::CommandTable) => {
                self.schedule_native_dynamic_math_call(program, call, pending);
            }
            None => self.refuse_native_dependency(),
        }
    }

    fn schedule_native_fixed_math_call84(
        &mut self,
        program: &NativeExpressionProgram,
        call: NativeExpressionCall,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        use tcl_runtime_api::native_compilation::{
            NativeMathFunctionPrerequisite, NativeMathFunctionResolution,
        };
        let NativeExpressionCall {
            function,
            args,
            start,
            end,
        } = call;
        let Some(entry) = self.native_entry else {
            self.refuse_native_dependency();
            return;
        };
        let Some(table) = entry.math_functions.as_ref() else {
            self.refuse_native_dependency();
            return;
        };
        let required = NativeMathFunctionPrerequisite {
            interpreter: entry.interpreter,
            table: table.clone(),
        };
        if self
            .math_table_prerequisite
            .as_ref()
            .is_some_and(|existing| existing != &required)
        {
            self.refuse_native_dependency();
            return;
        }
        self.math_table_prerequisite = Some(required);
        let binding = match table.lookup_bytes(&function) {
            NativeMathFunctionResolution::Present(binding) => Some(binding.clone()),
            NativeMathFunctionResolution::Absent => None,
            NativeMathFunctionResolution::Unknown => {
                self.refuse_native_dependency();
                return;
            }
        };
        if binding.as_ref().and_then(|binding| binding.arity) != Some(args.len()) {
            let failure = Self::native_fixed_math_call_failure(
                program,
                &function,
                &args,
                (start, end),
                table,
            );
            let Some(failure) = failure else {
                self.refuse_native_dependency();
                return;
            };
            let Some(message) = failure.message else {
                self.refuse_native_dependency();
                return;
            };
            pending.push(Task::ExpressionSyntax(
                message.into_bytes(),
                failure.error_code.map(String::into_bytes),
            ));
            let count = binding
                .as_ref()
                .and_then(|binding| binding.arity)
                .unwrap_or(0);
            for arg in args.into_iter().take(count).rev() {
                pending.push(Task::ExpressionNode(program.clone(), arg));
            }
            return;
        }
        let binding = binding.expect("known native function arity");
        if binding
            .registry_identity
            .as_deref()
            .and_then(tcl_registry::mathfunc::global_command_bare_name)
            .map(str::as_bytes)
            != Some(function.as_slice())
        {
            self.refuse_native_dependency();
            return;
        }
        let Ok(argc) = u8::try_from(args.len()) else {
            self.refuse_native_dependency();
            return;
        };
        pending.push(Task::FixedMathCall(binding, argc));
        for arg in args.into_iter().rev() {
            pending.push(Task::ExpressionNode(program.clone(), arg));
        }
    }

    fn native_fixed_math_call_failure(
        program: &NativeExpressionProgram,
        function: &[u8],
        args: &[tcl_syntax::expr::NativeExprNode],
        span: (u32, u32),
        table: &tcl_runtime_api::native_compilation::NativeMathFunctionTable,
    ) -> Option<tcl_registry::native_compilation::NativeCompilationFailure> {
        use tcl_registry::native_compilation::NativeMathFunctionResolution as Compiler;
        use tcl_runtime_api::native_compilation::NativeMathFunctionResolution;
        let (start, end) = span;
        let mut reached = program.clone();
        reached.tree = NativeExpressionTree::Parsed(ExprNode::Call {
            function: function.to_vec(),
            args: args.to_vec(),
            start,
            end,
        });
        reached
            .compiler_steps(|name| match table.lookup(name) {
                NativeMathFunctionResolution::Present(binding) => binding
                    .arity
                    .map_or(Compiler::Unknown, |arity| Compiler::Known { arity }),
                NativeMathFunctionResolution::Absent => Compiler::Absent,
                NativeMathFunctionResolution::Unknown => Compiler::Unknown,
            })
            .into_iter()
            .find_map(|step| match step {
                tcl_registry::native_compilation::NativeExpressionCompilerStep::Failure(
                    failure,
                ) => Some(failure),
                _ => None,
            })
    }

    fn schedule_native_dynamic_math_call(
        &mut self,
        program: &NativeExpressionProgram,
        call: NativeExpressionCall,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        let NativeExpressionCall { function, args, .. } = call;
        let count = args.len() + 1;
        pending.push(Task::Operation(
            if count < 256 {
                Op::INVOKE_STK1
            } else {
                Op::INVOKE_STK4
            },
            vec![Operand::Imm(super::super::bytecode_imm(count))],
        ));
        for arg in args.into_iter().rev() {
            pending.push(Task::ExpressionNode(program.clone(), arg));
        }
        let mut name = b"tcl::mathfunc::".to_vec();
        name.extend_from_slice(&function);
        // This is the expression compiler's original lookup, before
        // arguments. Retain its actual binding rather than resolving
        // the reported name of a selected command a second time.
        let selected = self.native_entry.and_then(|entry| {
            let namespace = entry
                .namespaces
                .iter()
                .find(|namespace| namespace.token == entry.current_namespace)?;
            let binding = entry
                .lookup_command_bytes(entry.current_namespace, &name)
                .ok()?
                .cloned();
            tcl_registry::native_command_literal::native_compiled_command_name_literal_from_lookup(
                entry.execution_point?,
                entry.command_name_policy()?,
                tcl_runtime_api::native_command_name::NativeLiteralContext {
                    interpreter: entry.interpreter,
                    namespace_token: entry.current_namespace,
                    entry_epoch: entry.epoch,
                    namespace_path: namespace.path.clone(),
                },
                &name,
                binding,
            )
            .ok()
        });
        if let Some(selected) = selected {
            pending.push(Task::NativeCommandLiteral(Box::new(selected)));
        } else {
            self.refuse_native_dependency();
        }
    }

    fn schedule_native_expression_policy(
        &mut self,
        program: &NativeExpressionProgram,
        pending: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_expression_program::ExpressionProgramEmission;
        let emission = self
            .native_entry
            .map_or(ExpressionProgramEmission::Unavailable, |entry| {
                tcl_registry::native_expression_program::expression_program_emission(program, entry)
            });
        match emission {
            ExpressionProgramEmission::Native => true,
            ExpressionProgramEmission::AuthoredSource => {
                pending.push(Task::Operation(Op::EXPR_STK, vec![]));
                pending.push(Task::Literal(program.source.clone()));
                false
            }
            ExpressionProgramEmission::Unavailable => {
                self.refuse_native_dependency();
                false
            }
        }
    }

    pub(super) fn schedule_native_expression_node(
        &mut self,
        program: NativeExpressionProgram,
        node: tcl_syntax::expr::NativeExprNode,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        if !self.schedule_native_expression_policy(&program, pending) {
            return;
        }
        let original = self.source.clone();
        let config = self.lexer_config();
        if self.schedule_native_pooled_expression(&program, &node, config, pending) {
            return;
        }
        let arena = |start: u32, end: u32| {
            tcl_lexer::ExecutablePartArena::decompose(
                original.clone(),
                tcl_lexer::Span::new(program.span.start() + start, program.span.start() + end + 1),
                tcl_lexer::SubstFlags::default(),
                config,
            )
        };
        match node {
            ExprNode::Literal { text, start, end } => {
                self.schedule_native_expression_literal(text, start, end, config, pending);
            }
            ExprNode::String { text, start, end } => {
                if text.first() == Some(&b'{') {
                    pending.push(Task::Literal(text[1..text.len() - 1].to_vec()));
                } else if let Ok(arena) = arena(start + 1, end - 1) {
                    let root = arena.root();
                    pending.push(Task::List(std::rc::Rc::new(arena), root, 0));
                } else {
                    self.refuse_native_dependency();
                }
            }
            ExprNode::Var { start, end, .. } => {
                if let Ok(arena) = arena(start, end) {
                    let root = arena.root();
                    pending.push(Task::List(std::rc::Rc::new(arena), root, 0));
                } else {
                    self.refuse_native_dependency();
                }
            }
            ExprNode::Command { start, end, .. } => pending.push(Task::Script(
                original,
                tcl_lexer::Span::new(program.span.start() + start + 1, program.span.start() + end),
                config,
            )),
            ExprNode::Unary { op, operand } => {
                pending.push(Task::Operation(
                    Op::from_unaryop(op).expect("selected native operator"),
                    vec![],
                ));
                pending.push(Task::ExpressionNode(program, *operand));
            }
            ExprNode::Binary { op, left, right } if matches!(op, BinOp::And | BinOp::Or) => {
                self.schedule_native_logical_expression(program, op, *left, *right, pending);
            }
            ExprNode::Binary { op, left, right } => {
                pending.push(Task::Operation(
                    Op::from_binop(op).expect("selected native operator"),
                    vec![],
                ));
                pending.push(Task::ExpressionNode(program.clone(), *right));
                pending.push(Task::ExpressionNode(program, *left));
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => self.schedule_native_ternary_expression(
                program,
                *condition,
                *true_branch,
                *false_branch,
                pending,
            ),
            ExprNode::Call {
                function,
                args,
                start,
                end,
            } => self.schedule_native_math_call(
                &program,
                NativeExpressionCall {
                    function,
                    args,
                    start,
                    end,
                },
                pending,
            ),
            _ => self.native_dependency_refusal = true,
        }
    }
}
