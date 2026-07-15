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

//! Typed lowering: a Tcl front-end CFG ([`tcl_compiler::cfg::Function`]) → typed
//! [`BpfProgram`], rejecting anything outside the DSL subset with span-anchored
//! diagnostics. This is where "typed Tcl, not dynamic Tcl" is *enforced*.

use std::collections::HashMap;

use tcl_compiler::cfg::{Block as CfgBlock, Function, Terminator};
use tcl_compiler::{BinOp, ExprNode, Statement, UnaryOp, parse_expr};
use tcl_lexer::Span;

use crate::diag::{BpfDiag, BpfError};
use crate::ir::{
    Block, BlockId, BpfProgram, CmpOp, Inst, IntBinOp, MapDef, ProgType, SlotId, Term, UnOp,
};
use crate::ty::{Region, Ty, Width};

/// 64 slots × 8 bytes = the 512-byte eBPF stack.
const MAX_SLOTS: usize = 64;

/// Lower a single CFG function to a typed [`BpfProgram`] of the given type.
///
/// # Errors
/// Returns a [`BpfError`] for any construct outside the typed DSL subset.
pub fn lower_function(func: &Function, prog_type: ProgType) -> Result<BpfProgram, BpfError> {
    // v1 supports no loops; reject any back-edge up front. Bounded loops are a
    // planned follow-on.
    if let Some(span) = first_loop_span(func) {
        return Err(BpfError::new(
            BpfDiag::UnboundedLoop,
            span,
            "loops are not supported yet — rewrite the handler without `while`/`for` (bounded loops are a follow-on)",
        ));
    }
    let mut l = Lowerer {
        func,
        prog_type,
        env: HashMap::new(),
        slot_types: Vec::new(),
        block_ids: HashMap::new(),
        lowered: HashMap::new(),
        map_index: HashMap::new(),
        map_defs: Vec::new(),
    };
    l.collect_maps()?;
    let entry = l.block_id(func.block_name(func.entry));

    // Worklist over the truncated graph: `accept`/`drop` cut a block short, so
    // only blocks reachable through the surviving terminators get lowered.
    let mut worklist = vec![func.block_name(func.entry).to_owned()];
    while let Some(name) = worklist.pop() {
        let succs = l.lower_block(&name)?;
        for s in succs {
            let sid = l.block_id(&s);
            if !l.lowered.contains_key(&sid.0) {
                worklist.push(s);
            }
        }
    }

    let mut blocks: Vec<Block> = l.lowered.into_values().collect();
    blocks.sort_by_key(|b| b.id.0);
    let num_slots = u32::try_from(l.slot_types.len()).unwrap_or(u32::MAX);
    Ok(BpfProgram {
        prog_type,
        entry,
        blocks,
        num_slots,
        slot_types: l.slot_types,
        maps: l.map_defs,
    })
}

struct Lowerer<'f> {
    func: &'f Function,
    /// Program type — selects verdict semantics (`accept`/`drop` vs `pass`/`tx`).
    prog_type: ProgType,
    /// Typed symbol table: variable name → (stable slot, type). Function-global
    /// because slots are mutable locals (no SSA).
    env: HashMap<String, (SlotId, Ty)>,
    slot_types: Vec<Ty>,
    block_ids: HashMap<String, BlockId>,
    /// Lowered blocks keyed by `BlockId.0`.
    lowered: HashMap<u32, Block>,
    /// Declared map name → dense index.
    map_index: HashMap<String, u32>,
    /// Declared maps in index order.
    map_defs: Vec<MapDef>,
}

