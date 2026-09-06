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

//! Main `generate()` dispatcher.
//!
//! Walks the CFG in block-order (from [`ordering::linearise`]) and
//! delegates per-block work to handlers. Runs peephole passes and
//! the layout pass to produce the final [`FunctionAsm`].

#![allow(dead_code)]

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use tcl_registry::InlineBodyErrorContext;

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ir::Procedure as IrProcedure;
use crate::ir::Statement;

use super::super::CodegenCtx;
use super::super::layout::{optimise_jumps, resolve_layout};
use super::super::{FunctionAsm, Op, Operand, SourceCommandBoundary};
use super::loop_blocks::{ComplexForeach, ForeachInfo, detect_complex_foreach, detect_foreach};
use super::ordering::{
    self, LOOP_BODY_PREFIXES, LOOP_END_PREFIXES, VALUE_JOIN_PREFIXES, linearise, starts_with_any,
};
use super::proc_defs::is_static_proc;
use super::try_blocks::{TryFinallyInfo, detect_try_finally};

/// Per-block `(continue, break)` loop-target labels, innermost-first.
/// Continue is `None` for a `for`-step block (continue propagates out).
type LoopContext = HashMap<String, (Option<String>, String)>;

/// Convert a target-neutral Tcl inline-body error context to the bytecode
/// runtime's body-frame label ABI.
const fn inline_body_frame_label(context: InlineBodyErrorContext) -> &'static str {
    match context {
        InlineBodyErrorContext::SameFrameScriptEvaluation => "eval",
    }
}

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
    /// label placed at the `foreach_end` block's trailing pop.
    foreach_end_labels: HashMap<String, String>,
    /// For-body startCommand end labels: `if_end_N` → deferred label
    /// placed at the join block's trailing pop.
    for_body_end_labels: HashMap<String, String>,
    /// Pending startCommand end labels for if/switch join pops — each
    /// label is placed *before* the join pop once it is emitted.
    pending_join_labels: HashMap<String, String>,
    /// Explicit inline structured-command continuation block to its replay
    /// label. The continuation pops either the optimised region's result or the
    /// transparent plain-dispatch child's result into `last_result`.
    command_boundary_end_labels: HashMap<String, String>,
    /// Proc-only constant-true `if` bodies whose first source command begins at
    /// the same bytecode position as the enclosing command. Tcl represents the
    /// pair with one count-two `START_CMD`, not nested markers.
    first_command_covered_by_if: HashSet<String>,
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
            command_boundary_end_labels: HashMap::new(),
            first_command_covered_by_if: HashSet::new(),
        }
    }
}

/// Read-only foreach detection results, bundled so the per-block
/// emission helpers can borrow them without a long argument list.
struct ForeachData {
    /// Simple foreach loops keyed by their header block.
    info: HashMap<String, ForeachInfo>,
    /// Body blocks of simple foreach loops.
    bodies: HashSet<String>,
    /// Synthetic loop-control landing pads for simple loops, keyed by their
    /// body block.  A `continue` must run `FOREACH_STEP`, rather than re-enter
    /// `FOREACH_START` and reset the iterator; a `break` must run
    /// `FOREACH_END` to release the iterator state.  These are data about the
    /// opcode layout, not command-specific behaviour in the VM.
    simple_control: HashMap<String, SimpleForeachControl>,
    /// Complex foreach loops (bodies with a Branch terminator) keyed by header.
    complex: HashMap<String, ComplexForeach>,
    /// `foreach_end` block name → its header block name (complex only).
    end_to_header: HashMap<String, String>,
    /// Body blocks belonging to a complex foreach loop.
    complex_body_blocks: HashSet<String>,
    /// Body blocks of *collecting* (simple `lmap`) loops — their trailing `POP`
    /// is stripped and an `LMAP_COLLECT` appended (only simple `lmap` lowers
    /// inline; a branching `lmap` body stays on the runtime builtin).
    collect_bodies: HashSet<String>,
    /// End blocks of collecting loops — their loop-result `""` push is suppressed
    /// because the paired `FOREACH_END` already pushed `list(accum)`.
    collect_ends: HashSet<String>,
}

/// Synthetic landing pads for one straight-line foreach body.
#[derive(Clone)]
struct SimpleForeachControl {
    /// Label immediately before `FOREACH_STEP`.
    step_label: String,
    /// Label immediately before `FOREACH_END`.
    break_label: String,
}

