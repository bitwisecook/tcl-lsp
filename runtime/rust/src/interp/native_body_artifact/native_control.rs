// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained original expression operands and structured native control ranges.
use super::*;
use tcl_registry::native_control_compilation::{NativeControlCompilation, NativeControlOutcome};
use tcl_registry::native_control_instructions::{
    NativeControlBody, NativeControlInstruction, NativeControlTest,
};
use tcl_registry::native_expression_program::{NativeExpressionProgram, NativeExpressionTree};
use tcl_syntax::expr::ExprNode;

pub(super) struct PreparedNativeExpression {
    program: NativeExpressionProgram,
    literals: HashMap<(u32, u32), usize>,
    arenas: HashMap<u32, ArenaInstruction>,
    commands: HashMap<u32, Span>,
    function_heads: HashMap<u32, usize>,
    fixed_functions: HashMap<u32, tcl_runtime_api::native_compilation::NativeMathFunctionBinding>,
    rejected: Option<(usize, usize)>,
    folded: HashMap<tcl_syntax::expr::NativeExprNode, Result<usize, (usize, usize)>>,
    logical_left84: HashMap<tcl_syntax::expr::NativeExprNode, [usize; 2]>,
}

struct ConstantExpressionContext {
    dialect: tcl_registry::InvocationDialect,
    literals: HashMap<(u32, u32), obj::Owned>,
}
impl ConstantExpressionContext {
    fn prepare(
        tree: &tcl_syntax::expr::NativeExprNode,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Self, ValueError> {
        let mut literals = HashMap::new();
        let mut pending = vec![tree];
        while let Some(node) = pending.pop() {
            match node {
                ExprNode::Literal { text, start, end } => {
                    let original = obj::Owned::fresh(obj::new_string_bytes(text));
                    let text = core::str::from_utf8(text)
                        .map_err(|_| unavailable("native constant numeral bytes"))?;
                    if let Some(number) = tcl_syntax::number::parse_whole_with(
                        text,
                        tcl_syntax::number::ParseFlags::for_syntax(dialect.numbers),
                    ) {
                        obj::adopt_native_scalar_cache(
                            original.as_ptr(),
                            tcl_syntax::scalar_getter::NativeScalarCache::Number(number),
                            dialect
                                .native_scalar_getter_protocol()
                                .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                        )?;
                    }
                    literals.insert((*start, *end), original);
                }
                ExprNode::String { text, start, end } => {
                    let text = core::str::from_utf8(text)
                        .map_err(|_| unavailable("native constant string bytes"))?;
                    let text = tcl_syntax::expr::fixed_string_operand(text)
                        .ok_or_else(|| unavailable("native constant string preparation"))?;
                    literals.insert(
                        (*start, *end),
                        obj::Owned::fresh(obj::new_string_bytes(text.as_bytes())),
                    );
                }
                ExprNode::Unary { operand, .. } => pending.push(operand),
                ExprNode::Binary { left, right, .. } => {
                    pending.push(right);
                    pending.push(left);
                }
                ExprNode::Ternary {
                    condition,
                    true_branch,
                    false_branch,
                } => {
                    pending.push(false_branch);
                    pending.push(true_branch);
                    pending.push(condition);
                }
                _ => return Err(unavailable("native constant expression leaf")),
            }
        }
        Ok(Self { dialect, literals })
    }
}
impl crate::expr::ExprCtx for ConstantExpressionContext {
    fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
        self.dialect
    }
    fn compiled_literal(&mut self, start: u32, end: u32) -> Option<obj::Owned> {
        self.literals.get(&(start, end)).cloned()
    }
    fn read_var(&mut self, _name: &str) -> Result<obj::Owned, crate::expr_error::ExprError> {
        Err(crate::expr_error::ExprError::host_refusal(
            unavailable("native constant expression variable")
                .native_access_refusal()
                .expect("typed refusal"),
        ))
    }
    fn eval_command(&mut self, _script: &str) -> Result<obj::Owned, crate::expr_error::ExprError> {
        Err(crate::expr_error::ExprError::host_refusal(
            unavailable("native constant expression script")
                .native_access_refusal()
                .expect("typed refusal"),
        ))
    }
    fn call_function(
        &mut self,
        _name: &str,
        _args: &[obj::Owned],
    ) -> Result<obj::Owned, crate::expr_error::ExprError> {
        Err(crate::expr_error::ExprError::host_refusal(
            unavailable("native constant expression function")
                .native_access_refusal()
                .expect("typed refusal"),
        ))
    }
}

pub(super) struct ControlOperation {
    pub(super) recipe: NativeControlCompilation<NativeControlInstruction>,
    pub(super) prepared: super::native_control_preparation::PreparedControlOperands,
    names: HashMap<Vec<u8>, usize>,
    empty: Option<usize>,
    zero: Option<usize>,
}
pub(super) struct ExpressionOperation {
    recipe: tcl_registry::native_expression_program::NativeExpressionInstruction,
    pub(super) prepared: super::native_control_preparation::PreparedControlOperands,
    static_program: Option<PreparedNativeExpression>,
    separator: Option<usize>,
}

