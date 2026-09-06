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

//! CFG terminator emission.
//!
//! Extends [`CodegenCtx`] with methods for emitting the bytecode for
//! each [`Terminator`] variant (goto / branch / return).

#![allow(
    clippy::similar_names,
    clippy::match_same_arms,
    clippy::cast_possible_truncation,
    clippy::doc_markdown
)]

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;

use super::super::cmd_subst::is_pure_cmd_subst;
use super::super::{CodegenCtx, Op, Operand};
use super::ordering::fold_const_branch;

impl CodegenCtx<'_> {
    /// Emit the bytecode for a CFG terminator.
    ///
    /// `next_block` is the block name that will be emitted immediately
    /// after this block (for fallthrough elision), or `None` if this is
    /// the last block in layout order.
    pub fn emit_term(&mut self, cfg: &CfgFunction, term: &Terminator, next_block: Option<&str>) {
        // Carry the terminator's source span (jump condition / return
        // value) onto the instructions it lowers to, when one exists.
        self.current_span = term.span();
        match term {
            Terminator::Goto { target, .. } => {
                let target = cfg.block_name(*target);
                if Some(target) != next_block {
                    self.emit_comment(
                        Op::JUMP4,
                        vec![Operand::Label(target.to_owned())],
                        &format!("-> {target}"),
                    );
                }
            }
            Terminator::Branch {
                condition,
                true_target,
                false_target,
                ..
            } => {
                let true_target = cfg.block_name(*true_target);
                let false_target = cfg.block_name(*false_target);
                self.emit_branch(condition, true_target, false_target, next_block);
            }
            Terminator::Return { value, expr, .. } => {
                self.emit_return(value.as_deref(), expr.as_ref());
            }
        }
    }

    fn emit_branch(
        &mut self,
        cond: &ExprNode,
        true_target: &str,
        false_target: &str,
        next_block: Option<&str>,
    ) {
        // In proc bodies, the branch condition counts as a command
        // for startCommand numbering.
        if self.is_proc {
            self.cmd_index += 1;
        }
        match fold_const_branch(cond) {
            Some(true) => {
                if Some(true_target) != next_block {
                    self.emit_comment(
                        Op::JUMP4,
                        vec![Operand::Label(true_target.to_owned())],
                        &format!("-> {true_target}"),
                    );
                }
            }
            Some(false) => {
                if Some(false_target) != next_block {
                    self.emit_comment(
                        Op::JUMP4,
                        vec![Operand::Label(false_target.to_owned())],
                        &format!("-> {false_target}"),
                    );
                }
            }
            None => {
                self.emit_expr(cond);
                // tclsh emits a nop between a simple variable condition
                // and the conditional jump (placeholder for tryCvtToNumeric).
                if matches!(cond, ExprNode::Var { .. } | ExprNode::Raw { .. }) {
                    self.emit(Op::NOP, vec![]);
                } else if let ExprNode::Command { text, .. } = cond {
                    let inner = text.trim();
                    let stripped = inner.strip_prefix('[').unwrap_or(inner);
                    if stripped.starts_with("catch ") {
                        self.emit(Op::NOP, vec![]);
                    }
                }
                if Some(false_target) == next_block {
                    self.emit_comment(
                        Op::JUMP_TRUE4,
                        vec![Operand::Label(true_target.to_owned())],
                        &format!("-> {true_target}"),
                    );
                } else if Some(true_target) == next_block {
                    self.emit_comment(
                        Op::JUMP_FALSE4,
                        vec![Operand::Label(false_target.to_owned())],
                        &format!("-> {false_target}"),
                    );
                } else {
                    self.emit_comment(
                        Op::JUMP_FALSE4,
                        vec![Operand::Label(false_target.to_owned())],
                        &format!("!-> {false_target}"),
                    );
                    self.emit_comment(
                        Op::JUMP4,
                        vec![Operand::Label(true_target.to_owned())],
                        &format!("-> {true_target}"),
                    );
                }
            }
        }
    }

    fn emit_return(&mut self, value: Option<&str>, expr: Option<&ExprNode>) {
        if let Some(e) = expr {
            // Proc with `return [expr {...}]` lowered to an expression
            let guaranteed_numeric = self.emit_expr(e);
            if !guaranteed_numeric {
                self.emit(Op::TRY_CVT_TO_NUMERIC, vec![]);
            }
        } else {
            let val = value.unwrap_or("");
            self.emit_value(val, true);
        }
        if self.is_proc {
            self.emit(Op::DONE, vec![]);
        } else {
            // Top-level `RETURN_IMM` pops [result, options]; push the empty
            // options dict so the pair is the top of stack even when a prior
            // statement (e.g. `if` with no `else`) left a value behind.
            //
            // A plain `return` outside a proc is `(code 0, level 1)` — C's
            // default `-level` (`TclMergeReturnOptions`), carried on the operands
            // by `CompileReturnInternal`. `(0, 0)` is `TclProcessReturn` at level
            // 0 with code OK, which falls through to the next instruction instead
            // of ending the script.
            self.push_lit("");
            self.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(1)]);
        }
    }

    /// Emit a proc-body `return` with `startCommand` wrapping and
    /// dead-code jumps.
    ///
    /// tclsh wraps each compiled command in a proc body with
    /// `startCommand`. When this `return` is inside a then-branch or
    /// switch arm, tclsh also emits a dead-code `jump` past the else
    /// path after the `done`.
    pub fn emit_proc_return(
        &mut self,
        term: &Terminator,
        bname: &str,
        next_block: Option<&str>,
        block_order: &[String],
        block_idx: usize,
        cfg: &CfgFunction,
    ) {
        let Terminator::Return { value, expr, .. } = term else {
            unreachable!("emit_proc_return called with non-Return terminator");
        };
        // Stamp the return's source span onto its instructions.
        self.current_span = term.span();
        let val = value.as_deref().unwrap_or("");
        // The *whole* value must be one balanced `[…]`: the inline emitter
        // strips the outer brackets and word-splits what is left, so a value
        // that merely begins and ends with a bracket (`[llength $a]:[join $a ,]`
        // — a three-part concatenation) would be mangled into a single bogus
        // command. `is_pure_cmd_subst` matches the bracket, not the ends.
        let is_cmd_subst = expr.is_none() && is_pure_cmd_subst(val);
        let is_final = next_block.is_none();

        // startCommand count: 2 when return wraps [expr {...}]
        let count = if is_cmd_subst { 2 } else { 1 };

        // Find join block for dead-code jump after done.
        let mut join_block: Option<String> = None;
        if !is_final && bname.starts_with("if_then_") {
            for future_name in &block_order[block_idx + 1..] {
                if future_name.starts_with("if_end_") {
                    join_block = Some(future_name.clone());
                    break;
                }
            }
        } else if !is_final
            && (bname.starts_with("switch_arm_body_") || bname.starts_with("switch_default_"))
        {
            if self.proc_exit_label.is_none() {
                self.proc_exit_label = Some(self.fresh_label("proc_exit"));
            }
            join_block.clone_from(&self.proc_exit_label);
        }

        // Only emit startCommand for non-first commands in the proc body.
        let end_label = if self.cmd_index > 0 {
            let l = self.fresh_label("ret_end");
            self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(l.clone()), Operand::Imm(count)],
                "",
            );
            Some(l)
        } else {
            None
        };
        self.cmd_index += 1;

        if let Some(e) = expr.as_ref() {
            let guaranteed_numeric = self.emit_expr(e);
            if !guaranteed_numeric {
                self.emit(Op::TRY_CVT_TO_NUMERIC, vec![]);
            }
        } else if self.is_proc && is_cmd_subst {
            // In a proc body, a return value of [cmd ...] inlines.
            self.emit_inline_cmd_subst(val);
        } else {
            self.emit_value(val, true);
        }
        self.emit(Op::DONE, vec![]);

        if let Some(ref l) = end_label {
            self.place_label(l);
        }

        // tclsh 9.0 appends an unreachable `done` as the function exit
        // point when the tail return has a startCommand wrapper.
        let has_branches = cfg
            .blocks
            .values()
            .any(|b| matches!(b.terminator, Some(Terminator::Branch { .. })));
        if is_final && end_label.is_some() && !has_branches {
            self.emit(Op::DONE, vec![]);
        }

        if let Some(jb) = join_block {
            self.emit_comment(Op::JUMP4, vec![Operand::Label(jb)], "dead-skip-else");
        }
    }

    /// Detect a switch dispatch chain and emit a `jumpTable` opcode.
    ///
    /// Returns `true` if a jump table was emitted (caller should skip
    /// normal terminator emission and mark the intermediate dispatch
    /// blocks in `skip_blocks`).
    ///
    /// A switch dispatch chain is a sequence of `Branch` blocks with
    /// condition `STR_EQ(subject, literal)` where `subject` is the
    /// same expression across all blocks. The chain ends with a
    /// `Goto` to the default target.
    pub fn try_emit_jump_table(
        &mut self,
        cfg: &CfgFunction,
        blk: &crate::cfg::Block,
        next_block: Option<&str>,
        skip_blocks: &mut std::collections::HashSet<String>,
    ) -> bool {
        use crate::expr_ast::{BinOp, ExprNode, render_expr};

        let Some(Terminator::Branch { .. }) = &blk.terminator else {
            return false;
        };

        // Walk the chain of STR_EQ branches sharing the same subject.
        let mut cases: Vec<(String, String)> = Vec::new();
        let mut subject: Option<String> = None;
        let mut subject_expr: Option<ExprNode> = None;
        let mut dispatch_blocks: Vec<String> = Vec::new();
        let mut current_term = blk.terminator.clone();

        while let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            ..
        }) = &current_term
        {
            let ExprNode::Binary { op, left, right } = condition else {
                break;
            };
            if *op != BinOp::StrEq {
                break;
            }
            // A jump table keys on *literal* strings, so only a pattern that
            // is one qualifies: a braced arm list's decoded element, or the
            // `Literal` shape this used to be handed. A substituting pattern
            // (the multi-word `switch $s $pat …` form) has no key until run
            // time and keeps the branch chain.
            let (ExprNode::Literal { text: pattern, .. }
            | ExprNode::CompiledWord {
                text: pattern,
                braced: true,
            }) = right.as_ref()
            else {
                break;
            };

            let this_subject = render_expr(left);
            match &subject {
                None => {
                    subject = Some(this_subject.clone());
                    subject_expr = Some((**left).clone());
                }
                Some(s) if *s != this_subject => break,
                _ => {}
            }

            cases.push((pattern.clone(), cfg.block_name(*true_target).to_owned()));

            // Follow false_target to next dispatch block.
            let Some(next_blk) = cfg.blocks.get(false_target) else {
                break;
            };
            dispatch_blocks.push(cfg.block_name(*false_target).to_owned());
            // Dispatch blocks should have no statements.
            if !next_blk.statements.is_empty() {
                break;
            }
            current_term = next_blk.terminator.clone();
        }

        if cases.len() < 2 {
            return false;
        }
        let Some(subject_expr) = subject_expr else {
            return false;
        };

        // The final dispatch block should have a Goto to the default.
        let default_target = match &current_term {
            Some(Terminator::Goto { target, .. }) => cfg.block_name(*target).to_owned(),
            _ => return false,
        };

        // Emit: push subject, jumpTable, jump default.
        self.emit_expr(&subject_expr);

        // Tcl 9.0's jumpTable entries appear in hash-table iteration order.
        let ordered = super::super::helpers::tcl_hash_table_order(&cases);
        let jt: std::collections::HashMap<String, String> = ordered.into_iter().collect();
        let idx = self.emit(Op::JUMP_TABLE, vec![Operand::Imm(0)]);
        self.instructions[idx].jump_table = Some(jt);

        if Some(default_target.as_str()) != next_block {
            self.emit_comment(
                Op::JUMP4,
                vec![Operand::Label(default_target.clone())],
                &format!("-> {default_target}"),
            );
        }

        // Mark intermediate dispatch blocks to skip.
        for db in dispatch_blocks {
            skip_blocks.insert(db);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Function as CfgFunction, Terminator};
    use crate::codegen::CodegenCtx;
    use crate::expr_ast::ExprNode;
    use tcl_registry::CommandRegistry;

    fn lit(text: &str) -> ExprNode {
        ExprNode::Literal {
            text: text.into(),
            start: 0,
            end: text.len() as u32,
        }
    }

    /// A CFG whose interner maps `names` (in order) to ids, so terminator
    /// targets can reference them. The first name is the entry.
    fn cfg_with_blocks(names: &[&str]) -> CfgFunction {
        use crate::cfg::Block;
        let mut cfg = CfgFunction::new("::top", names[0]);
        for name in &names[1..] {
            let id = cfg.intern_block(*name);
            cfg.blocks.insert(id, Block::new(*name));
        }
        cfg
    }

    fn tgt(cfg: &CfgFunction, name: &str) -> crate::cfg::BlockId {
        cfg.block_id(name).expect("interned")
    }

    #[test]
    fn emit_goto_with_fallthrough() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let cfg = cfg_with_blocks(&["entry", "next_0"]);
        let term = Terminator::Goto {
            target: tgt(&cfg, "next_0"),
            span: None,
        };
        ctx.emit_term(&cfg, &term, Some("next_0"));
        // No jump emitted on fallthrough
        assert!(ctx.instructions.is_empty());
    }

    #[test]
    fn emit_goto_without_fallthrough() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let cfg = cfg_with_blocks(&["entry", "far_0"]);
        let term = Terminator::Goto {
            target: tgt(&cfg, "far_0"),
            span: None,
        };
        ctx.emit_term(&cfg, &term, Some("other_0"));
        assert_eq!(ctx.instructions.len(), 1);
        assert_eq!(ctx.instructions[0].op, Op::JUMP4);
    }

    #[test]
    fn emit_branch_const_true() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let cfg = cfg_with_blocks(&["entry", "tt", "ft"]);
        let term = Terminator::Branch {
            condition: lit("1"),
            true_target: tgt(&cfg, "tt"),
            false_target: tgt(&cfg, "ft"),
            span: None,
            condition_base: None,
        };
        ctx.emit_term(&cfg, &term, Some("other"));
        assert_eq!(ctx.instructions[0].op, Op::JUMP4);
        // Jump target should be true branch
        assert!(matches!(
            &ctx.instructions[0].operands[0],
            Operand::Label(s) if s == "tt"
        ));
    }

    #[test]
    fn emit_branch_const_false() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let cfg = cfg_with_blocks(&["entry", "tt", "ft"]);
        let term = Terminator::Branch {
            condition: lit("0"),
            true_target: tgt(&cfg, "tt"),
            false_target: tgt(&cfg, "ft"),
            span: None,
            condition_base: None,
        };
        ctx.emit_term(&cfg, &term, Some("other"));
        assert_eq!(ctx.instructions[0].op, Op::JUMP4);
        assert!(matches!(
            &ctx.instructions[0].operands[0],
            Operand::Label(s) if s == "ft"
        ));
    }

    #[test]
    fn emit_branch_runtime() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let cfg = cfg_with_blocks(&["entry", "tt", "ft"]);
        let cond = ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        };
        let term = Terminator::Branch {
            condition: cond,
            true_target: tgt(&cfg, "tt"),
            false_target: tgt(&cfg, "ft"),
            span: None,
            condition_base: None,
        };
        ctx.emit_term(&cfg, &term, Some("tt")); // true is fallthrough
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::JUMP_FALSE4));
    }

    #[test]
    fn emit_return_proc_emits_done() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let cfg = cfg_with_blocks(&["entry"]);
        let term = Terminator::Return {
            value: Some("hello".into()),
            span: None,
            expr: None,
            braced: false,
        };
        ctx.emit_term(&cfg, &term, None);
        assert_eq!(ctx.instructions.last().map(|i| i.op), Some(Op::DONE));
    }

    #[test]
    fn emit_return_toplevel_emits_return_imm() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let cfg = cfg_with_blocks(&["entry"]);
        let term = Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        };
        ctx.emit_term(&cfg, &term, None);
        let last = ctx.instructions.last().expect("terminator emitted");
        assert_eq!(last.op, Op::RETURN_IMM);
        // A plain `return` outside a proc is C's `returnImm 0 1`
        // (`TclMergeReturnOptions` defaults `-level` to 1). `(0, 0)` would fall
        // through to the next instruction instead of ending the script.
        assert_eq!(last.operands, vec![Operand::Imm(0), Operand::Imm(1)]);
    }

    #[test]
    fn jump_table_not_for_plain_branch() {
        use crate::cfg::Block;
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let cfg = CfgFunction::new("::top", "entry_0");
        let blk = Block::new("entry_0");
        let mut skip = std::collections::HashSet::new();
        assert!(!ctx.try_emit_jump_table(&cfg, &blk, None, &mut skip));
    }

    #[test]
    fn jump_table_emitted_for_str_eq_chain() {
        use crate::cfg::{Block, BlockId, Function as CfgFunction};
        use crate::expr_ast::{BinOp, ExprNode};

        fn str_eq_branch(
            subj_name: &str,
            pattern: &str,
            true_tgt: BlockId,
            false_tgt: BlockId,
        ) -> Terminator {
            Terminator::Branch {
                condition: ExprNode::Binary {
                    op: BinOp::StrEq,
                    left: Box::new(ExprNode::Var {
                        text: format!("${subj_name}"),
                        name: subj_name.into(),
                        start: 0,
                        end: 0,
                    }),
                    right: Box::new(ExprNode::Literal {
                        text: pattern.into(),
                        start: 0,
                        end: pattern.len() as u32,
                    }),
                },
                true_target: true_tgt,
                false_target: false_tgt,
                span: None,
                condition_base: None,
            }
        }

        // Chain: d1 ("a" → arm_a) → d2 ("b" → arm_b) → d3 (goto default).
        // d3 is an intermediate dispatch block with a bare Goto.
        let mut cfg = CfgFunction::new("::top", "entry");
        for name in ["d1", "d2", "d3", "arm_a", "arm_b", "default"] {
            let id = cfg.intern_block(name);
            cfg.blocks.insert(id, Block::new(name));
        }
        let id = |c: &CfgFunction, n: &str| c.block_id(n).unwrap();
        let (d1, d2, d3) = (id(&cfg, "d1"), id(&cfg, "d2"), id(&cfg, "d3"));
        let (arm_a, arm_b, default) = (id(&cfg, "arm_a"), id(&cfg, "arm_b"), id(&cfg, "default"));

        cfg.blocks.get_mut(&d1).unwrap().terminator = Some(str_eq_branch("x", "a", arm_a, d2));
        cfg.blocks.get_mut(&d2).unwrap().terminator = Some(str_eq_branch("x", "b", arm_b, d3));
        cfg.blocks.get_mut(&d3).unwrap().terminator = Some(Terminator::Goto {
            target: default,
            span: None,
        });

        let registry = CommandRegistry::build_default();

        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let mut skip = std::collections::HashSet::new();
        let d1_blk = cfg.blocks[&d1].clone();
        let emitted = ctx.try_emit_jump_table(&cfg, &d1_blk, Some("default"), &mut skip);
        assert!(emitted, "expected jump table emission");
        assert!(skip.contains("d2"), "expected d2 to be skipped");
        assert!(skip.contains("d3"), "expected d3 to be skipped");

        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::JUMP_TABLE));
    }
}