impl Lowerer<'_> {
    fn fresh_slot(&mut self, ty: Ty, span: Span) -> Result<SlotId, BpfError> {
        if self.slot_types.len() >= MAX_SLOTS {
            return Err(BpfError::new(
                BpfDiag::StackOverflow,
                span,
                "program needs more than 64 values (the 512-byte eBPF stack is exceeded)",
            ));
        }
        let id = SlotId(u32::try_from(self.slot_types.len()).unwrap_or(u32::MAX));
        self.slot_types.push(ty);
        Ok(id)
    }

    /// Stable slot for a named variable; allocates on first use, checks the type
    /// matches on reuse.
    fn var_slot(&mut self, name: &str, ty: Ty, span: Span) -> Result<SlotId, BpfError> {
        if let Some((slot, existing)) = self.env.get(name).copied() {
            if existing != ty {
                return Err(BpfError::new(
                    BpfDiag::TypeMismatch,
                    span,
                    format!(
                        "`{name}` was already declared as {existing:?}; cannot redeclare as {ty:?}"
                    ),
                ));
            }
            return Ok(slot);
        }
        let slot = self.fresh_slot(ty, span)?;
        self.env.insert(name.to_owned(), (slot, ty));
        Ok(slot)
    }

    fn block_id(&mut self, name: &str) -> BlockId {
        if let Some(id) = self.block_ids.get(name) {
            return *id;
        }
        let id = BlockId(u32::try_from(self.block_ids.len()).unwrap_or(u32::MAX));
        self.block_ids.insert(name.to_owned(), id);
        id
    }

    /// Pre-scan every block for `map` declarations so `map_get`/`map_set` can
    /// resolve names regardless of ordering.
    fn collect_maps(&mut self) -> Result<(), BpfError> {
        let func = self.func;
        for block in func.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call {
                    command,
                    canonical_command,
                    args,
                    span,
                    ..
                } = stmt
                    && canonical_command.as_deref().unwrap_or(command.as_str()) == "map"
                {
                    self.declare_map(args, *span)?;
                }
            }
        }
        Ok(())
    }

    fn declare_map(&mut self, args: &[String], span: Span) -> Result<(), BpfError> {
        // map NAME hash KEYSZ VALSZ MAX  (the kind word is metadata in v1)
        if args.len() != 5 {
            return Err(arity(span, "map", "NAME hash KEYSZ VALSZ MAX"));
        }
        let name = args[0].clone();
        if self.map_index.contains_key(&name) {
            return Err(BpfError::new(
                BpfDiag::TypeMismatch,
                span,
                format!("map `{name}` is already declared"),
            ));
        }
        let key_size = parse_u32(&args[2]).ok_or_else(|| {
            BpfError::new(BpfDiag::BadInt, span, "map key size must be an integer")
        })?;
        let value_size = parse_u32(&args[3]).ok_or_else(|| {
            BpfError::new(BpfDiag::BadInt, span, "map value size must be an integer")
        })?;
        let max_entries = parse_u32(&args[4]).ok_or_else(|| {
            BpfError::new(BpfDiag::BadInt, span, "map max-entries must be an integer")
        })?;
        let index = u32::try_from(self.map_defs.len()).unwrap_or(u32::MAX);
        self.map_index.insert(name.clone(), index);
        self.map_defs.push(MapDef {
            name,
            index,
            key_size,
            value_size,
            max_entries,
            span,
        });
        Ok(())
    }

    fn resolve_map(&self, name: &str, span: Span) -> Result<u32, BpfError> {
        self.map_index.get(name).copied().ok_or_else(|| {
            BpfError::new(
                BpfDiag::UndefinedVar,
                span,
                format!("undefined map `{name}`"),
            )
        })
    }

    fn lower_map_get(
        &mut self,
        args: &[String],
        span: Span,
        insts: &mut Vec<Inst>,
    ) -> Result<Option<Term>, BpfError> {
        // map_get DST NAME {KEY}
        if args.len() != 3 {
            return Err(arity(span, "map_get", "DST NAME {KEY}"));
        }
        let map = self.resolve_map(&args[1], span)?;
        let key = self.lower_expr(&parse_expr(&args[2], None), insts, span)?;
        let dst = self.var_slot(&args[0], Ty::Int, span)?;
        insts.push(Inst::MapGet {
            dst,
            map,
            key,
            span,
        });
        Ok(None)
    }

    fn lower_map_set(
        &mut self,
        args: &[String],
        span: Span,
        insts: &mut Vec<Inst>,
    ) -> Result<Option<Term>, BpfError> {
        // map_set NAME {KEY} {VAL}
        if args.len() != 3 {
            return Err(arity(span, "map_set", "NAME {KEY} {VAL}"));
        }
        let map = self.resolve_map(&args[0], span)?;
        let key = self.lower_expr(&parse_expr(&args[1], None), insts, span)?;
        let val = self.lower_expr(&parse_expr(&args[2], None), insts, span)?;
        insts.push(Inst::MapSet {
            map,
            key,
            val,
            span,
        });
        Ok(None)
    }

    fn lower_load(
        &mut self,
        cmd: &str,
        args: &[String],
        span: Span,
        insts: &mut Vec<Inst>,
    ) -> Result<Option<Term>, BpfError> {
        if args.len() != 3 {
            return Err(arity(span, cmd, "DST SRC OFFSET"));
        }
        let ptr = self.expect_ctx_ptr(&args[1], span)?;
        let off_i64 = parse_int(&args[2]).ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadInt,
                span,
                format!("invalid offset `{}`", args[2]),
            )
        })?;
        let off = i32::try_from(off_i64)
            .map_err(|_| BpfError::new(BpfDiag::BadInt, span, "load offset out of range"))?;
        let width = match cmd {
            "load8" => Width::B8,
            "load16" => Width::B16,
            _ => Width::B32,
        };
        let dst = self.var_slot(&args[0], Ty::Int, span)?;
        insts.push(Inst::Load {
            dst,
            width,
            ptr,
            off,
            span,
        });
        Ok(None)
    }

    /// Lower a verdict command (`accept`/`drop`/`pass`/`tx`), validated against
    /// the program type, into a `Return`.
    fn lower_verdict(
        &mut self,
        cmd: &str,
        args: &[String],
        span: Span,
        insts: &mut Vec<Inst>,
    ) -> Result<Option<Term>, BpfError> {
        let verdict = match cmd {
            "accept" => {
                if self.prog_type != ProgType::SocketFilter {
                    return Err(BpfError::new(
                        BpfDiag::OutOfSubset,
                        span,
                        "`accept` is a socket-filter verdict; use `pass`/`drop`/`tx` for XDP",
                    ));
                }
                if args.is_empty() {
                    let dst = self.fresh_slot(Ty::Int, span)?;
                    insts.push(Inst::CtxLen { dst, span });
                    dst
                } else if args.len() == 1 {
                    self.lower_expr(&parse_expr(&args[0], None), insts, span)?
                } else {
                    return Err(arity(span, "accept", "?N?"));
                }
            }
            "pass" | "tx" => {
                if self.prog_type != ProgType::Xdp {
                    return Err(BpfError::new(
                        BpfDiag::OutOfSubset,
                        span,
                        format!(
                            "`{cmd}` is an XDP verdict; use `accept`/`drop` for socket filters"
                        ),
                    ));
                }
                if !args.is_empty() {
                    return Err(arity(span, cmd, "(no arguments)"));
                }
                let val = if cmd == "pass" { 2 } else { 3 };
                self.const_slot(val, span, insts)?
            }
            // `drop` is valid for both; the value differs by program type.
            _ => {
                if !args.is_empty() {
                    return Err(arity(span, "drop", "(no arguments)"));
                }
                let val = self.drop_verdict();
                self.const_slot(val, span, insts)?
            }
        };
        Ok(Some(Term::Return { verdict, span }))
    }

    fn const_slot(
        &mut self,
        val: i64,
        span: Span,
        insts: &mut Vec<Inst>,
    ) -> Result<SlotId, BpfError> {
        let dst = self.fresh_slot(Ty::Int, span)?;
        insts.push(Inst::Const { dst, val, span });
        Ok(dst)
    }

    /// The `drop` verdict value for this program type.
    fn drop_verdict(&self) -> i64 {
        match self.prog_type {
            ProgType::Xdp => 1, // XDP_DROP
            ProgType::SocketFilter => 0,
        }
    }

    fn expect_ctx_ptr(&self, name: &str, span: Span) -> Result<SlotId, BpfError> {
        match self.env.get(name) {
            Some((slot, Ty::Ptr(Region::Ctx))) => Ok(*slot),
            Some((_, other)) => Err(BpfError::new(
                BpfDiag::TypeMismatch,
                span,
                format!("`{name}` is {other:?}; expected a packet buffer (bind one with `setbuf`)"),
            )),
            None => Err(BpfError::new(
                BpfDiag::UndefinedVar,
                span,
                format!("undefined buffer `{name}`"),
            )),
        }
    }

    fn lower_block(&mut self, name: &str) -> Result<Vec<String>, BpfError> {
        let bid = self.block_id(name);
        if self.lowered.contains_key(&bid.0) {
            return Ok(Vec::new());
        }
        let func = self.func;
        let Some(cfg_block) = func.block_by_name(name) else {
            return Err(BpfError::new(
                BpfDiag::Internal,
                Span::empty(0),
                format!("missing CFG block `{name}`"),
            ));
        };

        let mut insts = Vec::new();
        let mut early: Option<Term> = None;
        for stmt in &cfg_block.statements {
            if let Some(t) = self.lower_stmt(stmt, &mut insts)? {
                early = Some(t);
                break;
            }
        }
        let (term, succs) = match early {
            Some(t) => (t, Vec::new()),
            None => self.lower_terminator(cfg_block, &mut insts)?,
        };
        self.lowered.insert(
            bid.0,
            Block {
                id: bid,
                insts,
                term,
            },
        );
        Ok(succs)
    }

    /// Lower one statement. Returns `Some(term)` for `accept`/`drop` (which
    /// terminate the block early), `None` otherwise.
    fn lower_stmt(
        &mut self,
        stmt: &Statement,
        insts: &mut Vec<Inst>,
    ) -> Result<Option<Term>, BpfError> {
        match stmt {
            Statement::Call {
                command,
                canonical_command,
                args,
                span,
                ..
            } => {
                let cmd = canonical_command.as_deref().unwrap_or(command.as_str());
                self.lower_call(cmd, args, *span, insts)
            }
            Statement::AssignConst { span, .. }
            | Statement::AssignValue { span, .. }
            | Statement::AssignExpr { span, .. }
            | Statement::Incr { span, .. } => Err(BpfError::new(
                BpfDiag::OutOfSubset,
                *span,
                "plain `set`/`incr` are not allowed — use typed `setint`/`seti32`/`setbuf`",
            )),
            Statement::Return { span, .. } => Err(BpfError::new(
                BpfDiag::OutOfSubset,
                *span,
                "use `accept`/`drop` to return a verdict",
            )),
            Statement::ExprEval { span, .. } => Err(BpfError::new(
                BpfDiag::OutOfSubset,
                *span,
                "a bare expression has no effect here",
            )),
            other => Err(BpfError::new(
                BpfDiag::OutOfSubset,
                statement_span(other),
                "unsupported construct in a BPF-Tcl handler",
            )),
        }
    }

    /// Truncate a computed value `tmp` into `dst` to the named integer width:
    /// `seti32` sign-extends the low 32 bits (`(v << 32) >> 32`, arithmetic), so
    /// a value overflowing 32 bits becomes its signed 32-bit truncation; `setu32`
    /// zeroes the high half (`v & ((1 << 32) - 1)`, the mask computed because a
    /// bare `0xFFFFFFFF` immediate exceeds the v1 32-bit const limit); `setint`
    /// keeps the full 64-bit value (`RUST_ISSUE_172`).
    fn emit_width_set(
        &mut self,
        cmd: &str,
        tmp: SlotId,
        dst: SlotId,
        insts: &mut Vec<Inst>,
        span: Span,
    ) -> Result<(), BpfError> {
        let mut konst = |this: &mut Self, val: i64| -> Result<SlotId, BpfError> {
            let s = this.fresh_slot(Ty::Int, span)?;
            insts.push(Inst::Const { dst: s, val, span });
            Ok(s)
        };
        match cmd {
            "seti32" => {
                let sh = konst(self, 32)?;
                let hi = self.fresh_slot(Ty::Int, span)?;
                insts.push(Inst::Bin {
                    dst: hi,
                    op: IntBinOp::Shl,
                    a: tmp,
                    b: sh,
                    span,
                });
                insts.push(Inst::Bin {
                    dst,
                    op: IntBinOp::Shr,
                    a: hi,
                    b: sh,
                    span,
                });
            }
            "setu32" => {
                let one = konst(self, 1)?;
                let sh = konst(self, 32)?;
                let hi = self.fresh_slot(Ty::Int, span)?;
                insts.push(Inst::Bin {
                    dst: hi,
                    op: IntBinOp::Shl,
                    a: one,
                    b: sh,
                    span,
                });
                let mask = self.fresh_slot(Ty::Int, span)?;
                insts.push(Inst::Bin {
                    dst: mask,
                    op: IntBinOp::Sub,
                    a: hi,
                    b: one,
                    span,
                });
                insts.push(Inst::Bin {
                    dst,
                    op: IntBinOp::And,
                    a: tmp,
                    b: mask,
                    span,
                });
            }
            _ => insts.push(Inst::Copy {
                dst,
                src: tmp,
                span,
            }),
        }
        Ok(())
    }

    fn lower_call(
        &mut self,
        cmd: &str,
        args: &[String],
        span: Span,
        insts: &mut Vec<Inst>,
    ) -> Result<Option<Term>, BpfError> {
        match cmd {
            "setint" | "seti32" | "setu32" => {
                if args.len() != 2 {
                    return Err(arity(span, cmd, "NAME {EXPR}"));
                }
                let expr = parse_expr(&args[1], None);
                let tmp = self.lower_expr(&expr, insts, span)?;
                let dst = self.var_slot(&args[0], Ty::Int, span)?;
                self.emit_width_set(cmd, tmp, dst, insts, span)?;
                Ok(None)
            }
            "setbuf" => {
                // `setbuf NAME ctx` or `setbuf NAME = ctx`.
                if args.is_empty() {
                    return Err(arity(span, "setbuf", "NAME ctx"));
                }
                if !args.iter().any(|a| a == "ctx") {
                    return Err(BpfError::new(
                        BpfDiag::OutOfSubset,
                        span,
                        "the `setbuf` source must be `ctx` (the packet) in v1",
                    ));
                }
                let dst = self.var_slot(&args[0], Ty::Ptr(Region::Ctx), span)?;
                insts.push(Inst::CtxPtr { dst, span });
                Ok(None)
            }
            "load8" | "load16" | "load32" => self.lower_load(cmd, args, span, insts),
            "pktlen" => {
                if args.len() != 2 {
                    return Err(arity(span, "pktlen", "DST SRC"));
                }
                self.expect_ctx_ptr(&args[1], span)?;
                let dst = self.var_slot(&args[0], Ty::Int, span)?;
                insts.push(Inst::CtxLen { dst, span });
                Ok(None)
            }
            "accept" | "drop" | "pass" | "tx" => self.lower_verdict(cmd, args, span, insts),
            // `map NAME hash KEYSZ VALSZ MAX` is a declaration, collected up front.
            "map" => Ok(None),
            "map_get" => self.lower_map_get(args, span, insts),
            "map_set" => self.lower_map_set(args, span, insts),
            "loop" => Err(BpfError::new(
                BpfDiag::OutOfSubset,
                span,
                "`loop` must appear at the handler/loop top level, not nested inside `if` (v1)",
            )),
            other if is_concurrency_command(other) => Err(BpfError::new(
                BpfDiag::OutOfSubset,
                span,
                format!(
                    "`{other}`: concurrency is not supported on the eBPF backend \
                     (no coroutines, threads, or event loop — an eBPF program is a \
                     single bounded run to a verdict)"
                ),
            )),
            other => Err(BpfError::new(
                BpfDiag::UnknownCommand,
                span,
                format!("unknown BPF-Tcl command `{other}`"),
            )),
        }
    }

    fn lower_expr(
        &mut self,
        node: &ExprNode,
        insts: &mut Vec<Inst>,
        span: Span,
    ) -> Result<SlotId, BpfError> {
        match node {
            ExprNode::Literal { text, .. } => {
                let val = parse_int(text).ok_or_else(|| {
                    BpfError::new(
                        BpfDiag::BadInt,
                        span,
                        format!("invalid integer literal `{text}`"),
                    )
                })?;
                let dst = self.fresh_slot(Ty::Int, span)?;
                insts.push(Inst::Const { dst, val, span });
                Ok(dst)
            }
            ExprNode::Var { name, .. } => match self.env.get(name).copied() {
                Some((slot, Ty::Int)) => Ok(slot),
                Some((_, other)) => Err(BpfError::new(
                    BpfDiag::TypeMismatch,
                    span,
                    format!("`${name}` is {other:?} and cannot be used in an integer expression"),
                )),
                None => Err(BpfError::new(
                    BpfDiag::UndefinedVar,
                    span,
                    format!("undefined variable `${name}`"),
                )),
            },
            ExprNode::Binary { op, left, right } => {
                let a = self.lower_expr(left, insts, span)?;
                let b = self.lower_expr(right, insts, span)?;
                if let Some(cmp) = map_cmp(*op) {
                    let dst = self.fresh_slot(Ty::Int, span)?;
                    insts.push(Inst::Cmp {
                        dst,
                        op: cmp,
                        a,
                        b,
                        span,
                    });
                    Ok(dst)
                } else if let Some(bin) = map_bin(*op) {
                    let dst = self.fresh_slot(Ty::Int, span)?;
                    insts.push(Inst::Bin {
                        dst,
                        op: bin,
                        a,
                        b,
                        span,
                    });
                    Ok(dst)
                } else {
                    Err(BpfError::new(
                        BpfDiag::OutOfSubset,
                        span,
                        format!(
                            "operator `{}` is not supported in BPF-Tcl (v1: arithmetic + comparison only)",
                            op.as_str()
                        ),
                    ))
                }
            }
            ExprNode::Unary { op, operand } => {
                if matches!(op, UnaryOp::Pos) {
                    return self.lower_expr(operand, insts, span);
                }
                let a = self.lower_expr(operand, insts, span)?;
                let uop = map_un(*op).ok_or_else(|| {
                    BpfError::new(
                        BpfDiag::OutOfSubset,
                        span,
                        format!("unary `{}` is not supported", op.as_str()),
                    )
                })?;
                let dst = self.fresh_slot(Ty::Int, span)?;
                insts.push(Inst::Un {
                    dst,
                    op: uop,
                    a,
                    span,
                });
                Ok(dst)
            }
            ExprNode::String { .. }
            | ExprNode::Command { .. }
            | ExprNode::Ternary { .. }
            | ExprNode::Call { .. }
            | ExprNode::Raw { .. } => Err(BpfError::new(
                BpfDiag::OutOfSubset,
                span,
                "only integer expressions over typed variables are allowed in BPF-Tcl",
            )),
        }
    }

    fn lower_terminator(
        &mut self,
        block: &CfgBlock,
        insts: &mut Vec<Inst>,
    ) -> Result<(Term, Vec<String>), BpfError> {
        match &block.terminator {
            Some(Terminator::Goto { target, span }) => {
                let name = self.func.block_name(*target).to_owned();
                let t = self.block_id(&name);
                Ok((
                    Term::Goto {
                        target: t,
                        span: span.unwrap_or_else(|| Span::empty(0)),
                    },
                    vec![name],
                ))
            }
            Some(Terminator::Branch {
                condition,
                true_target,
                false_target,
                span,
                ..
            }) => {
                let sp = span.unwrap_or_else(|| Span::empty(0));
                let cond = self.lower_expr(condition, insts, sp)?;
                let tname = self.func.block_name(*true_target).to_owned();
                let fname = self.func.block_name(*false_target).to_owned();
                let t = self.block_id(&tname);
                let f = self.block_id(&fname);
                Ok((
                    Term::BranchNz {
                        cond,
                        t,
                        f,
                        span: sp,
                    },
                    vec![tname, fname],
                ))
            }
            // A fall-through with no explicit verdict defaults to the program's
            // `drop` value (0 for socket filters, XDP_DROP for XDP).
            Some(Terminator::Return { span, .. }) => {
                let sp = span.unwrap_or_else(|| Span::empty(0));
                let val = self.drop_verdict();
                let dst = self.const_slot(val, sp, insts)?;
                Ok((
                    Term::Return {
                        verdict: dst,
                        span: sp,
                    },
                    Vec::new(),
                ))
            }
            None => {
                let sp = Span::empty(0);
                let val = self.drop_verdict();
                let dst = self.const_slot(val, sp, insts)?;
                Ok((
                    Term::Return {
                        verdict: dst,
                        span: sp,
                    },
                    Vec::new(),
                ))
            }
        }
    }
}

