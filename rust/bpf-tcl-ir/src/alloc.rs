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

//! Liveness-based stack-slot allocation.
//!
//! Lowering hands every value its own virtual slot; this pass computes slot
//! liveness over the block graph and re-colours the virtual slots so values
//! with disjoint live ranges share one 8-byte stack slot. It also computes
//! the *zero-init set*: the slots that may be read on some path before every
//! write — the only slots a target must zero at entry (blanket zeroing of
//! every slot is both wasted instructions and wasted verifier state).
//!
//! Two virtual slots may share a physical slot only when they never live at
//! the same time **and** have the same [`Ty`] (so `slot_types` stays
//! meaningful). The 512-byte eBPF stack limit (64 slots × 8 bytes) is
//! enforced *after* reuse, target-independently, with the source span of the
//! first value that no longer fits.

use std::collections::HashMap;

use tcl_lexer::Span;

use crate::diag::{BpfDiag, BpfError};
use crate::ir::{BpfProgram, Inst, SlotId, Term};
use crate::ty::Ty;

/// 64 slots × 8 bytes = the 512-byte eBPF stack.
pub const MAX_SLOTS: usize = 64;

/// A dense per-slot bit set.
#[derive(Clone, PartialEq, Eq)]
struct SlotSet {
    bits: Vec<bool>,
}

impl SlotSet {
    fn new(n: usize) -> Self {
        Self {
            bits: vec![false; n],
        }
    }

    fn insert(&mut self, s: SlotId) -> bool {
        let i = s.0 as usize;
        let was = self.bits[i];
        self.bits[i] = true;
        !was
    }

    fn remove(&mut self, s: SlotId) {
        self.bits[s.0 as usize] = false;
    }

    fn contains(&self, s: SlotId) -> bool {
        self.bits[s.0 as usize]
    }

    fn union_with(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for (a, b) in self.bits.iter_mut().zip(&other.bits) {
            if *b && !*a {
                *a = true;
                changed = true;
            }
        }
        changed
    }

    fn iter(&self) -> impl Iterator<Item = SlotId> + '_ {
        self.bits
            .iter()
            .enumerate()
            .filter(|(_, b)| **b)
            .map(|(i, _)| SlotId(u32::try_from(i).unwrap_or(u32::MAX)))
    }
}

/// The slot an instruction defines, if any.
fn inst_def(inst: &Inst) -> Option<SlotId> {
    match inst {
        Inst::Const { dst, .. }
        | Inst::Copy { dst, .. }
        | Inst::Bin { dst, .. }
        | Inst::Un { dst, .. }
        | Inst::Cmp { dst, .. }
        | Inst::CtxPtr { dst, .. }
        | Inst::CtxLen { dst, .. }
        | Inst::Load { dst, .. }
        | Inst::MapGet { dst, .. }
        | Inst::MapHas { dst, .. } => Some(*dst),
        Inst::MapSet { .. } => None,
    }
}

/// The slots an instruction reads.
fn inst_uses(inst: &Inst, mut f: impl FnMut(SlotId)) {
    match inst {
        Inst::Const { .. } | Inst::CtxPtr { .. } | Inst::CtxLen { .. } => {}
        Inst::Copy { src, .. } => f(*src),
        Inst::Bin { a, b, .. } | Inst::Cmp { a, b, .. } => {
            f(*a);
            f(*b);
        }
        Inst::Un { a, .. } => f(*a),
        Inst::Load { ptr, .. } => f(*ptr),
        Inst::MapGet { key, .. } | Inst::MapHas { key, .. } => f(*key),
        Inst::MapSet { key, val, .. } => {
            f(*key);
            f(*val);
        }
    }
}

/// The slots a terminator reads.
fn term_uses(term: &Term, mut f: impl FnMut(SlotId)) {
    match term {
        Term::Goto { .. } => {}
        Term::BranchNz { cond, .. } => f(*cond),
        Term::Return { verdict, .. } => f(*verdict),
    }
}

/// The span of an instruction (for diagnostics).
fn inst_span(inst: &Inst) -> Span {
    match inst {
        Inst::Const { span, .. }
        | Inst::Copy { span, .. }
        | Inst::Bin { span, .. }
        | Inst::Un { span, .. }
        | Inst::Cmp { span, .. }
        | Inst::CtxPtr { span, .. }
        | Inst::CtxLen { span, .. }
        | Inst::Load { span, .. }
        | Inst::MapGet { span, .. }
        | Inst::MapHas { span, .. }
        | Inst::MapSet { span, .. } => *span,
    }
}

