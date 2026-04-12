//! Main `generate()` dispatcher.
//!
//! Walks the CFG in block-order (from [`ordering::linearise`]) and
//! delegates per-block work to handlers. Runs peephole passes and
//! the layout pass to produce the final [`FunctionAsm`].

#![allow(
    clippy::too_many_lines,
    clippy::if_not_else,
    dead_code
)]

use std::collections::{HashMap, HashSet};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ir::Procedure as IrProcedure;

use super::super::layout::{optimise_jumps, resolve_layout};
use super::super::{FunctionAsm, Op};
use super::super::CodegenCtx;
use super::ordering::{
    self, linearise, starts_with_any, LOOP_BODY_PREFIXES, LOOP_END_PREFIXES,
    VALUE_JOIN_PREFIXES,
};
use super::proc_defs::is_static_proc;
use super::try_blocks::{detect_try_finally, TryFinallyInfo};

/// Transient state passed between the per-block handlers in `generate`.
struct GenerateState {
    /// Pending proc definitions, sorted by source offset.
    pending_proc_defs: Vec<IrProcedure>,
    /// Block names to skip — consumed by jump tables or try/finally.
    skip_blocks: HashSet<String>,
    /// Detected try/finally chains keyed by `try_body` block.
    try_finally_info: HashMap<String, TryFinallyInfo>,
}

impl GenerateState {
    fn new(proc_defs: &[IrProcedure]) -> Self {
        let mut pending: Vec<IrProcedure> = proc_defs
            .iter()
            .filter(|p| is_static_proc(p))
            .cloned()
            .collect();
        pending.sort_by_key(|p| p.span.start());
        Self {
            pending_proc_defs: pending,
            skip_blocks: HashSet::new(),
            try_finally_info: HashMap::new(),
        }
    }
}