/// Detect foreach loops, allocate their opcode-layout control labels on `ctx`,
/// and redirect `loop_ctx` so every body reaches the appropriate step/end
/// opcode for a non-local loop completion.
fn setup_foreach(
    ctx: &mut CodegenCtx,
    cfg: &CfgFunction,
    loop_ctx: &mut LoopContext,
) -> ForeachData {
    // Detect foreach loops; bodies are marked so we can append
    // foreach_step/foreach_end after their statements.
    let info: HashMap<String, ForeachInfo> = detect_foreach(cfg);
    let bodies: HashSet<String> = info.values().map(|i| i.body.clone()).collect();
    // Collecting (`lmap`) loops: gather their body + end blocks so the emitter can
    // strip the body's trailing `POP` (→ `LMAP_COLLECT`) and suppress the end
    // block's `""` result push. Only *simple* lmaps reach codegen (a branching
    // body is barriered to the runtime builtin), so these are simple-loop blocks.
    let collect_bodies: HashSet<String> = info
        .values()
        .filter(|i| i.collect)
        .map(|i| i.body.clone())
        .collect();
    let collect_ends: HashSet<String> = info
        .values()
        .filter(|i| i.collect)
        .map(|i| i.end.clone())
        .collect();

    // Detect complex foreach loops (bodies with Branch terminator).
    // For these, foreach_step/foreach_end are emitted at the foreach_end
    // block (bottom-tested), and continue/break are rerouted through
    // synthetic step/break labels. Allocate labels directly on ctx so
    // they share the main label-counter pool.
    let mut tmp_counter: u32 = 0;
    let complex: HashMap<String, ComplexForeach> =
        detect_complex_foreach(cfg, &info, &mut tmp_counter)
            .into_iter()
            .map(|(hdr, mut ci)| {
                ci.step_label = ctx.fresh_label("foreach_continue");
                ci.break_label = ctx.fresh_label("foreach_break");
                (hdr, ci)
            })
            .collect();
    let end_to_header: HashMap<String, String> = complex
        .iter()
        .map(|(h, ci)| (ci.end.clone(), h.clone()))
        .collect();
    // Redirect loop context: body blocks of a complex foreach route
    // continue → step_label, break → break_label.
    let mut complex_body_blocks: HashSet<String> = HashSet::new();
    for (hdr, ci) in &complex {
        for bb in &ci.body_blocks {
            if let Some((cont, _brk)) = loop_ctx.get(bb)
                && cont.as_deref() == Some(hdr.as_str())
            {
                loop_ctx.insert(
                    bb.clone(),
                    (Some(ci.step_label.clone()), ci.break_label.clone()),
                );
                complex_body_blocks.insert(bb.clone());
            }
        }
    }

    // Straight-line bodies emit their paired opcodes inline.  They still need
    // synthetic targets: the CFG header would run `FOREACH_START` again, which
    // reconstructs the iterator at element zero, and the CFG end would skip
    // `FOREACH_END`, which owns the iterator teardown.  Keeping the targets in
    // this shared layout descriptor also makes literal and command-produced
    // `break`/`continue` take the same route.
    let mut simple_control = HashMap::new();
    for (header, foreach) in &info {
        if complex.contains_key(header) {
            continue;
        }
        let control = SimpleForeachControl {
            step_label: ctx.fresh_label("foreach_continue"),
            break_label: ctx.fresh_label("foreach_break"),
        };
        if let Some((cont, _)) = loop_ctx.get(&foreach.body)
            && cont.as_deref() == Some(header.as_str())
        {
            loop_ctx.insert(
                foreach.body.clone(),
                (
                    Some(control.step_label.clone()),
                    control.break_label.clone(),
                ),
            );
        }
        simple_control.insert(foreach.body.clone(), control);
    }

    ForeachData {
        info,
        bodies,
        simple_control,
        complex,
        end_to_header,
        complex_body_blocks,
        collect_bodies,
        collect_ends,
    }
}

