// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete original command inventory for one exact procedure allocation.
//!
//! Source offsets identify retained carriers only. Implementation identity is
//! obtained from positioned source lookup, never from a qualified display name.

use super::super::PassContext;
use crate::command_binding::{CommandAllocation, SourceCommandDefinitionKind};
use crate::ir::{CommandTokens, Procedure, Script, Statement};
use std::collections::{BTreeMap, BTreeSet};
use tcl_lexer::{ExecutablePart, NativeWord, SourceImage, Span};
use tcl_registry::script_body_flow::ScriptBodyFlow;

#[derive(Clone)]
enum SelfCallArguments {
    Written,
    Unproved,
}

#[derive(Clone)]
pub(super) struct SelfCall {
    arguments: SelfCallArguments,
    pub tailcall_command_preserved: bool,
    pub assignments_preserve_effects: bool,
    pub retains_formal_roots: bool,
}

impl SelfCall {
    pub fn written_arguments(&self) -> bool {
        matches!(self.arguments, SelfCallArguments::Written)
    }
}

struct CheckedExpressionCommands<'a> {
    source: &'a str,
    context: &'a tcl_syntax::expr::parser::ExprParseContext,
    commands: Vec<Span>,
    returned: Option<u32>,
}

#[derive(Clone, Copy)]
struct OriginalExpressionOperand<'a> {
    word: &'a NativeWord,
    written: usize,
    dialect: tcl_registry::InvocationDialect,
}

pub(super) struct Inventory {
    allocation: CommandAllocation,
    pub formal_topology: Option<crate::command_binding::formal_topology::OriginalFormalTopology>,
    pub calls: BTreeMap<u32, SelfCall>,
    pub readonly_calls: usize,
    pub complete: bool,
    pub loop_commands_preserved: bool,
    pub frame_effects_closed: bool,
    loop_commands: Vec<&'static str>,
    visited: BTreeSet<u32>,
    pub accumulator_returns: BTreeSet<u32>,
}

impl Inventory {
    pub fn capture(
        ctx: &PassContext<'_>,
        module: &crate::ir::Module,
        procedure: &Procedure,
    ) -> Option<Self> {
        let source = procedure.body.executed_source.as_deref()?;
        let mut originals = module
            .procedure_implementation_bodies
            .iter()
            .filter(|body| {
                body.allocation.site.offset == procedure.span.start()
                    && body.source == *source
                    && body.source.origin.source_image() == &module.source
                    && body.source.base() == procedure.body_offset
            });
        let first = originals.next()?;
        if originals.any(|other| other.allocation != first.allocation) {
            return None;
        }
        let mut result = Self {
            allocation: first.allocation.clone(),
            formal_topology: first.original_parameters.clone(),
            calls: BTreeMap::new(),
            readonly_calls: 0,
            complete: true,
            loop_commands_preserved: true,
            frame_effects_closed: true,
            loop_commands: vec!["while", "return", "continue"],
            visited: BTreeSet::new(),
            accumulator_returns: BTreeSet::new(),
        };
        result.script(ctx, &procedure.body, 0);
        Some(result)
    }

    pub fn call_at(&self, span: Span) -> Option<&SelfCall> {
        self.calls.get(&span.start())
    }

