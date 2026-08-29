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
//!
//! Dispatch is **registry-described**: every BPF-Tcl command's spec carries a
//! typed [`BpfOpSpec`](tcl_registry::bpf_op::BpfOpSpec) descriptor (scalar
//! width, packet-load width, map role, verdict family + compatible program
//! types), and this lowerer matches on the descriptor — never on the command
//! name — so registry, lowering, capability policy, and documentation cannot
//! drift (issue #1202).

use std::collections::HashMap;

use tcl_compiler::cfg::{Block as CfgBlock, Function, Terminator};
use tcl_compiler::{BinOp, ExprNode, Statement, UnaryOp, parse_expr};
use tcl_lexer::Span;
use tcl_registry::Traits;
use tcl_registry::bpf_op::{
    BpfDeclKind, BpfOpKind, BpfProgTypeSet, BpfScalarWidth, BpfVerdictKind,
};
use tcl_registry::registry::CommandRegistry;

use crate::alloc::assign_slots;
use crate::diag::{BpfDiag, BpfError};
use crate::ir::{
    Block, BlockId, BpfProgram, CmpOp, Inst, IntBinOp, MapConcurrency, MapDef, MapKind, OobAction,
    ProgType, SlotId, Term, UnOp,
};
use crate::ty::{ByteOrder, Region, Ty, Width};
use tcl_syntax::number::{Number, NumberSyntax, ParseFlags, parse_whole_with};

/// Guard against runaway macro/loop expansion during lowering. The *real*
/// stack cap (64 slots / 512 bytes) is enforced after liveness-based slot
/// reuse by [`crate::alloc::assign_slots`]; this raw cap only exists so a
/// pathological expansion fails fast instead of allocating without bound.
const MAX_RAW_SLOTS: usize = 4096;

/// Lower a single CFG function to a typed [`BpfProgram`] of the given type.
///
/// `registry` must be the profile-stamped BPF registry: its `BpfOpSpec`
/// descriptors drive command dispatch and its Tcl 9.0 embedding supplies the
/// numeric grammar. Slot allocation (liveness-based reuse + the 512-byte stack
/// cap) runs as part of lowering, so the returned program is ready for any
/// codegen target.
///
/// # Errors
/// Returns a [`BpfError`] for any construct outside the typed DSL subset, a
/// handler path with no explicit verdict, or a program that exceeds the eBPF
/// stack even after slot reuse.
pub fn lower_function(
    func: &Function,
    prog_type: ProgType,
    registry: &CommandRegistry,
) -> Result<BpfProgram, BpfError> {
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
        registry,
        // Numerals in the DSL follow the grammar of the dialect this registry
        // was loaded for — the same compile-target rule the bytecode backend
        // uses. With no profile loaded, fall back to the grammar the process
        // was built for.
        numbers: registry.numbers(),
        expr_dialect: registry.profile().map(|profile| profile.name),
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
    let mut program = BpfProgram {
        prog_type,
        entry,
        blocks,
        num_slots,
        slot_types: l.slot_types,
        raw_slot_count: num_slots,
        zero_init: Vec::new(),
        maps: l.map_defs,
    };
    assign_slots(&mut program)?;
    Ok(program)
}

