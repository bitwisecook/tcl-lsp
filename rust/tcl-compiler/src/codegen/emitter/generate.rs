//! Main `generate()` dispatcher.
//!
//! Walks the CFG in block-order (from [`ordering::linearise`]) and
//! delegates per-block work to handlers. Runs peephole passes and
//! the layout pass to produce the final [`FunctionAsm`].

#![allow(clippy::too_many_lines, clippy::if_not_else, dead_code)]

use std::collections::{HashMap, HashSet, VecDeque};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ir::Procedure as IrProcedure;
use crate::ir::Statement;

use super::super::layout::{optimise_jumps, resolve_layout};
use super::super::CodegenCtx;
use super::super::{FunctionAsm, Op, Operand};
use super::ordering::{
    self, linearise, starts_with_any, LOOP_BODY_PREFIXES, LOOP_END_PREFIXES, VALUE_JOIN_PREFIXES,
};
use super::proc_defs::is_static_proc;
use super::try_blocks::{detect_try_finally, TryFinallyInfo};

/// Transient state passed between the per-block handlers in `generate`.
struct GenerateState {
    /// Pending proc definitions, sorted by source offset. A
    /// `VecDeque` keeps `pop_front` O(1) during interleaved drain.
    pending_proc_defs: VecDeque<IrProcedure>,
    /// Block names to skip — consumed by jump tables or try/finally.
    skip_blocks: HashSet<String>,
    /// Detected try/finally chains keyed by `try_body` block.
    try_finally_info: HashMap<String, TryFinallyInfo>,
    /// While-loop startCommand end labels: `while_end_N` → deferred
    /// label to place at the end block.
    while_end_labels: HashMap<String, String>,
    /// For-init startCommand end labels: `for_end_N` → deferred label.
    for_init_end_labels: HashMap<String, String>,
    /// Foreach startCommand end labels: `foreach_end_N` → deferred
    /// label placed at the `foreach_end` block's trailing pop (C18).
    foreach_end_labels: HashMap<String, String>,
    /// For-body startCommand end labels: `if_end_N` → deferred label
    /// placed at the join block's trailing pop (C18).
    for_body_end_labels: HashMap<String, String>,
    /// Pending startCommand end labels for if/switch join pops — each
    /// label is placed *before* the join pop once it is emitted (C18).
    pending_join_labels: HashMap<String, String>,
}

impl GenerateState {
    fn new(proc_defs: &[IrProcedure]) -> Self {
        let mut sorted: Vec<IrProcedure> = proc_defs
            .iter()
            .filter(|p| is_static_proc(p))
            .cloned()
            .collect();
        sorted.sort_by_key(|p| p.span.start());
        Self {
            pending_proc_defs: VecDeque::from(sorted),
            skip_blocks: HashSet::new(),
            try_finally_info: HashMap::new(),
            while_end_labels: HashMap::new(),
            for_init_end_labels: HashMap::new(),
            foreach_end_labels: HashMap::new(),
            for_body_end_labels: HashMap::new(),
            pending_join_labels: HashMap::new(),
        }
    }
}

