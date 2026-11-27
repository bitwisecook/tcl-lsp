// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Projecting `if`, condition loops, cursor loops and `switch` into executable edges, and completing the region where they join.

use super::super::footprints::expr_footprint;
use super::super::region_descriptors::structured_region_projection;
use super::super::{
    CellReference, ExecutableBlockId, ExecutableExpr, ExecutableInstruction, ExecutableTerminator,
    ExecutableValueId, IteratorGroup, OriginalExpressionCommandEffect,
    OriginalExpressionEffectOwner, SourceCompatibilityDecline, StructuredRegion, SwitchPattern,
};
use super::{
    ControlContext, FunctionBuilder, LOOP_BODY_SLOT, LOOP_INIT_SLOT, LOOP_NEXT_SLOT,
    SWITCH_DEFAULT_SLOT, StatementTail, child_path,
};
use crate::expr_ast::ExprNode;
use crate::ir::Script;
use crate::ir::Statement;
use crate::ir::SwitchMode;
use crate::registry_invocation::InvocationMetadataContext;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{CommandRegistry, SemanticOperationId};
use tcl_source::words::NodeId;
use tcl_source::words::SourceSite;

impl FunctionBuilder {
    pub(super) fn retain_expression_command_effect(
        &mut self,
        value: ExecutableValueId,
        expression: &ExecutableExpr,
        original_tokens: Option<&crate::ir::CommandTokens>,
        operand_argument: Option<usize>,
    ) {
        let effect = match expression {
            ExecutableExpr::Condition { expr, base } => original_tokens.and_then(|tokens| {
                let (config, _) = crate::registry_invocation::original_lowered_expression_syntax(
                    expr, *base, tokens,
                )?;
                // A failed variable-reference parse cannot prove the absence of
                // command substitutions. Keep ordinary configured variables,
                // including their parsed array-index expressions, on the path.
                let variables_available = expr.evaluation_nodes().into_iter().all(|node| {
                    !matches!(node, ExprNode::Var { .. })
                        || matches!(node.variable_reference(config), Ok(Some(_)))
                });
                variables_available.then(|| expr_footprint(expr, *base, Some(tokens)).runs_commands)
            }),
            ExecutableExpr::Operand { text, braced } => original_tokens.and_then(|tokens| {
                let argument = operand_argument?;
                crate::registry_invocation::original_argument_command_effect(
                    tokens, argument, text, *braced,
                )
            }),
            // This is the registry-parsed handler prefix value, not source
            // text requiring Tcl substitution.
            ExecutableExpr::TrapPrefix { .. } => Some(false),
        };
        let owner = if matches!(expression, ExecutableExpr::TrapPrefix { .. }) {
            Some(OriginalExpressionEffectOwner::TrapPrefix)
        } else {
            original_tokens.map(|tokens| {
                OriginalExpressionEffectOwner::Source(std::sync::Arc::new(tokens.clone()))
            })
        };
        self.original_expression_command_effects.insert(
            value,
            OriginalExpressionCommandEffect {
                expression: expression.clone(),
                owner,
                runs_commands: effect,
            },
        );
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(super) fn emit_structured_region(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        statement: &Statement,
        original_tokens: Option<&crate::ir::CommandTokens>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
        tail: bool,
    ) -> Result<StatementTail, SourceCompatibilityDecline> {
        match statement {
            Statement::If {
                clauses, else_body, ..
            } => self.emit_if(
                registry,
                context,
                statement,
                original_tokens,
                clauses,
                else_body.as_ref(),
                node,
                block,
                control,
                tail,
            ),
            Statement::While {
                condition,
                condition_span,
                condition_base,
                body,
                ..
            } => self.emit_condition_loop(
                registry,
                context,
                statement,
                original_tokens,
                &LoopParts {
                    init: None,
                    condition,
                    condition_span: *condition_span,
                    condition_base: *condition_base,
                    next: None,
                    body,
                },
                node,
                block,
                control,
                tail,
            ),
            Statement::For {
                init,
                condition,
                condition_span,
                condition_base,
                next,
                body,
                ..
            } => self.emit_condition_loop(
                registry,
                context,
                statement,
                original_tokens,
                &LoopParts {
                    init: Some(init),
                    condition,
                    condition_span: *condition_span,
                    condition_base: *condition_base,
                    next: Some(next),
                    body,
                },
                node,
                block,
                control,
                tail,
            ),
            Statement::Foreach {
                iterators, body, ..
            } => self.emit_cursor_loop(
                registry,
                context,
                statement,
                original_tokens,
                iterators,
                body,
                node,
                block,
                control,
                tail,
            ),
            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => self.emit_catch(
                registry,
                context,
                statement,
                body,
                result_var.as_deref(),
                options_var.as_deref(),
                node,
                block,
                control,
                tail,
            ),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => self.emit_try(
                registry,
                context,
                statement,
                body,
                handlers,
                finally_body.as_ref(),
                node,
                block,
                control,
                tail,
            ),
            Statement::Switch {
                subject,
                subject_braced,
                subject_span,
                arms,
                default_body,
                mode,
                nocase,
                patterns_braced,
                ..
            } => self.emit_switch(
                registry,
                context,
                statement,
                original_tokens,
                &SwitchParts {
                    subject,
                    subject_braced: *subject_braced,
                    subject_span: *subject_span,
                    arms,
                    default_body: default_body.as_ref(),
                    mode: *mode,
                    nocase: *nocase,
                    patterns_braced: *patterns_braced,
                },
                node,
                block,
                control,
                tail,
            ),
            _ => unreachable!("structured_region_projection selected a non-structured statement"),
        }
    }

    /// Produce the region's completion where its interior edges join, then
    /// dispatch it in the enclosing control context.
    pub(super) fn complete_region(
        &mut self,
        statement: &Statement,
        node: &NodeId,
        join: ExecutableBlockId,
        control: ControlContext,
        tail: bool,
    ) -> StatementTail {
        let (descriptor, kind) = structured_region_projection(statement)
            .expect("only a projected statement reaches region completion");
        let completion = self.allocator.completion();
        self.push(
            join,
            ExecutableInstruction::CompleteStructuredRegion(StructuredRegion {
                completion,
                descriptor,
                kind,
                statement: std::sync::Arc::new(statement.clone()),
                node: node.clone(),
                source: SourceSite::source(statement.span()),
            }),
        );
        self.finish(join, completion, control, tail)
    }

    fn emit_condition(
        &mut self,
        condition: &ExprNode,
        condition_span: tcl_lexer::Span,
        condition_base: Option<u32>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> (ExecutableBlockId, ExecutableValueId) {
        let value = self.allocator.value();
        let completion = self.allocator.completion();
        self.push(
            block,
            ExecutableInstruction::EvaluateExpr {
                value,
                completion,
                expr: ExecutableExpr::Condition {
                    expr: Box::new(condition.clone()),
                    base: condition_base,
                },
                node: node.clone(),
                source: SourceSite::source(condition_span),
            },
        );
        (self.dispatch(block, completion, control), value)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_if(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        statement: &Statement,
        original_tokens: Option<&crate::ir::CommandTokens>,
        clauses: &[crate::ir::IfClause],
        else_body: Option<&Script>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
        tail: bool,
    ) -> Result<StatementTail, SourceCompatibilityDecline> {
        let join = self.new_block();
        let mut current = block;
        for (index, clause) in clauses.iter().enumerate() {
            let (decided, condition) = self.emit_condition(
                &clause.condition,
                clause.condition_span,
                clause.condition_base,
                node,
                current,
                control,
            );
            let expression = ExecutableExpr::Condition {
                expr: Box::new(clause.condition.clone()),
                base: clause.condition_base,
            };
            self.retain_expression_command_effect(condition, &expression, original_tokens, None);
            let body_entry = self.new_block();
            let next_test = self.new_block();
            self.terminate(
                decided,
                ExecutableTerminator::Branch {
                    condition,
                    then_target: body_entry,
                    else_target: next_test,
                },
            );
            let path = child_path(node, u32::try_from(index).unwrap_or(u32::MAX));
            let flow =
                self.emit_nested(registry, context, &clause.body, &path, body_entry, control)?;
            self.terminate(flow, ExecutableTerminator::Goto(join));
            current = next_test;
        }
        if let Some(else_body) = else_body {
            let path = child_path(node, u32::try_from(clauses.len()).unwrap_or(u32::MAX));
            let flow = self.emit_nested(registry, context, else_body, &path, current, control)?;
            self.terminate(flow, ExecutableTerminator::Goto(join));
        } else {
            self.terminate(current, ExecutableTerminator::Goto(join));
        }
        Ok(self.complete_region(statement, node, join, control, tail))
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_lines)]
    fn emit_condition_loop(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        statement: &Statement,
        original_tokens: Option<&crate::ir::CommandTokens>,
        parts: &LoopParts<'_>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
        tail: bool,
    ) -> Result<StatementTail, SourceCompatibilityDecline> {
        let mut current = block;
        if let Some(init) = parts.init {
            let path = child_path(node, LOOP_INIT_SLOT);
            let flow = self.emit_nested(registry, context, init, &path, current, control)?;
            current = flow;
        }
        let header = self.new_block();
        self.terminate(current, ExecutableTerminator::Goto(header));
        let exit = self.new_block();
        let (decided, condition) = self.emit_condition(
            parts.condition,
            parts.condition_span,
            parts.condition_base,
            node,
            header,
            control,
        );
        let expression = ExecutableExpr::Condition {
            expr: Box::new(parts.condition.clone()),
            base: parts.condition_base,
        };
        self.retain_expression_command_effect(condition, &expression, original_tokens, None);
        let body_entry = self.new_block();
        self.terminate(
            decided,
            ExecutableTerminator::Branch {
                condition,
                then_target: body_entry,
                else_target: exit,
            },
        );
        // `continue` re-runs the `next` script of a `for` before the header;
        // a `while` continues straight at the header.
        let continue_target = if parts.next.is_some() {
            self.new_block()
        } else {
            header
        };
        let body_control = control.loop_body(exit, continue_target);
        let path = child_path(node, LOOP_BODY_SLOT);
        let flow = self.emit_nested(
            registry,
            context,
            parts.body,
            &path,
            body_entry,
            body_control,
        )?;
        if let Some(next) = parts.next {
            self.terminate(flow, ExecutableTerminator::Goto(continue_target));
            let path = child_path(node, LOOP_NEXT_SLOT);
            let next_flow =
                self.emit_nested(registry, context, next, &path, continue_target, control)?;
            self.terminate(next_flow, ExecutableTerminator::Goto(header));
        } else {
            self.terminate(flow, ExecutableTerminator::Goto(header));
        }
        Ok(self.complete_region(statement, node, exit, control, tail))
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_cursor_loop(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        statement: &Statement,
        original_tokens: Option<&crate::ir::CommandTokens>,
        iterators: &[crate::ir::ForeachIterator],
        body: &Script,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
        tail: bool,
    ) -> Result<StatementTail, SourceCompatibilityDecline> {
        let sources = original_list_iterator_sources(registry, statement, iterators).ok_or(
            SourceCompatibilityDecline::UnsupportedStatement {
                statement_index: node.path().last().copied().unwrap_or(0) as usize,
                kind: "unowned or non-list iteration",
            },
        )?;
        let source = SourceSite::source(statement.span());
        let mut current = block;
        let mut groups = Vec::with_capacity(iterators.len());
        for (iterator, (names, argument)) in iterators.iter().zip(sources) {
            let value = self.allocator.value();
            let completion = self.allocator.completion();
            self.push(
                current,
                ExecutableInstruction::EvaluateExpr {
                    value,
                    completion,
                    expr: ExecutableExpr::Operand {
                        text: iterator.list_arg.clone(),
                        braced: iterator.list_braced,
                    },
                    node: node.clone(),
                    source: source.clone(),
                },
            );
            let expression = ExecutableExpr::Operand {
                text: iterator.list_arg.clone(),
                braced: iterator.list_braced,
            };
            self.retain_expression_command_effect(
                value,
                &expression,
                original_tokens,
                Some(argument),
            );
            current = self.dispatch(current, completion, control);
            groups.push(IteratorGroup {
                list: value,
                variables: names
                    .iter()
                    .map(|name| CellReference::from_name_data(name))
                    .collect(),
            });
        }
        let header = self.new_block();
        self.terminate(current, ExecutableTerminator::Goto(header));
        let exit = self.new_block();
        let has_iteration = self.allocator.value();
        let completion = self.allocator.completion();
        self.push(
            header,
            ExecutableInstruction::IterateLists {
                has_iteration,
                completion,
                groups,
                node: node.clone(),
                source,
            },
        );
        let decided = self.dispatch(header, completion, control);
        let body_entry = self.new_block();
        self.terminate(
            decided,
            ExecutableTerminator::Branch {
                condition: has_iteration,
                then_target: body_entry,
                else_target: exit,
            },
        );
        let body_control = control.loop_body(exit, header);
        let path = child_path(node, LOOP_BODY_SLOT);
        let flow = self.emit_nested(registry, context, body, &path, body_entry, body_control)?;
        self.terminate(flow, ExecutableTerminator::Goto(header));
        Ok(self.complete_region(statement, node, exit, control, tail))
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_switch(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        statement: &Statement,
        original_tokens: Option<&crate::ir::CommandTokens>,
        parts: &SwitchParts<'_>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
        tail: bool,
    ) -> Result<StatementTail, SourceCompatibilityDecline> {
        let subject_source = original_tokens
            .and_then(|tokens| original_switch_subject_source(registry, statement, tokens, parts));
        let subject_braced = parts.subject_braced;
        let subject_value = self.allocator.value();
        let subject_completion = self.allocator.completion();
        self.push(
            block,
            ExecutableInstruction::EvaluateExpr {
                value: subject_value,
                completion: subject_completion,
                expr: ExecutableExpr::Operand {
                    text: parts.subject.to_owned(),
                    braced: subject_braced,
                },
                node: node.clone(),
                source: SourceSite::source(parts.subject_span),
            },
        );
        let expression = ExecutableExpr::Operand {
            text: parts.subject.to_owned(),
            braced: subject_braced,
        };
        self.retain_expression_command_effect(
            subject_value,
            &expression,
            original_tokens,
            subject_source.map(|(argument, _)| argument),
        );
        let mut current = self.dispatch(block, subject_completion, control);
        let join = self.new_block();
        // A `-` body falls through to the next arm that has one, so the shared
        // body is emitted once and every falling arm branches to it.
        let bodies: Vec<Option<ExecutableBlockId>> = parts
            .arms
            .iter()
            .map(|arm| arm.body.as_ref().map(|_| self.new_block()))
            .collect();
        for (index, arm) in parts.arms.iter().enumerate() {
            let Some(target) = next_switch_body(&bodies, index) else {
                continue;
            };
            let value = self.allocator.value();
            let completion = self.allocator.completion();
            self.push(
                current,
                ExecutableInstruction::MatchPattern {
                    value,
                    completion,
                    subject: subject_value,
                    pattern: SwitchPattern {
                        text: arm.pattern.clone(),
                        mode: parts.mode,
                        nocase: parts.nocase,
                        literal: parts.patterns_braced,
                    },
                    node: node.clone(),
                    source: SourceSite::source(arm.pattern_span),
                },
            );
            let decided = self.dispatch(current, completion, control);
            let next_test = self.new_block();
            self.terminate(
                decided,
                ExecutableTerminator::Branch {
                    condition: value,
                    then_target: target,
                    else_target: next_test,
                },
            );
            current = next_test;
        }
        for (index, arm) in parts.arms.iter().enumerate() {
            let (Some(body), Some(entry)) = (arm.body.as_ref(), bodies[index]) else {
                continue;
            };
            let path = child_path(node, u32::try_from(index).unwrap_or(u32::MAX));
            let flow = self.emit_nested(registry, context, body, &path, entry, control)?;
            self.terminate(flow, ExecutableTerminator::Goto(join));
        }
        if let Some(default_body) = parts.default_body {
            let path = child_path(node, SWITCH_DEFAULT_SLOT);
            let flow =
                self.emit_nested(registry, context, default_body, &path, current, control)?;
            self.terminate(flow, ExecutableTerminator::Goto(join));
        } else {
            self.terminate(current, ExecutableTerminator::Goto(join));
        }
        Ok(self.complete_region(statement, node, join, control, tail))
    }
}

fn original_switch_subject_source(
    registry: &CommandRegistry,
    statement: &Statement,
    tokens: &crate::ir::CommandTokens,
    parts: &SwitchParts<'_>,
) -> Option<(usize, bool)> {
    let (command, _) = tokens
        .source_binding
        .as_ref()?
        .original_recorded_command()?;
    (command.span == statement.span()).then_some(())?;
    let selected = crate::registry_invocation::original_source_name_roles(registry, tokens)?;
    (selected.facts().operation == SemanticOperationId::StructuredLowering(LoweringHookId::Switch))
        .then_some(())?;
    tokens.words().iter().enumerate().find_map(|(index, word)| {
        let braced = matches!(word, tcl_source::words::WordExpr::BracedLiteral { .. });
        if word.source().span == parts.subject_span
            && word.legacy_text() == parts.subject
            && braced == parts.subject_braced
        {
            Some((index.checked_sub(1)?, braced))
        } else {
            None
        }
    })
}

/// The `for`/`while` operands one loop projection needs.
struct LoopParts<'a> {
    init: Option<&'a Script>,
    pub(in super::super) condition: &'a ExprNode,
    condition_span: tcl_lexer::Span,
    condition_base: Option<u32>,
    next: Option<&'a Script>,
    pub(super) body: &'a Script,
}

/// The registry-parsed `switch` operands one decision tree needs.
struct SwitchParts<'a> {
    pub(in super::super) subject: &'a str,
    subject_braced: bool,
    subject_span: tcl_lexer::Span,
    arms: &'a [crate::ir::SwitchArm],
    default_body: Option<&'a Script>,
    mode: SwitchMode,
    nocase: bool,
    patterns_braced: bool,
}

/// The block that runs when a `switch` arm matches, following `-` fallthrough.
fn next_switch_body(
    bodies: &[Option<ExecutableBlockId>],
    index: usize,
) -> Option<ExecutableBlockId> {
    bodies[index..].iter().find_map(|body| *body)
}

#[cfg(test)]
pub(in super::super) fn original_list_iterator_names(
    registry: &CommandRegistry,
    statement: &Statement,
    iterators: &[crate::ir::ForeachIterator],
) -> Option<Vec<Vec<String>>> {
    original_list_iterator_sources(registry, statement, iterators)
        .map(|sources| sources.into_iter().map(|(names, _)| names).collect())
}

fn original_list_iterator_sources(
    registry: &CommandRegistry,
    statement: &Statement,
    iterators: &[crate::ir::ForeachIterator],
) -> Option<Vec<(Vec<String>, usize)>> {
    let Statement::Foreach {
        raw_tokens: Some(tokens),
        is_lmap,
        ..
    } = statement
    else {
        return None;
    };
    let (command, _) = tokens
        .source_binding
        .as_ref()?
        .original_recorded_command()?;
    (command.span == statement.span()).then_some(())?;
    let source = crate::registry_invocation::original_source_name_roles(registry, tokens)?;
    let hook = if *is_lmap {
        LoweringHookId::Lmap
    } else {
        LoweringHookId::Foreach
    };
    (source.facts().operation == SemanticOperationId::StructuredLowering(hook)).then_some(())?;
    let arguments: Vec<_> = source
        .roles()
        .iter()
        .filter_map(|&(argument, role)| {
            (role == tcl_registry::ArgRole::LoopVarList).then_some(argument)
        })
        .collect();
    (arguments.len() == iterators.len()).then_some(())?;
    iterators
        .iter()
        .zip(arguments)
        .map(|(iterator, argument)| {
            let names = crate::registry_invocation::original_name_role_values(
                registry,
                tokens,
                argument,
                tcl_registry::ArgRole::LoopVarList,
            )?;
            (names == iterator.vars).then_some((names, argument.checked_add(1)?))
        })
        .collect()
}
