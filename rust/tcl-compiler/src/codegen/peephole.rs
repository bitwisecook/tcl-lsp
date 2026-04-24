//! Peephole optimisation passes.
//!
//! Post-emission passes that clean up the instruction stream to match
//! tclsh 9.0 output.  Ported from `core/compiler/codegen/_peephole.py`.

use super::statements::{NO_DEDUP_TAG, SC_GENERIC_TAG};
use super::{CodegenCtx, Instruction, Op, Operand};

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
impl CodegenCtx {
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
            let after_unset = i > 0 && self.instructions[i - 1].op == Op::UNSET_STK;
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
            if matches!(instr.op, Op::PUSH1 | Op::PUSH4) {
                if let Some(&Operand::Imm(idx)) = instr.operands.first() {
                    let idx = idx as usize;
                    if idx < entries.len() {
                        live_indices.insert(idx);
                    }
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

    /// Replace a final `returnImm 0 0` with `done`.
    ///
    /// tclsh uses `done` for the proc body's last return, not
    /// `returnImm`.  Only applies to proc bodies.
    pub fn fold_tail_return_to_done(&mut self) {
        if !self.is_proc || self.instructions.is_empty() {
            return;
        }
        let last = self.instructions.last().unwrap();
        if last.op == Op::RETURN_IMM && last.operands == [Operand::Imm(0), Operand::Imm(0)] {
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

        let has_generic = self.instructions.iter().any(|i| {
            generic_ops.contains(&i.op)
                || replaced_ops.contains(&i.op)
                || (i.op == Op::RETURN_IMM && i.operands != [Operand::Imm(0), Operand::Imm(0)])
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

    #[test]
    fn remove_trailing_pop_basic() {
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        ctx.emit(Op::POP, vec![]);
        ctx.emit(Op::DONE, vec![]);
        ctx.remove_trailing_pop();
        // Should NOT remove pop after reverse
        assert_eq!(ctx.instructions.len(), 3);
    }

    #[test]
    fn fold_tail_return_to_done_proc() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.push_lit("");
        ctx.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(0)]);
        ctx.fold_tail_return_to_done();
        assert_eq!(ctx.instructions.last().unwrap().op, Op::DONE);
    }

    #[test]
    fn fold_tail_return_non_zero_preserved() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.push_lit("");
        ctx.emit(Op::RETURN_IMM, vec![Operand::Imm(1), Operand::Imm(0)]);
        ctx.fold_tail_return_to_done();
        // Non-0,0 should be preserved
        assert_eq!(ctx.instructions.last().unwrap().op, Op::RETURN_IMM);
    }

    #[test]
    fn fold_tail_return_toplevel_noop() {
        let mut ctx = CodegenCtx::new(false, &[]); // not proc
        ctx.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(0)]);
        ctx.fold_tail_return_to_done();
        // Top-level — not changed
        assert_eq!(ctx.instructions.last().unwrap().op, Op::RETURN_IMM);
    }

    #[test]
    fn strip_unused_start_cmd_no_generic() {
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
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

    #[test]
    fn strip_unused_start_cmd_proc_always_keeps() {
        let mut ctx = CodegenCtx::new(true, &[]); // proc
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
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.push_lit_no_dedup("x");
        assert!(ctx.instructions[0].comment.contains(NO_DEDUP_TAG));
        ctx.strip_nodedup_tags();
        assert!(!ctx.instructions[0].comment.contains(NO_DEDUP_TAG));
    }
}