/// Generate bytecode for one CFG function.
///
/// Handles: straight-line code, if/else, switch jump tables, proc
/// returns with dead-code jumps, foreach loops (simple and
/// complex-body variants) with native `foreach_start`/`step`/`end`
/// opcodes, for-init / while `startCommand` wrapping with deferred
/// end labels at the loop-end pop, try/finally CFG patterns,
/// bottom-tested loop layout (via `ordering::reorder_bottom_tested`).
///
/// Still deferred to follow-up chunks (see C18–C21 in `rust-rewrite.md`):
/// foreach/for-body/complex-foreach startCommand placement nuances
/// for tclsh byte-for-byte matching, value-emission extras
/// (`try_list_expand_concat`, inline list with `break`/`continue`,
/// `try_format_fold`), differential test harness, and registry-backed
/// codegen hooks.
pub fn generate(ctx: &mut CodegenCtx, cfg: &CfgFunction, proc_defs: &[IrProcedure]) -> FunctionAsm {
    use super::loop_blocks::{detect_complex_foreach, detect_foreach, ComplexForeach, ForeachInfo};

    let block_order = linearise(cfg);
    let mut loop_ctx = ordering::build_loop_context(cfg);
    let mut state = GenerateState::new(proc_defs);
    state.try_finally_info = detect_try_finally(cfg, &block_order);

    // Detect foreach loops; bodies are marked so we can append
    // foreach_step/foreach_end after their statements.
    let foreach_info: HashMap<String, ForeachInfo> = detect_foreach(cfg);
    let foreach_bodies: HashSet<String> = foreach_info.values().map(|i| i.body.clone()).collect();

    // Detect complex foreach loops (bodies with Branch terminator).
    // For these, foreach_step/foreach_end are emitted at the foreach_end
    // block (bottom-tested), and continue/break are rerouted through
    // synthetic step/break labels. Allocate labels directly on ctx so
    // they share the main label-counter pool.
    let mut tmp_counter: u32 = 0;
    let complex_foreach: HashMap<String, ComplexForeach> =
        detect_complex_foreach(cfg, &foreach_info, &mut tmp_counter)
            .into_iter()
            .map(|(hdr, mut info)| {
                info.step_label = ctx.fresh_label("foreach_continue");
                info.break_label = ctx.fresh_label("foreach_break");
                (hdr, info)
            })
            .collect();
    let foreach_end_to_header: HashMap<String, String> = complex_foreach
        .iter()
        .map(|(h, ci)| (ci.end.clone(), h.clone()))
        .collect();
    // Redirect loop context: body blocks of a complex foreach route
    // continue → step_label, break → break_label.
    let mut complex_body_blocks: HashSet<String> = HashSet::new();
    for (hdr, info) in &complex_foreach {
        for bb in &info.body_blocks {
            if let Some((cont, _brk)) = loop_ctx.get(bb) {
                if cont == hdr {
                    loop_ctx.insert(
                        bb.clone(),
                        (info.step_label.clone(), info.break_label.clone()),
                    );
                    complex_body_blocks.insert(bb.clone());
                }
            }
        }
    }
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

        // Complex foreach: emit foreach_step + foreach_end at the
        // bottom of the loop body, before the loop-result push/pop.
        if let Some(header) = foreach_end_to_header.get(bname) {
            if let Some(info) = complex_foreach.get(header) {
                ctx.place_label(&info.step_label);
                ctx.emit(Op::FOREACH_STEP, vec![]);
                ctx.place_label(&info.break_label);
                ctx.emit(Op::FOREACH_END, vec![]);
            }
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

        // Place deferred for-init / while-loop / foreach startCommand
        // end labels at the loop-end block's pop (C17 + C18).
        if let Some(lbl) = state.for_init_end_labels.remove(bname) {
            place_label_before_trailing_pop(ctx, &lbl);
        }
        if let Some(lbl) = state.while_end_labels.remove(bname) {
            place_label_before_trailing_pop(ctx, &lbl);
        }
        if let Some(lbl) = state.foreach_end_labels.remove(bname) {
            place_label_before_trailing_pop(ctx, &lbl);
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

        // For-body startCommand (C18 case 2): a `for_body_*` block
        // whose terminator is a Branch (the first statement is an
        // `if`) gets a startCommand that spans from here to the
        // `if_end_*` join pop. The end label is deferred into
        // `for_body_end_labels` keyed by the join block name.
        if bname.starts_with("for_body_")
            && matches!(blk.terminator, Some(Terminator::Branch { .. }))
            && ctx.cmd_index > 0
        {
            let fb_end_label = ctx.fresh_label("for_body_end");
            ctx.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(fb_end_label.clone()), Operand::Imm(1)],
                "",
            );
            // Find the convergence point by following the true branch.
            if let Some(Terminator::Branch { true_target, .. }) = &blk.terminator {
                if let Some(tt_blk) = cfg.blocks.get(true_target) {
                    if let Some(Terminator::Goto { target: join, .. }) = &tt_blk.terminator {
                        if join.starts_with("if_end_") {
                            state.for_body_end_labels.insert(join.clone(), fb_end_label);
                        }
                    }
                }
            }
        }

        // Complex-foreach if-condition startCommand (C18 case 3):
        // body blocks inside a complex foreach whose terminator is a
        // Branch (the first element is an `if`) also get a per-body
        // startCommand, with the end label deferred to the `if_end_*`
        // or `if_next_*` join pop.
        if complex_body_blocks.contains(bname)
            && matches!(blk.terminator, Some(Terminator::Branch { .. }))
            && ctx.cmd_index > 0
            && !bname.starts_with("foreach_header_")
        {
            let fif_end_label = ctx.fresh_label("foreach_if_end");
            ctx.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(fif_end_label.clone()), Operand::Imm(1)],
                "",
            );
            ctx.seen_generic_invoke = true;
            if let Some(Terminator::Branch { true_target, .. }) = &blk.terminator {
                if let Some(tt_blk) = cfg.blocks.get(true_target) {
                    if let Some(Terminator::Goto { target: join, .. }) = &tt_blk.terminator {
                        if join.starts_with("if_end_") || join.starts_with("if_next_") {
                            state
                                .for_body_end_labels
                                .insert(join.clone(), fif_end_label);
                        }
                    }
                }
            }
        }

        // Defer for-body end label to the join block's pop (C18).
        if let Some(lbl) = state.for_body_end_labels.remove(bname) {
            state.pending_join_labels.insert(bname.clone(), lbl);
        }

        // Pop incoming arm value at join blocks with work. Place any
        // pending startCommand end label *before* the pop so the
        // startCommand covers only the arm body, not the result
        // cleanup (C18).
        if starts_with_any(bname, VALUE_JOIN_PREFIXES) && block_has_work(blk, ctx.is_proc) {
            if let Some(lbl) = state.pending_join_labels.remove(bname) {
                ctx.place_label(&lbl);
            }
            ctx.emit(Op::POP, vec![]);
        }

        // foreach_header: emit list loads + foreach_start opcode, skip
        // the normal Branch terminator (the opcode replaces it). When
        // this foreach is not the first command in its script, wrap it
        // with a startCommand whose end label is deferred to the
        // foreach_end block's trailing pop (C18).
        if let Some(fi) = foreach_info.get(bname) {
            if ctx.cmd_index > 0 {
                let fe_lbl = ctx.fresh_label("cmd_end");
                ctx.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(fe_lbl.clone()), Operand::Imm(1)],
                    "",
                );
                ctx.seen_generic_invoke = true;
                state.foreach_end_labels.insert(fi.end.clone(), fe_lbl);
            }
            for la in &fi.list_args {
                ctx.emit_value(la, false);
            }
            ctx.emit(Op::FOREACH_START, vec![Operand::Imm(0)]);
            ctx.cmd_index += 1;
            continue;
        }

        // foreach_body (simple only): emit body then
        // foreach_step + foreach_end. Complex foreach bodies fall
        // through to normal block processing; their step/end is
        // emitted at the foreach_end block (see above).
        if foreach_bodies.contains(bname) && !complex_body_blocks.contains(bname) {
            for stmt in &blk.statements {
                ctx.emit_pending_proc_defs(&mut state.pending_proc_defs, stmt.span().start());
                ctx.emit_stmt_with_start_cmd(stmt, None, None);
            }
            ctx.emit(Op::FOREACH_STEP, vec![]);
            ctx.emit(Op::FOREACH_END, vec![]);
            continue;
        }

        // For-init blocks: the last statement before a Goto to a
        // for_header_N block is the for command's init clause. Wrap
        // it with a count=2 startCommand spanning the whole for loop.
        let for_init_last_idx = detect_for_init_last_stmt(ctx, cfg, blk);

        // Emit statements.
        for (stmt_idx, stmt) in blk.statements.iter().enumerate() {
            ctx.emit_pending_proc_defs(&mut state.pending_proc_defs, stmt.span().start());
            if Some(stmt_idx) == for_init_last_idx {
                // For-init: emit startCommand with deferred end label
                // and count=2 (for + init both start at this offset).
                if let Some(Terminator::Goto {
                    target: fi_header, ..
                }) = &blk.terminator
                {
                    if let Some(fi_header_blk) = cfg.blocks.get(fi_header) {
                        if let Some(Terminator::Branch {
                            false_target: for_end,
                            ..
                        }) = &fi_header_blk.terminator
                        {
                            let fi_label = ctx.fresh_label("for_cmd_end");
                            state
                                .for_init_end_labels
                                .insert(for_end.clone(), fi_label.clone());
                            ctx.emit_stmt_with_start_cmd(stmt, Some(2), Some(&fi_label));
                            continue;
                        }
                    }
                }
            }
            // C18 case 5: synthetic `<cond>` placeholders get a
            // startCommand whose end label is deferred until the
            // ExprCommand in the branch condition has been emitted.
            // The label is placed by `emit_expr` in expressions.rs.
            if let Statement::Call { command, .. } = stmt {
                if command == "<cond>" {
                    let cond_label = ctx.fresh_label("cmd_end");
                    ctx.pending_cond_end_label = Some(cond_label.clone());
                    ctx.emit_stmt_with_start_cmd(stmt, None, Some(&cond_label));
                    continue;
                }
            }
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

        // Complex foreach: suppress back-edge Gotos from body blocks
        // to the foreach header. Fall through to foreach_step/foreach_end
        // at the foreach_end block, or jump to step label if not adjacent.
        let mut foreach_backedge = false;
        if complex_body_blocks.contains(bname) {
            if let Some(Terminator::Goto { target, .. }) = &blk.terminator {
                if let Some(info) = complex_foreach.get(target) {
                    let next_peek = block_order.get(i + 1).map(String::as_str);
                    if next_peek != Some(info.end.as_str()) {
                        ctx.emit_comment(
                            Op::JUMP4,
                            vec![Operand::Label(info.step_label.clone())],
                            "foreach continue",
                        );
                    }
                    foreach_backedge = true;
                }
            }
        }

        // While-loop startCommand: emit before the jump to while_header.
        if ctx.is_proc && ctx.cmd_index > 0 {
            if let Some(Terminator::Goto {
                target: wh_header, ..
            }) = &blk.terminator
            {
                let is_while_entry = wh_header.starts_with("while_header_")
                    && !bname.starts_with("while_body_")
                    && !bname.starts_with("while_step_");
                if is_while_entry {
                    if let Some(wh_blk) = cfg.blocks.get(wh_header) {
                        if let Some(Terminator::Branch {
                            false_target: wh_end,
                            ..
                        }) = &wh_blk.terminator
                        {
                            let wh_label = ctx.fresh_label("while_cmd_end");
                            ctx.emit_comment(
                                Op::START_CMD,
                                vec![Operand::Label(wh_label.clone()), Operand::Imm(1)],
                                "",
                            );
                            state.while_end_labels.insert(wh_end.clone(), wh_label);
                            ctx.cmd_index += 1;
                        }
                    }
                }
            }
        }

        let next_block = block_order.get(i + 1).map(String::as_str);
        if foreach_backedge {
            // Back-edge already handled above.
            continue;
        }
        if let Some(term) = &blk.terminator {
            // C18 case 4: constant-folded `if {1}` startCommand in
            // non-proc scripts. tclsh preserves a command boundary for
            // the `if` even when the condition is dead — the
            // startCommand's end label is placed before the join pop.
            if !ctx.is_proc {
                if let Terminator::Branch {
                    condition,
                    true_target,
                    ..
                } = term
                {
                    if super::ordering::fold_const_branch(condition) == Some(true) {
                        if let Some(tt_blk) = cfg.blocks.get(true_target) {
                            if let Some(Terminator::Goto { target: join, .. }) = &tt_blk.terminator
                            {
                                if join.starts_with("if_end_") {
                                    let end_label = ctx.fresh_label("cmd_end");
                                    ctx.emit_comment(
                                        Op::START_CMD,
                                        vec![Operand::Label(end_label.clone()), Operand::Imm(1)],
                                        "",
                                    );
                                    ctx.cmd_index += 1;
                                    ctx.seen_generic_invoke = true;
                                    state.pending_join_labels.insert(join.clone(), end_label);
                                }
                            }
                        }
                    }
                }
            }

            // Try switch-dispatch jump-table emission first.
            if ctx.try_emit_jump_table(cfg, blk, next_block, &mut state.skip_blocks) {
                // Switch dispatch counts as a command so the first arm
                // body gets its own startCommand.
                ctx.cmd_index += 1;
            } else if ctx.is_proc && matches!(term, Terminator::Return { .. }) {
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

/// Return `Some(idx)` where `idx` is the index of the for-init
/// statement within `blk.statements`. Returns `None` if this block
/// is not a for-init block, or if `ctx.is_proc` is false.
///
/// A for-init block ends with a `Goto` to a `for_header_N` block and
/// is not itself a `for_step_*` block. The init statement is the
/// last one in the block (the Python implementation prefers statements
/// whose source offset is within the for command's span, falling back
/// to the last statement).
fn detect_for_init_last_stmt(
    ctx: &CodegenCtx,
    cfg: &CfgFunction,
    blk: &crate::cfg::Block,
) -> Option<usize> {
    if !ctx.is_proc {
        return None;
    }
    let Terminator::Goto { target, .. } = blk.terminator.as_ref()? else {
        return None;
    };
    if !target.starts_with("for_header_") {
        return None;
    }
    if blk.name.starts_with("for_step_") {
        return None;
    }
    if blk.statements.is_empty() {
        return None;
    }
    // Ensure the target header block exists and has a Branch.
    let header = cfg.blocks.get(target)?;
    if !matches!(header.terminator, Some(Terminator::Branch { .. })) {
        return None;
    }
    Some(blk.statements.len() - 1)
}

/// Place `label` at the instruction immediately before a trailing
/// `pop` (if any), or at the current instruction index otherwise.
///
/// Used for deferred startCommand end labels that must land at the
/// loop-result cleanup pop.
fn place_label_before_trailing_pop(ctx: &mut CodegenCtx, label: &str) {
    let pos = if ctx.instructions.last().is_some_and(|i| i.op == Op::POP) {
        ctx.instructions.len() - 1
    } else {
        ctx.instructions.len()
    };
    ctx.label_positions.insert(label.to_owned(), pos);
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
    use crate::ir::Statement;
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;

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
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
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
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let asm = generate(&mut ctx, &cfg, &[]);
        // Should contain push and store (tail pop is folded away)
        let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::PUSH1));
        assert!(ops.contains(&Op::STORE_STK));
    }

    #[test]
    fn generate_proc_empty_body() {
        let cfg = CfgFunction::new("::foo", "entry_0");
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
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
        cfg.blocks
            .insert("if_then_1".into(), Block::new("if_then_1"));
        cfg.blocks
            .insert("if_else_1".into(), Block::new("if_else_1"));
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
        cfg.blocks.get_mut("if_then_1").unwrap().terminator = Some(Terminator::Goto {
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
        cfg.blocks.get_mut("if_else_1").unwrap().terminator = Some(Terminator::Goto {
            target: "if_end_1".into(),
            span: None,
        });
        cfg.blocks.get_mut("if_end_1").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let registry = CommandRegistry::build_default();

        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let asm = generate(&mut ctx, &cfg, &[]);
        let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
        // Should include a conditional jump (maybe shrunk to 1-byte)
        assert!(
            ops.contains(&Op::JUMP_FALSE4) || ops.contains(&Op::JUMP_FALSE1),
            "expected conditional jump, got {ops:?}"
        );
    }
}
