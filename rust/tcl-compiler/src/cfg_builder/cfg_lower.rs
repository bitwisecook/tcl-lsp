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

//! Per-construct CFG lowering methods.
//!
//! Each method flattens one structured IR construct (`If`, `For`,
//! `While`, `Foreach`, `Switch`, `Catch`, `Try`) into basic blocks
//! with terminators, returning the name of the "continuation" block
//! (or `None` if control doesn't fall through).

#[cfg(test)]
use tcl_lexer::LexerConfig;
use tcl_lexer::Span;

use crate::cfg::Terminator;
#[cfg(test)]
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;
use crate::ir_helpers::expr_has_command;

use super::CfgBuilder;

mod catch;
mod loops;
mod switch;
mod try_construct;

pub use loops::matches_foreach_header;
pub(super) use switch::subject_may_scan_as_option;

impl CfgBuilder<'_> {
    // if

    /// Flatten `Statement::If` into cascaded branch blocks.
    pub(super) fn lower_if(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::If {
            span,
            clauses,
            else_body,
            else_span,
            ..
        } = stmt
        else {
            unreachable!("lower_if called with non-If");
        };

        let end_block = self.new_block("if_end");
        self.branch_command_joins
            .insert(block_name.to_owned(), end_block.clone());
        let owner = self.command_boundary_sites.get(block_name).cloned();
        let mut dispatch = block_name.to_owned();

        for clause in clauses {
            // When the branch condition contains a command substitution,
            // append a synthetic `<cond>` Statement::Call so the emitter
            // can wrap the ExprNode::Command with its own startCommand
            // boundary.
            if expr_has_command(&clause.condition) {
                // A `catch`/`regexp`/`scan` substitution — or a call to a
                // known upvar / global-writing user proc — in the condition
                // writes result variables; record them as defs so a read in
                // the guarded body is not flagged read-before-set (W210), and
                // its reads so the store feeding an `[incr n]` there is not
                // taken for a dead one.
                self.push_condition_effects(&clause.condition, *span, &dispatch);
            }
            let then_block = self.new_block("if_then");
            let next_dispatch = self.new_block("if_next");
            self.copy_command_boundary(block_name, &dispatch);
            self.copy_condition_binding(block_name, &dispatch);

            let true_target = self.bid(&then_block);
            let false_target = self.bid(&next_dispatch);
            self.condition_owners.insert(
                clause.condition_span,
                crate::cfg::ConditionOwner {
                    command: *span,
                    skips_constant_dead_script: true,
                },
            );
            self.set_terminator(
                &dispatch,
                Terminator::Branch {
                    condition: clause.condition.clone(),
                    true_target,
                    false_target,
                    span: Some(clause.condition_span),
                    condition_base: clause.condition_base,
                },
            );

            if let Some(tail) = self.lower_body(owner.as_ref(), &clause.body, &then_block) {
                self.ensure_goto(&tail, &end_block, Some(clause.body_span));
            }

            dispatch = next_dispatch;
        }

        if let Some(eb) = else_body {
            if let Some(tail) = self.lower_body(owner.as_ref(), eb, &dispatch) {
                self.ensure_goto(&tail, &end_block, *else_span);
            }
        } else {
            self.ensure_goto(&dispatch, &end_block, Some(*span));
        }

        end_block
    }

    // for

    /// Flatten `Statement::For` into init → header → body → step → header loop.
    /// Push the placeholder an empty `for` init / next clause takes, so the
    /// clause keeps its place in the instruction stream (codegen emits the
    /// three `nop`s tclsh's bytecode has there).
    ///
    /// The statement is a *synthetic marker*: the `command` spelling is only a
    /// label for the disassembly, and codegen recognises it by the typed
    /// [`crate::ir::SyntheticMarker`] on its tokens — `<empty_clause>` is a
    /// legal Tcl command name a script may define and call.
    fn push_empty_clause(&mut self, block: &str, span: Span) {
        self.push_statement(
            block,
            Statement::Call {
                span,
                command: "<empty_clause>".into(),
                canonical_command: None,
                args: vec![],
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::EmptyClause,
                )),
                foreach_groups: None,
            },
        );
    }
}

#[cfg(test)]
mod tests;