/// Generate bytecode for one CFG function.
///
/// MVP implementation: handles straight-line code, if/else, simple
/// loops (via generic invokeStk), and try/finally CFG patterns.
/// Foreach opcode compilation, switch jump tables, bottom-tested
/// loop reordering, and complex-foreach bodies are deferred to
/// follow-up chunks.
pub fn generate(
    ctx: &mut CodegenCtx,
    cfg: &CfgFunction,
    proc_defs: &[IrProcedure],
) -> FunctionAsm {
    let block_order = linearise(cfg);
    let loop_ctx = ordering::build_loop_context(cfg);
    let mut state = GenerateState::new(proc_defs);
    state.try_finally_info = detect_try_finally(cfg, &block_order);
    // Mark try_end/try_finally/try_after_finally as skipped — they
    // are emitted as part of the try_body block's inline sequence.
    for info in state.try_finally_info.values() {
        state.skip_blocks.insert(info.try_end.clone());
        state.skip_blocks.insert(info.try_finally.clone());
        state.skip_blocks.insert(info.try_after.clone());
    }

    // Emit proc defs that appear before the first statement (for cases
    // where the entry block has no statements).
    if !state.pending_proc_defs.is_empty() {
        let first_stmt_offset: Option<u32> = cfg
            .blocks
            .values()
            .flat_map(|blk| blk.statements.iter())
            .map(|s| s.span().start())
            .min();
        if let Some(off) = first_stmt_offset {
            ctx.emit_pending_proc_defs(&mut state.pending_proc_defs, off);
        }
    }

    for (i, bname) in block_order.iter().enumerate() {
        if state.skip_blocks.contains(bname) {
            ctx.place_label(bname);
            continue;
        }
        let blk = &cfg.blocks[bname];
        ctx.place_label(bname);

        // try/finally inline compilation at try_body block.
        if let Some(info) = state.try_finally_info.get(bname).cloned() {
            ctx.emit_try_finally_inline(cfg, bname, &info.try_finally);
            continue;
        }

        // Update loop context for break/continue compilation.
        if let Some((cont, brk)) = loop_ctx.get(bname) {
            ctx.continue_target = Some(cont.clone());
            ctx.break_target = Some(brk.clone());
        }

        // Loop-end blocks push "" as the loop command's result.
        let is_loop_end = starts_with_any(bname, LOOP_END_PREFIXES);
        if is_loop_end && blk.statements.is_empty() {
            let target = match &blk.terminator {
                Some(Terminator::Goto { target, .. }) => Some(target.clone()),
                _ => None,
            };
            if let Some(t) = &target {
                if !t.starts_with("exit_") {
                    ctx.literals.intern("");
                    ctx.emit(Op::NOP, vec![]);
                    ctx.emit(Op::NOP, vec![]);
                    ctx.emit(Op::NOP, vec![]);
                } else {
                    ctx.push_lit("");
                }
            } else {
                ctx.push_lit("");
            }
        }

        // Empty loop body: emit 3 nops.
        if starts_with_any(bname, LOOP_BODY_PREFIXES)
            && blk.statements.is_empty()
            && matches!(blk.terminator, Some(Terminator::Goto { .. }))
        {
            ctx.literals.intern("");
            ctx.emit(Op::NOP, vec![]);
            ctx.emit(Op::NOP, vec![]);
            ctx.emit(Op::NOP, vec![]);
        }

        // Pop incoming arm value at join blocks with work.
        if starts_with_any(bname, VALUE_JOIN_PREFIXES) && block_has_work(blk, ctx.is_proc) {
            ctx.emit(Op::POP, vec![]);
        }

        // Emit statements.
        for stmt in &blk.statements {
            ctx.emit_pending_proc_defs(&mut state.pending_proc_defs, stmt.span().start());
            ctx.emit_stmt_with_start_cmd(stmt, None, None);
        }

        // If/switch arms: keep last statement value on TOS instead of
        // popping — the value is the arm's result.
        if let Some(Terminator::Goto { target, .. }) = &blk.terminator {
            if starts_with_any(target, VALUE_JOIN_PREFIXES) {
                if !blk.statements.is_empty()
                    && ctx.instructions.last().is_some_and(|i| i.op == Op::POP)
                {
                    ctx.instructions.pop();
                } else if blk.statements.is_empty() && !is_loop_end {
                    // Else-less if: false path needs an empty-string result.
                    ctx.push_lit("");
                }
            }
        }

        let next_block = block_order.get(i + 1).map(String::as_str);
        if let Some(term) = &blk.terminator {
            if ctx.is_proc && matches!(term, Terminator::Return { .. }) {
                ctx.emit_proc_return(term, bname, next_block, &block_order, i, cfg);
            } else {
                ctx.emit_term(term, next_block);
            }
        } else if next_block.is_some() {
            // Terminal block not last in layout — emit done to prevent
            // fallthrough into successor blocks.
            ctx.emit(Op::DONE, vec![]);
        }
    }

    // Flush any remaining proc defs.
    ctx.flush_proc_defs(&mut state.pending_proc_defs);

    // Empty proc body: push "" as the implicit return value.
    if ctx.is_proc && ctx.instructions.is_empty() {
        ctx.push_lit("");
    }

    // Proc bodies with control flow: trailing done as proc-level exit.
    let has_branches = cfg
        .blocks
        .values()
        .any(|b| matches!(b.terminator, Some(Terminator::Branch { .. })));
    if ctx.is_proc && has_branches {
        if let Some(lbl) = ctx.proc_exit_label.clone() {
            ctx.place_label(&lbl);
        }
        ctx.emit(Op::DONE, vec![]);
    } else if !ctx
        .instructions
        .last()
        .is_some_and(|i| matches!(i.op, Op::DONE | Op::RETURN_IMM))
    {
        ctx.emit(Op::DONE, vec![]);
    }

    // Peephole passes.
    ctx.remove_trailing_pop();
    ctx.fold_tail_return_to_done();
    ctx.strip_unused_start_cmd();
    ctx.fixup_top_level_start_cmd();
    ctx.fold_const_push_pop_nops();
    ctx.dedup_push_literals();
    ctx.strip_nodedup_tags();

    // Layout pass.
    optimise_jumps(&mut ctx.instructions, &ctx.label_positions, 10);
    let labels = resolve_layout(&mut ctx.instructions, &ctx.label_positions);
    FunctionAsm {
        name: cfg.name.clone(),
        literals: std::mem::take(&mut ctx.literals),
        lvt: std::mem::take(&mut ctx.lvt),
        instructions: std::mem::take(&mut ctx.instructions),
        labels,
    }
}