/// Rewrite every slot reference in an instruction through `map`.
fn remap_inst(inst: &mut Inst, map: &[SlotId]) {
    let m = |s: &mut SlotId| *s = map[s.0 as usize];
    match inst {
        Inst::Const { dst, .. } | Inst::CtxPtr { dst, .. } | Inst::CtxLen { dst, .. } => m(dst),
        Inst::Copy { dst, src, .. } => {
            m(dst);
            m(src);
        }
        Inst::Bin { dst, a, b, .. } | Inst::Cmp { dst, a, b, .. } => {
            m(dst);
            m(a);
            m(b);
        }
        Inst::Un { dst, a, .. } => {
            m(dst);
            m(a);
        }
        Inst::Load { dst, ptr, .. } => {
            m(dst);
            m(ptr);
        }
        Inst::MapGet { dst, key, .. } | Inst::MapHas { dst, key, .. } => {
            m(dst);
            m(key);
        }
        Inst::MapSet { key, val, .. } => {
            m(key);
            m(val);
        }
    }
}

fn remap_term(term: &mut Term, map: &[SlotId]) {
    match term {
        Term::Goto { .. } => {}
        Term::BranchNz { cond, .. } => *cond = map[cond.0 as usize],
        Term::Return { verdict, .. } => *verdict = map[verdict.0 as usize],
    }
}

/// Per-block liveness facts.
struct BlockFacts {
    /// Slots read before any write in this block (upward-exposed uses).
    upward: SlotSet,
    /// Slots written in this block.
    def: SlotSet,
    live_in: SlotSet,
    live_out: SlotSet,
    /// Successor block ids.
    succs: Vec<u32>,
}

/// Re-colour `prog`'s virtual slots with liveness-based reuse, compute the
/// zero-init set, and enforce the 64-slot / 512-byte stack cap.
///
/// # Errors
/// [`BpfDiag::StackOverflow`] when even after reuse the program needs more
/// than [`MAX_SLOTS`] slots; the span points at the first value that does not
/// fit and the message reports the raw and allocated counts.
pub fn assign_slots(prog: &mut BpfProgram) -> Result<(), BpfError> {
    let n = prog.slot_types.len();
    prog.raw_slot_count = u32::try_from(n).unwrap_or(u32::MAX);
    if n == 0 {
        prog.num_slots = 0;
        prog.zero_init = Vec::new();
        return Ok(());
    }

    let facts = solve_liveness(prog, n);
    // Zero-init set: slots live into the entry block (some path reads them
    // before writing).
    let entry_zero: Vec<SlotId> = facts
        .get(&prog.entry.0)
        .map_or_else(|| SlotSet::new(n), |f| f.live_in.clone())
        .iter()
        .collect();
    let interferes = build_interference(prog, &facts, &entry_zero, n);
    let colouring = colour(prog, &interferes, n)?;

    // Rewrite the program with the physical colours.
    for block in &mut prog.blocks {
        for inst in &mut block.insts {
            remap_inst(inst, &colouring.of);
        }
        remap_term(&mut block.term, &colouring.of);
    }
    let mut zero: Vec<SlotId> = entry_zero
        .iter()
        .map(|s| colouring.of[s.0 as usize])
        .collect();
    zero.sort_unstable_by_key(|s| s.0);
    zero.dedup();
    prog.zero_init = zero;
    prog.num_slots = u32::try_from(colouring.types.len()).unwrap_or(u32::MAX);
    prog.slot_types = colouring.types;
    Ok(())
}

