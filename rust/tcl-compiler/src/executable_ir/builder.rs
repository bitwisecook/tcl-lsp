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

//! Building one executable function from source-faithful IR: blocks, completions, invocations and structured regions.

use super::footprints::lowered_operation_footprint;
use super::region_descriptors::{
    lowered_operation_descriptor, opaque_region_descriptor, structured_region_projection,
};
use super::{
    CompletionCase, CompletionId, ExecutableArgvId, ExecutableBlock, ExecutableBlockId,
    ExecutableFunctionId, ExecutableInstruction, ExecutableTerminator, ExecutableValueId,
    LoweredOperation, OpaqueRegion, OriginalExecutableWordSource, OriginalExpressionCommandEffect,
    SourceCompatibilityDecline,
};
use crate::ir::Script;
use crate::ir::Statement;
use crate::registry_invocation::InvocationMetadataContext;
use std::collections::BTreeMap;
use tcl_core_types::Code as CompletionCode;
use tcl_registry::CommandRegistry;
use tcl_source::words::NodeId;
use tcl_source::words::SourceSite;

mod exception_regions;
mod invocations;
mod structured_regions;

#[cfg(test)]
pub(super) use structured_regions::original_list_iterator_names;

/// Where an abnormal completion goes from the statement being emitted.
///
/// This is what makes "any non-OK code unwinds" a graph fact rather than a
/// backend convention: a completion whose code is not routed by one of these
/// targets reaches the `default` arm of a [`ExecutableTerminator::CompletionSwitch`]
/// and leaves the function carrying its own triple.
#[derive(Clone, Copy)]
pub(super) struct ControlContext {
    /// Block that receives a `TCL_BREAK` completion, inside a loop body.
    break_target: Option<ExecutableBlockId>,
    /// Block that receives a `TCL_CONTINUE` completion, inside a loop body.
    continue_target: Option<ExecutableBlockId>,
    /// Block that joins every other abnormal completion — a `catch` or `try`
    /// handler. `None` leaves the function with the completion.
    unwind: Option<ExecutableBlockId>,
}

impl ControlContext {
    /// The function body itself: nothing is caught, so every abnormal code
    /// leaves the function.
    pub(super) const FUNCTION_BODY: Self = Self {
        break_target: None,
        continue_target: None,
        unwind: None,
    };

    const fn loop_body(
        self,
        break_target: ExecutableBlockId,
        continue_target: ExecutableBlockId,
    ) -> Self {
        Self {
            break_target: Some(break_target),
            continue_target: Some(continue_target),
            unwind: self.unwind,
        }
    }

    /// A `catch`/`try` body: every abnormal code, loop control included, joins
    /// the handler instead of leaving the region.
    const fn caught_body(handler: ExecutableBlockId) -> Self {
        Self {
            break_target: None,
            continue_target: None,
            unwind: Some(handler),
        }
    }
}

/// Where control goes after one emitted statement.
enum StatementTail {
    /// Normal completion continues at this still-unterminated block.
    Falls {
        /// Block reached on normal completion.
        block: ExecutableBlockId,
        /// Completion the statement produced.
        completion: CompletionId,
    },
    /// The statement was the function body's tail and returned its completion.
    Returns,
}

/// Where control goes after one emitted script.
pub(super) enum ScriptTail {
    /// Normal completion continues at this still-unterminated block.
    Falls {
        /// Block reached on normal completion.
        block: ExecutableBlockId,
        /// Completion of the last statement, or `None` for an empty script.
        completion: Option<CompletionId>,
    },
    /// The script was the function body and its tail returned.
    Returns,
}

struct RegionEmissionSite<'a> {
    context: Option<InvocationMetadataContext<'a>>,
    pub(super) statement: &'a Statement,
    pub(super) node: &'a NodeId,
    control: ControlContext,
    tail: bool,
}

fn statement_evaluated_region(
    statement: &Statement,
) -> Option<&crate::execution_region::EvaluatedBodyRegion> {
    match statement {
        Statement::Call {
            tokens: Some(tokens),
            ..
        }
        | Statement::Barrier {
            tokens: Some(tokens),
            ..
        } => tokens.evaluated_body(),
        _ => None,
    }
}