struct Lowerer<'f> {
    func: &'f Function,
    /// The profile-stamped BPF command registry whose `BpfOpSpec` descriptors
    /// drive all command dispatch.
    registry: &'f CommandRegistry,
    /// The numeric-literal grammar of the dialect being compiled for — every
    /// integer literal in the DSL is read through it.
    numbers: NumberSyntax,
    /// Named target profile for expression parsing. Unlike an unpinned parser,
    /// this cannot inherit the ambient grammar of an interpreter that happened
    /// to run earlier on the current thread.
    expr_dialect: Option<&'static str>,
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
        if self.slot_types.len() >= MAX_RAW_SLOTS {
            return Err(BpfError::new(
                BpfDiag::StackOverflow,
                span,
                format!(
                    "program computes more than {MAX_RAW_SLOTS} values — \
                     shrink the unrolled `loop`/`use` expansion"
                ),
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

    /// The registry descriptor for a command, if it is a BPF-Tcl command.
    fn bpf_op(&self, cmd: &str) -> Option<&'static tcl_registry::bpf_op::BpfOpSpec> {
        self.registry.bpf_op(cmd)
    }

    /// Whether this concrete invocation belongs to Tcl's coroutine family.
    ///
    /// The registry owns family membership and canonical alias resolution;
    /// the BPF consumer owns only its backend policy that the family cannot be
    /// represented by a single bounded kernel-program invocation.
    fn is_coroutine_primitive(&self, cmd: &str, args: &[String]) -> bool {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.registry
            .invocation_traits(cmd, &args, self.registry.own_surface_query())
            .contains(Traits::COROUTINE_PRIMITIVE)
    }

    /// Pre-scan every block for `map` declarations so `map_get`/`map_set`/
    /// `map_has` can resolve names regardless of ordering.
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
                {
                    let cmd = canonical_command.as_deref().unwrap_or(command.as_str());
                    if self.bpf_op(cmd).map(|op| op.kind) == Some(BpfOpKind::MapDeclare) {
                        self.declare_map(args, *span)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn declare_map(&mut self, args: &[String], span: Span) -> Result<(), BpfError> {
        // map NAME hash|array KEYSZ VALSZ MAX ?shared|percpu?
        if args.len() < 5 || args.len() > 6 {
            return Err(arity(
                span,
                "map",
                "NAME hash|array KEYSZ VALSZ MAX ?shared|percpu?",
            ));
        }
        let name = args[0].clone();
        if self.map_index.contains_key(&name) {
            return Err(BpfError::new(
                BpfDiag::BadMap,
                span,
                format!("map `{name}` is already declared"),
            ));
        }
        let kind = MapKind::parse(&args[1]).ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadMap,
                span,
                format!("unknown map kind `{}` (want hash or array)", args[1]),
            )
        })?;
        let key_size = parse_size(&args[2], self.numbers).ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadMap,
                span,
                "map key size must be 1, 2, 4, or 8 bytes (keys are passed by value)",
            )
        })?;
        let value_size = parse_size(&args[3], self.numbers).ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadMap,
                span,
                "map value size must be 1, 2, 4, or 8 bytes (values are passed by value)",
            )
        })?;
        if kind == MapKind::Array && key_size != 4 {
            return Err(BpfError::new(
                BpfDiag::BadMap,
                span,
                "an array map's key is a 4-byte index (declare key size 4)",
            ));
        }
        let max_entries = parse_u32(&args[4], self.numbers)
            .filter(|m| *m >= 1)
            .ok_or_else(|| {
                BpfError::new(
                    BpfDiag::BadMap,
                    span,
                    "map max-entries must be a positive integer",
                )
            })?;
        let concurrency = match args.get(5) {
            None => MapConcurrency::default(),
            Some(word) => MapConcurrency::parse(word).ok_or_else(|| {
                BpfError::new(
                    BpfDiag::BadMap,
                    span,
                    format!("unknown map concurrency `{word}` (want shared or percpu)"),
                )
            })?,
        };
        let index = u32::try_from(self.map_defs.len()).unwrap_or(u32::MAX);
        self.map_index.insert(name.clone(), index);
        self.map_defs.push(MapDef {
            name,
            index,
            kind,
            key_size,
            value_size,
            max_entries,
            concurrency,
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
        let key = self.lower_expr(&self.parse_expr(&args[2]), insts, span)?;
        let dst = self.var_slot(&args[0], Ty::Int, span)?;
        insts.push(Inst::MapGet {
            dst,
            map,
            key,
            span,
        });
        Ok(None)
    }

    fn lower_map_has(
        &mut self,
        args: &[String],
        span: Span,
        insts: &mut Vec<Inst>,
    ) -> Result<Option<Term>, BpfError> {
        // map_has DST NAME {KEY}
        if args.len() != 3 {
            return Err(arity(span, "map_has", "DST NAME {KEY}"));
        }
        let map = self.resolve_map(&args[1], span)?;
        let key = self.lower_expr(&self.parse_expr(&args[2]), insts, span)?;
        let dst = self.var_slot(&args[0], Ty::Int, span)?;
        insts.push(Inst::MapHas {
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
        let key = self.lower_expr(&self.parse_expr(&args[1]), insts, span)?;
        let val = self.lower_expr(&self.parse_expr(&args[2]), insts, span)?;
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
        width_bits: u8,
        args: &[String],
        span: Span,
        insts: &mut Vec<Inst>,
    ) -> Result<Option<Term>, BpfError> {
        if args.len() < 3 || args.len() > 4 {
            return Err(arity(span, cmd, "DST SRC OFFSET ?be|le|native?"));
        }
        let ptr = self.expect_ctx_ptr(&args[1], span)?;
        let off_i64 = parse_int(&args[2], self.numbers).ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadInt,
                span,
                format!("invalid offset `{}`", args[2]),
            )
        })?;
        let off = i32::try_from(off_i64)
            .map_err(|_| BpfError::new(BpfDiag::BadInt, span, "load offset out of range"))?;
        if off < 0 {
            return Err(BpfError::new(
                BpfDiag::BadInt,
                span,
                "load offset must be non-negative",
            ));
        }
        let order = match args.get(3) {
            None => ByteOrder::Native,
            Some(word) => ByteOrder::parse(word).ok_or_else(|| {
                BpfError::new(
                    BpfDiag::OutOfSubset,
                    span,
                    format!("unknown byte order `{word}` (want be, le, or native)"),
                )
            })?,
        };
        let width = match width_bits {
            8 => Width::B8,
            16 => Width::B16,
            _ => Width::B32,
        };
        let dst = self.var_slot(&args[0], Ty::Int, span)?;
        insts.push(Inst::Load {
            dst,
            width,
            ptr,
            off,
            order,
            oob: OobAction::ReturnVerdict(self.drop_verdict()),
            span,
        });
        Ok(None)
    }

    /// Lower a verdict command, validated against the program type, into a
    /// `Return`.
    fn lower_verdict(
        &mut self,
        cmd: &str,
        kind: BpfVerdictKind,
        prog_types: BpfProgTypeSet,
        args: &[String],
        span: Span,
        insts: &mut Vec<Inst>,
    ) -> Result<Option<Term>, BpfError> {
        let compatible = prog_types.contains(crate::event::registry_prog_type(self.prog_type));
        if !compatible {
            let hint = match self.prog_type {
                ProgType::SocketFilter => "use `accept`/`drop` for socket filters",
                ProgType::Xdp => "use `pass`/`drop`/`tx` for XDP",
                ProgType::TcIngress | ProgType::TcEgress => "use `pass`/`drop` for TC",
                ProgType::CgroupInet4Connect | ProgType::CgroupInet4Bind => {
                    "use `pass`/`drop` for a cgroup sock-addr hook"
                }
            };
            return Err(BpfError::new(
                BpfDiag::OutOfSubset,
                span,
                format!("`{cmd}` is not a valid verdict for this program type; {hint}"),
            ));
        }
        let verdict = match kind {
            BpfVerdictKind::Accept => {
                if args.is_empty() {
                    let dst = self.fresh_slot(Ty::Int, span)?;
                    insts.push(Inst::CtxLen { dst, span });
                    dst
                } else if args.len() == 1 {
                    self.lower_expr(&self.parse_expr(&args[0]), insts, span)?
                } else {
                    return Err(arity(span, cmd, "?N?"));
                }
            }
            // `pass` is valid for XDP, TC, and the cgroup sock-addr hooks; the
            // compatibility gate above already rejected it elsewhere. `tx` stays
            // XDP-only (`XDP_TX`), so it needs no program-type branch.
            BpfVerdictKind::Pass => {
                if !args.is_empty() {
                    return Err(arity(span, cmd, "(no arguments)"));
                }
                self.const_slot(self.pass_verdict(), span, insts)?
            }
            BpfVerdictKind::Tx => {
                if !args.is_empty() {
                    return Err(arity(span, cmd, "(no arguments)"));
                }
                self.const_slot(3, span, insts)? // XDP_TX
            }
            // `drop` is valid for both; the value differs by program type.
            BpfVerdictKind::Drop => {
                if !args.is_empty() {
                    return Err(arity(span, cmd, "(no arguments)"));
                }
                let val = self.drop_verdict();
                self.const_slot(val, span, insts)?
            }
            // `next` — the explicit non-terminal continuation. The handler
            // returns the reserved continuation sentinel; the composition model
            // reads it as "run the next handler" (issue #1204).
            BpfVerdictKind::Next => {
                if !args.is_empty() {
                    return Err(arity(span, cmd, "(no arguments)"));
                }
                self.const_slot(crate::ir::CONTINUE_SENTINEL, span, insts)?
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
            // A socket filter's `drop` and a cgroup sock-addr's `deny` share the
            // same wire value (0), coincidentally.
            ProgType::SocketFilter | ProgType::CgroupInet4Connect | ProgType::CgroupInet4Bind => 0,
            ProgType::TcIngress | ProgType::TcEgress => 2, // TC_ACT_SHOT
        }
    }

    /// The `pass` verdict value for this program type (not called for a
    /// socket filter, which has no `pass` verb — `accept` covers that role).
    fn pass_verdict(&self) -> i64 {
        match self.prog_type {
            ProgType::Xdp => 2,                                            // XDP_PASS
            ProgType::TcIngress | ProgType::TcEgress => 0,                 // TC_ACT_OK
            ProgType::CgroupInet4Connect | ProgType::CgroupInet4Bind => 1, // allow
            ProgType::SocketFilter => unreachable!(
                "socket filters have no `pass` verdict (BpfProgTypeSet::PASS_LIKE excludes it); \
                 the compatibility gate in lower_verdict rejects it before this is called"
            ),
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

    /// Lower one statement. Returns `Some(term)` for a verdict (which
    /// terminates the block early), `None` otherwise.
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
            Statement::Barrier {
                command,
                canonical_command,
                args,
                span,
                ..
            } if self.is_coroutine_primitive(
                canonical_command.as_deref().unwrap_or(command.as_str()),
                args,
            ) =>
            {
                let cmd = canonical_command.as_deref().unwrap_or(command.as_str());
                Err(concurrency_error(cmd, *span))
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

    /// Truncate a computed value `tmp` into `dst` to the descriptor's integer
    /// width: [`BpfScalarWidth::I32SignExtended`] sign-extends the low 32 bits
    /// (`(v << 32) >> 32`, arithmetic), so a value overflowing 32 bits becomes
    /// its signed 32-bit truncation; [`BpfScalarWidth::U32ZeroExtended`] zeroes
    /// the high half (`v & 0xFFFF_FFFF`); [`BpfScalarWidth::I64`] keeps the
    /// full 64-bit value.
    fn emit_width_set(
        &mut self,
        width: BpfScalarWidth,
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
        match width {
            BpfScalarWidth::I32SignExtended => {
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
            BpfScalarWidth::U32ZeroExtended => {
                let mask = konst(self, 0xFFFF_FFFF)?;
                insts.push(Inst::Bin {
                    dst,
                    op: IntBinOp::And,
                    a: tmp,
                    b: mask,
                    span,
                });
            }
            BpfScalarWidth::I64 => insts.push(Inst::Copy {
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
        let Some(op) = self.bpf_op(cmd) else {
            if self.is_coroutine_primitive(cmd, args) || is_unregistered_thread_command(cmd) {
                return Err(concurrency_error(cmd, span));
            }
            return Err(BpfError::new(
                BpfDiag::UnknownCommand,
                span,
                format!("unknown BPF-Tcl command `{cmd}`"),
            ));
        };
        match op.kind {
            BpfOpKind::ScalarSet(width) => {
                if args.len() != 2 {
                    return Err(arity(span, cmd, "NAME {EXPR}"));
                }
                let expr = self.parse_expr(&args[1]);
                let tmp = self.lower_expr(&expr, insts, span)?;
                let dst = self.var_slot(&args[0], Ty::Int, span)?;
                self.emit_width_set(width, tmp, dst, insts, span)?;
                Ok(None)
            }
            BpfOpKind::BindPacket => {
                // `setbuf NAME ctx` or `setbuf NAME = ctx`.
                if args.is_empty() {
                    return Err(arity(span, cmd, "NAME ctx"));
                }
                if !args.iter().any(|a| a == "ctx") {
                    return Err(BpfError::new(
                        BpfDiag::OutOfSubset,
                        span,
                        "the `setbuf` source must be `ctx` (the packet) in v1",
                    ));
                }
                // The cgroup sock-addr hooks' context (`bpf_sock_addr`) has no
                // packet body — no `data`/`data_end` pointers for `setbuf`'s
                // prologue to load, and codegen's `ctx_layout` has nothing
                // meaningful to offer them (issue #1310). Named context-field
                // reads (`user_ip4`/`user_port`/`family`, already registry
                // schema — `BpfCtxField`) are not wired to a DSL accessor for
                // *any* program type yet, so there is no packet-free
                // context-reading path today either; reject cleanly rather
                // than emit a prologue that reads past the struct.
                if self.prog_type.is_cgroup_sock_addr() {
                    return Err(BpfError::new(
                        BpfDiag::OutOfSubset,
                        span,
                        "`setbuf` binds a packet buffer, but a cgroup sock-addr hook has no \
                         packet — this program type has no packet-based subset yet",
                    ));
                }
                let dst = self.var_slot(&args[0], Ty::Ptr(Region::Ctx), span)?;
                insts.push(Inst::CtxPtr { dst, span });
                Ok(None)
            }
            BpfOpKind::PacketLoad { width_bits } => {
                self.lower_load(cmd, width_bits, args, span, insts)
            }
            BpfOpKind::PacketLen => {
                if args.len() != 2 {
                    return Err(arity(span, cmd, "DST SRC"));
                }
                self.expect_ctx_ptr(&args[1], span)?;
                let dst = self.var_slot(&args[0], Ty::Int, span)?;
                insts.push(Inst::CtxLen { dst, span });
                Ok(None)
            }
            BpfOpKind::Verdict(kind) => {
                self.lower_verdict(cmd, kind, op.prog_types, args, span, insts)
            }
            // A `map` declaration was collected up front.
            BpfOpKind::MapDeclare => Ok(None),
            BpfOpKind::MapGet => self.lower_map_get(args, span, insts),
            BpfOpKind::MapHas => self.lower_map_has(args, span, insts),
            BpfOpKind::MapSet => self.lower_map_set(args, span, insts),
            BpfOpKind::LoopMacro => Err(BpfError::new(
                BpfDiag::OutOfSubset,
                span,
                "`loop` must appear at the handler/loop top level, not nested inside `if` (v1)",
            )),
            BpfOpKind::Framework(decl) => Err(BpfError::new(
                BpfDiag::OutOfSubset,
                span,
                framework_in_handler_message(cmd, decl),
            )),
        }
    }

    /// Parse a BPF expression under the profile stamped on this registry.
    fn parse_expr(&self, source: &str) -> ExprNode {
        parse_expr(source, self.expr_dialect)
    }

    fn lower_expr(
        &mut self,
        node: &ExprNode,
        insts: &mut Vec<Inst>,
        span: Span,
    ) -> Result<SlotId, BpfError> {
        match node {
            ExprNode::Literal { text, .. } => {
                let val = parse_int(text, self.numbers).ok_or_else(|| {
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
            // Control reaches the end of the handler with no explicit
            // verdict. Never synthesize one silently — this is a hard error
            // (issue #1202: "define whether verdict-less handlers are an
            // error … never synthesize it silently").
            Some(Terminator::Return { span, .. }) => {
                Err(self.missing_verdict(span.unwrap_or_else(|| Span::empty(0))))
            }
            None => Err(self.missing_verdict(Span::empty(0))),
        }
    }

    fn missing_verdict(&self, span: Span) -> BpfError {
        let verdicts = match self.prog_type {
            ProgType::SocketFilter => "`accept`/`drop`",
            ProgType::Xdp => "`pass`/`drop`/`tx`",
            ProgType::TcIngress
            | ProgType::TcEgress
            | ProgType::CgroupInet4Connect
            | ProgType::CgroupInet4Bind => "`pass`/`drop`",
        };
        BpfError::new(
            BpfDiag::MissingVerdict,
            span,
            format!(
                "control can reach the end of the handler without a verdict — \
                 end every path with an explicit verdict ({verdicts})"
            ),
        )
    }
}

/// The diagnostic for a framework declaration reaching the typed lowering
/// (i.e. written inside a handler body where it has no meaning).
fn framework_in_handler_message(cmd: &str, decl: BpfDeclKind) -> String {
    match decl {
        BpfDeclKind::Field => format!(
            "`{cmd}` declarations belong inside a `profile NAME {{ … }}` body, \
             not in a handler"
        ),
        BpfDeclKind::Use => format!(
            "`{cmd}` could not be expanded here — template expansion happens \
             before lowering, so this `use` is malformed or out of place"
        ),
        _ => format!(
            "`{cmd}` is a framework declaration — it must appear at the file \
             top level, not inside a handler"
        ),
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

fn concurrency_error(cmd: &str, span: Span) -> BpfError {
    BpfError::new(
        BpfDiag::OutOfSubset,
        span,
        format!(
            "`{cmd}`: concurrency is not supported on the eBPF backend \
             (no coroutines, threads, or event loop — an eBPF program is a \
             single bounded run to a verdict)"
        ),
    )
}

/// Whether `cmd` names the optional Thread package, which has no registry pack
/// yet. Core Tcl coroutine membership is deliberately registry-owned above;
/// this narrow prefix test is only a diagnostic hint for otherwise unknown
/// extension commands and must not drive lowering semantics.
fn is_unregistered_thread_command(cmd: &str) -> bool {
    let cmd = cmd.trim_start_matches(':');
    cmd.starts_with("thread::") || cmd.starts_with("tsv::") || cmd.starts_with("tpool::")
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

/// Parse a Tcl integer literal that must fit a wide, under `numbers` — the
/// numeric grammar of the dialect this program is being compiled for.
///
/// Delegates to the toolchain's single number facility
/// ([`tcl_syntax::number`]) rather than hand-rolling radix handling, so the DSL
/// reads a numeral exactly as the compiler, VM and analyser would: the same
/// prefixes (`0x`/`0o`/`0b`/`0d`, each gated on the release that introduced
/// it), the same octal-by-leading-zero rule, and the same `_` separators.
/// `integer_only` makes a fractional part trailing junk, and a magnitude past
/// `i64` is rejected here because every BPF operand is a wide or narrower.
pub(crate) fn parse_int(text: &str, numbers: NumberSyntax) -> Option<i64> {
    let flags = ParseFlags {
        integer_only: true,
        ..ParseFlags::for_syntax(numbers)
    };
    match parse_whole_with(text, flags)? {
        Number::Int(v) => Some(v),
        // Bignum, float and NaN literals have no eBPF representation.
        Number::Big { .. } | Number::Double(_) | Number::Nan { .. } => None,
    }
}

/// Parse a non-negative integer literal into a `u32`.
fn parse_u32(s: &str, numbers: NumberSyntax) -> Option<u32> {
    parse_int(s, numbers).and_then(|v| u32::try_from(v).ok())
}

/// Parse a by-value map key/value size: 1, 2, 4, or 8 bytes.
fn parse_size(s: &str, numbers: NumberSyntax) -> Option<u32> {
    parse_u32(s, numbers).filter(|v| matches!(v, 1 | 2 | 4 | 8))
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

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::expr::operators::{ALL_BIN_OPS, ALL_UNARY_OPS};

    /// BPF-Tcl v1's documented scope (see `map_bin`/`map_cmp`'s "arithmetic +
    /// comparison only" error text): plain arithmetic, bitwise, shift, and
    /// C-style numeric comparison. Everything else — `**` (no integer BPF
    /// instruction), `&&`/`||` (short-circuit boolean logic, not modelled as
    /// an int-producing instruction here), every string-typed operator
    /// (`eq`/`ne`/TIP 461 `lt`/`le`/`gt`/`ge`), list membership (`in`/`ni`),
    /// and every iRules word operator — is out of subset by design, not a
    /// coverage gap.
    const SUPPORTED_BINOPS: &[BinOp] = &[
        BinOp::Add,
        BinOp::Sub,
        BinOp::Mul,
        BinOp::Div,
        BinOp::Mod,
        BinOp::BitAnd,
        BinOp::BitOr,
        BinOp::BitXor,
        BinOp::LShift,
        BinOp::RShift,
        BinOp::Eq,
        BinOp::Ne,
        BinOp::Lt,
        BinOp::Le,
        BinOp::Gt,
        BinOp::Ge,
    ];

    /// `UnaryOp::Pos` is a no-op pass-through in `lower_expr` before
    /// `map_un` is ever consulted, so it deliberately appears in neither
    /// this list nor `map_un`'s own match arms — it always "lowers"
    /// without being counted as one of `map_un`'s supported variants.
    const SUPPORTED_UNARYOPS: &[UnaryOp] = &[UnaryOp::Neg, UnaryOp::Not, UnaryOp::BitNot];

    /// Issue #983 (Phase I, bpf-tcl-ir completeness): `map_bin`/`map_cmp`
    /// already fall back to a wildcard `_ => None` rather than an
    /// exhaustive match, so a newly added `BinOp` variant can never panic
    /// here by construction — `lower_expr` funnels every `None` result
    /// into `BpfDiag::OutOfSubset` (BPF001) unconditionally, regardless of
    /// which operator produced it. What this test pins down is the
    /// *intentional* v1 scope itself: every operator in `SUPPORTED_BINOPS`
    /// must actually lower, every operator not in it (including every
    /// operator `tcl-syntax` currently defines, so a future addition
    /// defaults to correctly-diagnosed rather than silently mishandled)
    /// must be rejected, and `map_cmp`/`map_bin` must never both claim the
    /// same operator — `lower_expr`'s `if let ... else if let ...` would
    /// otherwise resolve that silently by picking whichever runs first.
    #[test]
    fn every_binop_variant_matches_the_documented_v1_subset_exactly() {
        for op in ALL_BIN_OPS.iter().copied() {
            let cmp = map_cmp(op);
            let bin = map_bin(op);
            assert!(
                !(cmp.is_some() && bin.is_some()),
                "{op:?} matched both map_cmp and map_bin"
            );
            let supported = cmp.is_some() || bin.is_some();
            let expected = SUPPORTED_BINOPS.contains(&op);
            assert_eq!(
                supported, expected,
                "{op:?}: map_cmp/map_bin support {supported}, expected {expected}"
            );
        }
    }

    #[test]
    fn every_unaryop_variant_matches_the_documented_v1_subset_exactly() {
        for op in ALL_UNARY_OPS.iter().copied() {
            if matches!(op, UnaryOp::Pos) {
                continue;
            }
            let supported = map_un(op).is_some();
            let expected = SUPPORTED_UNARYOPS.contains(&op);
            assert_eq!(
                supported, expected,
                "{op:?}: map_un support {supported}, expected {expected}"
            );
        }
    }

    /// End-to-end companion to the direct `map_*` checks above: an
    /// unsupported operator reaching `lower_expr` through a real compile
    /// (not just a direct function call) surfaces as `BpfDiag::OutOfSubset`
    /// — never a panic, and never some other diagnostic code that would
    /// suggest the rejection path itself is broken.
    #[test]
    fn out_of_subset_binop_in_a_real_program_is_rejected_not_panicked() {
        let src = "when SOCKET_FILTER {\n  setbuf pkt ctx\n  pktlen len pkt\n  if {$len eq 36} { accept }\n  drop\n}\n";
        let err = crate::frontend::compile_module(src).unwrap_err();
        assert_eq!(err.code, BpfDiag::OutOfSubset);
    }

    #[test]
    fn coroutine_barrier_uses_registry_identity_for_specific_diagnostic() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let mut func = Function::new("::handler", "entry");
        let entry = func.entry;
        func.blocks
            .get_mut(&entry)
            .expect("entry block")
            .statements
            .push(Statement::Barrier {
                span: Span::new(4, 11),
                reason: "opaque callback".into(),
                command: "local_yield".into(),
                canonical_command: Some("yield".into()),
                args: vec!["value".into()],
                tokens: None,
            });

        let err = lower_function(&func, ProgType::SocketFilter, registry)
            .expect_err("a coroutine barrier cannot lower to one bounded BPF run");
        assert_eq!(err.code, BpfDiag::OutOfSubset);
        assert_eq!(err.span, Span::new(4, 11));
        assert!(err.msg.contains("`yield`: concurrency is not supported"));
    }

    /// Integer literals in the DSL come from the toolchain's one number
    /// facility, so the eBPF backend reads a numeral exactly as the bytecode
    /// backend and the VM would under the same dialect — no second, narrower
    /// grammar that quietly disagrees.
    ///
    /// The `bpf` profile is a 9.x dialect, so all four radix prefixes exist,
    /// `_` is a digit separator, and a bare leading zero is *decimal* (9.0
    /// defines `KILL_OCTAL`, retiring octal-by-leading-zero). The old
    /// hand-rolled parser knew only `0x`, `0b` and decimal, so `0o17`, `0d99`
    /// and `1_000` were all rejected as bad integers.
    #[test]
    fn integer_literals_follow_the_target_dialects_number_grammar() {
        let numbers = tcl_registry::model::ingress::static_context_for("bpf")
            .commands()
            .numbers();
        for (text, want) in [
            ("42", Some(42)),
            ("-42", Some(-42)),
            ("+42", Some(42)),
            ("0x1f", Some(31)),
            ("0b1011", Some(11)),
            // 9.x forms the hand-rolled parser could not read.
            ("0o17", Some(15)),
            ("0d99", Some(99)),
            ("1_000", Some(1000)),
            // A leading zero is decimal in 9.0, so this one numeral the old
            // parser did agree on — for the wrong reason.
            ("010", Some(10)),
            // Not integers under any release.
            ("0o8", None),
            ("0x", None),
            ("3.5", None),
            ("nope", None),
            ("99999999999999999999999", None),
        ] {
            assert_eq!(parse_int(text, numbers), want, "parse_int({text:?})");
        }
    }

    /// The grammar is a property of the dialect, not a constant. Compiled for
    /// an older release the same text reads differently — most sharply `010`,
    /// which is octal 8 up to 8.6 and decimal 10 from 9.0. A backend with its
    /// own numeral parser cannot express that difference at all; going through
    /// the shared facility gets it for free.
    #[test]
    fn an_older_target_reads_the_same_numerals_differently() {
        use tcl_dialect::NumberSyntax;
        // Octal-by-leading-zero, alive until 9.0 killed it.
        assert_eq!(parse_int("010", NumberSyntax::Tcl84), Some(8));
        assert_eq!(parse_int("010", NumberSyntax::Tcl85), Some(8));
        assert_eq!(parse_int("010", NumberSyntax::Tcl90), Some(10));
        // `0o`/`0b` arrived in 8.5, `0d` in 9.0, `_` in 9.0: before its
        // release each is trailing junk, so the whole-string parse fails
        // rather than reading a prefix of it.
        assert_eq!(parse_int("0o17", NumberSyntax::Tcl84), None);
        assert_eq!(parse_int("0o17", NumberSyntax::Tcl85), Some(15));
        assert_eq!(parse_int("0d99", NumberSyntax::Tcl85), None);
        assert_eq!(parse_int("1_000", NumberSyntax::Tcl85), None);
    }
}