/// The primary span of any statement variant.
fn statement_span(s: &Statement) -> Span {
    use Statement::{
        AssignConst, AssignExpr, AssignValue, Barrier, Block, Call, Catch, ExprEval, For, Foreach,
        If, Incr, Return, Switch, Try, UpFrame, While,
    };
    match s {
        AssignConst { span, .. }
        | AssignExpr { span, .. }
        | AssignValue { span, .. }
        | Incr { span, .. }
        | ExprEval { span, .. }
        | Call { span, .. }
        | Return { span, .. }
        | Barrier { span, .. }
        | Block { span, .. }
        | UpFrame { span, .. }
        | If { span, .. }
        | For { span, .. }
        | While { span, .. }
        | Foreach { span, .. }
        | Catch { span, .. }
        | Try { span, .. }
        | Switch { span, .. } => *span,
    }
}

fn arity(span: Span, cmd: &str, usage: &str) -> BpfError {
    BpfError::new(
        BpfDiag::BadArity,
        span,
        format!("`{cmd}` expects: {cmd} {usage}"),
    )
}

/// Whether `cmd` is a Tcl concurrency primitive — coroutines
/// (`coroutine`/`yield`/`yieldto`/`coroinject`/`coroprobe`) or the `thread`
/// package (`thread::*`, `tsv::*`, `tpool::*`). None can exist on eBPF (a program
/// is a single bounded run to a verdict), so they earn a specific `OutOfSubset`
/// diagnostic rather than the generic "unknown command". (The event loop —
/// `after`/`vwait`/`update` — is rejected earlier by the typed front-end as an
/// out-of-subset construct.)
fn is_concurrency_command(cmd: &str) -> bool {
    matches!(
        cmd,
        "coroutine" | "yield" | "yieldto" | "coroinject" | "coroprobe"
    ) || cmd.starts_with("thread::")
        || cmd.starts_with("tsv::")
        || cmd.starts_with("tpool::")
}