pub(super) struct FunctionBuilder {
    allocator: IdAllocator,
    pub(super) blocks: Vec<ExecutableBlock>,
    pub(super) original_word_sources:
        BTreeMap<ExecutableValueId, Option<OriginalExecutableWordSource>>,
    pub(super) original_expression_command_effects:
        BTreeMap<ExecutableValueId, OriginalExpressionCommandEffect>,
}

impl FunctionBuilder {
    pub(super) fn new(function: ExecutableFunctionId) -> Self {
        Self {
            allocator: IdAllocator::new(function),
            blocks: Vec::new(),
            original_word_sources: BTreeMap::new(),
            original_expression_command_effects: BTreeMap::new(),
        }
    }

    /// Allocate one empty block. Allocation order is block-ID order, which is
    /// the deterministic vector position [`ExecutableFunction::validate`]
    /// requires.
    pub(super) fn new_block(&mut self) -> ExecutableBlockId {
        let id = self.allocator.block();
        self.blocks.push(ExecutableBlock::new(id));
        id
    }

    pub(super) fn push(&mut self, block: ExecutableBlockId, instruction: ExecutableInstruction) {
        self.blocks[block.index()].instructions.push(instruction);
    }

    pub(super) fn terminate(&mut self, block: ExecutableBlockId, terminator: ExecutableTerminator) {
        self.blocks[block.index()].terminator = Some(terminator);
    }

    /// Terminate `block` on `completion`, either returning it from the
    /// function — the tail statement of the function body — or dispatching it
    /// onwards.
    fn finish(
        &mut self,
        block: ExecutableBlockId,
        completion: CompletionId,
        control: ControlContext,
        tail: bool,
    ) -> StatementTail {
        if tail {
            self.terminate(block, ExecutableTerminator::ReturnCompletion(completion));
            return StatementTail::Returns;
        }
        StatementTail::Falls {
            block: self.dispatch(block, completion, control),
            completion,
        }
    }

    /// Terminate `block` by dispatching `completion`, and return the block
    /// where normal completion continues.
    pub(super) fn dispatch(
        &mut self,
        block: ExecutableBlockId,
        completion: CompletionId,
        control: ControlContext,
    ) -> ExecutableBlockId {
        let ok = self.new_block();
        let mut cases = vec![CompletionCase {
            code: CompletionCode::Ok,
            target: ok,
        }];
        if let Some(target) = control.break_target {
            cases.push(CompletionCase {
                code: CompletionCode::Break,
                target,
            });
        }
        if let Some(target) = control.continue_target {
            cases.push(CompletionCase {
                code: CompletionCode::Continue,
                target,
            });
        }
        let default = control.unwind.unwrap_or_else(|| {
            let leave = self.new_block();
            self.terminate(leave, ExecutableTerminator::ReturnCompletion(completion));
            leave
        });
        self.terminate(
            block,
            ExecutableTerminator::CompletionSwitch {
                completion,
                cases,
                default,
            },
        );
        ok
    }