/// Emit proc defs that appear before the first statement (for cases
/// where the entry block has no statements).
fn pre_emit_proc_defs(ctx: &mut CodegenCtx, cfg: &CfgFunction, state: &mut GenerateState) {
    if state.pending_proc_defs.is_empty() {
        return;
    }
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

/// Emit the synthetic per-block prologue: complex-foreach step/end at the
/// loop bottom, the loop-end result push, deferred loop end labels, and the
/// empty-loop-body NOPs. Returns whether `bname` is a loop-end block (its
/// result push must not be re-pushed by the arm-value handling).
fn emit_block_prologue(
    ctx: &mut CodegenCtx,
    cfg: &CfgFunction,
    state: &mut GenerateState,
    fe: &ForeachData,
    bname: &str,
    blk: &crate::cfg::Block,
) -> bool {
    if let Some(label) = state.command_boundary_end_labels.remove(bname) {
        ctx.place_label(&label);
        ctx.emit(Op::POP, vec![]);
    }

    // Complex foreach: emit foreach_step + foreach_end at the
    // bottom of the loop body, before the loop-result push/pop.
    if let Some(header) = fe.end_to_header.get(bname)
        && let Some(info) = fe.complex.get(header)
    {
        ctx.place_label(&info.step_label);
        ctx.emit(Op::FOREACH_STEP, vec![]);
        ctx.place_label(&info.break_label);
        ctx.emit(Op::FOREACH_END, vec![]);
    }

    // Loop-end blocks push "" as the loop command's result — except a
    // *collecting* loop (`lmap`), whose `list(accum)` result is already on the
    // stack from the paired `FOREACH_END`, so its end block pushes nothing.
    let is_loop_end = starts_with_any(bname, LOOP_END_PREFIXES);
    if is_loop_end && blk.statements.is_empty() && !fe.collect_ends.contains(bname) {
        let target = match &blk.terminator {
            Some(Terminator::Goto { target, .. }) => Some(cfg.block_name(*target).to_owned()),
            _ => None,
        };
        if let Some(t) = &target {
            if t.starts_with("exit_") {
                ctx.push_lit("");
            } else {
                ctx.literals.intern("");
                ctx.emit(Op::NOP, vec![]);
                ctx.emit(Op::NOP, vec![]);
                ctx.emit(Op::NOP, vec![]);
            }
        } else {
            ctx.push_lit("");
        }
    }

    // Place deferred for-init / while-loop / foreach startCommand
    // end labels at the loop-end block's pop.
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

    is_loop_end
}

/// Emit the per-block startCommand markers for for-body and complex-foreach
/// if-condition blocks, deferring their end labels to the relevant join pop,
/// then place any pending for-body end label and pop the incoming arm value
/// at join blocks with work.
fn emit_block_start_cmds(
    ctx: &mut CodegenCtx,
    cfg: &CfgFunction,
    state: &mut GenerateState,
    fe: &ForeachData,
    bname: &str,
    blk: &crate::cfg::Block,
) {
    // For-body startCommand: a `for_body_*` block
    // whose terminator is a Branch (the first statement is an
    // `if`) gets a startCommand that spans from here to the
    // `if_end_*` join pop. The end label is deferred into
    // `for_body_end_labels` keyed by the join block name.
    if bname.starts_with("for_body_")
        && matches!(blk.terminator, Some(Terminator::Branch { .. }))
        && ctx.cmd_index > 0
    {
        ctx.set_command_boundary_site(
            cfg.block_id(bname)
                .and_then(|id| cfg.command_boundary_sites.get(&id)),
        );
        let fb_end_label = ctx.fresh_label("for_body_end");
        ctx.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(fb_end_label.clone()), Operand::Imm(1)],
            "",
        );
        // Find the convergence point by following the true branch.
        if let Some(Terminator::Branch { true_target, .. }) = &blk.terminator
            && let Some(tt_blk) = cfg.blocks.get(true_target)
            && let Some(Terminator::Goto { target: join, .. }) = &tt_blk.terminator
            && cfg.block_name(*join).starts_with("if_end_")
        {
            state
                .for_body_end_labels
                .insert(cfg.block_name(*join).to_owned(), fb_end_label);
        }
    }

    // Complex-foreach if-condition startCommand:
    // body blocks inside a complex foreach whose terminator is a
    // Branch (the first element is an `if`) also get a per-body
    // startCommand, with the end label deferred to the `if_end_*`
    // or `if_next_*` join pop.
    if fe.complex_body_blocks.contains(bname)
        && matches!(blk.terminator, Some(Terminator::Branch { .. }))
        && ctx.cmd_index > 0
        && !bname.starts_with("foreach_header_")
    {
        ctx.set_command_boundary_site(
            cfg.block_id(bname)
                .and_then(|id| cfg.command_boundary_sites.get(&id)),
        );
        let fif_end_label = ctx.fresh_label("foreach_if_end");
        ctx.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(fif_end_label.clone()), Operand::Imm(1)],
            "",
        );
        ctx.seen_generic_invoke = true;
        if let Some(Terminator::Branch { true_target, .. }) = &blk.terminator
            && let Some(tt_blk) = cfg.blocks.get(true_target)
            && let Some(Terminator::Goto { target: join, .. }) = &tt_blk.terminator
            && {
                let j = cfg.block_name(*join);
                j.starts_with("if_end_") || j.starts_with("if_next_")
            }
        {
            state
                .for_body_end_labels
                .insert(cfg.block_name(*join).to_owned(), fif_end_label);
        }
    }

    // Defer for-body end label to the join block's pop.
    if let Some(lbl) = state.for_body_end_labels.remove(bname) {
        state.pending_join_labels.insert(bname.to_owned(), lbl);
    }

    // Pop incoming arm value at join blocks with work. Place any
    // pending startCommand end label *before* the pop so the
    // startCommand covers only the arm body, not the result
    // cleanup.
    if starts_with_any(bname, VALUE_JOIN_PREFIXES) {
        if let Some(lbl) = state.pending_join_labels.remove(bname) {
            ctx.place_label(&lbl);
        }
        if block_has_work(cfg, blk, ctx.is_proc) {
            ctx.emit(Op::POP, vec![]);
        }
    }
}

/// Emit a `foreach_header` block's list loads + `FOREACH_START` opcode.
/// Returns `true` if the block was a foreach header (caller should
/// `return` past normal statement/terminator emission).
fn emit_foreach_header(
    ctx: &mut CodegenCtx,
    cfg: &CfgFunction,
    state: &mut GenerateState,
    fe: &ForeachData,
    bname: &str,
) -> bool {
    let Some(fi) = fe.info.get(bname) else {
        return false;
    };
    if ctx.cmd_index > 0 {
        ctx.set_command_boundary_site(
            cfg.block_id(bname)
                .and_then(|id| cfg.command_boundary_sites.get(&id)),
        );
        let fe_lbl = ctx.fresh_label("cmd_end");
        ctx.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(fe_lbl.clone()), Operand::Imm(1)],
            "",
        );
        ctx.seen_generic_invoke = true;
        state.foreach_end_labels.insert(fi.end.clone(), fe_lbl);
    }
    for (i, la) in fi.list_args.iter().enumerate() {
        // A braced list word is a literal in every direction: `TclFindElement`'s
        // brace semantics keep `$` / `[…]` inert both for the word itself and
        // for any braced element inside it, so it must be pushed verbatim.
        // Pushing it as an ordinary literal let the VM's `subst_word` run the
        // substitution at loop entry — `foreach e {{a[b]c} x}` raised
        // `invalid command name "b"` where tclsh prints `a[b]c` (issue #1572).
        if fi.list_braced.get(i).copied().unwrap_or(false) {
            ctx.push_lit_verbatim(la);
        } else {
            ctx.emit_value(la, false);
        }
    }
    let fs_idx = ctx.emit(Op::FOREACH_START, vec![Operand::Imm(0)]);
    // Carry the loop-variable groups (C Tcl `ForeachInfo.varLists`) so
    // the VM can bind them; not rendered in disassembly.
    ctx.instructions[fs_idx].foreach_vars = Some(fi.var_groups.clone());
    // A collecting loop (`lmap`) tells the VM to accumulate each iteration's
    // result and yield `list(accum)` at `FOREACH_END`.
    ctx.instructions[fs_idx].foreach_collect = fi.collect;
    ctx.cmd_index += 1;
    true
}

