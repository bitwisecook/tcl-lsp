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

//! Flattening `catch` and closing a flattened catch region.

use super::super::CfgBuilder;
use crate::ir::Statement;

impl CfgBuilder<'_> {
    /// Flatten `Statement::Catch` into body → end CFG, the analogue of
    /// [`Self::lower_try`] for the simpler construct.
    ///
    /// `catch` has no handler clauses, no fallthrough groups and no handler
    /// variable binding, so the shape is just body → end with exception edges
    /// from the body's throw points. What it buys is what the opaque form
    /// costs: with the body as real blocks the ordinary emitters compile it,
    /// so its variables reach the LVT and the optimiser can see inside — C
    /// allocates `q` in `catch {set q $x}` as a compiled local, and before
    /// this the whole body was one `invokeStk` (#2207, and the missing slot
    /// behind #2173).
    ///
    /// `result_var` / `options_var` are defined on *both* paths — normal
    /// completion stores the body's result, an error stores the message — so
    /// unlike a `try` handler's variables they are defined at the end block,
    /// which both paths reach, rather than on the error path alone.
    pub(in super::super) fn lower_catch(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::Catch {
            span,
            body,
            body_span,
            ..
        } = stmt
        else {
            unreachable!("lower_catch called with non-Catch");
        };

        let body_block = self.new_block("catch_body");
        let end_block = self.new_block("catch_end");
        self.copy_command_boundary(block_name, &body_block);
        self.command_boundary_continuations
            .insert(body_block.clone(), end_block.clone());
        let owner = self.command_boundary_sites.get(block_name).cloned();
        self.ensure_goto(block_name, &body_block, Some(*span));

        // Same throw-block bookkeeping as `lower_try`: install a fresh list
        // around the body so the exception edges are sourced from each
        // explicit `error`/`throw` point — where the body's prior defs are
        // live — and not from the pre-`catch` block. Restoring the outer list
        // keeps a nested `catch`'s throws attributed to its own region.
        let outer_throw_blocks = self.throw_blocks.take();
        self.throw_blocks = Some(Vec::new());
        // Any command of the body may fail, and what the body has stored when
        // it does is the handler's state, so in an analysis build each
        // statement ends a block an exception edge leaves from.
        let outer_region = std::mem::replace(
            &mut self.split_region,
            self.faithful_exceptions
                .then_some(super::super::SplitRegion::Catch),
        );
        let outer_split_points = std::mem::take(&mut self.split_points);
        let (raw_body_tail, body_terminal) = if self.faithful_exceptions {
            // The same captured-completion owner handles package lifecycle
            // phases and catch bodies. Captured return/break/continue must
            // reach this merge, rather than remain procedure/loop exits.
            self.lower_region_phase(body, &body_block, Some(&end_block), Some(&end_block));
            (None, None)
        } else {
            (
                self.lower_body(owner.as_ref(), body, &body_block),
                self.last_terminal_block.take(),
            )
        };
        let split_points = std::mem::replace(&mut self.split_points, outer_split_points);
        self.split_region = outer_region;
        let body_throw_blocks = self.throw_blocks.take().unwrap_or_default();
        self.throw_blocks = outer_throw_blocks;

        // A body that did not fall through (a bare `return`, an `error`) must
        // not edge to the end block as normal completion; `catch` still
        // resumes there, but by catching, which is an exception edge.
        let body_tail = if body_terminal.is_none() {
            raw_body_tail
        } else {
            None
        };
        if let Some(tail) = &body_tail {
            self.ensure_goto(tail, &end_block, Some(*body_span));
        }

        // Every way the body can fail reaches the end block, because `catch`
        // catches everything. Analysis-only edges (SSA phi predecessors and
        // SCCP reachability), exactly as `push_try_handler_exception_edges`
        // records them for a handler.
        //
        // The edge from the *pre-catch* block is the one that matters for
        // soundness, and it is not optional: the body can fail at its very
        // first command, so the state before the `catch` reaches the end
        // untouched by anything the body writes. Without it
        // `set cmd safe; catch {set cmd risky}; $cmd` joins to `risky`
        // alone — a single may-target that would license a destructive
        // rename, where the truth is `safe` or `risky`. `lower_try` gets
        // this from `ensure_goto(block_name, &handler_block, …)`; a `catch`
        // has no handler block to edge to, so it is recorded here.
        let mut throw_sources: Vec<String> = vec![block_name.to_owned()];
        throw_sources.extend(split_points.into_iter().map(|point| point.block));
        for tb in &body_throw_blocks {
            if !throw_sources.contains(tb) {
                throw_sources.push(tb.clone());
            }
        }
        if let Some(terminal) = &body_terminal
            && !throw_sources.contains(terminal)
        {
            throw_sources.push(terminal.clone());
        }
        if let Some(tail) = &body_tail
            && !throw_sources.contains(tail)
        {
            throw_sources.push(tail.clone());
        }
        for src in throw_sources {
            self.exception_edges.push((src, end_block.clone()));
        }
        if self.faithful_exceptions {
            self.region_entries.push((
                block_name.to_owned(),
                end_block.clone(),
                body_block.clone(),
            ));
        }
        self.total_interceptors.insert(end_block.clone());

        self.end_flattened_catch(stmt, (block_name, &end_block));

        end_block
    }

    /// The statement that ends a flattened `catch` region: it defines the
    /// result and options variables however the body ended, so they belong at
    /// the merge rather than on one path, and the script's last command's
    /// stores are observed there, since `catch` stores that command's value in
    /// the result variable — a store nothing else reads is not a dead one, and
    /// deleting it changes what the result variable holds.
    ///
    /// The statement has no words, so the code generator skips it; the
    /// analysis build keeps the `catch` as written beside it, for the solver to
    /// evaluate over the state before the body.
    fn end_flattened_catch(&mut self, stmt: &Statement, (block_name, end_block): (&str, &str)) {
        let Statement::Catch {
            span,
            body,
            result_var,
            options_var,
            raw_args,
            tokens,
            ..
        } = stmt
        else {
            unreachable!("end_flattened_catch called with non-Catch");
        };
        if result_var.is_some()
            && let Some(last) = body.statements.last()
        {
            let observed = if self.command_classes.metadata.is_standalone() {
                crate::ssa::defs_of_with_registry(last, Some(self.registry))
            } else {
                crate::ssa::defs_of_with_metadata_context(
                    last,
                    self.registry,
                    self.command_classes
                        .metadata_context()
                        .and_then(crate::registry_invocation::InvocationMetadataSelection::context),
                )
            };
            self.alias_observed_vars.extend(observed);
        }
        let mut defs = Vec::new();
        if let Some(rv) = result_var {
            defs.push(rv.clone());
        }
        if let Some(ov) = options_var {
            defs.push(ov.clone());
        }
        if defs.is_empty() {
            return;
        }
        if self.faithful_exceptions {
            self.catch_ends.push((
                block_name.to_owned(),
                end_block.to_owned(),
                Statement::Call {
                    span: *span,
                    command: "catch".into(),
                    canonical_command: None,
                    args: raw_args.clone(),
                    defs: defs.clone(),
                    reads: vec![],
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: tokens.clone(),
                    foreach_groups: None,
                },
            ));
        }
        let mut output_tokens = tokens.clone().unwrap_or_else(|| {
            crate::ir::CommandTokens::marker(crate::ir::SyntheticMarker::CapturedCatchOutputs)
        });
        output_tokens.synthetic = Some(crate::ir::SyntheticMarker::CapturedCatchOutputs);
        let command = output_tokens
            .argv_texts
            .first()
            .cloned()
            .unwrap_or_default();
        self.push_statement(
            end_block,
            Statement::Call {
                span: *span,
                command,
                canonical_command: None,
                args: raw_args.clone(),
                defs,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(output_tokens),
                foreach_groups: None,
            },
        );
    }
}