impl Builder<'_> {
    fn expression_syntax_message(&mut self, message: &[u8]) -> usize {
        if self
            .interp
            .native_invocation_dialect()
            .native_return_options_application(
                tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
            )
            .is_some_and(|recipe| recipe.syntax_options_share_message())
        {
            self.literals.register_unshared(message)
        } else {
            self.literals.intern_bytes(message)
        }
    }

    pub(super) fn expression_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        recipe: tcl_registry::native_expression_program::NativeExpressionInstruction,
        depth: u32,
    ) -> Result<ExpressionOperation, ValueError> {
        let static_program = recipe
            .program
            .as_ref()
            .map(|program| self.prepare_body_expression(program, depth))
            .transpose()?;
        let visits = if static_program.is_some() {
            Vec::new()
        } else {
            recipe
                .operands
                .iter()
                .cloned()
                .map(tcl_registry::native_control_compilation::NativeControlPreparationStep::Word)
                .collect::<Vec<_>>()
        };
        let prepared = self.prepare_control_steps(captured, &visits, depth)?;
        let separator = (recipe.operands.len() > 1).then(|| self.literals.intern_bytes(b" "));
        Ok(ExpressionOperation {
            recipe,
            prepared,
            static_program,
            separator,
        })
    }
    pub(super) fn prepare_body_expression(
        &mut self,
        program: &NativeExpressionProgram,
        depth: u32,
    ) -> Result<PreparedNativeExpression, ValueError> {
        let mut prepared = PreparedNativeExpression {
            program: program.clone(),
            literals: HashMap::new(),
            arenas: HashMap::new(),
            commands: HashMap::new(),
            function_heads: HashMap::new(),
            fixed_functions: HashMap::new(),
            rejected: None,
            folded: HashMap::new(),
            logical_left84: HashMap::new(),
        };
        let NativeExpressionTree::Parsed(tree) = &program.tree else {
            if self.stamp.physical == tcl_dialect::TclVersion::V8_4 {
                let NativeExpressionTree::Rejected {
                    message,
                    error_code,
                } = &program.tree
                else {
                    unreachable!()
                };
                return Err(self.reject_native_compilation(
                    tcl_registry::native_compilation::NativeCompilationFailure {
                        message: String::from_utf8(message.clone()).ok(),
                        error_code: error_code
                            .as_ref()
                            .and_then(|bytes| String::from_utf8(bytes.clone()).ok()),
                        error_info: None,
                    },
                ));
            }
            if !program.evaluation_policy_matches(
                self.stamp.expression_policy.as_ref(),
                tcl_registry::InvocationDialect::for_version(self.stamp.physical),
            ) {
                return Err(unavailable("native expression error evaluation policy"));
            }
            let NativeExpressionTree::Rejected {
                message,
                error_code,
            } = &program.tree
            else {
                unreachable!()
            };
            let message = self.expression_syntax_message(message);
            let options = self.literals.register_private_return_options(
                tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral {
                    protocol: self.stamp.source_protocol,
                    words: vec![
                        tcl_runtime_api::native_return_literal::NativeKnownWordLiteral {
                            pieces: vec![b"-errorcode".to_vec()],
                            composite: false,
                        },
                        tcl_runtime_api::native_return_literal::NativeKnownWordLiteral {
                            pieces: vec![error_code.clone().unwrap_or_else(|| b"NONE".to_vec())],
                            composite: false,
                        },
                    ],
                    code: 0,
                    level: 1,
                    size: 1,
                },
            );
            prepared.rejected = Some((message, options));
            if self
                .interp
                .native_invocation_dialect()
                .native_return_options_application(
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                )
                .is_some_and(|recipe| recipe.syntax_options_share_message())
            {
                self.literals.retain_syntax_error_info(options, message);
            }
            return Ok(prepared);
        };
        let physical = tcl_registry::InvocationDialect::for_version(self.stamp.physical);
        if tcl_registry::native_expression_program::expression_program_emission_for_policy(
            program,
            self.stamp.expression_policy.as_ref(),
            physical,
        ) != tcl_registry::native_expression_program::ExpressionProgramEmission::Native
        {
            return Err(unavailable("native expression evaluation policy"));
        }
        enum Preparation<'a> {
            Node(&'a tcl_syntax::expr::NativeExprNode),
            LogicalLeft84(&'a tcl_syntax::expr::NativeExprNode),
            Failure(tcl_registry::native_compilation::NativeCompilationFailure),
        }
        let mut pending = vec![Preparation::Node(tree)];
        while let Some(preparation) = pending.pop() {
            let node = match preparation {
                Preparation::Node(node) => node,
                Preparation::LogicalLeft84(node) => {
                    let zero = self.literals.intern_bytes(b"0");
                    let one = self.literals.intern_bytes(b"1");
                    prepared.logical_left84.insert(node.clone(), [zero, one]);
                    continue;
                }
                Preparation::Failure(failure) => {
                    return Err(self.reject_native_compilation(failure));
                }
            };
            if tcl_registry::runtime_expr_validation::native_expression_pooled_subtrees(
                node,
                program.context.native_syntax,
            )
            .is_some_and(|trees| trees.first().is_some_and(|tree| *tree == node))
            {
                let dialect = self.interp.native_invocation_dialect();
                let value = crate::expr::eval_compiler_constant(
                    node,
                    &mut ConstantExpressionContext::prepare(node, dialect)?,
                );
                let folded = match value {
                    Ok(original) => {
                        let index = if tcl_registry::native_expression_program::native_expression_boolean_operator(node) {
                            let boolean = crate::expr::to_bool_in(original.as_ptr(), dialect)
                                .map_err(|_| unavailable("native folded Boolean producer"))?;
                            if tcl_registry::native_expression_program::native_expression_private_logical_boolean85(node, self.stamp.physical) {
                                self.literals.register_private_logical_boolean85(boolean)
                            } else {
                                self.literals.intern_bytes(if boolean { b"1" } else { b"0" })
                            }
                        } else {
                            if obj::has_string_rep(original.as_ptr()) {
                                let bytes = obj::bytes_of(original.as_ptr());
                                match obj::native_scalar_cache(original.as_ptr())? {
                                    Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(number)) => {
                                        let number = tcl_bytecode::NativeExpressionNumberLiteral::from_number(number).ok_or_else(|| unavailable("native resident constant numeric payload"))?;
                                        self.literals.intern_expression_number(&bytes, self.stamp.physical, number)
                                    }
                                    None => self.literals.intern_bytes(&bytes),
                                    Some(tcl_syntax::scalar_getter::NativeScalarCache::WordBoolean(_))
                                        if self.stamp.physical == tcl_dialect::TclVersion::V8_4 => {
                                        self.literals.intern_expression_boolean84(&bytes)
                                    }
                                    Some(_) => return Err(unavailable("native resident constant primary")),
                                }
                            } else {
                                self.original_literal(original)
                            }
                        };
                        Ok(index)
                    }
                    Err(error) => {
                        if error.native_access_refusal.is_some()
                            || error.native_execution_refusal.is_some()
                        {
                            return Err(unavailable("native constant expression execution"));
                        }
                        let message = self.expression_syntax_message(&error.msg);
                        let options = self.literals.register_private_return_options(
                            tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral {
                                protocol: self.stamp.source_protocol,
                                words: vec![
                                    tcl_runtime_api::native_return_literal::NativeKnownWordLiteral {
                                        pieces: vec![b"-errorcode".to_vec()], composite: false,
                                    },
                                    tcl_runtime_api::native_return_literal::NativeKnownWordLiteral {
                                        pieces: vec![error.code.unwrap_or_else(|| b"NONE".to_vec())], composite: false,
                                    },
                                ],
                                code: 0, level: 1, size: 1,
                            },
                        );
                        if self.interp.native_invocation_dialect()
                            .native_return_options_application(tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax)
                            .is_some_and(|recipe| recipe.syntax_options_share_message())
                        {
                            self.literals.retain_syntax_error_info(options, message);
                        }
                        Err((message, options))
                    }
                };
                prepared.folded.insert(node.clone(), folded);
                continue;
            }
            match node {
                ExprNode::Literal { text, start, end } => {
                    let index = if self.stamp.physical >= tcl_dialect::TclVersion::V8_5 {
                        let dialect = self.interp.native_invocation_dialect();
                        let original = ConstantExpressionContext::prepare(node, dialect)?;
                        let original = &original.literals[&(*start, *end)];
                        if let Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(number)) =
                            obj::native_scalar_cache(original.as_ptr())?
                        {
                            let number =
                                tcl_bytecode::NativeExpressionNumberLiteral::from_number(number)
                                    .ok_or_else(|| {
                                        unavailable("native expression literal numeric payload")
                                    })?;
                            self.literals.intern_expression_number(
                                text,
                                self.stamp.physical,
                                number,
                            )
                        } else {
                            self.literals.intern_bytes(text)
                        }
                    } else {
                        if tcl_registry::native_expression_program::native_expression_boolean_word84(text)
                        {
                            self.literals.intern_expression_boolean84(text)
                        } else {
                            self.literals.intern_bytes(text)
                        }
                    };
                    prepared.literals.insert((*start, *end), index);
                }
                ExprNode::String { text, start, end } => {
                    if text.first() == Some(&b'{') {
                        let value = tcl_syntax::backslash::collapse_brace_continuations_for(
                            &text[1..text.len() - 1],
                            self.stamp.grammar.brace_backslash_newline,
                        );
                        prepared
                            .literals
                            .insert((*start, *end), self.literals.intern_bytes(&value));
                    } else {
                        let span =
                            Span::new(program.span.start() + start + 1, program.span.start() + end);
                        let original = ExecutablePartArena::decompose(
                            self.image.clone(),
                            span,
                            tcl_lexer::SubstFlags::default(),
                            self.stamp.grammar,
                        )
                        .map_err(|_| unavailable("native expression quote arena"))?;
                        prepared
                            .arenas
                            .insert(*start, self.arena(&original, depth)?);
                    }
                }
                ExprNode::Var { start, end, .. } => {
                    let original = ExecutablePartArena::decompose(
                        self.image.clone(),
                        Span::new(program.span.start() + start, program.span.start() + end + 1),
                        tcl_lexer::SubstFlags::default(),
                        self.stamp.grammar,
                    )
                    .map_err(|_| unavailable("native expression variable arena"))?;
                    prepared
                        .arenas
                        .insert(*start, self.arena(&original, depth)?);
                }
                ExprNode::Command { start, end, .. } => {
                    let span =
                        Span::new(program.span.start() + start + 1, program.span.start() + end);
                    self.script(span, depth + 1)?;
                    prepared.commands.insert(*start, span);
                }
                ExprNode::Unary { operand, .. } => pending.push(Preparation::Node(operand)),
                ExprNode::Binary { op, left, right }
                    if matches!(op, tcl_syntax::expr::BinOp::And | tcl_syntax::expr::BinOp::Or)
                        && tcl_registry::native_expression_program::native_logical_expression_compilation(
                            program.context.native_syntax,
                        ) == Some(tcl_registry::native_expression_program::NativeLogicalExpressionCompilation::NormalizedLeft84) => {
                    pending.push(Preparation::Node(right));
                    pending.push(Preparation::LogicalLeft84(node));
                    pending.push(Preparation::Node(left));
                }
                ExprNode::Binary { left, right, .. } => {
                    pending.push(Preparation::Node(right));
                    pending.push(Preparation::Node(left));
                }
                ExprNode::Ternary {
                    condition,
                    true_branch,
                    false_branch,
                } => {
                    pending.push(Preparation::Node(false_branch));
                    pending.push(Preparation::Node(true_branch));
                    pending.push(Preparation::Node(condition));
                }
                ExprNode::Call {
                    function,
                    args,
                    start,
                    ..
                } => {
                    if tcl_registry::native_expression_program::expression_function_dispatch(
                        self.stamp.expression_policy.as_ref(), physical,
                    ) == Some(tcl_registry::mathfunc::NativeMathFunctionDispatch::CommandTable) {
                        let mut head = b"tcl::mathfunc::".to_vec();
                        head.extend_from_slice(function);
                        prepared
                            .function_heads
                            .insert(*start, self.selected_command_literal(&head)?);
                    } else {
                        use tcl_registry::native_compilation::NativeMathFunctionResolution as Compiler;
                        use tcl_runtime_api::native_compilation::NativeMathFunctionResolution as Actual;
                        let table = self
                            .stamp
                            .fixed_math
                            .as_ref()
                            .ok_or_else(|| unavailable("native fixed-function table"))?;
                        let binding = match table.lookup_bytes(function) {
                            Actual::Present(binding) => Some(binding.clone()),
                            Actual::Absent => None,
                            Actual::Unknown => {
                                return Err(unavailable("native fixed-function registration"))
                            }
                        };
                        if binding.as_ref().and_then(|binding| binding.arity) != Some(args.len()) {
                            let mut reached = program.clone();
                            reached.tree = NativeExpressionTree::Parsed(node.clone());
                            let failure = reached.compiler_steps(|name| match table.lookup(name) {
                                Actual::Present(binding) => binding.arity.map_or(Compiler::Unknown, |arity| Compiler::Known { arity }),
                                Actual::Absent => Compiler::Absent,
                                Actual::Unknown => Compiler::Unknown,
                            }).into_iter().find_map(|step| match step {
                                tcl_registry::native_compilation::NativeExpressionCompilerStep::Failure(failure) => Some(failure),
                                _ => None,
                            }).ok_or_else(|| unavailable("native fixed-function compiler rejection"))?;
                            pending.push(Preparation::Failure(failure));
                            for arg in args
                                .iter()
                                .take(
                                    binding
                                        .as_ref()
                                        .and_then(|binding| binding.arity)
                                        .unwrap_or(0),
                                )
                                .rev()
                            {
                                pending.push(Preparation::Node(arg));
                            }
                            continue;
                        }
                        let binding = binding.expect("known fixed function arity");
                        if binding
                            .registry_identity
                            .as_deref()
                            .and_then(tcl_registry::mathfunc::global_command_bare_name)
                            .map(str::as_bytes)
                            != Some(function.as_slice())
                        {
                            return Err(unavailable("native fixed-function implementation"));
                        }
                        prepared.fixed_functions.insert(*start, binding);
                    }
                    for arg in args.iter().rev() {
                        pending.push(Preparation::Node(arg));
                    }
                }
                _ => return Err(unavailable("native expression executable tree")),
            }
        }
        Ok(prepared)
    }

    pub(super) fn control_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        mut recipe: NativeControlCompilation<NativeControlInstruction>,
        depth: u32,
    ) -> Result<ControlOperation, ValueError> {
        let prepared = self.prepare_control_steps(captured, &recipe.preparations, depth)?;
        if let NativeControlOutcome::Rejected(failure) = &recipe.outcome {
            return Err(self.reject_native_compilation(failure.clone()));
        }
        self.validate_control_boolean_probes(captured, &recipe.preparations)?;
        if prepared.declined_script {
            recipe.outcome = NativeControlOutcome::Generic;
        }
        let mut names = HashMap::new();
        let protocol = self
            .interp
            .native_invocation_dialect()
            .native_compiled_variable_protocol()
            .ok_or_else(|| unavailable("native control local issuer"))?;
        let mut empty = None;
        let mut zero = None;
        if let NativeControlOutcome::Inline(instruction) = &recipe.outcome {
            match instruction {
                NativeControlInstruction::Catch {
                    result, options, ..
                } => {
                    for name in result.iter().chain(options.iter()) {
                        let slot = self
                            .lvt
                            .find_native(protocol, name)
                            .ok_or_else(|| unavailable("native catch local preparation"))?;
                        names.insert(name.clone(), slot);
                    }
                    zero = Some(self.literals.intern_bytes(b"0"));
                }
                NativeControlInstruction::Conditional(clauses)
                    if clauses.last().is_some_and(|clause| {
                        clause.test.is_none()
                            || clause.test == Some(NativeControlTest::Constant(true))
                    }) => {}
                _ => empty = Some(self.literals.intern_bytes(b"")),
            }
        }
        Ok(ControlOperation {
            recipe,
            prepared,
            names,
            empty,
            zero,
        })
    }
}