/// Emit a simple `foreach_body` block: its statements followed by
/// `foreach_step` + `foreach_end`. Returns `true` if the block was a simple
/// foreach body (caller should `return`).
fn emit_foreach_body(
    ctx: &mut CodegenCtx,
    state: &mut GenerateState,
    fe: &ForeachData,
    bname: &str,
    blk: &crate::cfg::Block,
) -> bool {
    if !fe.bodies.contains(bname) || fe.complex_body_blocks.contains(bname) {
        return false;
    }
    for stmt in &blk.statements {
        ctx.emit_pending_proc_defs(&mut state.pending_proc_defs, stmt.span().start());
        ctx.emit_stmt_with_start_cmd(stmt, None, None);
    }
    // foreach_step/foreach_end are synthetic loop machinery with no
    // source construct; clear the sticky statement span so they
    // serialise as null rather than inheriting the last body
    // statement's range.
    ctx.clear_source_site();
    // Collecting loop (`lmap`): strip the body's trailing `POP` so its result
    // stays on the stack, then append it VM-side via `LMAP_COLLECT` on the
    // fall-through path (a break/continue redirect jumps past it, contributing
    // nothing — as C `lmap` does). A result-less body (empty, or ending in a
    // `NOP`-padded shape) has no trailing `POP`, so push the empty string it
    // returns. This mirrors `dict map`'s keep-last-result trick.
    if fe.collect_bodies.contains(bname) {
        if ctx.instructions.last().map(|i| i.op) == Some(Op::POP) {
            ctx.instructions.pop();
        } else {
            ctx.push_lit("");
        }
        ctx.emit(Op::LMAP_COLLECT, vec![]);
    }
    let control = fe
        .simple_control
        .get(bname)
        .expect("every simple foreach body has loop-control labels");
    ctx.place_label(&control.step_label);
    ctx.emit(Op::FOREACH_STEP, vec![]);
    ctx.place_label(&control.break_label);
    ctx.emit(Op::FOREACH_END, vec![]);
    true
}

/// Emit the statements of a normal (non-foreach) block, including for-init
/// and `<cond>` startCommand wrapping.
fn emit_block_statements(
    ctx: &mut CodegenCtx,
    cfg: &CfgFunction,
    state: &mut GenerateState,
    bname: &str,
    blk: &crate::cfg::Block,
) {
    // For-init blocks: the last statement before a Goto to a
    // for_header_N block is the for command's init clause. Wrap
    // it with a count=2 startCommand spanning the whole for loop.
    let for_init_last_idx = detect_for_init_last_stmt(ctx, cfg, blk);

    let first_command_covered = state.first_command_covered_by_if.remove(bname);
    for (stmt_idx, stmt) in blk.statements.iter().enumerate() {
        ctx.emit_pending_proc_defs(&mut state.pending_proc_defs, stmt.span().start());
        if first_command_covered && stmt_idx == 0 {
            ctx.emit_stmt_under_start_cmd(stmt);
            continue;
        }
        if Some(stmt_idx) == for_init_last_idx {
            // For-init: emit startCommand with deferred end label
            // and count=2 (for + init both start at this offset).
            if let Some(Terminator::Goto {
                target: fi_header, ..
            }) = &blk.terminator
                && let Some(fi_header_blk) = cfg.blocks.get(fi_header)
                && let Some(Terminator::Branch {
                    false_target: for_end,
                    ..
                }) = &fi_header_blk.terminator
            {
                let fi_label = ctx.fresh_label("for_cmd_end");
                state
                    .for_init_end_labels
                    .insert(cfg.block_name(*for_end).to_owned(), fi_label.clone());
                let before = ctx.instructions.len();
                ctx.emit_stmt_with_start_cmd(stmt, Some(2), Some(&fi_label));
                if let Some(start_cmd) = (before..ctx.instructions.len())
                    .find(|&idx| ctx.instructions[idx].op == Op::START_CMD)
                {
                    ctx.stamp_command_boundary(
                        start_cmd,
                        cfg.block_id(bname)
                            .and_then(|id| cfg.command_boundary_sites.get(&id)),
                    );
                }
                continue;
            }
        }
        // Synthetic condition placeholders get a
        // startCommand whose end label is deferred until the
        // ExprCommand in the branch condition has been emitted.
        // The label is placed by `emit_expr` in expressions.rs.
        // Keyed on the typed marker, not the `<cond>` spelling — that is a
        // legal Tcl command name a script may define and call (see
        // `crate::ir::SyntheticMarker`).
        if let Statement::Call { tokens, .. } = stmt
            && tokens
                .as_ref()
                .and_then(|t| t.synthetic)
                .is_some_and(|m| m == crate::ir::SyntheticMarker::Condition)
        {
            let cond_label = ctx.fresh_label("cmd_end");
            ctx.pending_cond_end_label = Some(cond_label.clone());
            ctx.emit_stmt_with_start_cmd(stmt, None, Some(&cond_label));
            continue;
        }
        ctx.emit_stmt_with_start_cmd(stmt, None, None);
    }
}