    /// Emit an interior script of a structured region. Only the function body
    /// itself can return its tail, so this always falls through to a block.
    fn emit_nested(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        script: &Script,
        path: &[u32],
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<ExecutableBlockId, SourceCompatibilityDecline> {
        match self.emit_script(registry, context, script, path, block, control, false)? {
            ScriptTail::Falls { block, .. } => Ok(block),
            ScriptTail::Returns => {
                unreachable!("only the function body's own tail returns its completion")
            }
        }
    }

    /// Emit every statement of `script` starting at the empty block `block`.
    ///
    /// `tail` marks the function body itself, whose last statement returns the
    /// function's completion instead of dispatching it onwards.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_script(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        script: &Script,
        path: &[u32],
        block: ExecutableBlockId,
        control: ControlContext,
        tail: bool,
    ) -> Result<ScriptTail, SourceCompatibilityDecline> {
        let mut current = block;
        let mut last = None;
        for (statement_index, statement) in script.statements.iter().enumerate() {
            let node = child_node(path, statement_index);
            // A retained `return` ends the sequence: what follows it in the
            // same script is unreachable, exactly as the source-faithful
            // lowering already recorded.
            let terminates = matches!(statement, Statement::Return { .. });
            let is_last = terminates || statement_index + 1 == script.statements.len();
            let statement_tail = tail && is_last;
            match self.emit_statement(
                registry,
                context,
                statement,
                script.retained_source_tokens_for_statement(statement),
                &node,
                current,
                control,
                statement_tail,
            )? {
                StatementTail::Returns => return Ok(ScriptTail::Returns),
                StatementTail::Falls { block, completion } => {
                    current = block;
                    last = Some(completion);
                }
            }
            if terminates {
                break;
            }
        }
        Ok(ScriptTail::Falls {
            block: current,
            completion: last,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_statement(
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
        let source = SourceSite::source(statement.span());
        if let Some(region) = statement_evaluated_region(statement) {
            return self.emit_evaluated_region(
                registry,
                &RegionEmissionSite {
                    context,
                    statement,
                    node,
                    control,
                    tail,
                },
                block,
                region,
            );
        }
        if let Some(descriptor) = lowered_operation_descriptor(statement) {
            let completion = self.allocator.completion();
            self.push(
                block,
                ExecutableInstruction::ExecuteLowered(LoweredOperation {
                    completion,
                    descriptor,
                    footprint: lowered_operation_footprint(statement, original_tokens),
                    statement: statement.clone(),
                    original_tokens: original_tokens.cloned().map(std::sync::Arc::new),
                    node: node.clone(),
                    source,
                }),
            );
            return Ok(self.finish(block, completion, control, tail));
        }
        if structured_region_projection(statement).is_some() {
            return self.emit_structured_region(
                registry,
                context,
                statement,
                original_tokens,
                node,
                block,
                control,
                tail,
            );
        }
        if let Some(descriptor) = opaque_region_descriptor(statement) {
            let completion = self.allocator.completion();
            self.push(
                block,
                ExecutableInstruction::ExecuteOpaqueRegion(OpaqueRegion {
                    completion,
                    descriptor: descriptor.lowering_hook(),
                    statement: statement.clone(),
                    node: node.clone(),
                    source,
                }),
            );
            return Ok(self.finish(block, completion, control, tail));
        }
        self.emit_call(
            registry, context, statement, node, block, control, source, tail,
        )
    }
}

/// Reserved child-node slots for the interior scripts of a structured region.
///
/// They sit above any plausible statement index so an interior script's nodes
/// can never collide with a sibling clause body's.
const LOOP_BODY_SLOT: u32 = 1 << 24;

const LOOP_INIT_SLOT: u32 = (1 << 24) + 1;

const LOOP_NEXT_SLOT: u32 = (1 << 24) + 2;

const TRY_FINALLY_SLOT: u32 = (1 << 24) + 3;

const SWITCH_DEFAULT_SLOT: u32 = (1 << 24) + 4;

const TRY_HANDLER_SLOT: u32 = 1 << 25;

fn child_node(path: &[u32], index: usize) -> NodeId {
    let mut child = path.to_vec();
    child.push(u32::try_from(index).unwrap_or(u32::MAX));
    NodeId::from_path(child)
}

fn child_path(node: &NodeId, slot: u32) -> Vec<u32> {
    let mut path = node.path().to_vec();
    path.push(slot);
    path
}

pub(super) struct IdAllocator {
    pub(super) function: ExecutableFunctionId,
    next_block: usize,
    next_value: usize,
    next_argv: usize,
    next_completion: usize,
}

impl IdAllocator {
    pub(super) const fn new(function: ExecutableFunctionId) -> Self {
        Self {
            function,
            next_block: 0,
            next_value: 0,
            next_argv: 0,
            next_completion: 0,
        }
    }

    pub(super) fn block(&mut self) -> ExecutableBlockId {
        let id = ExecutableBlockId::new(self.function, self.next_block);
        self.next_block += 1;
        id
    }

    pub(super) fn value(&mut self) -> ExecutableValueId {
        let id = ExecutableValueId::new(self.function, self.next_value);
        self.next_value += 1;
        id
    }

    pub(super) fn argv(&mut self) -> ExecutableArgvId {
        let id = ExecutableArgvId::new(self.function, self.next_argv);
        self.next_argv += 1;
        id
    }

    pub(super) fn completion(&mut self) -> CompletionId {
        let id = CompletionId::new(self.function, self.next_completion);
        self.next_completion += 1;
        id
    }
}
