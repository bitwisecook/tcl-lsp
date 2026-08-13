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

//! Peephole optimisation passes.
//!
//! Post-emission passes that clean up the instruction stream to match
//! tclsh 9.0 output.

use super::statements::{NO_DEDUP_TAG, SC_GENERIC_TAG};
use super::{CodegenCtx, Instruction, Op, Operand};

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
impl CodegenCtx<'_> {
    /// Remove `pop` immediately before the final `done`.
    ///
    /// In tclsh, the last command's result stays on TOS and `done`
    /// returns it.  Our codegen always pops after each statement, so
    /// the final `pop; done` pair should be collapsed to `done`.
    pub fn remove_trailing_pop(&mut self) {
        if self.instructions.len() < 2 {
            return;
        }
        let n = self.instructions.len();
        if self.instructions[n - 1].op != Op::DONE {
            return;
        }
        if self.instructions[n - 2].op != Op::POP {
            return;
        }
        // Don't strip pop after reverse — it's part of catch epilogue.
        if n >= 3 && self.instructions[n - 3].op == Op::REVERSE {
            return;
        }

        let done_old_idx = n - 1;
        let pop_idx = n - 2;
        self.instructions.remove(pop_idx);

        // Labels that pointed at the old done index now point at the
        // new position (shifted down by one).
        let done_new_idx = self.instructions.len() - 1;
        for pos in self.label_positions.values_mut() {
            if *pos == done_old_idx {
                *pos = done_new_idx;
            }
        }
    }

    /// Replace `push1 N; pop` with `nop; nop; nop`.
    ///
    /// tclsh constant-folds `expr` commands that aren't the script's
    /// last statement into 3 nops.  Skip pairs inside a `startCommand`
    /// wrapper — tclsh keeps those so the epoch check spans a real
    /// instruction sequence.
    pub fn fold_const_push_pop_nops(&mut self) {
        let mut i = 0;
        while i + 1 < self.instructions.len() {
            let after_start_cmd = i > 0 && self.instructions[i - 1].op == Op::START_CMD;
            // The empty-string result of a declaration command folds to nops:
            // `unset`/`variable` leave it directly after their op, while
            // `global`/`upvar` leave it after the `pop` that discards the
            // reused namespace/level reference (… nsupvar|upvar ; pop ; push "" ; pop).
            let after_unset = (i > 0
                && matches!(
                    self.instructions[i - 1].op,
                    Op::UNSET_STK | Op::UNSET_SCALAR | Op::UNSET_ARRAY
                ))
                || (i >= 2
                    && self.instructions[i - 1].op == Op::POP
                    && matches!(self.instructions[i - 2].op, Op::UPVAR | Op::NSUPVAR));
            if self.instructions[i].op == Op::PUSH1
                && self.instructions[i + 1].op == Op::POP
                && (self.instructions[i].comment != "\"\"" || after_unset)
                && !self.instructions[i].no_fold
                && !after_start_cmd
            {
                self.instructions[i] = Instruction::new(Op::NOP, vec![]);
                self.instructions[i + 1] = Instruction::new(Op::NOP, vec![]);
                self.instructions
                    .insert(i + 2, Instruction::new(Op::NOP, vec![]));
                // Shift labels past the insertion point.
                for pos in self.label_positions.values_mut() {
                    if *pos > i + 1 {
                        *pos += 1;
                    }
                }
                i += 3;
            } else {
                i += 1;
            }
        }
    }

    /// Re-dedup push operands whose earliest slot was nop-ed.
    ///
    /// After `fold_const_push_pop_nops` converts dead `push; pop`
    /// pairs to nops, surviving push instructions may reference later
    /// literal slots that duplicate an earlier nop-ed slot.  Patch
    /// surviving pushes to reuse the earliest occurrence only if that
    /// slot is no longer referenced by any other surviving push.
    pub fn dedup_push_literals(&mut self) {
        let entries = self.literals.entries().to_vec();

        // Collect literal indices still referenced by surviving pushes.
        let mut live_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for instr in &self.instructions {
            if matches!(instr.op, Op::PUSH1 | Op::PUSH4)
                && let Some(&Operand::Imm(idx)) = instr.operands.first()
            {
                let idx = idx as usize;
                if idx < entries.len() {
                    live_indices.insert(idx);
                }
            }
        }

        // Build first-occurrence map.
        let mut first: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for (idx, val) in entries.iter().enumerate() {
            first.entry(val.as_str()).or_insert(idx);
        }

        // Patch surviving pushes to use the earliest slot, but only
        // when that earliest slot is NOT live.
        for i in 0..self.instructions.len() {
            if !matches!(self.instructions[i].op, Op::PUSH1 | Op::PUSH4) {
                continue;
            }
            // Skip no-dedup pushes
            if self.instructions[i].comment.contains(NO_DEDUP_TAG) {
                continue;
            }
            if let Some(&Operand::Imm(lit_idx)) = self.instructions[i].operands.first() {
                let lit_idx = lit_idx as usize;
                if lit_idx < entries.len() {
                    let earliest = first[entries[lit_idx].as_str()];
                    if earliest != lit_idx && !live_indices.contains(&earliest) {
                        let mut new_operands = self.instructions[i].operands.clone();
                        new_operands[0] = Operand::Imm(earliest as i32);
                        let mut new_instr = Instruction::new(self.instructions[i].op, new_operands);
                        new_instr.comment.clone_from(&self.instructions[i].comment);
                        self.instructions[i] = new_instr;
                        live_indices.remove(&lit_idx);
                        live_indices.insert(earliest);
                    }
                }
            }
        }
    }

    /// Replace a final plain `returnImm 0 1` with `done`.
    ///
    /// tclsh uses `done` for the proc body's last return, not `returnImm`
    /// (`TclCompileReturnCmd`'s `INST_DONE` optimisation — in fact C folds *any*
    /// plain `return` in a proc with no enclosing `catch`, not just the tail
    /// one). Only applies to proc bodies.
    ///
    /// The key is the plain-return encoding `(code 0, level 1)`: C's default
    /// `-level` is 1, so `(0, 0)` is not a plain return at all — it is
    /// "push the result and fall through".
    pub fn fold_tail_return_to_done(&mut self) {
        if !self.is_proc || self.instructions.is_empty() {
            return;
        }
        let last = self.instructions.last().unwrap();
        if last.op == Op::RETURN_IMM && last.operands == [Operand::Imm(0), Operand::Imm(1)] {
            let n = self.instructions.len();
            self.instructions[n - 1] = Instruction::new(Op::DONE, vec![]);
        }
    }

    /// Strip all `startCommand` when no generic invoke exists.
    ///
    /// tclsh 9.0 only emits `startCommand` in top-level scripts where
    /// generic invokes are present.  In proc bodies, `startCommand` is
    /// always kept.
    pub fn strip_unused_start_cmd(&mut self) {
        if self.is_proc {
            return;
        }
        let generic_ops = [
            Op::INVOKE_STK1,
            Op::INVOKE_STK4,
            Op::INVOKE_EXPANDED,
            Op::INVOKE_REPLACE,
        ];
        let replaced_ops = [Op::UPVAR, Op::NSUPVAR];

        // A `returnImm` that is *not* the plain `(code 0, level 1)` return stands
        // in for a non-trivial `return`/`error`/`syntax` (which tclsh reaches via
        // a generic invoke). Keyed on the plain-return encoding, not `(0, 0)`:
        // C's default `-level` is 1.
        let has_generic = self.instructions.iter().any(|i| {
            generic_ops.contains(&i.op)
                || replaced_ops.contains(&i.op)
                || (i.op == Op::RETURN_IMM && i.operands != [Operand::Imm(0), Operand::Imm(1)])
        });

        if has_generic {
            return;
        }

        // No generic invokes — strip all startCommand instructions.
        let mut i = 0;
        while i < self.instructions.len() {
            if self.instructions[i].op == Op::START_CMD {
                self.instructions.remove(i);
                for pos in self.label_positions.values_mut() {
                    if *pos > i {
                        *pos -= 1;
                    }
                }
            } else {
                i += 1;
            }
        }
    }

    /// Remove `startCommand` for generic invoke commands in top-level.
    ///
    /// In top-level compilation, Tcl 9.0 only wraps *compiled* commands
    /// with `startCommand`.  Commands that fall through to `invokeStk`
    /// never get `startCommand`.
    pub fn fixup_top_level_start_cmd(&mut self) {
        if self.is_proc {
            return;
        }

        // Collect indices of all generic-tagged startCommand instructions.
        let to_remove: Vec<usize> = self
            .instructions
            .iter()
            .enumerate()
            .filter(|(_, instr)| instr.op == Op::START_CMD && instr.comment == SC_GENERIC_TAG)
            .map(|(i, _)| i)
            .collect();

        if to_remove.is_empty() {
            return;
        }

        // Remove in reverse order so earlier indices stay valid.
        for &idx in to_remove.iter().rev() {
            self.instructions.remove(idx);
            for pos in self.label_positions.values_mut() {
                if *pos > idx {
                    *pos -= 1;
                }
            }
        }
    }

    /// Strip internal no-dedup tags from comments.
    pub fn strip_nodedup_tags(&mut self) {
        for instr in &mut self.instructions {
            if instr.comment.contains(NO_DEDUP_TAG) {
                instr.comment = instr.comment.replace(NO_DEDUP_TAG, "");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodegenCtx;
    use tcl_registry::CommandRegistry;

    #[test]
    fn remove_trailing_pop_basic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.push_lit("hello");
        ctx.emit(Op::POP, vec![]);
        ctx.emit(Op::DONE, vec![]);
        assert_eq!(ctx.instructions.len(), 3);
        ctx.remove_trailing_pop();
        assert_eq!(ctx.instructions.len(), 2);
        assert_eq!(ctx.instructions[1].op, Op::DONE);
    }

    #[test]
    fn remove_trailing_pop_preserves_catch() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        ctx.emit(Op::POP, vec![]);
        ctx.emit(Op::DONE, vec![]);
        ctx.remove_trailing_pop();
        // Should NOT remove pop after reverse
        assert_eq!(ctx.instructions.len(), 3);
    }

    // The fold is keyed on the *plain-return* encoding, which C emits as
    // `(code 0, level 1)` (`TclMergeReturnOptions` defaults `-level` to 1).
    // `fold_tail_return_to_done_proc` and `fold_tail_return_toplevel_noop`
    // previously used `(0, 0)`, which encoded the old compensating-VM bug: under
    // C semantics `(0, 0)` is a fall-through, not a return, so it must never be
    // what the `done` fold matches.
    #[test]
    fn fold_tail_return_to_done_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.push_lit("");
        ctx.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(1)]);
        ctx.fold_tail_return_to_done();
        assert_eq!(ctx.instructions.last().unwrap().op, Op::DONE);
    }

    /// `error msg` compiles to `returnImm 1 0` (`TclCompileErrorCmd`), which is
    /// not a plain return and must survive the fold.
    #[test]
    fn fold_tail_return_non_zero_preserved() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.push_lit("");
        ctx.emit(Op::RETURN_IMM, vec![Operand::Imm(1), Operand::Imm(0)]);
        ctx.fold_tail_return_to_done();
        assert_eq!(ctx.instructions.last().unwrap().op, Op::RETURN_IMM);
    }

    /// A `(0, 0)` pair is *not* a plain return — C only elides it because the
    /// value push alone is equivalent — so the fold must leave it alone even in
    /// a proc body, where a `done` would wrongly consume the options dict.
    #[test]
    fn fold_tail_return_level_zero_preserved() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.push_lit("");
        ctx.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(0)]);
        ctx.fold_tail_return_to_done();
        assert_eq!(ctx.instructions.last().unwrap().op, Op::RETURN_IMM);
    }

    #[test]
    fn fold_tail_return_toplevel_noop() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry); // not proc
        ctx.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(1)]);
        ctx.fold_tail_return_to_done();
        // Top-level — not changed
        assert_eq!(ctx.instructions.last().unwrap().op, Op::RETURN_IMM);
    }

    #[test]
    fn strip_unused_start_cmd_no_generic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit(
            Op::START_CMD,
            vec![Operand::Label("end_0".into()), Operand::Imm(1)],
        );
        ctx.push_lit("42");
        ctx.emit(Op::DONE, vec![]);
        assert_eq!(ctx.instructions.len(), 3);
        ctx.strip_unused_start_cmd();
        // startCommand removed since no generic invoke
        assert_eq!(ctx.instructions.len(), 2);
        assert!(!ctx.instructions.iter().any(|i| i.op == Op::START_CMD));
    }

    #[test]
    fn strip_unused_start_cmd_with_generic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit(
            Op::START_CMD,
            vec![Operand::Label("end_0".into()), Operand::Imm(1)],
        );
        ctx.push_lit("cmd");
        ctx.emit(Op::INVOKE_STK1, vec![Operand::Imm(1)]);
        ctx.emit(Op::DONE, vec![]);
        let n = ctx.instructions.len();
        ctx.strip_unused_start_cmd();
        // Should keep startCommand since generic invoke exists
        assert_eq!(ctx.instructions.len(), n);
    }

    /// The `returnImm` proxy is keyed on the plain-return pair: a top-level
    /// plain `return` is `(0, 1)` and must NOT read as a generic invoke, or
    /// `startCommand` stops being stripped and the emitted bytes change.
    #[test]
    fn strip_unused_start_cmd_plain_return_is_not_generic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit(
            Op::START_CMD,
            vec![Operand::Label("end_0".into()), Operand::Imm(1)],
        );
        ctx.push_lit("");
        ctx.push_lit("");
        ctx.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(1)]);
        ctx.strip_unused_start_cmd();
        assert!(!ctx.instructions.iter().any(|i| i.op == Op::START_CMD));
    }

    /// A non-plain `returnImm` (here `error msg`'s `(1, 0)`) still stands in for
    /// a generic invoke, so `startCommand` is kept.
    #[test]
    fn strip_unused_start_cmd_non_plain_return_is_generic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit(
            Op::START_CMD,
            vec![Operand::Label("end_0".into()), Operand::Imm(1)],
        );
        ctx.push_lit("boom");
        ctx.push_lit("");
        ctx.emit(Op::RETURN_IMM, vec![Operand::Imm(1), Operand::Imm(0)]);
        ctx.strip_unused_start_cmd();
        assert!(ctx.instructions.iter().any(|i| i.op == Op::START_CMD));
    }

    #[test]
    fn strip_unused_start_cmd_proc_always_keeps() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry); // proc
        ctx.emit(
            Op::START_CMD,
            vec![Operand::Label("end_0".into()), Operand::Imm(1)],
        );
        ctx.push_lit("42");
        ctx.emit(Op::DONE, vec![]);
        let n = ctx.instructions.len();
        ctx.strip_unused_start_cmd();
        // Proc always keeps startCommand
        assert_eq!(ctx.instructions.len(), n);
    }

    #[test]
    fn fixup_top_level_removes_generic_tagged() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        // Tagged as generic
        let mut sc = Instruction::new(
            Op::START_CMD,
            vec![Operand::Label("end_0".into()), Operand::Imm(1)],
        );
        sc.comment = SC_GENERIC_TAG.to_owned();
        ctx.instructions.push(sc);
        ctx.push_lit("cmd");
        ctx.emit(Op::INVOKE_STK1, vec![Operand::Imm(1)]);
        ctx.emit(Op::DONE, vec![]);
        let n = ctx.instructions.len();
        ctx.fixup_top_level_start_cmd();
        assert_eq!(ctx.instructions.len(), n - 1);
    }

    #[test]
    fn fold_const_push_pop_nops_basic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.push_lit("42");
        ctx.emit(Op::POP, vec![]);
        ctx.push_lit("result");
        ctx.emit(Op::DONE, vec![]);
        ctx.fold_const_push_pop_nops();
        // First push+pop → 3 nops
        assert_eq!(ctx.instructions[0].op, Op::NOP);
        assert_eq!(ctx.instructions[1].op, Op::NOP);
        assert_eq!(ctx.instructions[2].op, Op::NOP);
        // Rest unchanged
        assert_eq!(ctx.instructions[3].op, Op::PUSH1);
    }

    #[test]
    fn strip_nodedup_tags_basic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.push_lit_no_dedup("x");
        assert!(ctx.instructions[0].comment.contains(NO_DEDUP_TAG));
        ctx.strip_nodedup_tags();
        assert!(!ctx.instructions[0].comment.contains(NO_DEDUP_TAG));
    }
}