/// Emit the owning command boundary for a structured block that CFG lowering
/// split into an explicit entry/body/continuation region.
fn emit_inline_block_command_boundary(
    ctx: &mut CodegenCtx,
    cfg: &CfgFunction,
    state: &mut GenerateState,
    bname: &str,
) {
    let Some(entry) = cfg.block_id(bname) else {
        return;
    };
    let Some(continuation) = cfg.command_boundary_continuations.get(&entry) else {
        return;
    };
    let Some(site) = cfg.command_boundary_sites.get(&entry) else {
        return;
    };

    ctx.set_command_boundary_site(Some(site));
    let end_label = ctx.fresh_label("inline_block_cmd_end");
    ctx.emit_comment(
        Op::START_CMD,
        vec![Operand::Label(end_label.clone()), Operand::Imm(1)],
        "",
    );
    state
        .command_boundary_end_labels
        .insert(cfg.block_name(*continuation).to_owned(), end_label);
    ctx.cmd_index += 1;
    ctx.seen_generic_invoke = true;
}

/// Emit a block's terminator: constant-folded `if {1}` startCommand,
/// switch jump tables, proc returns, or the generic terminator.
fn emit_block_terminator(
    ctx: &mut CodegenCtx,
    cfg: &CfgFunction,
    state: &mut GenerateState,
    bname: &str,
    blk: &crate::cfg::Block,
    block_order: &[String],
    i: usize,
) {
    let next_block = block_order.get(i + 1).map(String::as_str);
    if let Some(term) = &blk.terminator {
        // A constant-true `if` and its first selected-body command begin at the
        // same bytecode offset. In a proc, Tcl emits the owning count-two
        // marker only when an earlier generic invocation could have changed
        // the compile epoch; the first body command then shares that marker.
        // Top-level emission retains its historical marker policy and later
        // peepholes remove it when the unit has no generic invocation.
        if let Terminator::Branch {
            condition,
            true_target,
            ..
        } = term
            && super::ordering::fold_const_branch(condition) == Some(true)
            && let Some(tt_blk) = cfg.blocks.get(true_target)
            && let Some(Terminator::Goto { target: join, .. }) = &tt_blk.terminator
            && cfg.block_name(*join).starts_with("if_end_")
        {
            if ctx.is_proc {
                state
                    .first_command_covered_by_if
                    .insert(cfg.block_name(*true_target).to_owned());
            }
            if !ctx.is_proc || ctx.seen_generic_invoke {
                ctx.set_command_boundary_site(
                    cfg.block_id(bname)
                        .and_then(|id| cfg.command_boundary_sites.get(&id)),
                );
                let end_label = ctx.fresh_label("cmd_end");
                ctx.emit_comment(
                    Op::START_CMD,
                    vec![
                        Operand::Label(end_label.clone()),
                        Operand::Imm(if ctx.is_proc { 2 } else { 1 }),
                    ],
                    "",
                );
                ctx.cmd_index += 1;
                ctx.seen_generic_invoke = true;
                state
                    .pending_join_labels
                    .insert(cfg.block_name(*join).to_owned(), end_label);
            }
        }

        // Try switch-dispatch jump-table emission first.
        if ctx.try_emit_jump_table(cfg, blk, next_block, &mut state.skip_blocks) {
            // Switch dispatch counts as a command so the first arm
            // body gets its own startCommand.
            ctx.cmd_index += 1;
        } else if ctx.is_proc && matches!(term, Terminator::Return { .. }) {
            ctx.emit_proc_return(term, bname, next_block, block_order, i, cfg);
        } else {
            ctx.emit_term(cfg, term, next_block);
        }
    } else if next_block.is_some() {
        // Terminal block not last in layout — emit done to prevent
        // fallthrough into successor blocks.
        ctx.emit(Op::DONE, vec![]);
    }
}