struct CompiledExpressionContext<'a> {
    interp: &'a mut Interp,
    artifact: &'a NativeBodyArtifact,
    prepared: &'a PreparedNativeExpression,
    execution: &'a mut BodyExecution,
    propagated: Option<Code>,
}
impl CompiledExpressionContext<'_> {
    fn error(&mut self, code: Code) -> crate::expr_error::ExprError {
        self.propagated = Some(code);
        match self.interp.native_execution_refusal() {
            Some(error) => crate::expr_error::ExprError::from_execution_refusal(error),
            None => crate::expr_error::ExprError::from_bytes(Vec::new()),
        }
    }
}
impl crate::expr::ExprCtx for CompiledExpressionContext<'_> {
    fn numeric_host(&self) -> Option<Rc<dyn tcl_platform::Host>> {
        Some(self.interp.host())
    }
    fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
        self.interp.native_invocation_dialect()
    }
    fn f5_string_predicate_provider(
        &self,
    ) -> Option<tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider> {
        tcl_registry::native_expression_program::authored_f5_string_predicate_provider(
            self.artifact.stamp.expression_policy.as_ref(),
        )
    }
    fn has_compiled_nodes(&self) -> bool {
        !self.prepared.folded.is_empty() || !self.prepared.logical_left84.is_empty()
    }
    fn compiled_node(
        &mut self,
        node: &tcl_syntax::expr::NativeExprNode,
    ) -> Option<Result<obj::Owned, crate::expr_error::ExprError>> {
        if let Some(indices) = self.prepared.logical_left84.get(node).copied() {
            let ExprNode::Binary { op, left, right } = node else {
                unreachable!("original C84 logical recipe")
            };
            return Some((|| {
                let left = crate::expr::eval_compiled_expression_node(left.as_ref(), self)?;
                let dialect = self.interp.native_invocation_dialect();
                let truth = crate::expr::native_jump_boolean84(left.as_ptr(), dialect)?;
                drop(left);
                let normalized = obj::Owned::retain(
                    self.artifact
                        .literals
                        .original(indices[usize::from(truth)])
                        .expect("C84 original logical normalisation literal"),
                );
                let conjunction = *op == tcl_syntax::expr::BinOp::And;
                let selected = crate::expr::native_jump_boolean84(normalized.as_ptr(), dialect)?;
                if selected != conjunction {
                    return Ok(normalized);
                }
                let right = crate::expr::eval_compiled_expression_node(right.as_ref(), self)?;
                let host = self.interp.host();
                crate::expr::native_logical84(
                    dialect,
                    normalized,
                    right,
                    conjunction,
                    host.numeric_environment(),
                )
            })());
        }
        let folded = *self.prepared.folded.get(node)?;
        Some(match folded {
            Ok(index) => Ok(obj::Owned::retain(
                self.artifact
                    .literals
                    .original(index)
                    .expect("native folded expression original"),
            )),
            Err((message, options)) => {
                let original_message = self
                    .artifact
                    .literals
                    .original(message)
                    .expect("native folded expression diagnostic");
                let original_options = self
                    .artifact
                    .literals
                    .original(options)
                    .expect("native folded expression options");
                let code = match self.interp.process_original_c_return_options(
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                    1,
                    0,
                    original_options,
                ) {
                    Ok(code) => code,
                    Err(error) => self.interp.report_cmd_error(error.into()),
                };
                self.interp.set_result(original_message);
                if code == Code::Error {
                    self.interp.clear_error_logged();
                    self.interp.capture_original_return_instruction_context(
                        tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                        original_message,
                        original_options,
                    );
                }
                Err(self.error(code))
            }
        })
    }
    fn compiled_literal(&mut self, start: u32, end: u32) -> Option<obj::Owned> {
        self.prepared.literals.get(&(start, end)).map(|index| {
            obj::Owned::retain(
                self.artifact
                    .literals
                    .original(*index)
                    .expect("compiled expression original literal"),
            )
        })
    }
    fn read_var(&mut self, _name: &str) -> Result<obj::Owned, crate::expr_error::ExprError> {
        Err(crate::expr_error::ExprError::host_refusal(
            unavailable("compiled expression unpositioned variable")
                .native_access_refusal()
                .expect("typed refusal"),
        ))
    }
    fn eval_command(&mut self, _script: &str) -> Result<obj::Owned, crate::expr_error::ExprError> {
        Err(crate::expr_error::ExprError::host_refusal(
            unavailable("compiled expression unpositioned script")
                .native_access_refusal()
                .expect("typed refusal"),
        ))
    }
    fn compiled_variable(
        &mut self,
        _reference: &[u8],
        start: u32,
    ) -> Result<obj::Owned, crate::expr_error::ExprError> {
        let arena = self
            .prepared
            .arenas
            .get(&start)
            .expect("compiled original variable arena");
        self.interp
            .body_arena(self.artifact, arena, self.execution)
            .map_err(|code| self.error(code))
    }
    fn compiled_string(
        &mut self,
        _inner: &[u8],
        start: u32,
        _end: u32,
    ) -> Result<obj::Owned, crate::expr_error::ExprError> {
        let arena = self
            .prepared
            .arenas
            .get(&start)
            .expect("compiled original quote arena");
        self.interp
            .body_arena(self.artifact, arena, self.execution)
            .map_err(|code| self.error(code))
    }
    fn compiled_command(
        &mut self,
        _script: &[u8],
        start: u32,
        _end: u32,
    ) -> Result<obj::Owned, crate::expr_error::ExprError> {
        let span = self.prepared.commands[&start];
        let code = self
            .interp
            .execute_body_region(self.artifact, span, self.execution);
        if code != Code::Ok {
            return Err(self.error(code));
        }
        Ok(obj::Owned::retain(self.interp.result_obj()))
    }
    fn call_function(
        &mut self,
        name: &str,
        args: &[obj::Owned],
    ) -> Result<obj::Owned, crate::expr_error::ExprError> {
        let pointers = args.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
        let dispatch = tcl_registry::native_expression_program::expression_function_dispatch(
            self.artifact.stamp.expression_policy.as_ref(),
            tcl_registry::InvocationDialect::for_version(self.artifact.stamp.physical),
        )
        .ok_or_else(|| {
            crate::expr_error::ExprError::host_refusal(
                unavailable("native expression function policy")
                    .native_access_refusal()
                    .expect("typed refusal"),
            )
        })?;
        let code = if dispatch == tcl_registry::mathfunc::NativeMathFunctionDispatch::FixedTable {
            let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
                tcl_dialect::DialectProfile::find("tcl8.4").expect("actual C8.4 grammar"),
            );
            match surface.math_function_call_target(name) {
                tcl_registry::expr_surface::MathFunctionCallTarget::FixedBuiltin(spec) => {
                    self.interp.eval_fixed_math_call(spec, &pointers)
                }
                _ => self.interp.set_error(b"unknown math function"),
            }
        } else {
            self.interp.eval_math_call(name.as_bytes(), &pointers)
        };
        if code != Code::Ok {
            return Err(self.error(code));
        }
        Ok(obj::Owned::retain(self.interp.result_obj()))
    }
    fn compiled_call(
        &mut self,
        name: &str,
        args: &[obj::Owned],
        start: u32,
    ) -> Result<obj::Owned, crate::expr_error::ExprError> {
        let dispatch = tcl_registry::native_expression_program::expression_function_dispatch(
            self.artifact.stamp.expression_policy.as_ref(),
            tcl_registry::InvocationDialect::for_version(self.artifact.stamp.physical),
        )
        .ok_or_else(|| {
            crate::expr_error::ExprError::host_refusal(
                unavailable("native expression function policy")
                    .native_access_refusal()
                    .expect("typed refusal"),
            )
        })?;
        if dispatch == tcl_registry::mathfunc::NativeMathFunctionDispatch::FixedTable {
            let binding = &self.prepared.fixed_functions[&start];
            if binding.name.as_bytes() != name.as_bytes() || binding.arity != Some(args.len()) {
                return Err(crate::expr_error::ExprError::host_refusal(
                    unavailable("native fixed-function operand contract")
                        .native_access_refusal()
                        .expect("typed refusal"),
                ));
            }
            return self.call_function(name, args);
        }
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push(
            self.artifact
                .literals
                .original(self.prepared.function_heads[&start])
                .expect("native expression function literal"),
        );
        argv.extend(args.iter().map(obj::Owned::as_ptr));
        let code = self.interp.dispatch(&argv);
        if code != Code::Ok {
            return Err(self.error(code));
        }
        Ok(obj::Owned::retain(self.interp.result_obj()))
    }
}