fn map_bin(op: BinOp) -> Option<IntBinOp> {
    Some(match op {
        BinOp::Add => IntBinOp::Add,
        BinOp::Sub => IntBinOp::Sub,
        BinOp::Mul => IntBinOp::Mul,
        BinOp::Div => IntBinOp::Div,
        BinOp::Mod => IntBinOp::Mod,
        BinOp::BitAnd => IntBinOp::And,
        BinOp::BitOr => IntBinOp::Or,
        BinOp::BitXor => IntBinOp::Xor,
        BinOp::LShift => IntBinOp::Shl,
        BinOp::RShift => IntBinOp::Shr,
        _ => return None,
    })
}

fn map_cmp(op: BinOp) -> Option<CmpOp> {
    Some(match op {
        BinOp::Eq => CmpOp::Eq,
        BinOp::Ne => CmpOp::Ne,
        BinOp::Lt => CmpOp::Lt,
        BinOp::Le => CmpOp::Le,
        BinOp::Gt => CmpOp::Gt,
        BinOp::Ge => CmpOp::Ge,
        _ => return None,
    })
}

fn map_un(op: UnaryOp) -> Option<UnOp> {
    Some(match op {
        UnaryOp::Neg => UnOp::Neg,
        UnaryOp::Not => UnOp::Not,
        UnaryOp::BitNot => UnOp::BitNot,
        _ => return None,
    })
}