/// Emit one CFG block (the body of the main block-order loop). Skips
/// try/finally-consumed blocks, dispatches foreach headers/bodies, emits
/// statements, and finishes with the block's terminator.
fn emit_block(
    ctx: &mut CodegenCtx,
    cfg: &CfgFunction,
    state: &mut GenerateState,
    fe: &ForeachData,
    loop_ctx: &LoopContext,
    block_order: &[String],
    i: usize,
) {
    let bname = &block_order[i];
    if state.skip_blocks.contains(bname) {
        ctx.place_label(bname);
        return;
    }
    let blk = cfg.block_by_name(bname).expect("layout block exists");
    ctx.place_label(bname);

    // Synthetic per-block instructions (loop-result pushes, padding
    // NOPs, startCommand boundary markers emitted before any statement)
    // carry no direct source span; statement / terminator emission
    // sets a real span below.
    ctx.clear_source_site();

    // try/finally inline compilation at try_body block.
    if let Some(info) = state.try_finally_info.get(bname).cloned() {
        ctx.set_command_boundary_site(
            cfg.block_id(bname)
                .and_then(|id| cfg.command_boundary_sites.get(&id)),
        );
        ctx.emit_try_finally_inline(cfg, bname, &info.try_finally);
        return;
    }

    // Update loop context for break/continue compilation. The continue
    // target is `None` for a `for`-step block (continue propagates out).
    if let Some((cont, brk)) = loop_ctx.get(bname) {
        ctx.continue_target.clone_from(cont);
        ctx.break_target = Some(brk.clone());
    }

    let is_loop_end = emit_block_prologue(ctx, cfg, state, fe, bname, blk);
    emit_block_start_cmds(ctx, cfg, state, fe, bname, blk);

    if emit_foreach_header(ctx, cfg, state, fe, bname) {
        return;
    }
    if emit_foreach_body(ctx, state, fe, bname, blk) {
        return;
    }

    emit_block_statements(ctx, cfg, state, bname, blk);

    emit_inline_block_command_boundary(ctx, cfg, state, bname);

    // If/switch arms: keep last statement value on TOS instead of
    // popping — the value is the arm's result.
    if let Some(Terminator::Goto { target, .. }) = &blk.terminator
        && starts_with_any(cfg.block_name(*target), VALUE_JOIN_PREFIXES)
    {
        if !blk.statements.is_empty() && ctx.instructions.last().is_some_and(|i| i.op == Op::POP) {
            ctx.instructions.pop();
        } else if blk.statements.is_empty() && !is_loop_end {
            // Else-less if: false path needs an empty-string result.
            ctx.push_lit("");
        }
    }

    // Complex foreach: suppress back-edge Gotos from body blocks
    // to the foreach header. Fall through to foreach_step/foreach_end
    // at the foreach_end block, or jump to step label if not adjacent.
    if fe.complex_body_blocks.contains(bname)
        && let Some(Terminator::Goto { target, .. }) = &blk.terminator
        && let Some(info) = fe.complex.get(cfg.block_name(*target))
    {
        let next_peek = block_order.get(i + 1).map(String::as_str);
        if next_peek != Some(info.end.as_str()) {
            ctx.emit_comment(
                Op::JUMP4,
                vec![Operand::Label(info.step_label.clone())],
                "foreach continue",
            );
        }
        // Back-edge handled; skip terminator emission.
        return;
    }

    // While-loop startCommand: emit before the jump to while_header.
    if ctx.cmd_index > 0
        && let Some(Terminator::Goto {
            target: wh_header, ..
        }) = &blk.terminator
    {
        let is_while_entry = cfg.block_name(*wh_header).starts_with("while_header_")
            && !bname.starts_with("while_body_")
            && !bname.starts_with("while_step_");
        if is_while_entry
            && let Some(wh_blk) = cfg.blocks.get(wh_header)
            && let Some(Terminator::Branch {
                false_target: wh_end,
                ..
            }) = &wh_blk.terminator
        {
            ctx.set_command_boundary_site(
                cfg.block_id(bname)
                    .and_then(|id| cfg.command_boundary_sites.get(&id)),
            );
            let wh_label = ctx.fresh_label("while_cmd_end");
            ctx.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(wh_label.clone()), Operand::Imm(1)],
                "",
            );
            state
                .while_end_labels
                .insert(cfg.block_name(*wh_end).to_owned(), wh_label);
            ctx.cmd_index += 1;
        }
    }

    emit_block_terminator(ctx, cfg, state, bname, blk, block_order, i);
}

/// Emit the trailing `DONE` / proc-exit machinery after all blocks.
fn emit_function_tail(ctx: &mut CodegenCtx, cfg: &CfgFunction, state: &mut GenerateState) {
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
}