/// Return `true` if the join block has work beyond a single fallthrough.
fn block_has_work(blk: &crate::cfg::Block, is_proc: bool) -> bool {
    if !blk.statements.is_empty() {
        return true;
    }
    match &blk.terminator {
        Some(Terminator::Branch { .. }) => true,
        Some(Terminator::Return { .. }) => is_proc,
        Some(Terminator::Goto { target, .. }) => !target.starts_with("exit_"),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Function as CfgFunction, Terminator};
    use crate::codegen::CodegenCtx;
    use crate::ir::{Statement};
    use tcl_lexer::Span;

    fn sp() -> Span {
        Span::new(0, 0)
    }

    fn trivial_cfg() -> CfgFunction {
        let mut cfg = CfgFunction::new("::top", "entry_0");
        cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        cfg
    }

    #[test]
    fn generate_empty_toplevel_terminates() {
        let cfg = trivial_cfg();
        let mut ctx = CodegenCtx::new(false, &[]);
        let asm = generate(&mut ctx, &cfg, &[]);
        assert_eq!(asm.name, "::top");
        assert!(!asm.instructions.is_empty());
        // Top-level scripts terminate with RETURN_IMM or DONE
        let last = asm.instructions.last().unwrap().op;
        assert!(
            matches!(last, Op::DONE | Op::RETURN_IMM),
            "expected DONE or RETURN_IMM, got {last:?}"
        );
    }

    #[test]
    fn generate_simple_set() {
        let mut cfg = CfgFunction::new("::top", "entry_0");
        cfg.blocks
            .get_mut("entry_0")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: sp(),
                name: "x".into(),
                value: "42".into(),
            });
        cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let mut ctx = CodegenCtx::new(false, &[]);
        let asm = generate(&mut ctx, &cfg, &[]);
        // Should contain push and store (tail pop is folded away)
        let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::PUSH1));
        assert!(ops.contains(&Op::STORE_STK));
    }

    #[test]
    fn generate_proc_empty_body() {
        let cfg = CfgFunction::new("::foo", "entry_0");
        let mut ctx = CodegenCtx::new(true, &[]);
        // Give the entry block a return terminator
        let mut cfg = cfg;
        cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let asm = generate(&mut ctx, &cfg, &[]);
        // Proc with empty body should push "" and emit done
        assert_eq!(asm.instructions.last().unwrap().op, Op::DONE);
    }

    #[test]
    fn generate_if_diamond() {
        use crate::expr_ast::ExprNode;
        let mut cfg = CfgFunction::new("::top", "entry_0");
        cfg.blocks.insert("if_then_1".into(), Block::new("if_then_1"));
        cfg.blocks.insert("if_else_1".into(), Block::new("if_else_1"));
        cfg.blocks.insert("if_end_1".into(), Block::new("if_end_1"));

        cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Branch {
            condition: ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            },
            true_target: "if_then_1".into(),
            false_target: "if_else_1".into(),
            span: None,
        });
        cfg.blocks
            .get_mut("if_then_1")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: sp(),
                name: "r".into(),
                value: "1".into(),
            });
        cfg.blocks.get_mut("if_then_1").unwrap().terminator =
            Some(Terminator::Goto {
                target: "if_end_1".into(),
                span: None,
            });
        cfg.blocks
            .get_mut("if_else_1")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: sp(),
                name: "r".into(),
                value: "2".into(),
            });
        cfg.blocks.get_mut("if_else_1").unwrap().terminator =
            Some(Terminator::Goto {
                target: "if_end_1".into(),
                span: None,
            });
        cfg.blocks.get_mut("if_end_1").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ctx = CodegenCtx::new(false, &[]);
        let asm = generate(&mut ctx, &cfg, &[]);
        let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
        // Should include a conditional jump (maybe shrunk to 1-byte)
        assert!(
            ops.contains(&Op::JUMP_FALSE4) || ops.contains(&Op::JUMP_FALSE1),
            "expected conditional jump, got {ops:?}"
        );
    }
}