impl Interp {
    pub(super) fn execute_body_expression(
        &mut self,
        artifact: &NativeBodyArtifact,
        prepared: &PreparedNativeExpression,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        if let Some((message, options)) = prepared.rejected {
            let options = artifact
                .literals
                .original(options)
                .expect("native expression SYNTAX options");
            let code = self
                .process_original_c_return_options(
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                    1,
                    0,
                    options,
                )
                .map_err(|error| self.report_cmd_error(error.into()))?;
            self.set_result(
                artifact
                    .literals
                    .original(message)
                    .expect("native expression SYNTAX result"),
            );
            if code == Code::Error {
                self.clear_error_logged();
                self.capture_original_return_instruction_context(
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                    self.result_obj(),
                    options,
                );
            }
            return Err(code);
        }
        let NativeExpressionTree::Parsed(tree) = &prepared.program.tree else {
            unreachable!()
        };
        let numeric_host = self.host();
        let mut context = CompiledExpressionContext {
            interp: self,
            artifact,
            prepared,
            execution,
            propagated: None,
        };
        let result =
            crate::expr::eval_compiled_expression_node(tree, &mut context).and_then(|value| {
                if prepared.program.context.native_syntax
                    == tcl_syntax::expr::parser::NativeExprSyntax::Tcl(
                        tcl_dialect::TclVersion::V8_4,
                    )
                    && matches!(
                        tree,
                        ExprNode::Literal { .. }
                            | ExprNode::String { .. }
                            | ExprNode::Var { .. }
                            | ExprNode::Command { .. }
                            | ExprNode::Call { .. }
                    )
                {
                    crate::expr::normalize_compiled_primary84(
                        value,
                        context.interp.native_invocation_dialect(),
                        numeric_host.numeric_environment(),
                    )
                } else {
                    Ok(value)
                }
            });
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                let code = context.propagated;
                Err(code.unwrap_or_else(|| self.report_expr_error(error)))
            }
        }
    }
    pub(super) fn execute_control_body(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        prepared: &super::native_control_preparation::PreparedControlOperands,
        body: &NativeControlBody,
        execution: &mut BodyExecution,
    ) -> Code {
        if let Some(span) = body.script {
            return self.execute_body_region(artifact, span, execution);
        }
        let original = match self.body_control_operand(
            artifact,
            command,
            prepared,
            &body.operand,
            execution,
        ) {
            Ok(value) => value,
            Err(code) => return code,
        };
        self.eval_generic_control_body(original.as_ptr())
    }
    pub(super) fn body_control_operand(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        prepared: &super::native_control_preparation::PreparedControlOperands,
        operand: &tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        match operand {
            tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(
                index,
            ) => self.body_word(artifact, &command.words[*index], execution),
            _ => Ok(obj::Owned::retain(
                artifact
                    .literals
                    .original(prepared.literals[operand])
                    .expect("original expanded control operand"),
            )),
        }
    }
    pub(super) fn execute_body_expression_instruction(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        expression: &ExpressionOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        if let Some(prepared) = &expression.static_program {
            let value = self.execute_body_expression(artifact, prepared, execution)?;
            self.set_result(value.as_ptr());
            return Ok(Code::Ok);
        }
        let mut parts = Vec::new();
        for (index, operand) in expression.recipe.operands.iter().enumerate() {
            parts.push(self.body_control_operand(
                artifact,
                command,
                &expression.prepared,
                operand,
                execution,
            )?);
            if index + 1 < expression.recipe.operands.len() {
                parts.push(obj::Owned::retain(
                    artifact
                        .literals
                        .original(expression.separator.expect("emitted expression separator"))
                        .expect("original expression separator"),
                ));
            }
        }
        let source = self.concatenate_body_values(parts)?;
        Ok(crate::builtins::eval_expr_original(self, source.as_ptr()))
    }
    fn control_boolean(
        &mut self,
        artifact: &NativeBodyArtifact,
        control: &ControlOperation,
        test: &NativeControlTest,
        execution: &mut BodyExecution,
    ) -> Result<bool, Code> {
        match test {
            NativeControlTest::Constant(value) => Ok(*value),
            NativeControlTest::Expression(program) => {
                let value = self.execute_body_expression(
                    artifact,
                    &control.prepared.expressions[&program.operand],
                    execution,
                )?;
                crate::expr::to_bool_in(value.as_ptr(), self.native_invocation_dialect())
                    .map_err(|error| self.report_expr_error(error))
            }
        }
    }
    pub(super) fn execute_body_control(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        control: &ControlOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let NativeControlOutcome::Inline(instruction) = &control.recipe.outcome else {
            return Err(self
                .report_cmd_error(unavailable("native control declined execution recipe").into()));
        };
        match instruction {
            NativeControlInstruction::Conditional(clauses) => {
                for clause in clauses {
                    if clause
                        .test
                        .as_ref()
                        .map(|test| self.control_boolean(artifact, control, test, execution))
                        .transpose()?
                        .unwrap_or(true)
                    {
                        return Ok(self.execute_control_body(
                            artifact,
                            command,
                            &control.prepared,
                            &clause.body,
                            execution,
                        ));
                    }
                }
            }
            NativeControlInstruction::While { test, body } => {
                while self.control_boolean(artifact, control, test, execution)? {
                    if let Some(code) = self.limit_check_tick() {
                        return Ok(code);
                    }
                    let code = self.execute_control_body(
                        artifact,
                        command,
                        &control.prepared,
                        body,
                        execution,
                    );
                    if execution.done || self.host_refusal_pending() {
                        return Ok(code);
                    }
                    match code {
                        Code::Ok | Code::Continue => {}
                        Code::Break => break,
                        other => return Ok(other),
                    }
                }
            }
            NativeControlInstruction::For {
                start,
                test,
                next,
                body,
            } => {
                let code = self.execute_control_body(
                    artifact,
                    command,
                    &control.prepared,
                    start,
                    execution,
                );
                if code != Code::Ok || execution.done {
                    return Ok(code);
                }
                loop {
                    if let Some(code) = self.limit_check_tick() {
                        return Ok(code);
                    }
                    let value = self.execute_body_expression(
                        artifact,
                        &control.prepared.expressions[&test.operand],
                        execution,
                    )?;
                    if !crate::expr::to_bool_in(value.as_ptr(), self.native_invocation_dialect())
                        .map_err(|error| self.report_expr_error(error))?
                    {
                        break;
                    }
                    drop(value);
                    let code = self.execute_control_body(
                        artifact,
                        command,
                        &control.prepared,
                        body,
                        execution,
                    );
                    if execution.done || self.host_refusal_pending() {
                        return Ok(code);
                    }
                    match code {
                        Code::Ok | Code::Continue => {}
                        Code::Break => break,
                        other => return Ok(other),
                    }
                    let code = self.execute_control_body(
                        artifact,
                        command,
                        &control.prepared,
                        next,
                        execution,
                    );
                    if execution.done || self.host_refusal_pending() {
                        return Ok(code);
                    }
                    match code {
                        Code::Ok => {}
                        Code::Break => break,
                        other => return Ok(other),
                    }
                }
            }
            NativeControlInstruction::Catch {
                protocol,
                body,
                result,
                options,
            } => {
                // Construction of a substituted script precedes the protected
                // range. Its errors propagate instead of becoming catch results.
                let dynamic = if body.script.is_none() {
                    Some(self.body_control_operand(
                        artifact,
                        command,
                        &control.prepared,
                        &body.operand,
                        execution,
                    )?)
                } else {
                    None
                };
                let code = if let Some(original) = dynamic {
                    self.eval_generic_control_body(original.as_ptr())
                } else {
                    self.execute_body_region(
                        artifact,
                        body.script.expect("static catch script"),
                        execution,
                    )
                };
                if self.host_refusal_pending() || execution.done {
                    return Ok(code);
                }
                obj::check_native_liveness(self.result_obj())
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                let empty = obj::Owned::fresh(obj::new_string_bytes(b""));
                let original_result = self.result.replace(empty.into_raw());
                // SAFETY: PUSH_RESULT transfers the interpreter's existing
                // reference to the protected result on the evaluation stack.
                let original_result = unsafe { obj::Owned::from_raw(original_result) };
                let original_options = if options.is_some() {
                    let options = crate::cmd_error::completion_options(self, code)
                        .map_err(|error| self.refuse_native_execution(error))?;
                    Some(obj::Owned::fresh(options))
                } else {
                    None
                };
                if protocol.resets_result_before_stores() {
                    // END_CATCH reaches Tcl_ResetResult after the original
                    // result/options have been captured, before either store.
                    // Its real global writes can invoke guest trace callbacks.
                    self.publish_and_reset_error();
                    if self.host_refusal_pending() {
                        return Err(Code::Error);
                    }
                    self.set_return_state(1, Code::Ok);
                }
                self.clear_return_options();
                let result_store = result.as_ref().map(|name| (name, &original_result));
                let options_store = options.as_ref().zip(original_options.as_ref());
                let stores = if protocol.result_before_options() {
                    [result_store, options_store]
                } else {
                    [options_store, result_store]
                };
                for (name, value) in stores.into_iter().flatten() {
                    let target = EvaluatedTarget {
                        root: name.clone(),
                        element: None,
                        original_name: None,
                        original_index: None,
                        combined: false,
                    };
                    let store_code = self.body_store(&target, Some(control.names[name]), value);
                    if store_code != Code::Ok {
                        return Ok(store_code);
                    }
                }
                if code == Code::Ok {
                    self.set_result(
                        artifact
                            .literals
                            .original(control.zero.expect("native catch zero"))
                            .expect("native catch zero literal"),
                    );
                } else {
                    self.set_result(
                        obj::Owned::fresh(obj::new_wide_int_obj(code.as_int())).as_ptr(),
                    );
                }
                return Ok(Code::Ok);
            }
        }
        self.set_result(
            artifact
                .literals
                .original(control.empty.expect("native empty control result"))
                .expect("registered control result"),
        );
        Ok(Code::Ok)
    }
}