/// Parse a Tcl integer literal: decimal, `0x` hex, or `0b` binary, with an
/// optional leading sign.
fn parse_int(text: &str) -> Option<i64> {
    let t = text.trim();
    let (neg, rest) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let v = if let Some(h) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        i64::from_str_radix(h, 16).ok()?
    } else if let Some(b) = rest.strip_prefix("0b").or_else(|| rest.strip_prefix("0B")) {
        i64::from_str_radix(b, 2).ok()?
    } else {
        rest.parse::<i64>().ok()?
    };
    Some(if neg { -v } else { v })
}

/// Parse a non-negative integer literal into a `u32`.
fn parse_u32(s: &str) -> Option<u32> {
    parse_int(s).and_then(|v| u32::try_from(v).ok())
}

/// The span of some loop in `func`, if it contains a cycle (back-edge).
fn first_loop_span(func: &Function) -> Option<Span> {
    if !has_cycle(func) {
        return None;
    }
    Some(
        func.loop_nodes
            .values()
            .next()
            .map_or_else(|| Span::empty(0), |n| n.span),
    )
}

/// Iterative DFS back-edge detection over the CFG.
fn has_cycle(func: &Function) -> bool {
    enum Step {
        Enter(String),
        Exit(String),
    }
    // 1 = on the current path (gray), 2 = fully explored (black).
    let mut color: HashMap<String, u8> = HashMap::new();
    let mut stack = vec![Step::Enter(func.block_name(func.entry).to_owned())];
    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(node) => {
                if color.get(&node).copied() == Some(2) {
                    continue;
                }
                color.insert(node.clone(), 1);
                stack.push(Step::Exit(node.clone()));
                let succs = func
                    .block_id(&node)
                    .map(|id| func.block_successors(id))
                    .unwrap_or_default();
                for succ_id in succs {
                    let succ = func.block_name(succ_id).to_owned();
                    match color.get(&succ).copied() {
                        Some(1) => return true,
                        Some(2) => {}
                        _ => stack.push(Step::Enter(succ)),
                    }
                }
            }
            Step::Exit(node) => {
                color.insert(node, 2);
            }
        }
    }
    false
}