/// Compute per-block gen/def and solve the backwards live-in/live-out
/// fixpoint (`live_in = upward ∪ (live_out − def)`).
fn solve_liveness(prog: &BpfProgram, n: usize) -> HashMap<u32, BlockFacts> {
    let mut facts: HashMap<u32, BlockFacts> = HashMap::new();
    for block in &prog.blocks {
        let mut upward = SlotSet::new(n);
        let mut def = SlotSet::new(n);
        for inst in &block.insts {
            inst_uses(inst, |s| {
                if !def.contains(s) {
                    upward.insert(s);
                }
            });
            if let Some(d) = inst_def(inst) {
                def.insert(d);
            }
        }
        term_uses(&block.term, |s| {
            if !def.contains(s) {
                upward.insert(s);
            }
        });
        let succs = match &block.term {
            Term::Goto { target, .. } => vec![target.0],
            Term::BranchNz { t, f, .. } => vec![t.0, f.0],
            Term::Return { .. } => Vec::new(),
        };
        facts.insert(
            block.id.0,
            BlockFacts {
                upward,
                def,
                live_in: SlotSet::new(n),
                live_out: SlotSet::new(n),
                succs,
            },
        );
    }

    let block_ids: Vec<u32> = prog.blocks.iter().map(|b| b.id.0).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for id in block_ids.iter().rev() {
            let succ_ins: Vec<SlotSet> = facts[id]
                .succs
                .iter()
                .filter_map(|s| facts.get(s).map(|f| f.live_in.clone()))
                .collect();
            let fact = facts.get_mut(id).expect("fact exists");
            for si in &succ_ins {
                if fact.live_out.union_with(si) {
                    changed = true;
                }
            }
            let mut live_in = fact.upward.clone();
            for s in fact.live_out.iter() {
                if !fact.def.contains(s) {
                    live_in.insert(s);
                }
            }
            if live_in != fact.live_in {
                fact.live_in = live_in;
                changed = true;
            }
        }
    }
    facts
}

/// Build the slot interference graph: a def interferes with everything live
/// after it, and all zero-init slots mutually interfere (they are all
/// notionally defined together at entry).
fn build_interference(
    prog: &BpfProgram,
    facts: &HashMap<u32, BlockFacts>,
    entry_zero: &[SlotId],
    n: usize,
) -> Vec<SlotSet> {
    let mut interferes = vec![SlotSet::new(n); n];
    let add_edge = |g: &mut Vec<SlotSet>, a: SlotId, b: SlotId| {
        if a != b {
            g[a.0 as usize].insert(b);
            g[b.0 as usize].insert(a);
        }
    };
    for block in &prog.blocks {
        let mut live = facts[&block.id.0].live_out.clone();
        term_uses(&block.term, |s| {
            live.insert(s);
        });
        for inst in block.insts.iter().rev() {
            if let Some(d) = inst_def(inst) {
                for l in live.iter() {
                    add_edge(&mut interferes, d, l);
                }
                live.remove(d);
            }
            inst_uses(inst, |s| {
                live.insert(s);
            });
        }
    }
    for (i, a) in entry_zero.iter().enumerate() {
        for b in &entry_zero[i + 1..] {
            add_edge(&mut interferes, *a, *b);
        }
    }
    interferes
}

/// The result of greedy colouring: the physical slot each virtual slot maps
/// to, and the type of each physical slot.
struct Colouring {
    of: Vec<SlotId>,
    types: Vec<Ty>,
}

/// Greedy colouring in virtual-slot order; a colour may only be shared by
/// slots of the same type. Fails when more than [`MAX_SLOTS`] colours are
/// needed, pointing at the first value that no longer fits.
fn colour(prog: &BpfProgram, interferes: &[SlotSet], n: usize) -> Result<Colouring, BpfError> {
    let mut of: Vec<SlotId> = Vec::with_capacity(n);
    let mut types: Vec<Ty> = Vec::new();
    let mut first_overflow: Option<SlotId> = None;
    for (v, &ty) in prog.slot_types.iter().enumerate() {
        let mut taken = vec![false; types.len()];
        for neigh in interferes[v].iter() {
            let ni = neigh.0 as usize;
            if ni < v {
                taken[of[ni].0 as usize] = true;
            }
        }
        let picked = (0..types.len())
            .find(|&c| !taken[c] && types[c] == ty)
            .unwrap_or_else(|| {
                types.push(ty);
                types.len() - 1
            });
        if picked >= MAX_SLOTS && first_overflow.is_none() {
            first_overflow = Some(SlotId(u32::try_from(v).unwrap_or(u32::MAX)));
        }
        of.push(SlotId(u32::try_from(picked).unwrap_or(u32::MAX)));
    }

    if let Some(over) = first_overflow {
        // Source span for the overflowing value: the first instruction that
        // defines it.
        let span = prog
            .blocks
            .iter()
            .flat_map(|b| &b.insts)
            .find(|i| inst_def(i) == Some(over))
            .map_or_else(|| Span::empty(0), inst_span);
        return Err(BpfError::new(
            BpfDiag::StackOverflow,
            span,
            format!(
                "program needs {} stack slots even after liveness-based reuse \
                 ({n} values before reuse; the 512-byte eBPF stack holds {MAX_SLOTS}) \
                 — split the handler, or shrink the unrolled `loop`/`use` \
                 expansion that defines this value",
                types.len(),
            ),
        ));
    }
    Ok(Colouring { of, types })
}
