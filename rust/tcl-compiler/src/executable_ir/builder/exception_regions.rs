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

//! Projecting `catch` and `try` with its handler clauses and `finally` into executable edges.

use super::super::region_descriptors::try_handler_code_in;
use super::super::{
    CellReference, CompletionCase, CompletionId, CompletionPayload, ExecutableBlockId,
    ExecutableExpr, ExecutableInstruction, ExecutableTerminator, SourceCompatibilityDecline,
};
use super::{
    ControlContext, FunctionBuilder, LOOP_BODY_SLOT, StatementTail, TRY_FINALLY_SLOT,
    TRY_HANDLER_SLOT, child_path,
};
use crate::ir::Script;
use crate::ir::Statement;
use crate::registry_invocation::InvocationMetadataContext;
use std::collections::{BTreeMap, BTreeSet};
use tcl_core_types::Code as CompletionCode;
use tcl_registry::CommandRegistry;
use tcl_source::words::NodeId;
use tcl_source::words::SourceSite;

impl FunctionBuilder {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_catch(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        statement: &Statement,
        body: &Script,
        result_var: Option<&str>,
        options_var: Option<&str>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
        tail: bool,
    ) -> Result<StatementTail, SourceCompatibilityDecline> {
        let source = SourceSite::source(statement.span());
        let handler = self.new_block();
        let join = self.new_block();
        let path = child_path(node, LOOP_BODY_SLOT);
        let flow = self.emit_nested(
            registry,
            context,
            body,
            &path,
            block,
            ControlContext::caught_body(handler),
        )?;
        self.terminate(flow, ExecutableTerminator::Goto(join));

        let caught = self.allocator.completion();
        self.push(
            handler,
            ExecutableInstruction::JoinCompletion {
                completion: caught,
                node: node.clone(),
                source: source.clone(),
            },
        );
        let mut current = self.new_block();
        self.terminate(handler, ExecutableTerminator::Goto(current));
        for (variable, payload) in [
            (result_var, CompletionPayload::Result),
            (options_var, CompletionPayload::Options),
        ] {
            let Some(variable) = variable else { continue };
            let completion = self.allocator.completion();
            self.push(
                current,
                ExecutableInstruction::WriteCompletionCell {
                    completion,
                    payload_of: caught,
                    payload,
                    cell: CellReference::from_name_data(variable),
                    node: node.clone(),
                    source: source.clone(),
                },
            );
            current = self.dispatch(current, completion, control);
        }
        self.terminate(current, ExecutableTerminator::Goto(join));
        Ok(self.complete_region(statement, node, join, control, tail))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_try(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        statement: &Statement,
        body: &Script,
        handlers: &[crate::ir::TryHandler],
        finally_body: Option<&Script>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
        tail: bool,
    ) -> Result<StatementTail, SourceCompatibilityDecline> {
        let source = SourceSite::source(statement.span());
        let dispatch_block = self.new_block();
        let finally_entry = self.new_block();
        let join = self.new_block();
        let path = child_path(node, LOOP_BODY_SLOT);
        let flow = self.emit_nested(
            registry,
            context,
            body,
            &path,
            block,
            ControlContext::caught_body(dispatch_block),
        )?;
        self.terminate(flow, ExecutableTerminator::Goto(finally_entry));

        let caught = self.allocator.completion();
        self.push(
            dispatch_block,
            ExecutableInstruction::JoinCompletion {
                completion: caught,
                node: node.clone(),
                source: source.clone(),
            },
        );
        // Every handler's abnormal completion also runs `finally`, so handler
        // bodies unwind into the same finally edge as the body.
        let handler_control = ControlContext::caught_body(finally_entry);
        // `emit_try_handlers` returns the cases already in ascending code
        // order, one entry per code, each the first clause that code tries.
        let cases = self.emit_try_handlers(
            registry,
            context,
            handlers,
            caught,
            node,
            &source,
            finally_entry,
            handler_control,
        )?;
        self.terminate(
            dispatch_block,
            ExecutableTerminator::CompletionSwitch {
                completion: caught,
                cases,
                // An unhandled code still runs `finally` before it leaves.
                default: finally_entry,
            },
        );

        let finally_flow = if let Some(finally_body) = finally_body {
            let path = child_path(node, TRY_FINALLY_SLOT);
            self.emit_nested(
                registry,
                context,
                finally_body,
                &path,
                finally_entry,
                ControlContext::caught_body(join),
            )?
        } else {
            finally_entry
        };
        self.terminate(finally_flow, ExecutableTerminator::Goto(join));
        Ok(self.complete_region(statement, node, join, control, tail))
    }

    /// Emit the `on`/`trap` handler clauses and return, per completion code,
    /// the first handler entry that code should try.
    ///
    /// Clauses are tried in source order: a `trap` whose `-errorcode` prefix
    /// does not match falls to the next clause selecting the same code, and
    /// only an unmatched *last* clause reaches `finally_entry`. A clause after
    /// an unconditional one for the same code can never run, so it is not
    /// emitted at all.
    #[allow(clippy::too_many_arguments)]
    fn emit_try_handlers(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        handlers: &[crate::ir::TryHandler],
        caught: CompletionId,
        node: &NodeId,
        source: &SourceSite,
        finally_entry: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<Vec<CompletionCase>, SourceCompatibilityDecline> {
        let numbers = context
            .and_then(InvocationMetadataContext::source_analysis_input)
            .map_or(tcl_syntax::number::Numbers::Unknown, |input| {
                tcl_syntax::number::Numbers::Target(
                    input
                        .lexer_config()
                        .grammar_over(input.unit_profile().grammar)
                        .numbers,
                )
            });
        let mut reachable: Vec<(usize, CompletionCode, bool)> = Vec::new();
        let mut settled: BTreeSet<i64> = BTreeSet::new();
        for (index, handler) in handlers.iter().enumerate() {
            let code = try_handler_code_in(handler, numbers).ok_or(
                SourceCompatibilityDecline::UnsupportedStatement {
                    statement_index: node.path().last().copied().unwrap_or(0) as usize,
                    kind: "try handler selector without exact source grammar",
                },
            )?;
            if settled.contains(&code.as_int()) {
                continue;
            }
            let conditional = handler.trap_pattern.is_some();
            if !conditional {
                settled.insert(code.as_int());
            }
            reachable.push((index, code, conditional));
        }

        // Build in reverse so each clause knows where a failed match falls.
        let mut next_for_code: BTreeMap<i64, ExecutableBlockId> = BTreeMap::new();
        for (index, code, _) in reachable.iter().copied().rev() {
            let handler = &handlers[index];
            let no_match = next_for_code
                .get(&code.as_int())
                .copied()
                .unwrap_or(finally_entry);
            let entry = self.new_block();
            let mut current = entry;
            if let Some(prefix) = handler.trap_pattern.clone() {
                // A `trap` selector narrows an error by its `-errorcode`
                // prefix; the prefix is the registry-owned parse lowering
                // already performed, never a local re-parse of the grammar.
                let value = self.allocator.value();
                let completion = self.allocator.completion();
                let expression = ExecutableExpr::TrapPrefix {
                    completion: caught,
                    prefix,
                };
                self.push(
                    current,
                    ExecutableInstruction::EvaluateExpr {
                        value,
                        completion,
                        expr: expression.clone(),
                        node: node.clone(),
                        source: source.clone(),
                    },
                );
                self.retain_expression_command_effect(value, &expression, None, None);
                let decided = self.dispatch(current, completion, control);
                let matched = self.new_block();
                self.terminate(
                    decided,
                    ExecutableTerminator::Branch {
                        condition: value,
                        then_target: matched,
                        else_target: no_match,
                    },
                );
                current = matched;
            }
            for (variable, payload) in [
                (handler.var_name.as_deref(), CompletionPayload::Result),
                (handler.options_var.as_deref(), CompletionPayload::Options),
            ] {
                let Some(variable) = variable else { continue };
                let completion = self.allocator.completion();
                self.push(
                    current,
                    ExecutableInstruction::WriteCompletionCell {
                        completion,
                        payload_of: caught,
                        payload,
                        cell: CellReference::from_name_data(variable),
                        node: node.clone(),
                        source: source.clone(),
                    },
                );
                current = self.dispatch(current, completion, control);
            }
            let path = child_path(node, TRY_HANDLER_SLOT + u32::try_from(index).unwrap_or(0));
            let body = resolve_try_fallthrough(handlers, index);
            let flow = self.emit_nested(registry, context, body, &path, current, control)?;
            self.terminate(flow, ExecutableTerminator::Goto(finally_entry));
            next_for_code.insert(code.as_int(), entry);
        }
        Ok(next_for_code
            .into_iter()
            .map(|(code, target)| CompletionCase {
                code: CompletionCode::from_int(i32::try_from(code).unwrap_or(i32::MAX)),
                target,
            })
            .collect())
    }
}

/// The body a `try` handler runs, following `-` fallthrough to the next
/// handler that has one.
fn resolve_try_fallthrough(handlers: &[crate::ir::TryHandler], index: usize) -> &Script {
    for handler in &handlers[index..] {
        if !handler.fallthrough {
            return &handler.body;
        }
    }
    &handlers[index].body
}