    fn script(&mut self, ctx: &PassContext<'_>, script: &Script, depth: u32) {
        if super::super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) || !script.is_authored_source() {
            self.complete = false;
            return;
        }
        for statement in &script.statements {
            if let Some(tokens) = script.retained_source_tokens_for_statement(statement) {
                self.command(
                    ctx,
                    tokens,
                    depth,
                    matches!(statement, Statement::Return { .. })
                        .then_some(statement.span().start()),
                );
            } else {
                self.complete = false;
            }
            for child in statement.child_scripts() {
                self.script(ctx, child, depth + 1);
            }
        }
    }

    fn command(
        &mut self,
        ctx: &PassContext<'_>,
        tokens: &CommandTokens,
        depth: u32,
        returned: Option<u32>,
    ) {
        if super::super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            self.complete = false;
            return;
        }
        let Some(head) = tokens.words().first() else {
            self.complete = false;
            return;
        };
        let offset = head.source().span.start();
        if !self.visited.insert(offset) {
            return;
        }
        let Some(binding) = tokens.source_binding.as_ref() else {
            self.complete = false;
            return;
        };
        let Some(registry) = ctx.registry else {
            self.complete = false;
            return;
        };
        self.loop_commands_preserved &=
            binding.original_proposed_registry_commands(tokens, registry, &self.loop_commands);
        let self_call = self.retain_self_call(binding, registry, tokens, offset);
        let Some(words) = Self::original_command_words(ctx, binding, tokens) else {
            self.complete = false;
            return;
        };
        self.retain_executable_parts(ctx, &words, tokens, depth, returned);
        // Its exact body is the inventory being constructed. Recursion is not
        // an unstamped Registry handler, nor an additional editable body.
        if self_call {
            return;
        }
        let Some(advice) = binding.declaration_operand_layout_advice(tokens) else {
            self.complete = false;
            return;
        };
        let Some(selected) =
            crate::registry_invocation::declaration_invocation_flow(registry, tokens, &advice)
        else {
            self.complete = false;
            return;
        };
        if !self_call && !selected.effects.lookup_stable {
            self.loop_commands_preserved = false;
        }
        self.retain_frame_effects(&selected);
        let expressions = match &selected.flow {
            ScriptBodyFlow::Expressions(indices) => indices.clone(),
            ScriptBodyFlow::ConcatenatedExpression { argument_offset } => {
                // Multi-argument concat has its own produced-script boundary;
                // a missing producer cannot be substituted with joined display.
                if selected.effective.words.len() != argument_offset + 2 {
                    self.complete = false;
                    return;
                }
                vec![*argument_offset]
            }
            ScriptBodyFlow::Conditional(pairs) => pairs
                .iter()
                .filter_map(|(condition, _)| *condition)
                .collect(),
            ScriptBodyFlow::Loop { conditions, .. } => conditions.clone(),
            ScriptBodyFlow::Unknown(_)
            | ScriptBodyFlow::ConcatenatedScript { .. }
            | ScriptBodyFlow::Lambda(_) => {
                self.complete = false;
                return;
            }
            _ => Vec::new(),
        };
        for argument in expressions {
            let Some(crate::registry_invocation::InvocationWordOrigin::Written(written)) =
                selected.effective.origins.get(argument + 1)
            else {
                self.complete = false;
                continue;
            };
            let Some(word) = words.get(*written) else {
                self.complete = false;
                continue;
            };
            self.expression(
                ctx,
                OriginalExpressionOperand {
                    word,
                    written: *written,
                    dialect: advice.dialect(),
                },
                tokens,
                depth + 1,
                returned,
            );
        }
    }

    fn original_command_words(
        ctx: &PassContext<'_>,
        binding: &crate::command_binding::SourceInvocationBinding,
        tokens: &CommandTokens,
    ) -> Option<Vec<NativeWord>> {
        let site = binding.invocation_site()?;
        let config = ctx.lexer_config();
        crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            config,
        )
    }

    fn retain_self_call(
        &mut self,
        binding: &crate::command_binding::SourceInvocationBinding,
        registry: &tcl_registry::CommandRegistry,
        tokens: &CommandTokens,
        offset: u32,
    ) -> bool {
        let mut self_call = false;
        if let Some(definition) = binding.original_declared_definition_reference(tokens, registry) {
            if definition.kind() == SourceCommandDefinitionKind::Procedure
                && definition.allocation() == &self.allocation
            {
                self_call = true;
                self.calls.insert(
                    offset,
                    SelfCall {
                        arguments: if binding.original_declared_call_keeps_written_arguments(tokens)
                        {
                            SelfCallArguments::Written
                        } else {
                            SelfCallArguments::Unproved
                        },
                        tailcall_command_preserved: binding.original_proposed_registry_commands(
                            tokens,
                            registry,
                            &["tailcall"],
                        ),
                        assignments_preserve_effects: self
                            .formal_topology
                            .as_ref()
                            .and_then(|topology| {
                                topology.fixed_scalar_binding_names(
                                    tokens.words().len().saturating_sub(1),
                                )
                            })
                            .is_some_and(|names| {
                                binding.original_proposed_formal_writes(tokens, registry, &names)
                            }),
                        retains_formal_roots: binding
                            .original_call_retains_formal_roots(tokens, registry),
                    },
                );
                if self
                    .calls
                    .get(&offset)
                    .is_some_and(|call| !call.retains_formal_roots)
                {
                    let assignment_commands: &[&str] = if self
                        .formal_topology
                        .as_ref()
                        .is_some_and(|topology| topology.parameters().len() <= 1)
                    {
                        &["set"]
                    } else {
                        &["lassign", "list"]
                    };
                    self.loop_commands_preserved &= binding.original_proposed_registry_commands(
                        tokens,
                        registry,
                        assignment_commands,
                    );
                }
            }
        } else if !binding
            .declaration_operand_layout_advice(tokens)
            .is_some_and(|advice| {
                advice.closed_lookup()
                    && !advice.has_opaque_handler_alternatives()
                    && advice.targets().iter().all(|target| target.registry_backed)
            })
        {
            self.complete = false;
        }
        self_call
    }

    fn retain_executable_parts(
        &mut self,
        ctx: &PassContext<'_>,
        words: &[NativeWord],
        tokens: &CommandTokens,
        depth: u32,
        returned: Option<u32>,
    ) {
        for word in words {
            let arena = word.executable_parts();
            for part in arena.all_parts() {
                match &part.part {
                    ExecutablePart::Command { body } => {
                        self.original_script(
                            ctx,
                            arena.image(),
                            *body,
                            tokens,
                            depth + 1,
                            returned,
                        );
                    }
                    ExecutablePart::Expression { .. } | ExecutablePart::ParseError(_) => {
                        self.complete = false;
                    }
                    ExecutablePart::Text(_) | ExecutablePart::Variable { .. } => {}
                }
            }
        }
    }

    fn retain_frame_effects(
        &mut self,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) {
        self.frame_effects_closed &= !selected.effects.frame_reachable()
            && !selected.effects.unknown_reads
            && !selected.effects.unknown_writes
            && selected.effects.lookup_stable
            && selected.writes.is_empty()
            && selected.possible_output_writes.is_empty()
            && selected.loop_bindings.is_empty()
            && selected.literal_stores.is_empty()
            && selected.increments.is_empty()
            && selected.accumulator_writes.is_empty()
            && selected.removes.is_empty()
            && selected.aliases.is_empty()
            && selected.caller_alias_operands.is_empty();
    }

    fn original_script(
        &mut self,
        ctx: &PassContext<'_>,
        image: &SourceImage,
        body: Span,
        parent: &CommandTokens,
        depth: u32,
        returned: Option<u32>,
    ) {
        let Some(bytes) = image.bytes().get(body.as_range()) else {
            self.complete = false;
            return;
        };
        let script = SourceImage::from_bytes(bytes, image.channel());
        let Some(commands) = crate::segmenter::segment_commands_image_with_offset_and_config(
            &script,
            body.start(),
            ctx.lexer_config(),
        ) else {
            self.complete = false;
            return;
        };
        for command in commands {
            if command.is_partial {
                self.complete = false;
                continue;
            }
            let mut tokens =
                CommandTokens::from_segmented(&image.source_map(), ctx.lexer_config(), &command);
            tokens.inherit_nested_bindings(parent);
            self.command(ctx, &tokens, depth, returned);
        }
    }

    fn expression(
        &mut self,
        ctx: &PassContext<'_>,
        operand: OriginalExpressionOperand<'_>,
        parent: &CommandTokens,
        depth: u32,
        returned: Option<u32>,
    ) {
        let OriginalExpressionOperand {
            word,
            written,
            dialect,
        } = operand;
        let Some(protocol) = dialect.native_string_protocol() else {
            self.complete = false;
            return;
        };
        let Ok(captured) = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            std::slice::from_ref(word),
            protocol,
        ) else {
            self.complete = false;
            return;
        };
        let Some(bytes) = captured.literal(0) else {
            self.complete = false;
            return;
        };
        let Ok(expression) = std::str::from_utf8(bytes) else {
            self.complete = false;
            return;
        };
        let Some(context) = expression_parse_context(ctx, dialect) else {
            self.complete = false;
            return;
        };
        let Some(commands) =
            tcl_syntax::expr::command_substitutions_in_checked_expression(expression, &context)
        else {
            self.complete = false;
            return;
        };
        let single_command = commands.len() == 1;
        let Ok(operand_span) = word.content_span() else {
            self.complete = false;
            return;
        };
        if word.image().bytes().get(operand_span.as_range()) != Some(bytes) {
            self.retain_readonly_expression_commands(
                ctx,
                parent,
                written,
                CheckedExpressionCommands {
                    source: expression,
                    context: &context,
                    commands,
                    returned,
                },
            );
            return;
        }
        let before = self.calls.len();
        for command in commands {
            let Some(start) = operand_span
                .start()
                .checked_add(command.start())
                .and_then(|value| value.checked_add(1))
            else {
                self.complete = false;
                return;
            };
            let Some(end) = operand_span
                .start()
                .checked_add(command.end())
                .and_then(|value| value.checked_sub(1))
            else {
                self.complete = false;
                return;
            };
            self.original_script(
                ctx,
                word.image(),
                Span::new(start, end),
                parent,
                depth,
                returned,
            );
        }
        if let Some(returned) = returned
            && single_command
            && self.calls.len() == before + 1
            && associative_expression(expression, &context)
        {
            self.accumulator_returns.insert(returned);
        }
    }
    fn retain_readonly_expression_commands(
        &mut self,
        ctx: &PassContext<'_>,
        parent: &CommandTokens,
        written: usize,
        checked: CheckedExpressionCommands<'_>,
    ) {
        let CheckedExpressionCommands {
            source,
            context,
            commands,
            returned,
        } = checked;
        let single_command = commands.len() == 1;
        let before = self.readonly_calls;
        for command in commands {
            let Some(start) = command.start().checked_add(1) else {
                self.complete = false;
                return;
            };
            let Some(end) = command.end().checked_sub(1) else {
                self.complete = false;
                return;
            };
            let Some(binding) = parent.source_binding.as_ref() else {
                self.complete = false;
                return;
            };
            let Some(registry) = ctx.registry else {
                self.complete = false;
                return;
            };
            let Some(definition) = binding.original_expression_command_definition(
                parent,
                written,
                Span::new(start, end),
                0,
                registry,
            ) else {
                self.complete = false;
                continue;
            };
            if definition.kind() == SourceCommandDefinitionKind::Procedure
                && definition.allocation() == &self.allocation
            {
                self.readonly_calls += 1;
            }
        }
        if let Some(returned) = returned
            && single_command
            && self.readonly_calls == before + 1
            && associative_expression(source, context)
        {
            self.accumulator_returns.insert(returned);
        }
    }
}

fn expression_parse_context(
    ctx: &PassContext<'_>,
    dialect: tcl_registry::InvocationDialect,
) -> Option<tcl_syntax::expr::parser::ExprParseContext> {
    let profile = ctx.dialect.or_else(|| {
        ctx.registry
            .and_then(tcl_registry::CommandRegistry::profile)
    })?;
    let mut context = tcl_syntax::expr::parser::ExprParseContext::for_profile(profile);
    context.lexer_grammar = ctx.lexer_config().grammar_over(dialect.lexer_grammar);
    Some(context)
}

fn associative_expression(
    source: &str,
    context: &tcl_syntax::expr::parser::ExprParseContext,
) -> bool {
    use tcl_syntax::expr::{BinOp, ExprNode};
    let tcl_syntax::expr::parser::CheckedExprParse::Parsed(expression) =
        tcl_syntax::expr::parser::parse_expr_checked_with_context(source, context)
    else {
        return false;
    };
    matches!(
        expression,
        ExprNode::Binary {
            op: BinOp::Add | BinOp::Mul,
            ..
        }
    )
}