impl PreparedNativeExpression {
    fn compaction_hazards(
        &self,
    ) -> Vec<tcl_registry::native_compiler_pass::NativeCompilerPassHazard> {
        if self.function_heads.is_empty() {
            Vec::new()
        } else {
            vec![tcl_registry::native_compiler_pass::NativeCompilerPassHazard::Invocation]
        }
    }
}
impl ExpressionOperation {
    pub(super) fn compaction_hazards(
        &self,
    ) -> Vec<tcl_registry::native_compiler_pass::NativeCompilerPassHazard> {
        self.static_program.as_ref().map_or_else(||vec![tcl_registry::native_compiler_pass::NativeCompilerPassHazard::ExpressionEvaluation],PreparedNativeExpression::compaction_hazards)
    }
}
impl ControlOperation {
    pub(super) fn compaction_hazards(
        &self,
    ) -> Vec<tcl_registry::native_compiler_pass::NativeCompilerPassHazard> {
        use tcl_registry::native_compiler_pass::NativeCompilerPassHazard as Hazard;
        let mut hazards = self
            .prepared
            .expressions
            .values()
            .flat_map(PreparedNativeExpression::compaction_hazards)
            .collect::<Vec<_>>();
        let dynamic = match &self.recipe.outcome {
            NativeControlOutcome::Generic => {
                hazards.push(Hazard::Invocation);
                false
            }
            NativeControlOutcome::Rejected(_) => false,
            NativeControlOutcome::Inline(instruction) => match instruction {
                NativeControlInstruction::Conditional(clauses) => {
                    clauses.iter().any(|clause| clause.body.script.is_none())
                }
                NativeControlInstruction::While {
                    test: NativeControlTest::Constant(false),
                    ..
                } => false,
                NativeControlInstruction::While { body, .. }
                | NativeControlInstruction::Catch { body, .. } => body.script.is_none(),
                NativeControlInstruction::For {
                    start, body, next, ..
                } => [start, body, next]
                    .into_iter()
                    .any(|body| body.script.is_none()),
            },
        };
        if dynamic {
            hazards.push(Hazard::ScriptEvaluation);
        }
        hazards
    }
}
