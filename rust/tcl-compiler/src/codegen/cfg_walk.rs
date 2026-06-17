//! The structured CFG walk — the shared, target-agnostic driver that
//! reconstructs **structured** control flow from the Tcl-shaped CFG and drives a
//! backend through the [`Emit`] seam.
//!
//! WASM (the first consumer) has no arbitrary jumps, so a backend cannot replay
//! the bytecode emitter's address-jump linearisation. But a Tcl CFG is always
//! *reducible* (built from `if`/`while`/`for`/`foreach`), so its structure is
//! recoverable: a [`Terminator::Branch`] with a reconvergence (join) block is an
//! `if`; the driver emits `begin_if` → then-region → `begin_else` → else-region →
//! `end_if`, then continues at the join.
//!
//! Stage 1 handles **linear chains + `if`/`else`**. Loops (`while`/`for`/
//! `foreach`, with `break`/`continue`/`return` completion codes), `switch`, and
//! `catch` are Stage 2 — they need the loop completion-code machinery and the
//! per-region metadata in [`crate::cfg::Function`].

use std::collections::{HashSet, VecDeque};

use crate::cfg::{Function, Terminator};
use crate::codegen::emit::Emit;

/// Walk `func` (using its original `source` for eval-fallback command text) and
/// drive `emit` with the recovered structured control flow.
pub fn walk<E: Emit>(emit: &mut E, func: &Function, source: &str) {
    walk_region(emit, func, source, &func.entry, None);
}

/// Emit the structured region from `start` up to (but excluding) `stop`.
fn walk_region<E: Emit>(
    emit: &mut E,
    func: &Function,
    source: &str,
    start: &str,
    stop: Option<&str>,
) {
    let mut cur = start.to_string();
    // Guard against back-edges (loops are Stage 2): a structured linear/if region
    // never revisits a block, so a repeat means a loop we don't yet lower.
    let mut guard = HashSet::new();
    loop {
        if stop == Some(cur.as_str()) || !guard.insert(cur.clone()) {
            return;
        }
        let Some(block) = func.blocks.get(&cur) else {
            return;
        };
        for stmt in &block.statements {
            emit.emit_command(slice(source, stmt.span().start(), stmt.span().end()));
        }
        match &block.terminator {
            Some(Terminator::Goto { target, .. }) => cur = target.clone(),
            Some(Terminator::Branch {
                true_target,
                false_target,
                span,
                ..
            }) => {
                let cond = span.map_or("", |s| slice(source, s.start(), s.end()));
                let (tt, ft) = (true_target.clone(), false_target.clone());
                let join = find_join(func, &tt, &ft);
                emit.begin_if(cond);
                walk_region(emit, func, source, &tt, join.as_deref());
                if join.as_deref() != Some(ft.as_str()) {
                    emit.begin_else();
                    walk_region(emit, func, source, &ft, join.as_deref());
                }
                emit.end_if();
                match join {
                    Some(j) if stop != Some(j.as_str()) => cur = j,
                    _ => return,
                }
            }
            Some(Terminator::Return { .. }) | None => return,
        }
    }
}

fn slice(source: &str, start: u32, end: u32) -> &str {
    &source[start as usize..end as usize]
}

/// The reconvergence (join) block of a branch: the first block on the
/// `false_target` path that is also reachable from `true_target`. For a
/// structured `if`/`else` this is the unique block where both arms rejoin (the
/// `false_target` itself when there is no `else`).
fn find_join(func: &Function, true_target: &str, false_target: &str) -> Option<String> {
    let reach_true = reach_forward(func, true_target);
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from([false_target.to_string()]);
    while let Some(b) = queue.pop_front() {
        if !seen.insert(b.clone()) {
            continue;
        }
        if reach_true.contains(&b) {
            return Some(b);
        }
        if let Some(block) = func.blocks.get(&b) {
            for s in block.successors() {
                queue.push_back(s.to_string());
            }
        }
    }
    None
}

/// All blocks reachable forward from `start` (following terminator successors).
fn reach_forward(func: &Function, start: &str) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut stack = vec![start.to_string()];
    while let Some(b) = stack.pop() {
        if !seen.insert(b.clone()) {
            continue;
        }
        if let Some(block) = func.blocks.get(&b) {
            for s in block.successors() {
                stack.push(s.to_string());
            }
        }
    }
    seen
}