/// Run the peephole passes and layout pass, then assemble the final
/// [`FunctionAsm`] (loop-target table + inline-body error regions).
// Finalisation is one ordered sequence of metadata-sensitive passes.
#[allow(clippy::too_many_lines)]
fn finalize_function(
    ctx: &mut CodegenCtx,
    cfg: &CfgFunction,
    block_order: &[String],
    loop_ctx: &LoopContext,
) -> FunctionAsm {
    // Peephole passes.
    ctx.remove_trailing_pop();
    ctx.fold_tail_return_to_done();
    ctx.strip_empty_start_cmd();
    ctx.strip_unused_start_cmd();
    ctx.fixup_top_level_start_cmd();
    ctx.fold_const_push_pop_nops();
    ctx.dedup_push_literals();
    ctx.strip_nodedup_tags();

    // Layout pass.
    optimise_jumps(&mut ctx.instructions, &ctx.label_positions, 10);
    let labels = resolve_layout(&mut ctx.instructions, &ctx.label_positions);

    // Mark actual source-command entry independently of diagnostic metadata.
    // START_CMD owns its own replay/continuation path and deliberately does not
    // change the implicit owner: nested inline catch/try instructions retain
    // the enclosing absolute span, and returning to that span is not a second
    // execution of the outer command.
    let mut source_owner: Option<(u32, Option<tcl_lexer::Span>, String, String)> = None;
    let mut nested_start_owner: Option<(u32, Option<tcl_lexer::Span>, String, String)> = None;
    for instruction in &mut ctx.instructions {
        if let Some(site) = instruction.source_span.and_then(|span| {
            cfg.command_binding_sites
                .iter()
                .rev()
                .find(|site| site.span == span)
        }) {
            instruction
                .source_command_namespace
                .clone_from(&site.binding.resolution_namespace);
        }
        if instruction.op == Op::START_CMD {
            if instruction.source_command_boundary.is_start() {
                source_owner = Some((
                    instruction.source_line,
                    instruction.source_span,
                    instruction.source_cmd_text.clone(),
                    instruction.source_command_namespace.clone(),
                ));
                nested_start_owner = None;
            } else if !instruction.source_cmd_text.is_empty() {
                nested_start_owner = Some((
                    instruction.source_line,
                    instruction.source_span,
                    instruction.source_cmd_text.clone(),
                    instruction.source_command_namespace.clone(),
                ));
            }
            continue;
        }
        instruction.source_command_boundary = SourceCommandBoundary::None;
        if instruction.source_cmd_text.is_empty() {
            nested_start_owner = None;
            continue;
        }
        let owner = (
            instruction.source_line,
            instruction.source_span,
            instruction.source_cmd_text.clone(),
            instruction.source_command_namespace.clone(),
        );
        if nested_start_owner.as_ref() == Some(&owner) {
            continue;
        }
        nested_start_owner = None;
        if source_owner.as_ref() != Some(&owner) {
            instruction.source_command_boundary = SourceCommandBoundary::Start;
            source_owner = Some(owner);
        }
    }

    // Loop-target table: each loop-body instruction → its loop's break/continue
    // byte offsets, so the executor can catch a command-returned break/continue
    // (`if {…} $z`, `eval break`). Built post-layout from the (innermost-first)
    // `loop_ctx` block targets + the final block index ranges. `label_positions`
    // is maintained through the peephole removals, so the indices are final.
    let loop_targets = build_loop_targets(
        block_order,
        loop_ctx,
        &ctx.label_positions,
        &labels,
        ctx.instructions.len(),
    );

    // Inline-body error regions: each registry-described body context the CFG
    // builder retained becomes a region keyed by the enclosing command's source
    // span. Source text is used only for Tcl's `invoked from within` payload;
    // the body-frame identity comes from semantic metadata, never re-parsing.
    let error_regions = cfg
        .inline_body_error_sites
        .iter()
        .filter_map(|site| {
            let span = site.span;
            let cmd_text = ctx.source_text(span);
            if cmd_text.is_empty() {
                return None;
            }
            let cmd_line = ctx.source_line(span);
            Some(tcl_bytecode::ErrorRegion {
                start: span.start(),
                end: span.end(),
                label: inline_body_frame_label(site.context).to_string(),
                cmd_text,
                line_base: cmd_line.saturating_sub(1),
                cmd_line,
            })
        })
        .collect();

    FunctionAsm {
        name: cfg.name.clone(),
        literals: std::mem::take(&mut ctx.literals),
        lvt: std::mem::take(&mut ctx.lvt),
        instructions: std::mem::take(&mut ctx.instructions),
        labels,
        loop_targets,
        body_base_line: 0,
        proc_body_src: None,
        error_regions,
        plain_command_dispatch: ctx.plain_command_dispatch,
        command_bindings: std::mem::take(&mut ctx.command_binding_requirements)
            .into_iter()
            .collect(),
        procedure_bindings: cfg
            .procedure_binding_requirements
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
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
pub fn generate(ctx: &mut CodegenCtx, cfg: &CfgFunction, proc_defs: &[IrProcedure]) -> FunctionAsm {
    if !ctx.plain_command_dispatch {
        for site in &cfg.command_binding_sites {
            ctx.require_command_binding(&site.binding);
        }
    }
    let block_order = linearise(cfg);
    let mut loop_ctx = ordering::build_loop_context(cfg);
    let mut state = GenerateState::new(proc_defs);
    state.try_finally_info = detect_try_finally(cfg, &block_order);

    // The same-frame script bodies folded into this function, so the variable
    // emitters can decline the compiled-local forms inside them
    // (`CodegenCtx::compiles_locals`). Same regions the `errorInfo` body frames
    // are built from at the end of this function.
    ctx.same_frame_eval_spans = cfg
        .inline_body_error_sites
        .iter()
        .filter(|site| {
            site.context == tcl_registry::InlineBodyErrorContext::SameFrameScriptEvaluation
        })
        .map(|site| (site.span.start(), site.span.end()))
        .collect();

    let fe = setup_foreach(ctx, cfg, &mut loop_ctx);

    // Mark try_end/try_finally/try_after_finally as skipped — they
    // are emitted as part of the try_body block's inline sequence.
    for info in state.try_finally_info.values() {
        state.skip_blocks.insert(info.try_end.clone());
        state.skip_blocks.insert(info.try_finally.clone());
        state.skip_blocks.insert(info.try_after.clone());
    }

    pre_emit_proc_defs(ctx, cfg, &mut state);

    for i in 0..block_order.len() {
        emit_block(ctx, cfg, &mut state, &fe, &loop_ctx, &block_order, i);
    }

    emit_function_tail(ctx, cfg, &mut state);
    finalize_function(ctx, cfg, &block_order, &loop_ctx)
}

/// Build the per-instruction loop-target table (see `FunctionAsm::loop_targets`).
///
/// `loop_ctx` maps each loop-body block to its `(continue, break)` target
/// labels (innermost-first). Each block occupies the instruction range
/// `[its start, the next block's start)` — `label_positions` gives the final
/// (post-peephole) start index of every block; `labels` resolves the target
/// labels to byte offsets. Stamp every instruction in each loop block's range.
fn build_loop_targets(
    block_order: &[String],
    loop_ctx: &HashMap<String, (Option<String>, String)>,
    label_positions: &HashMap<String, usize>,
    labels: &HashMap<String, usize>,
    n_instructions: usize,
) -> HashMap<usize, (Option<i32>, Option<i32>)> {
    let mut out: HashMap<usize, (Option<i32>, Option<i32>)> = HashMap::new();
    if loop_ctx.is_empty() {
        return out;
    }
    // (start index, block name) for each emitted block, in index order.
    let mut starts: Vec<(usize, &str)> = block_order
        .iter()
        .filter_map(|b| label_positions.get(b).map(|&i| (i, b.as_str())))
        .collect();
    starts.sort_by_key(|&(i, _)| i);
    for (k, &(start, bname)) in starts.iter().enumerate() {
        let Some((cont_lbl, brk_lbl)) = loop_ctx.get(bname) else {
            continue;
        };
        let end = starts.get(k + 1).map_or(n_instructions, |&(i, _)| i);
        let brk_off = labels.get(brk_lbl).and_then(|&o| i32::try_from(o).ok());
        let cont_off = cont_lbl
            .as_ref()
            .and_then(|l| labels.get(l))
            .and_then(|&o| i32::try_from(o).ok());
        for idx in start..end {
            out.insert(idx, (brk_off, cont_off));
        }
    }
    out
}

/// Return `Some(idx)` where `idx` is the index of the for-init
/// statement within `blk.statements`. Returns `None` if this block
/// is not a for-init block, or if `ctx.is_proc` is false.
///
/// A for-init block ends with a `Goto` to a `for_header_N` block and
/// is not itself a `for_step_*` block. The init statement is the
/// last one in the block.
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
    if !cfg.block_name(*target).starts_with("for_header_") {
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
fn block_has_work(cfg: &CfgFunction, blk: &crate::cfg::Block, is_proc: bool) -> bool {
    if !blk.statements.is_empty() {
        return true;
    }
    match &blk.terminator {
        Some(Terminator::Branch { .. }) => true,
        Some(Terminator::Return { .. }) => is_proc,
        Some(Terminator::Goto { target, .. }) => !cfg.block_name(*target).starts_with("exit_"),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Function as CfgFunction, InlineBodyErrorSite, Terminator};
    use crate::codegen::CodegenCtx;
    use crate::ir::Statement;
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;

    fn sp() -> Span {
        Span::new(0, 0)
    }

    fn trivial_cfg() -> CfgFunction {
        let mut cfg = CfgFunction::new("::top", "entry_0");
        let entry = cfg.entry;
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
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
    fn inline_body_error_region_uses_semantic_context_not_command_text() {
        let source = "renamed-body-command {error boom}";
        let span = Span::new(0, u32::try_from(source.len()).unwrap());
        let mut cfg = trivial_cfg();
        cfg.inline_body_error_sites.push(InlineBodyErrorSite {
            span,
            context: InlineBodyErrorContext::SameFrameScriptEvaluation,
        });
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.set_source(source);

        let asm = generate(&mut ctx, &cfg, &[]);

        let [region] = asm.error_regions.as_slice() else {
            panic!("semantic inline-body context should emit exactly one error region");
        };
        assert_eq!(region.label, "eval");
        assert_eq!(region.cmd_text, source);

        let mut source_less = CodegenCtx::new(false, &[], &registry);
        let source_less_asm = generate(&mut source_less, &cfg, &[]);
        assert!(
            source_less_asm.error_regions.is_empty(),
            "a source-less context retains the previous no-region behaviour",
        );
    }

    #[test]
    fn generate_stamps_statement_source_span() {
        // A statement with a non-trivial span: its lowered instructions
        // (push / store) must carry that span; the synthetic trailing
        // DONE carries none.
        let mut cfg = CfgFunction::new("::top", "entry_0");
        let entry = cfg.entry;
        let stmt_span = Span::new(4, 10);
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: stmt_span,
                name: "x".into(),
                value: "42".into(),
                name_braced: false,
                value_span: None,
            });
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let asm = generate(&mut ctx, &cfg, &[]);

        // The store-into-x instruction came from the AssignConst statement.
        let store = asm
            .instructions
            .iter()
            .find(|i| i.op == Op::STORE_STK)
            .expect("store emitted");
        assert_eq!(
            store.source_span,
            Some(stmt_span),
            "statement-driven op carries its source span"
        );
        // The synthetic terminator (Return with no span) leaves none.
        let done = asm
            .instructions
            .iter()
            .rev()
            .find(|i| matches!(i.op, Op::DONE | Op::RETURN_IMM))
            .expect("terminator emitted");
        assert_eq!(done.source_span, None, "synthetic terminator has no span");
    }

    #[test]
    fn generate_simple_set() {
        let mut cfg = CfgFunction::new("::top", "entry_0");
        let entry = cfg.entry;
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: sp(),
                name: "x".into(),
                value: "42".into(),
                name_braced: false,
                value_span: None,
            });
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
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
        let entry = cfg.entry;
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
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
        let entry = cfg.entry;
        let then = cfg.intern_block("if_then_1");
        cfg.blocks.insert(then, Block::new("if_then_1"));
        let els = cfg.intern_block("if_else_1");
        cfg.blocks.insert(els, Block::new("if_else_1"));
        let end = cfg.intern_block("if_end_1");
        cfg.blocks.insert(end, Block::new("if_end_1"));

        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
            condition: ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            },
            true_target: then,
            false_target: els,
            span: None,
            condition_base: None,
        });
        cfg.blocks
            .get_mut(&then)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: sp(),
                name: "r".into(),
                value: "1".into(),
                name_braced: false,
                value_span: None,
            });
        cfg.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Goto {
            target: end,
            span: None,
        });
        cfg.blocks
            .get_mut(&els)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: sp(),
                name: "r".into(),
                value: "2".into(),
                name_braced: false,
                value_span: None,
            });
        cfg.blocks.get_mut(&els).unwrap().terminator = Some(Terminator::Goto {
            target: end,
            span: None,
        });
        cfg.blocks.get_mut(&end).unwrap().terminator = Some(Terminator::Return {
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
