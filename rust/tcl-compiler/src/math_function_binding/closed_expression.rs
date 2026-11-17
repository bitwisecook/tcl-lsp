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

//! Whether a prepared expression runs nothing but its own arithmetic: no
//! command, no user code, and no read another can observe.

use super::{ExpressionMathBindings, ResolvedMathFunctionCall};
use crate::command_binding::{SourceVariableEvaluationOwner, SourceVariableReadResidual};
use crate::expr_ast::ExprNode;

/// The first reason an expression may run more than its arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenExpression {
    /// The site has no prepared original: one braced word the walk compiled.
    Unprepared,
    /// The expression substitutes a command.
    CommandSubstitution,
    /// A function call is not resolved to one proved implementation.
    UnresolvedFunction,
    /// A resolved function's declared effects reach beyond its result.
    FunctionEffects,
    /// An operand reads a variable another can name, and so trace: a
    /// global, a namespace variable, a link or an array element. A trace would
    /// fire again when a splice is dropped and the call invoked.
    ForeignOperand,
    /// An operand's read is not one retained read with a closed context.
    /// Its contents need not be known: a plain local of a spliced frame lives
    /// in an anonymous slot nothing can name or trace, and an expression that
    /// raises on what it reads has done nothing observable before the drop.
    OpenRead,
}

impl<'a> ExpressionMathBindings<'a> {
    /// Whether the expression the walk prepared for this site, as the
    /// interpreter compiles it, runs nothing a script can observe besides its
    /// result or its error: it substitutes no command; every function it
    /// calls is resolved to one proved implementation that `function_closed`
    /// accepts (given the call and its argument count); and every operand is
    /// a scalar `local` accepts, read once through a retained read whose
    /// context is closed.
    ///
    /// # Errors
    /// The first condition the expression fails.
    pub fn runs_only_its_arithmetic(
        &self,
        local: impl Fn(&str) -> bool,
        mut function_closed: impl FnMut(&ResolvedMathFunctionCall<'a>, usize) -> bool,
    ) -> Result<(), OpenExpression> {
        let preparation = self
            .preparation()
            .filter(|preparation| {
                preparation.witness.source().as_bytes() == preparation.source.text.bytes()
            })
            .ok_or(OpenExpression::Unprepared)?;
        let tree = preparation.witness.tree();
        if crate::optimiser::helpers::expr_simplify::expr_has_command_subst(tree) {
            return Err(OpenExpression::CommandSubstitution);
        }
        let mut pending = vec![tree];
        while let Some(node) = pending.pop() {
            match node {
                ExprNode::Literal { .. } => {}
                ExprNode::Binary { left, right, .. } => pending.extend([&**left, &**right]),
                ExprNode::Unary { operand, .. } => pending.push(operand),
                ExprNode::Ternary {
                    condition,
                    true_branch,
                    false_branch,
                } => pending.extend([&**condition, &**true_branch, &**false_branch]),
                ExprNode::Call {
                    function,
                    args,
                    start,
                    ..
                } => {
                    let call = self
                        .resolved_call(function, *start)
                        .ok_or(OpenExpression::UnresolvedFunction)?;
                    if !function_closed(&call, args.len()) {
                        return Err(OpenExpression::FunctionEffects);
                    }
                    pending.extend(args.iter());
                }
                ExprNode::Var { text, name, .. } => {
                    if text.contains('(') || name.contains('(') || !local(name) {
                        return Err(OpenExpression::ForeignOperand);
                    }
                    let (_, source) = preparation
                        .original_variable_source(node)
                        .ok_or(OpenExpression::OpenRead)?;
                    let mut reads = self.reads.iter().filter(|access| {
                        access.source == source
                            && matches!(&access.owner, SourceVariableEvaluationOwner::NativeExpression { invocation, .. } if invocation == &preparation.invocation)
                    });
                    let read = reads.next().ok_or(OpenExpression::OpenRead)?;
                    if reads.next().is_some()
                        || read.context_residual() != SourceVariableReadResidual::Closed
                    {
                        return Err(OpenExpression::OpenRead);
                    }
                }
                // A quoted operand substitutes what it holds; a raw or
                // compiled word is not a term the preparation proved.
                ExprNode::String { text, .. } => {
                    if text.contains(['$', '[']) {
                        return Err(OpenExpression::OpenRead);
                    }
                }
                ExprNode::Command { .. } => return Err(OpenExpression::CommandSubstitution),
                ExprNode::Raw { .. } | ExprNode::CompiledWord { .. } => {
                    return Err(OpenExpression::Unprepared);
                }
            }
        }
        Ok(())
    }
}
